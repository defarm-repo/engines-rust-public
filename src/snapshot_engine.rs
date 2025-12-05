//! Snapshot Engine for Git-like State Management
//!
//! This engine creates and manages state snapshots for items and circuits.
//! Each state change creates an immutable snapshot with:
//! - A unique hash (BLAKE3)
//! - Reference to parent snapshot
//! - Full state at that point in time
//! - IPFS CID for distributed storage
//! - Optional blockchain transaction for verification

use crate::ipfs_client::IpfsClient;
use crate::snapshot_types::{
    ChainVerification, CircuitSnapshotSummary, ItemSnapshotSummary, SnapshotEntityType,
    SnapshotError, SnapshotOperation, StateDiff, StateSnapshot,
};
use crate::stellar_client::StellarClient;
use crate::storage::StorageBackend;
use crate::types::{Event, Item, PublicAccessMode};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{info, warn};
use uuid::Uuid;

/// Configuration for the Snapshot Engine
#[derive(Debug, Clone)]
pub struct SnapshotEngineConfig {
    /// Enable IPFS upload for snapshots
    pub ipfs_enabled: bool,

    /// Enable blockchain recording for snapshots
    pub blockchain_enabled: bool,

    /// IPCM contract address on Stellar
    pub ipcm_contract_address: Option<String>,

    /// Network (testnet or mainnet)
    pub stellar_network: String,
}

impl Default for SnapshotEngineConfig {
    fn default() -> Self {
        Self {
            ipfs_enabled: true,
            blockchain_enabled: true,
            ipcm_contract_address: None,
            stellar_network: "testnet".to_string(),
        }
    }
}

/// Data structure for serializing item state to IPFS
#[derive(Debug, Serialize, Deserialize)]
struct ItemStatePayload {
    pub dfid: String,
    pub identifiers: Vec<crate::identifier_types::Identifier>,
    pub enriched_data: HashMap<String, Value>,
    pub events: Vec<EventSummary>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub last_modified: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
struct EventSummary {
    pub event_id: String,
    pub event_type: String,
    pub timestamp: DateTime<Utc>,
    pub source: String,
    pub metadata: HashMap<String, Value>,
}

/// Data structure for serializing circuit state to IPFS
#[derive(Debug, Serialize, Deserialize)]
struct CircuitStatePayload {
    pub circuit_id: String,
    pub name: String,
    pub owner_id: String,
    pub members: Vec<MemberSummary>,
    pub items: Vec<String>, // List of DFIDs
    pub is_public: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MemberSummary {
    pub user_id: String,
    pub role: String,
}

/// The Snapshot Engine manages state snapshots
pub struct SnapshotEngine {
    /// Storage backend for persisting snapshots
    storage: Arc<Mutex<dyn StorageBackend>>,

    /// IPFS client for distributed storage
    ipfs_client: Option<Arc<IpfsClient>>,

    /// Stellar client for blockchain recording
    stellar_client: Option<Arc<StellarClient>>,

    /// Engine configuration
    config: SnapshotEngineConfig,

    /// In-memory cache of recent snapshots (snapshot_id -> StateSnapshot)
    snapshot_cache: Mutex<HashMap<String, StateSnapshot>>,

    /// In-memory index of entity -> snapshots
    entity_index: Mutex<HashMap<String, Vec<String>>>,
}

impl SnapshotEngine {
    /// Create a new SnapshotEngine
    pub fn new(
        storage: Arc<Mutex<dyn StorageBackend>>,
        ipfs_client: Option<Arc<IpfsClient>>,
        stellar_client: Option<Arc<StellarClient>>,
        config: SnapshotEngineConfig,
    ) -> Self {
        Self {
            storage,
            ipfs_client,
            stellar_client,
            config,
            snapshot_cache: Mutex::new(HashMap::new()),
            entity_index: Mutex::new(HashMap::new()),
        }
    }

    /// Create a snapshot for an item state change
    pub async fn create_item_snapshot(
        &self,
        dfid: &str,
        operation: SnapshotOperation,
        user_id: &str,
        message: Option<String>,
    ) -> Result<StateSnapshot, SnapshotError> {
        info!(
            "Creating item snapshot for DFID={}, operation={:?}",
            dfid, operation
        );

        // 1. Get item from storage
        let item = {
            let storage = self.storage.lock().map_err(|e| {
                SnapshotError::StorageError(format!("Failed to lock storage: {}", e))
            })?;
            storage
                .get_item_by_dfid(dfid)
                .map_err(|e| SnapshotError::StorageError(format!("Failed to get item: {}", e)))?
        };

        let item = item.ok_or_else(|| SnapshotError::EntityNotFound {
            entity_type: SnapshotEntityType::Item,
            entity_id: dfid.to_string(),
        })?;

        // 2. Get all events for this item
        let events = {
            let storage = self.storage.lock().map_err(|e| {
                SnapshotError::StorageError(format!("Failed to lock storage: {}", e))
            })?;
            storage
                .get_events_by_dfid(dfid)
                .map_err(|e| SnapshotError::StorageError(format!("Failed to get events: {}", e)))?
        };

        // 3. Get the latest snapshot to find parent
        let (parent_hash, version) = self.get_parent_info(SnapshotEntityType::Item, dfid).await?;

        // 4. Build state payload
        let state_payload = self.build_item_state_payload(&item, &events);
        let state = serde_json::to_value(&state_payload)?;

        // 5. Create snapshot
        let mut snapshot = StateSnapshot::new(
            SnapshotEntityType::Item,
            dfid.to_string(),
            version,
            parent_hash,
            state,
            operation,
            user_id.to_string(),
        );

        if let Some(msg) = message {
            snapshot = snapshot.with_message(msg);
        }

        // 6. Compute hash
        snapshot = snapshot.with_computed_hash();

        // 7. Upload to IPFS if enabled
        if self.config.ipfs_enabled {
            if let Some(ref ipfs) = self.ipfs_client {
                match ipfs.upload_json(&snapshot).await {
                    Ok(cid) => {
                        info!("Snapshot uploaded to IPFS: CID={}", cid);
                        snapshot = snapshot.with_ipfs_cid(cid);
                    }
                    Err(e) => {
                        warn!("Failed to upload snapshot to IPFS: {}", e);
                        // Continue without IPFS - don't fail the whole operation
                    }
                }
            }
        }

        // 8. Record on blockchain if enabled
        if self.config.blockchain_enabled {
            if let (Some(ref stellar), Some(ref cid)) = (&self.stellar_client, &snapshot.ipfs_cid) {
                match self
                    .record_snapshot_on_blockchain(stellar, dfid, &snapshot.snapshot_id, cid)
                    .await
                {
                    Ok(tx_id) => {
                        info!("Snapshot recorded on blockchain: TX={}", tx_id);
                        snapshot = snapshot.with_blockchain_tx(tx_id);
                    }
                    Err(e) => {
                        warn!("Failed to record snapshot on blockchain: {}", e);
                        // Continue without blockchain - don't fail the whole operation
                    }
                }
            }
        }

        // 9. Store snapshot
        self.store_snapshot(&snapshot)?;

        info!(
            "Item snapshot created: id={}, version={}, ipfs={:?}",
            snapshot.snapshot_id, snapshot.version, snapshot.ipfs_cid
        );

        Ok(snapshot)
    }

    /// Create a snapshot for a circuit state change
    pub async fn create_circuit_snapshot(
        &self,
        circuit_id: &Uuid,
        operation: SnapshotOperation,
        user_id: &str,
        message: Option<String>,
    ) -> Result<StateSnapshot, SnapshotError> {
        info!(
            "Creating circuit snapshot for circuit_id={}, operation={:?}",
            circuit_id, operation
        );

        let circuit_id_str = circuit_id.to_string();

        // 1. Get circuit from storage
        let circuit = {
            let storage = self.storage.lock().map_err(|e| {
                SnapshotError::StorageError(format!("Failed to lock storage: {}", e))
            })?;
            storage
                .get_circuit(circuit_id)
                .map_err(|e| SnapshotError::StorageError(format!("Failed to get circuit: {}", e)))?
        };

        let circuit = circuit.ok_or_else(|| SnapshotError::EntityNotFound {
            entity_type: SnapshotEntityType::Circuit,
            entity_id: circuit_id_str.clone(),
        })?;

        // 2. Get parent info
        let (parent_hash, version) = self
            .get_parent_info(SnapshotEntityType::Circuit, &circuit_id_str)
            .await?;

        // 3. Build state payload
        // Get published items from public settings
        let items = circuit
            .public_settings
            .as_ref()
            .map(|ps| ps.published_items.clone())
            .unwrap_or_default();

        let state_payload = CircuitStatePayload {
            circuit_id: circuit_id_str.clone(),
            name: circuit.name.clone(),
            owner_id: circuit.owner_id.clone(),
            members: circuit
                .members
                .iter()
                .map(|member| MemberSummary {
                    user_id: member.member_id.clone(),
                    role: format!("{:?}", member.role),
                })
                .collect(),
            items,
            is_public: circuit
                .public_settings
                .as_ref()
                .map(|ps| {
                    matches!(
                        ps.access_mode,
                        PublicAccessMode::Public | PublicAccessMode::Protected
                    )
                })
                .unwrap_or(false),
            created_at: circuit.created_timestamp,
        };
        let state = serde_json::to_value(&state_payload)?;

        // 4. Create snapshot
        let mut snapshot = StateSnapshot::new(
            SnapshotEntityType::Circuit,
            circuit_id_str.clone(),
            version,
            parent_hash,
            state,
            operation,
            user_id.to_string(),
        );

        if let Some(msg) = message {
            snapshot = snapshot.with_message(msg);
        }

        // 5. Compute hash
        snapshot = snapshot.with_computed_hash();

        // 6. Upload to IPFS if enabled
        if self.config.ipfs_enabled {
            if let Some(ref ipfs) = self.ipfs_client {
                match ipfs.upload_json(&snapshot).await {
                    Ok(cid) => {
                        info!("Circuit snapshot uploaded to IPFS: CID={}", cid);
                        snapshot = snapshot.with_ipfs_cid(cid);
                    }
                    Err(e) => {
                        warn!("Failed to upload circuit snapshot to IPFS: {}", e);
                    }
                }
            }
        }

        // 7. Record on blockchain if enabled
        if self.config.blockchain_enabled {
            if let (Some(ref stellar), Some(ref cid)) = (&self.stellar_client, &snapshot.ipfs_cid) {
                match self
                    .record_snapshot_on_blockchain(
                        stellar,
                        &circuit_id_str,
                        &snapshot.snapshot_id,
                        cid,
                    )
                    .await
                {
                    Ok(tx_id) => {
                        info!("Circuit snapshot recorded on blockchain: TX={}", tx_id);
                        snapshot = snapshot.with_blockchain_tx(tx_id);
                    }
                    Err(e) => {
                        warn!("Failed to record circuit snapshot on blockchain: {}", e);
                    }
                }
            }
        }

        // 8. Store snapshot
        self.store_snapshot(&snapshot)?;

        info!(
            "Circuit snapshot created: id={}, version={}, ipfs={:?}",
            snapshot.snapshot_id, snapshot.version, snapshot.ipfs_cid
        );

        Ok(snapshot)
    }

    /// Get the snapshot chain for an entity
    pub fn get_snapshot_chain(
        &self,
        entity_type: SnapshotEntityType,
        entity_id: &str,
    ) -> Result<Vec<StateSnapshot>, SnapshotError> {
        let key = format!("{}:{}", entity_type, entity_id);

        let index = self
            .entity_index
            .lock()
            .map_err(|e| SnapshotError::StorageError(format!("Failed to lock index: {}", e)))?;

        let snapshot_ids = index.get(&key).cloned().unwrap_or_default();

        let cache = self
            .snapshot_cache
            .lock()
            .map_err(|e| SnapshotError::StorageError(format!("Failed to lock cache: {}", e)))?;

        let mut snapshots: Vec<StateSnapshot> = snapshot_ids
            .iter()
            .filter_map(|id| cache.get(id).cloned())
            .collect();

        // Sort by version
        snapshots.sort_by_key(|s| s.version);

        Ok(snapshots)
    }

    /// Get a specific snapshot by ID
    pub fn get_snapshot(&self, snapshot_id: &str) -> Result<Option<StateSnapshot>, SnapshotError> {
        let cache = self
            .snapshot_cache
            .lock()
            .map_err(|e| SnapshotError::StorageError(format!("Failed to lock cache: {}", e)))?;

        Ok(cache.get(snapshot_id).cloned())
    }

    /// Get the latest snapshot for an entity
    pub fn get_latest_snapshot(
        &self,
        entity_type: SnapshotEntityType,
        entity_id: &str,
    ) -> Result<Option<StateSnapshot>, SnapshotError> {
        let snapshots = self.get_snapshot_chain(entity_type, entity_id)?;
        Ok(snapshots.into_iter().last())
    }

    /// Verify the integrity of a snapshot chain
    pub fn verify_chain(
        &self,
        entity_type: SnapshotEntityType,
        entity_id: &str,
    ) -> Result<ChainVerification, SnapshotError> {
        let snapshots = self.get_snapshot_chain(entity_type, entity_id)?;

        if snapshots.is_empty() {
            return Ok(ChainVerification::failure(0, 0, vec![], vec![]));
        }

        let mut verified = 0;
        let mut broken_links = Vec::new();
        let mut tampered_hashes = Vec::new();

        for (i, snapshot) in snapshots.iter().enumerate() {
            // Verify hash
            let computed_hash = snapshot.compute_hash();
            if computed_hash != snapshot.snapshot_id {
                tampered_hashes.push(snapshot.snapshot_id.clone());
                continue;
            }

            // Verify parent link
            if i > 0 {
                let expected_parent = &snapshots[i - 1].snapshot_id;
                if snapshot.parent_hash.as_ref() != Some(expected_parent) {
                    broken_links.push(snapshot.snapshot_id.clone());
                    continue;
                }
            } else if snapshot.parent_hash.is_some() {
                // First snapshot should have no parent
                broken_links.push(snapshot.snapshot_id.clone());
                continue;
            }

            verified += 1;
        }

        if broken_links.is_empty() && tampered_hashes.is_empty() {
            let start = snapshots
                .first()
                .map(|s| s.timestamp)
                .unwrap_or_else(Utc::now);
            let end = snapshots
                .last()
                .map(|s| s.timestamp)
                .unwrap_or_else(Utc::now);
            Ok(ChainVerification::success(snapshots.len(), start, end))
        } else {
            Ok(ChainVerification::failure(
                snapshots.len(),
                verified,
                broken_links,
                tampered_hashes,
            ))
        }
    }

    /// Compute diff between two snapshots
    pub fn compute_diff(
        &self,
        from_snapshot_id: &str,
        to_snapshot_id: &str,
    ) -> Result<StateDiff, SnapshotError> {
        let from = self
            .get_snapshot(from_snapshot_id)?
            .ok_or_else(|| SnapshotError::NotFound(from_snapshot_id.to_string()))?;

        let to = self
            .get_snapshot(to_snapshot_id)?
            .ok_or_else(|| SnapshotError::NotFound(to_snapshot_id.to_string()))?;

        // Basic diff - compare state objects
        let changes = self.diff_json_values(&from.state, &to.state, "");

        Ok(StateDiff {
            from_snapshot: from_snapshot_id.to_string(),
            to_snapshot: to_snapshot_id.to_string(),
            changes,
            added_events: vec![], // Implementation pending
            version_from: from.version,
            version_to: to.version,
        })
    }

    /// Get summary for an item's snapshots
    pub fn get_item_snapshot_summary(
        &self,
        dfid: &str,
    ) -> Result<ItemSnapshotSummary, SnapshotError> {
        let snapshots = self.get_snapshot_chain(SnapshotEntityType::Item, dfid)?;

        Ok(ItemSnapshotSummary {
            dfid: dfid.to_string(),
            total_snapshots: snapshots.len() as u64,
            latest_version: snapshots.last().map(|s| s.version).unwrap_or(0),
            latest_snapshot_id: snapshots.last().map(|s| s.snapshot_id.clone()),
            latest_ipfs_cid: snapshots.last().and_then(|s| s.ipfs_cid.clone()),
            first_snapshot_at: snapshots.first().map(|s| s.timestamp),
            last_snapshot_at: snapshots.last().map(|s| s.timestamp),
        })
    }

    /// Get summary for a circuit's snapshots
    pub fn get_circuit_snapshot_summary(
        &self,
        circuit_id: &Uuid,
    ) -> Result<CircuitSnapshotSummary, SnapshotError> {
        let circuit_id_str = circuit_id.to_string();
        let snapshots = self.get_snapshot_chain(SnapshotEntityType::Circuit, &circuit_id_str)?;

        // Get item count from latest state
        let item_count = snapshots
            .last()
            .and_then(|s| s.state.get("items"))
            .and_then(|i| i.as_array())
            .map(|arr| arr.len())
            .unwrap_or(0);

        Ok(CircuitSnapshotSummary {
            circuit_id: *circuit_id,
            total_snapshots: snapshots.len() as u64,
            latest_version: snapshots.last().map(|s| s.version).unwrap_or(0),
            latest_snapshot_id: snapshots.last().map(|s| s.snapshot_id.clone()),
            latest_ipfs_cid: snapshots.last().and_then(|s| s.ipfs_cid.clone()),
            first_snapshot_at: snapshots.first().map(|s| s.timestamp),
            last_snapshot_at: snapshots.last().map(|s| s.timestamp),
            item_count,
        })
    }

    // Private helper methods

    async fn get_parent_info(
        &self,
        entity_type: SnapshotEntityType,
        entity_id: &str,
    ) -> Result<(Option<String>, u64), SnapshotError> {
        let latest = self.get_latest_snapshot(entity_type, entity_id)?;

        match latest {
            Some(s) => Ok((Some(s.snapshot_id), s.version + 1)),
            None => Ok((None, 1)),
        }
    }

    fn build_item_state_payload(&self, item: &Item, events: &[Event]) -> ItemStatePayload {
        ItemStatePayload {
            dfid: item.dfid.clone(),
            identifiers: item.identifiers.clone(),
            enriched_data: item.enriched_data.clone(),
            events: events
                .iter()
                .map(|e| EventSummary {
                    event_id: e.event_id.to_string(),
                    event_type: format!("{:?}", e.event_type),
                    timestamp: e.timestamp,
                    source: e.source.clone(),
                    metadata: e.metadata.clone(),
                })
                .collect(),
            status: format!("{:?}", item.status),
            created_at: item.creation_timestamp,
            last_modified: item.last_modified,
        }
    }

    fn store_snapshot(&self, snapshot: &StateSnapshot) -> Result<(), SnapshotError> {
        // Store in cache
        {
            let mut cache = self
                .snapshot_cache
                .lock()
                .map_err(|e| SnapshotError::StorageError(format!("Failed to lock cache: {}", e)))?;
            cache.insert(snapshot.snapshot_id.clone(), snapshot.clone());
        }

        // Update index
        {
            let key = format!("{}:{}", snapshot.entity_type, snapshot.entity_id);
            let mut index = self
                .entity_index
                .lock()
                .map_err(|e| SnapshotError::StorageError(format!("Failed to lock index: {}", e)))?;
            index
                .entry(key)
                .or_insert_with(Vec::new)
                .push(snapshot.snapshot_id.clone());
        }

        // Implementation pending
        // This would be done via the storage backend trait

        Ok(())
    }

    async fn record_snapshot_on_blockchain(
        &self,
        _stellar: &StellarClient,
        entity_id: &str,
        snapshot_id: &str,
        ipfs_cid: &str,
    ) -> Result<String, SnapshotError> {
        // For now, we'll use the existing IPCM update mechanism
        // In the future, we could add a dedicated snapshot recording function

        // The IPCM contract stores: dfid -> cid mapping
        // We can use the same mechanism for snapshots

        // Placeholder - actual implementation would call stellar_client.update_ipcm()
        info!(
            "Would record on blockchain: entity={}, snapshot={}, cid={}",
            entity_id, snapshot_id, ipfs_cid
        );

        // For now, return a placeholder TX ID
        // Implementation pending
        Ok(format!("pending_tx_{}", &snapshot_id[..16]))
    }

    fn diff_json_values(
        &self,
        from: &Value,
        to: &Value,
        path: &str,
    ) -> Vec<crate::snapshot_types::StateChange> {
        use crate::snapshot_types::{ChangeType, StateChange};

        let mut changes = Vec::new();

        match (from, to) {
            (Value::Object(from_obj), Value::Object(to_obj)) => {
                // Check for added/modified keys in 'to'
                for (key, to_val) in to_obj {
                    let new_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", path, key)
                    };

                    if let Some(from_val) = from_obj.get(key) {
                        if from_val != to_val {
                            // Recurse or mark as modified
                            if from_val.is_object() && to_val.is_object() {
                                changes.extend(self.diff_json_values(from_val, to_val, &new_path));
                            } else {
                                changes.push(StateChange {
                                    path: new_path,
                                    old_value: Some(from_val.clone()),
                                    new_value: Some(to_val.clone()),
                                    change_type: ChangeType::Modified,
                                });
                            }
                        }
                    } else {
                        changes.push(StateChange {
                            path: new_path,
                            old_value: None,
                            new_value: Some(to_val.clone()),
                            change_type: ChangeType::Added,
                        });
                    }
                }

                // Check for removed keys
                for (key, from_val) in from_obj {
                    if !to_obj.contains_key(key) {
                        let new_path = if path.is_empty() {
                            key.clone()
                        } else {
                            format!("{}.{}", path, key)
                        };
                        changes.push(StateChange {
                            path: new_path,
                            old_value: Some(from_val.clone()),
                            new_value: None,
                            change_type: ChangeType::Removed,
                        });
                    }
                }
            }
            _ => {
                if from != to {
                    changes.push(StateChange {
                        path: path.to_string(),
                        old_value: Some(from.clone()),
                        new_value: Some(to.clone()),
                        change_type: ChangeType::Modified,
                    });
                }
            }
        }

        changes
    }
}

// Implement Debug manually since we have mutexes
impl std::fmt::Debug for SnapshotEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SnapshotEngine")
            .field("config", &self.config)
            .field("ipfs_enabled", &self.ipfs_client.is_some())
            .field("stellar_enabled", &self.stellar_client.is_some())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_engine_config_default() {
        let config = SnapshotEngineConfig::default();
        assert!(config.ipfs_enabled);
        assert!(config.blockchain_enabled);
        assert_eq!(config.stellar_network, "testnet");
    }
}

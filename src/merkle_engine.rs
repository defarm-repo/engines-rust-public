//! Merkle Engine - Builds and manages Merkle trees from items and events
//!
//! This engine provides higher-level functions to:
//! - Hash events for Merkle leaf nodes
//! - Build item Merkle trees from events
//! - Build circuit Merkle trees from items
//! - Generate proofs for sync verification
//! - Compare states between users

use crate::merkle_tree::{
    ItemMerkleEntry, MerkleEntityType, MerkleError, MerkleProof, MerkleRootSummary, MerkleTree,
};
use crate::storage::StorageBackend;
use crate::types::Event;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Engine for building and managing Merkle trees
pub struct MerkleEngine<S: StorageBackend> {
    storage: S,
}

/// Result of comparing two circuit states
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncComparison {
    /// Whether the states are in sync
    pub in_sync: bool,
    /// Local circuit root hash
    pub local_root: String,
    /// Remote circuit root hash
    pub remote_root: String,
    /// DFIDs that differ between local and remote
    pub differing_items: Vec<String>,
    /// Items only in local
    pub local_only: Vec<String>,
    /// Items only in remote
    pub remote_only: Vec<String>,
    /// Items that exist in both but have different roots
    pub modified: Vec<String>,
}

/// Response for item Merkle root
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemMerkleRootResponse {
    pub dfid: String,
    pub merkle_root: String,
    pub event_count: usize,
    pub computed_at: DateTime<Utc>,
}

/// Response for circuit Merkle root
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitMerkleRootResponse {
    pub circuit_id: String,
    pub merkle_root: String,
    pub item_count: usize,
    pub items: Vec<ItemMerkleEntry>,
    pub computed_at: DateTime<Utc>,
}

/// Compute event hash for Merkle leaf node
///
/// Hash formula: BLAKE3(event_id|event_type|timestamp_nanos|source|metadata_json)
pub fn hash_event(event: &Event) -> String {
    let data = format!(
        "{}|{:?}|{}|{}|{}",
        event.event_id,
        event.event_type,
        event.timestamp.timestamp_nanos_opt().unwrap_or(0),
        event.source,
        serde_json::to_string(&event.metadata).unwrap_or_default()
    );
    blake3::hash(data.as_bytes()).to_hex().to_string()
}

impl<S: StorageBackend + Clone + 'static> MerkleEngine<S> {
    /// Create a new MerkleEngine with the given storage backend
    pub fn new(storage: S) -> Self {
        MerkleEngine { storage }
    }

    /// Build a Merkle tree from an item's events
    pub fn build_item_tree(&self, dfid: &str) -> Result<MerkleTree, MerkleError> {
        let events = self.storage.get_events_by_dfid(dfid).map_err(|e| {
            MerkleError::StorageError(format!("Failed to get events for {}: {}", dfid, e))
        })?;

        if events.is_empty() {
            return Err(MerkleError::EmptyTree);
        }

        // Hash each event and pair with event_id for lookup
        let leaf_data: Vec<(String, Option<String>)> = events
            .iter()
            .map(|e| (hash_event(e), Some(e.event_id.to_string())))
            .collect();

        Ok(MerkleTree::from_leaves_with_ids(leaf_data))
    }

    /// Get an item's Merkle root
    pub fn get_item_root(&self, dfid: &str) -> Result<ItemMerkleRootResponse, MerkleError> {
        let tree = self.build_item_tree(dfid)?;
        let root = tree.root().ok_or(MerkleError::EmptyTree)?.to_string();

        Ok(ItemMerkleRootResponse {
            dfid: dfid.to_string(),
            merkle_root: root,
            event_count: tree.leaf_count(),
            computed_at: Utc::now(),
        })
    }

    /// Build a Merkle tree from a circuit's items
    pub fn build_circuit_tree(
        &self,
        circuit_id: &Uuid,
    ) -> Result<(MerkleTree, Vec<ItemMerkleEntry>), MerkleError> {
        // Get all items in the circuit
        let items = self.storage.get_circuit_items(circuit_id).map_err(|e| {
            MerkleError::StorageError(format!(
                "Failed to get circuit items for {}: {}",
                circuit_id, e
            ))
        })?;

        if items.is_empty() {
            return Err(MerkleError::EmptyTree);
        }

        // Build item roots
        let mut item_entries: Vec<ItemMerkleEntry> = Vec::new();
        let mut leaf_data: Vec<(String, Option<String>)> = Vec::new();

        for item in items {
            match self.build_item_tree(&item.dfid) {
                Ok(item_tree) => {
                    if let Some(root) = item_tree.root() {
                        item_entries.push(ItemMerkleEntry {
                            dfid: item.dfid.clone(),
                            merkle_root: root.to_string(),
                            event_count: item_tree.leaf_count(),
                        });
                        leaf_data.push((root.to_string(), Some(item.dfid.clone())));
                    }
                }
                Err(MerkleError::EmptyTree) => {
                    // Item has no events, skip it
                    continue;
                }
                Err(e) => return Err(e),
            }
        }

        if leaf_data.is_empty() {
            return Err(MerkleError::EmptyTree);
        }

        Ok((MerkleTree::from_leaves_with_ids(leaf_data), item_entries))
    }

    /// Get a circuit's Merkle root
    pub fn get_circuit_root(
        &self,
        circuit_id: &Uuid,
    ) -> Result<CircuitMerkleRootResponse, MerkleError> {
        let (tree, items) = self.build_circuit_tree(circuit_id)?;
        let root = tree.root().ok_or(MerkleError::EmptyTree)?.to_string();

        Ok(CircuitMerkleRootResponse {
            circuit_id: circuit_id.to_string(),
            merkle_root: root,
            item_count: items.len(),
            items,
            computed_at: Utc::now(),
        })
    }

    /// Generate proof that an event exists in an item
    pub fn prove_event_in_item(
        &self,
        dfid: &str,
        event_id: &Uuid,
    ) -> Result<MerkleProof, MerkleError> {
        // Get all events for the item - ONCE
        let events = self.storage.get_events_by_dfid(dfid).map_err(|e| {
            MerkleError::StorageError(format!("Failed to get events for {}: {}", dfid, e))
        })?;

        if events.is_empty() {
            return Err(MerkleError::EmptyTree);
        }

        // Find the target event
        let target_event = events
            .iter()
            .find(|e| e.event_id == *event_id)
            .ok_or_else(|| {
                MerkleError::Other(format!("Event {} not found in item {}", event_id, dfid))
            })?;

        // Build tree from SAME events (not re-fetching)
        let leaf_data: Vec<(String, Option<String>)> = events
            .iter()
            .map(|e| (hash_event(e), Some(e.event_id.to_string())))
            .collect();
        let tree = MerkleTree::from_leaves_with_ids(leaf_data);

        // Find and generate proof for the event hash
        let event_hash = hash_event(target_event);
        tree.generate_proof_by_hash(&event_hash)
    }

    /// Generate proof that an item exists in a circuit
    pub fn prove_item_in_circuit(
        &self,
        circuit_id: &Uuid,
        dfid: &str,
    ) -> Result<MerkleProof, MerkleError> {
        // Build the circuit tree
        let (tree, items) = self.build_circuit_tree(circuit_id)?;

        // Find the item's merkle root
        let item_entry = items.iter().find(|e| e.dfid == dfid).ok_or_else(|| {
            MerkleError::Other(format!("Item {} not found in circuit {}", dfid, circuit_id))
        })?;

        // Generate proof for the item root
        tree.generate_proof_by_hash(&item_entry.merkle_root)
    }

    /// Compare two circuit states
    pub fn compare_circuit_states(
        &self,
        circuit_id: &Uuid,
        local_root: &str,
        remote_root: &str,
        remote_items: Option<&[ItemMerkleEntry]>,
    ) -> Result<SyncComparison, MerkleError> {
        // Get local items
        let (_, local_items) = self.build_circuit_tree(circuit_id)?;

        // Quick check: if roots match, we're in sync
        if local_root == remote_root {
            return Ok(SyncComparison {
                in_sync: true,
                local_root: local_root.to_string(),
                remote_root: remote_root.to_string(),
                differing_items: vec![],
                local_only: vec![],
                remote_only: vec![],
                modified: vec![],
            });
        }

        // If we have remote items, find differences
        let (local_only, remote_only, modified) = if let Some(remote) = remote_items {
            self.find_item_differences(&local_items, remote)
        } else {
            // Without remote items, we can only say roots differ
            (vec![], vec![], vec![])
        };

        let differing_items: Vec<String> = local_only
            .iter()
            .chain(remote_only.iter())
            .chain(modified.iter())
            .cloned()
            .collect();

        Ok(SyncComparison {
            in_sync: false,
            local_root: local_root.to_string(),
            remote_root: remote_root.to_string(),
            differing_items,
            local_only,
            remote_only,
            modified,
        })
    }

    /// Find differences between local and remote items
    fn find_item_differences(
        &self,
        local_items: &[ItemMerkleEntry],
        remote_items: &[ItemMerkleEntry],
    ) -> (Vec<String>, Vec<String>, Vec<String>) {
        use std::collections::HashMap;

        let local_map: HashMap<&str, &str> = local_items
            .iter()
            .map(|e| (e.dfid.as_str(), e.merkle_root.as_str()))
            .collect();

        let remote_map: HashMap<&str, &str> = remote_items
            .iter()
            .map(|e| (e.dfid.as_str(), e.merkle_root.as_str()))
            .collect();

        let mut local_only = Vec::new();
        let mut remote_only = Vec::new();
        let mut modified = Vec::new();

        // Find local-only and modified
        for (dfid, local_root) in &local_map {
            match remote_map.get(dfid) {
                Some(remote_root) => {
                    if local_root != remote_root {
                        modified.push(dfid.to_string());
                    }
                }
                None => {
                    local_only.push(dfid.to_string());
                }
            }
        }

        // Find remote-only
        for dfid in remote_map.keys() {
            if !local_map.contains_key(dfid) {
                remote_only.push(dfid.to_string());
            }
        }

        (local_only, remote_only, modified)
    }

    /// Create a root summary for an entity
    pub fn create_item_summary(&self, dfid: &str) -> Result<MerkleRootSummary, MerkleError> {
        let tree = self.build_item_tree(dfid)?;
        tree.create_summary(MerkleEntityType::Item, dfid.to_string())
            .ok_or(MerkleError::EmptyTree)
    }

    /// Create a root summary for a circuit
    pub fn create_circuit_summary(
        &self,
        circuit_id: &Uuid,
    ) -> Result<MerkleRootSummary, MerkleError> {
        let (tree, _) = self.build_circuit_tree(circuit_id)?;
        tree.create_summary(MerkleEntityType::Circuit, circuit_id.to_string())
            .ok_or(MerkleError::EmptyTree)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::InMemoryStorage;
    use crate::types::{EventType, EventVisibility, Item, ItemStatus};
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    fn create_test_storage() -> Arc<Mutex<InMemoryStorage>> {
        Arc::new(Mutex::new(InMemoryStorage::new()))
    }

    fn create_test_item(dfid: &str) -> Item {
        Item {
            dfid: dfid.to_string(),
            local_id: None,
            legacy_mode: false,
            identifiers: vec![],
            aliases: vec![],
            fingerprint: None,
            enriched_data: HashMap::new(),
            creation_timestamp: Utc::now(),
            last_modified: Utc::now(),
            source_entries: vec![],
            confidence_score: 1.0,
            status: ItemStatus::Active,
        }
    }

    fn create_test_event(dfid: &str, event_type: EventType) -> Event {
        let mut metadata = HashMap::new();
        metadata.insert("test".to_string(), serde_json::json!("data"));
        Event::new_with_metadata(
            dfid.to_string(),
            event_type,
            "test_source".to_string(),
            EventVisibility::Public,
            metadata,
        )
    }

    #[test]
    fn test_hash_event_deterministic() {
        let event1 = create_test_event("DFID-TEST-001", EventType::Created);

        // Same event should produce same hash
        let hash1 = hash_event(&event1);
        let hash2 = hash_event(&event1);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_event_different_events() {
        let event1 = create_test_event("DFID-TEST-001", EventType::Created);
        let event2 = create_test_event("DFID-TEST-002", EventType::Created);

        // Different events should produce different hashes
        let hash1 = hash_event(&event1);
        let hash2 = hash_event(&event2);
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_build_item_tree_empty() {
        let storage = create_test_storage();
        let engine = MerkleEngine::new(storage);

        // No events for this dfid
        let result = engine.build_item_tree("DFID-NONEXISTENT");
        assert!(matches!(result, Err(MerkleError::EmptyTree)));
    }

    #[test]
    fn test_build_item_tree_with_events() {
        let storage = create_test_storage();

        // Add an item and events
        {
            let mut s = storage.lock().unwrap();
            let item = create_test_item("DFID-TEST-001");
            s.store_item(&item).unwrap();

            let event1 = create_test_event("DFID-TEST-001", EventType::Created);
            s.store_event(&event1).unwrap();

            let event2 = create_test_event("DFID-TEST-001", EventType::Enriched);
            s.store_event(&event2).unwrap();
        }

        let engine = MerkleEngine::new(storage);
        let tree = engine.build_item_tree("DFID-TEST-001").unwrap();

        assert_eq!(tree.leaf_count(), 2);
        assert!(tree.root().is_some());
    }

    #[test]
    fn test_get_item_root() {
        let storage = create_test_storage();

        // Add an item and events
        {
            let mut s = storage.lock().unwrap();
            let item = create_test_item("DFID-TEST-001");
            s.store_item(&item).unwrap();

            let event = create_test_event("DFID-TEST-001", EventType::Created);
            s.store_event(&event).unwrap();
        }

        let engine = MerkleEngine::new(storage);
        let response = engine.get_item_root("DFID-TEST-001").unwrap();

        assert_eq!(response.dfid, "DFID-TEST-001");
        assert_eq!(response.event_count, 1);
        assert!(!response.merkle_root.is_empty());
    }

    #[test]
    fn test_find_item_differences() {
        let storage = create_test_storage();
        let engine = MerkleEngine::new(storage);

        let local = vec![
            ItemMerkleEntry {
                dfid: "DFID-1".to_string(),
                merkle_root: "root1".to_string(),
                event_count: 1,
            },
            ItemMerkleEntry {
                dfid: "DFID-2".to_string(),
                merkle_root: "root2".to_string(),
                event_count: 2,
            },
            ItemMerkleEntry {
                dfid: "DFID-3".to_string(),
                merkle_root: "root3_local".to_string(),
                event_count: 3,
            },
        ];

        let remote = vec![
            ItemMerkleEntry {
                dfid: "DFID-1".to_string(),
                merkle_root: "root1".to_string(),
                event_count: 1,
            },
            ItemMerkleEntry {
                dfid: "DFID-3".to_string(),
                merkle_root: "root3_remote".to_string(),
                event_count: 4,
            },
            ItemMerkleEntry {
                dfid: "DFID-4".to_string(),
                merkle_root: "root4".to_string(),
                event_count: 1,
            },
        ];

        let (local_only, remote_only, modified) = engine.find_item_differences(&local, &remote);

        assert_eq!(local_only, vec!["DFID-2"]);
        assert_eq!(remote_only, vec!["DFID-4"]);
        assert_eq!(modified, vec!["DFID-3"]);
    }
}

//! State Snapshot Types for Git-like State Management
//!
//! This module defines the types for tracking state changes with immutable snapshots.
//! Each state change (item or circuit) creates a new snapshot with a reference to the
//! previous snapshot, forming a hash chain that can be verified and audited.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Entity type for snapshots - either Item or Circuit
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SnapshotEntityType {
    Item,
    Circuit,
}

impl std::fmt::Display for SnapshotEntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SnapshotEntityType::Item => write!(f, "item"),
            SnapshotEntityType::Circuit => write!(f, "circuit"),
        }
    }
}

/// Operation that triggered the snapshot creation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum SnapshotOperation {
    // Item operations
    ItemCreated,
    ItemEnriched {
        fields: Vec<String>,
    },
    ItemIdentifiersAdded {
        count: usize,
    },
    ItemEventAdded {
        event_id: String,
        event_type: String,
        event_category: Option<String>,
    },
    ItemMerged {
        with_dfid: String,
    },
    ItemSplit {
        new_dfid: String,
    },
    ItemDeprecated,
    ItemPushedToCircuit {
        circuit_id: String,
    },
    ItemPulledFromCircuit {
        circuit_id: String,
    },

    // Circuit operations
    CircuitCreated,
    CircuitMemberAdded {
        member_id: String,
        role: String,
    },
    CircuitMemberRemoved {
        member_id: String,
    },
    CircuitPermissionChanged {
        permission: String,
        old_value: Option<serde_json::Value>,
        new_value: serde_json::Value,
    },
    CircuitItemPushed {
        dfid: String,
        item_snapshot_id: Option<String>,
    },
    CircuitItemPulled {
        dfid: String,
    },
    CircuitVisibilityChanged {
        is_public: bool,
        access_mode: Option<String>,
    },
    CircuitAdapterConfigured {
        adapter_type: String,
    },
}

impl SnapshotOperation {
    /// Get a human-readable description of the operation
    pub fn description(&self) -> String {
        match self {
            SnapshotOperation::ItemCreated => "Item created".to_string(),
            SnapshotOperation::ItemEnriched { fields } => {
                format!("Item enriched with {} field(s)", fields.len())
            }
            SnapshotOperation::ItemIdentifiersAdded { count } => {
                format!("{} identifier(s) added", count)
            }
            SnapshotOperation::ItemEventAdded {
                event_type,
                event_category,
                ..
            } => {
                if let Some(cat) = event_category {
                    format!("Event added: {} ({})", event_type, cat)
                } else {
                    format!("Event added: {}", event_type)
                }
            }
            SnapshotOperation::ItemMerged { with_dfid } => {
                format!("Item merged with {}", with_dfid)
            }
            SnapshotOperation::ItemSplit { new_dfid } => {
                format!("Item split, created {}", new_dfid)
            }
            SnapshotOperation::ItemDeprecated => "Item deprecated".to_string(),
            SnapshotOperation::ItemPushedToCircuit { circuit_id } => {
                format!("Pushed to circuit {}", circuit_id)
            }
            SnapshotOperation::ItemPulledFromCircuit { circuit_id } => {
                format!("Pulled from circuit {}", circuit_id)
            }
            SnapshotOperation::CircuitCreated => "Circuit created".to_string(),
            SnapshotOperation::CircuitMemberAdded { member_id, role } => {
                format!("Member {} added as {}", member_id, role)
            }
            SnapshotOperation::CircuitMemberRemoved { member_id } => {
                format!("Member {} removed", member_id)
            }
            SnapshotOperation::CircuitPermissionChanged { permission, .. } => {
                format!("Permission {} changed", permission)
            }
            SnapshotOperation::CircuitItemPushed { dfid, .. } => {
                format!("Item {} pushed", dfid)
            }
            SnapshotOperation::CircuitItemPulled { dfid } => {
                format!("Item {} pulled", dfid)
            }
            SnapshotOperation::CircuitVisibilityChanged { is_public, .. } => {
                if *is_public {
                    "Made public".to_string()
                } else {
                    "Made private".to_string()
                }
            }
            SnapshotOperation::CircuitAdapterConfigured { adapter_type } => {
                format!("Adapter configured: {}", adapter_type)
            }
        }
    }
}

/// A state snapshot representing the complete state at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    /// Unique snapshot ID (BLAKE3 hash of content)
    pub snapshot_id: String,

    /// Type of entity (Item or Circuit)
    pub entity_type: SnapshotEntityType,

    /// Entity identifier (DFID for items, UUID for circuits)
    pub entity_id: String,

    /// Sequential version number (1, 2, 3, ...)
    pub version: u64,

    /// Hash of the previous snapshot (None for first snapshot)
    pub parent_hash: Option<String>,

    /// Complete state at this point in time
    pub state: serde_json::Value,

    /// Diff from parent snapshot (optional, for compression)
    pub state_delta: Option<serde_json::Value>,

    /// Operation that triggered this snapshot
    pub operation: SnapshotOperation,

    /// Additional operation metadata
    pub operation_metadata: HashMap<String, serde_json::Value>,

    /// IPFS CID where snapshot is stored
    pub ipfs_cid: Option<String>,

    /// Blockchain transaction ID (Stellar)
    pub blockchain_tx: Option<String>,

    /// Timestamp of snapshot creation
    pub timestamp: DateTime<Utc>,

    /// User who triggered the state change
    pub created_by: String,

    /// Optional commit message
    pub message: Option<String>,
}

impl StateSnapshot {
    /// Create a new snapshot with required fields
    pub fn new(
        entity_type: SnapshotEntityType,
        entity_id: String,
        version: u64,
        parent_hash: Option<String>,
        state: serde_json::Value,
        operation: SnapshotOperation,
        created_by: String,
    ) -> Self {
        Self {
            snapshot_id: String::new(), // Will be computed
            entity_type,
            entity_id,
            version,
            parent_hash,
            state,
            state_delta: None,
            operation,
            operation_metadata: HashMap::new(),
            ipfs_cid: None,
            blockchain_tx: None,
            timestamp: Utc::now(),
            created_by,
            message: None,
        }
    }

    /// Compute the snapshot ID using BLAKE3
    pub fn compute_hash(&self) -> String {
        use blake3::Hasher;

        let mut hasher = Hasher::new();

        // Hash the essential fields
        hasher.update(self.entity_type.to_string().as_bytes());
        hasher.update(self.entity_id.as_bytes());
        hasher.update(&self.version.to_le_bytes());

        if let Some(ref parent) = self.parent_hash {
            hasher.update(parent.as_bytes());
        }

        // Hash the state
        if let Ok(state_json) = serde_json::to_string(&self.state) {
            hasher.update(state_json.as_bytes());
        }

        // Hash operation type
        if let Ok(op_json) = serde_json::to_string(&self.operation) {
            hasher.update(op_json.as_bytes());
        }

        // Hash timestamp
        hasher.update(self.timestamp.to_rfc3339().as_bytes());

        let hash = hasher.finalize();
        hash.to_hex().to_string()
    }

    /// Set the snapshot ID after computing the hash
    pub fn with_computed_hash(mut self) -> Self {
        self.snapshot_id = self.compute_hash();
        self
    }

    /// Add IPFS CID
    pub fn with_ipfs_cid(mut self, cid: String) -> Self {
        self.ipfs_cid = Some(cid);
        self
    }

    /// Add blockchain transaction ID
    pub fn with_blockchain_tx(mut self, tx: String) -> Self {
        self.blockchain_tx = Some(tx);
        self
    }

    /// Add commit message
    pub fn with_message(mut self, message: String) -> Self {
        self.message = Some(message);
        self
    }

    /// Add operation metadata
    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.operation_metadata.insert(key, value);
        self
    }

    /// Get URLs for viewing this snapshot
    pub fn get_urls(&self) -> SnapshotUrls {
        SnapshotUrls {
            ipfs_url: self
                .ipfs_cid
                .as_ref()
                .map(|cid| format!("https://offchain.defarm.net/ipfs/{}", cid)),
            stellar_url: self
                .blockchain_tx
                .as_ref()
                .map(|tx| format!("https://stellar.expert/explorer/testnet/tx/{}", tx)),
        }
    }
}

/// URLs for viewing snapshot data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotUrls {
    pub ipfs_url: Option<String>,
    pub stellar_url: Option<String>,
}

/// Change type in a state diff
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChangeType {
    Added,
    Modified,
    Removed,
}

/// A single change in a state diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateChange {
    /// JSON path to the changed field (e.g., "enriched_data.weight")
    pub path: String,

    /// Previous value (None if added)
    pub old_value: Option<serde_json::Value>,

    /// New value (None if removed)
    pub new_value: Option<serde_json::Value>,

    /// Type of change
    pub change_type: ChangeType,
}

/// Diff between two snapshots
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDiff {
    /// Source snapshot ID
    pub from_snapshot: String,

    /// Target snapshot ID
    pub to_snapshot: String,

    /// List of changes
    pub changes: Vec<StateChange>,

    /// Events added between snapshots
    pub added_events: Vec<String>,

    /// Version jump
    pub version_from: u64,
    pub version_to: u64,
}

/// Result of verifying a snapshot chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainVerification {
    /// Is the entire chain valid?
    pub is_valid: bool,

    /// Total number of snapshots in chain
    pub total_snapshots: usize,

    /// Number of successfully verified snapshots
    pub verified_snapshots: usize,

    /// Snapshots with broken parent links
    pub broken_links: Vec<String>,

    /// Snapshots with tampered/invalid hashes
    pub tampered_hashes: Vec<String>,

    /// First snapshot timestamp
    pub chain_start: Option<DateTime<Utc>>,

    /// Last snapshot timestamp
    pub chain_end: Option<DateTime<Utc>>,
}

impl ChainVerification {
    /// Create a successful verification result
    pub fn success(total: usize, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        Self {
            is_valid: true,
            total_snapshots: total,
            verified_snapshots: total,
            broken_links: vec![],
            tampered_hashes: vec![],
            chain_start: Some(start),
            chain_end: Some(end),
        }
    }

    /// Create a failed verification result
    pub fn failure(
        total: usize,
        verified: usize,
        broken: Vec<String>,
        tampered: Vec<String>,
    ) -> Self {
        Self {
            is_valid: false,
            total_snapshots: total,
            verified_snapshots: verified,
            broken_links: broken,
            tampered_hashes: tampered,
            chain_start: None,
            chain_end: None,
        }
    }
}

/// Summary of an item's snapshot chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemSnapshotSummary {
    pub dfid: String,
    pub total_snapshots: u64,
    pub latest_version: u64,
    pub latest_snapshot_id: Option<String>,
    pub latest_ipfs_cid: Option<String>,
    pub first_snapshot_at: Option<DateTime<Utc>>,
    pub last_snapshot_at: Option<DateTime<Utc>>,
}

/// Summary of a circuit's snapshot chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitSnapshotSummary {
    pub circuit_id: Uuid,
    pub total_snapshots: u64,
    pub latest_version: u64,
    pub latest_snapshot_id: Option<String>,
    pub latest_ipfs_cid: Option<String>,
    pub first_snapshot_at: Option<DateTime<Utc>>,
    pub last_snapshot_at: Option<DateTime<Utc>>,
    /// Number of items in circuit at last snapshot
    pub item_count: usize,
}

/// Error types for snapshot operations
#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    #[error("Snapshot not found: {0}")]
    NotFound(String),

    #[error("Entity not found: {entity_type} {entity_id}")]
    EntityNotFound {
        entity_type: SnapshotEntityType,
        entity_id: String,
    },

    #[error("Failed to serialize snapshot: {0}")]
    SerializationError(String),

    #[error("Failed to upload to IPFS: {0}")]
    IpfsError(String),

    #[error("Failed to record on blockchain: {0}")]
    BlockchainError(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Chain verification failed: {0}")]
    VerificationFailed(String),

    #[error("Parent snapshot not found: {0}")]
    ParentNotFound(String),

    #[error("Invalid snapshot state: {0}")]
    InvalidState(String),
}

impl From<serde_json::Error> for SnapshotError {
    fn from(err: serde_json::Error) -> Self {
        SnapshotError::SerializationError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_snapshot_hash_computation() {
        let snapshot = StateSnapshot::new(
            SnapshotEntityType::Item,
            "DFID-20251203-000001-40BA".to_string(),
            1,
            None,
            json!({"test": "data"}),
            SnapshotOperation::ItemCreated,
            "user-123".to_string(),
        )
        .with_computed_hash();

        assert!(!snapshot.snapshot_id.is_empty());
        assert_eq!(snapshot.snapshot_id.len(), 64); // BLAKE3 hex is 64 chars
    }

    #[test]
    fn test_snapshot_hash_changes_with_content() {
        let snapshot1 = StateSnapshot::new(
            SnapshotEntityType::Item,
            "DFID-1".to_string(),
            1,
            None,
            json!({"data": "one"}),
            SnapshotOperation::ItemCreated,
            "user".to_string(),
        )
        .with_computed_hash();

        let snapshot2 = StateSnapshot::new(
            SnapshotEntityType::Item,
            "DFID-1".to_string(),
            1,
            None,
            json!({"data": "two"}),
            SnapshotOperation::ItemCreated,
            "user".to_string(),
        )
        .with_computed_hash();

        assert_ne!(snapshot1.snapshot_id, snapshot2.snapshot_id);
    }

    #[test]
    fn test_operation_description() {
        assert_eq!(SnapshotOperation::ItemCreated.description(), "Item created");
        assert_eq!(
            SnapshotOperation::ItemEventAdded {
                event_id: "123".to_string(),
                event_type: "Enriched".to_string(),
                event_category: Some("vacinacao".to_string()),
            }
            .description(),
            "Event added: Enriched (vacinacao)"
        );
    }

    #[test]
    fn test_chain_verification() {
        let success = ChainVerification::success(5, Utc::now(), Utc::now());
        assert!(success.is_valid);
        assert_eq!(success.total_snapshots, 5);
        assert_eq!(success.verified_snapshots, 5);

        let failure = ChainVerification::failure(
            5,
            3,
            vec!["snap-3".to_string()],
            vec!["snap-4".to_string()],
        );
        assert!(!failure.is_valid);
        assert_eq!(failure.verified_snapshots, 3);
    }
}

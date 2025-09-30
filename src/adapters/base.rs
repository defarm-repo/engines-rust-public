use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use crate::types::*;
use crate::storage::StorageError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageMetadata {
    pub adapter_type: AdapterType,
    pub item_location: StorageLocation,
    pub event_locations: Vec<StorageLocation>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageLocation {
    Local { id: String },
    IPFS { cid: String, pinned: bool },
    Stellar {
        transaction_id: String,
        contract_address: String,
        asset_id: Option<String>
    },
    Ethereum {
        transaction_hash: String,
        contract_address: String,
        token_id: Option<String>
    },
    Arweave { transaction_id: String },
}

#[derive(Debug)]
pub struct AdapterResult<T> {
    pub data: T,
    pub metadata: StorageMetadata,
}

impl<T> AdapterResult<T> {
    pub fn new(data: T, metadata: StorageMetadata) -> Self {
        Self { data, metadata }
    }
}

#[async_trait]
pub trait StorageAdapter: Send + Sync {
    fn adapter_type(&self) -> AdapterType;

    async fn store_item(&self, item: &Item) -> Result<AdapterResult<String>, StorageError>;

    async fn store_event(&self, event: &Event, item_id: &str) -> Result<AdapterResult<String>, StorageError>;

    async fn get_item(&self, item_id: &str) -> Result<Option<AdapterResult<Item>>, StorageError>;

    async fn get_event(&self, event_id: &str) -> Result<Option<AdapterResult<Event>>, StorageError>;

    async fn get_item_events(&self, item_id: &str) -> Result<Vec<AdapterResult<Event>>, StorageError>;

    async fn sync_status(&self) -> Result<SyncStatus, StorageError>;

    async fn health_check(&self) -> Result<bool, StorageError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    pub adapter_type: AdapterType,
    pub is_synced: bool,
    pub pending_operations: u32,
    pub last_sync: Option<chrono::DateTime<chrono::Utc>>,
    pub error_count: u32,
    pub details: HashMap<String, serde_json::Value>,
}
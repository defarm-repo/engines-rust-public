// ============================================================================
// Storage History Reader
// ============================================================================
//
// This module provides read-only access to storage history records.
//
// IMPORTANT: Storage history recording happens directly in circuits_engine.rs
// during push operations (see lines 709-723 in push_local_item_to_circuit).
// This reader is only for querying existing history via API endpoints.
//
// The actual storage flow:
// 1. circuits_engine.rs calls adapter.store_new_item()
// 2. Adapter returns AdapterResult with StorageMetadata
// 3. circuits_engine.rs creates StorageRecord from metadata
// 4. circuits_engine.rs calls storage.add_storage_record() directly
//
// This reader then allows API endpoints to query that stored history.
// ============================================================================

use crate::adapters::base::StorageLocation;
use crate::storage::{StorageBackend, StorageError};
use crate::types::ItemStorageHistory;
use std::sync::Arc;

/// Read-only interface for querying storage history.
/// Does NOT record new history - that happens in circuits_engine.rs
pub struct StorageHistoryReader<S: StorageBackend> {
    storage: Arc<std::sync::Mutex<S>>,
}

impl<S: StorageBackend> StorageHistoryReader<S> {
    /// Create a new storage history reader
    pub fn new(storage: Arc<std::sync::Mutex<S>>) -> Self {
        Self { storage }
    }

    /// Get the complete storage history for an item by DFID
    pub async fn get_item_storage_history(
        &self,
        dfid: &str,
    ) -> Result<Option<ItemStorageHistory>, StorageError> {
        let storage = self.storage.lock().unwrap();
        storage.get_storage_history(dfid)
    }

    /// Get all storage locations where an item has been stored
    pub async fn get_all_storage_locations(
        &self,
        dfid: &str,
    ) -> Result<Vec<StorageLocation>, StorageError> {
        let storage = self.storage.lock().unwrap();
        if let Some(history) = storage.get_storage_history(dfid)? {
            Ok(history
                .storage_records
                .into_iter()
                .map(|record| record.storage_location)
                .collect())
        } else {
            Ok(Vec::new())
        }
    }

    /// Get the latest storage location for an item
    pub async fn get_latest_storage_location(
        &self,
        dfid: &str,
    ) -> Result<Option<StorageLocation>, StorageError> {
        let storage = self.storage.lock().unwrap();
        if let Some(history) = storage.get_storage_history(dfid)? {
            // Return the most recent active storage record
            let latest = history
                .storage_records
                .into_iter()
                .filter(|record| record.is_active)
                .max_by_key(|record| record.stored_at);

            Ok(latest.map(|record| record.storage_location))
        } else {
            Ok(None)
        }
    }

    /// Check if an item has been stored in any blockchain adapter
    pub async fn has_blockchain_storage(&self, dfid: &str) -> Result<bool, StorageError> {
        let locations = self.get_all_storage_locations(dfid).await?;
        Ok(locations.iter().any(|loc| {
            matches!(
                loc,
                StorageLocation::Stellar { .. } | StorageLocation::Ethereum { .. }
            )
        }))
    }

    /// Get storage history count for statistics
    pub async fn get_storage_count(&self, dfid: &str) -> Result<usize, StorageError> {
        let storage = self.storage.lock().unwrap();
        if let Some(history) = storage.get_storage_history(dfid)? {
            Ok(history.storage_records.len())
        } else {
            Ok(0)
        }
    }
}

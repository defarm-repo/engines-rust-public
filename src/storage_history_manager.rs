use crate::adapters::{base::StorageLocation, AdapterInstance};
use crate::storage::{StorageBackend, StorageError};
use crate::types::{AdapterType, ItemStorageHistory, StorageRecord};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

pub struct StorageHistoryManager<S: StorageBackend> {
    storage: Arc<std::sync::Mutex<S>>,
}

impl<S: StorageBackend> StorageHistoryManager<S> {
    pub fn new(storage: Arc<std::sync::Mutex<S>>) -> Self {
        Self { storage }
    }

    pub async fn record_item_storage(
        &self,
        dfid: &str,
        adapter_type: AdapterType,
        storage_id: String,
        _circuit_id: Option<Uuid>,
        user_id: &str,
    ) -> Result<(), StorageError> {
        let storage_location = match adapter_type {
            AdapterType::None => StorageLocation::Local {
                id: storage_id.clone(),
            },
            AdapterType::IpfsIpfs => StorageLocation::IPFS {
                cid: storage_id.clone(),
                pinned: true,
            },
            AdapterType::EthereumGoerliIpfs => StorageLocation::IPFS {
                cid: storage_id.clone(),
                pinned: true,
            },
            AdapterType::PolygonArweave => StorageLocation::Local {
                id: storage_id.clone(),
            }, // Implementation pending
            AdapterType::StellarTestnetIpfs | AdapterType::StellarMainnetIpfs => {
                StorageLocation::Stellar {
                    transaction_id: storage_id.clone(),
                    contract_address: "placeholder".to_string(), // Implementation pending
                    asset_id: None,
                }
            }
            AdapterType::Custom(_) => StorageLocation::Local {
                id: storage_id.clone(),
            }, // Fallback
        };

        let record = StorageRecord {
            adapter_type,
            storage_location,
            stored_at: Utc::now(),
            triggered_by: "store_item".to_string(),
            triggered_by_id: Some(user_id.to_string()),
            events_range: None,
            is_active: true,
            metadata: std::collections::HashMap::new(),
        };

        let storage = self.storage.lock().unwrap();
        storage.add_storage_record(dfid, record)?;
        Ok(())
    }

    pub async fn record_event_storage(
        &self,
        item_dfid: &str,
        event_id: String,
        adapter_type: AdapterType,
        storage_id: String,
        _circuit_id: Option<Uuid>,
        user_id: &str,
    ) -> Result<(), StorageError> {
        let storage_location = match adapter_type {
            AdapterType::None => StorageLocation::Local {
                id: storage_id.clone(),
            },
            AdapterType::IpfsIpfs => StorageLocation::IPFS {
                cid: storage_id.clone(),
                pinned: true,
            },
            AdapterType::EthereumGoerliIpfs => StorageLocation::IPFS {
                cid: storage_id.clone(),
                pinned: true,
            },
            AdapterType::PolygonArweave => StorageLocation::Local {
                id: storage_id.clone(),
            }, // Implementation pending
            AdapterType::StellarTestnetIpfs | AdapterType::StellarMainnetIpfs => {
                StorageLocation::Stellar {
                    transaction_id: storage_id.clone(),
                    contract_address: "placeholder".to_string(), // Implementation pending
                    asset_id: None,
                }
            }
            AdapterType::Custom(_) => StorageLocation::Local {
                id: storage_id.clone(),
            }, // Fallback
        };

        let record = StorageRecord {
            adapter_type,
            storage_location,
            stored_at: Utc::now(),
            triggered_by: format!("store_event:{event_id}"),
            triggered_by_id: Some(user_id.to_string()),
            events_range: None,
            is_active: false,
            metadata: HashMap::new(),
        };

        let storage = self.storage.lock().unwrap();
        storage.add_storage_record(item_dfid, record)?;
        Ok(())
    }

    pub async fn get_item_storage_history(
        &self,
        dfid: &str,
    ) -> Result<Option<ItemStorageHistory>, StorageError> {
        let storage = self.storage.lock().unwrap();
        storage.get_storage_history(dfid)
    }

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

    pub async fn set_primary_storage(
        &self,
        dfid: &str,
        location: StorageLocation,
    ) -> Result<(), StorageError> {
        let storage = self.storage.lock().unwrap();
        if let Some(mut history) = storage.get_storage_history(dfid)? {
            history.current_primary = Some(location);
            history.updated_at = Utc::now();
            storage.store_storage_history(&history)?;
        }
        Ok(())
    }

    pub async fn migrate_to_circuit_adapter(
        &self,
        dfid: &str,
        new_adapter: &AdapterInstance,
        circuit_id: Uuid,
        _user_id: &str,
    ) -> Result<(), StorageError> {
        // Get current storage history
        let _current_locations = self.get_all_storage_locations(dfid).await?;

        // Check if item is already stored in this adapter
        let adapter_type = new_adapter.adapter_type();
        // Implementation pending
        // let already_stored = current_locations.iter().any(|loc| loc.adapter_type == adapter_type);
        let already_stored = false; // Temporarily always migrate

        if !already_stored {
            // Need to actually copy the item to the new adapter
            // This would involve reading from existing storage and writing to new adapter
            // For now, we'll record the migration intent

            let storage_location = match adapter_type {
                AdapterType::None => StorageLocation::Local {
                    id: format!("migrated_{dfid}"),
                },
                AdapterType::IpfsIpfs => StorageLocation::IPFS {
                    cid: format!("migrated_{dfid}"),
                    pinned: true,
                },
                AdapterType::EthereumGoerliIpfs => StorageLocation::IPFS {
                    cid: format!("migrated_{dfid}"),
                    pinned: true,
                },
                AdapterType::PolygonArweave => StorageLocation::Arweave {
                    transaction_id: format!("migrated_{dfid}"),
                },
                AdapterType::StellarTestnetIpfs | AdapterType::StellarMainnetIpfs => {
                    StorageLocation::Stellar {
                        transaction_id: format!("migrated_{dfid}"),
                        contract_address: "placeholder".to_string(), // Implementation pending
                        asset_id: None,
                    }
                }
                AdapterType::Custom(_) => StorageLocation::Local {
                    id: format!("migrated_{dfid}"),
                }, // Fallback
            };

            let migration_record = StorageRecord {
                adapter_type,
                storage_location,
                stored_at: Utc::now(),
                triggered_by: "circuit_migration".to_string(),
                triggered_by_id: Some(circuit_id.to_string()),
                events_range: None,
                is_active: true,
                metadata: HashMap::new(),
            };

            let storage = self.storage.lock().unwrap();
            storage.add_storage_record(dfid, migration_record)?;
        }

        Ok(())
    }
}

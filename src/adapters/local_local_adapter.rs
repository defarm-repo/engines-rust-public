use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use chrono::Utc;
use crate::adapters::base::*;
use crate::types::*;
use crate::storage::StorageError;

#[derive(Debug)]
pub struct LocalLocalAdapter {
    items: Arc<Mutex<HashMap<String, Item>>>,
    events: Arc<Mutex<HashMap<String, Event>>>,
    metadata: Arc<Mutex<HashMap<String, StorageMetadata>>>,
}

impl LocalLocalAdapter {
    pub fn new() -> Self {
        Self {
            items: Arc::new(Mutex::new(HashMap::new())),
            events: Arc::new(Mutex::new(HashMap::new())),
            metadata: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn create_metadata(&self, _operation: &str) -> StorageMetadata {
        let now = Utc::now();
        StorageMetadata {
            adapter_type: AdapterType::LocalLocal,
            item_location: StorageLocation::Local {
                id: format!("local_{}", now.timestamp_millis()),
            },
            event_locations: vec![StorageLocation::Local {
                id: format!("local_event_{}", now.timestamp_millis()),
            }],
            created_at: now,
            updated_at: now,
        }
    }
}

#[async_trait]
impl StorageAdapter for LocalLocalAdapter {
    fn adapter_type(&self) -> AdapterType {
        AdapterType::LocalLocal
    }

    async fn store_item(&self, item: &Item) -> Result<AdapterResult<String>, StorageError> {
        let mut items = self.items.lock().map_err(|_| StorageError::IoError("Failed to acquire items lock".to_string()))?;
        let mut metadata_store = self.metadata.lock().map_err(|_| StorageError::IoError("Failed to acquire metadata lock".to_string()))?;

        let item_id = item.dfid.clone();
        items.insert(item_id.clone(), item.clone());

        let metadata = self.create_metadata("store_item");
        metadata_store.insert(item_id.clone(), metadata.clone());

        Ok(AdapterResult::new(item_id, metadata))
    }

    async fn store_event(&self, event: &Event, _item_id: &str) -> Result<AdapterResult<String>, StorageError> {
        let mut events = self.events.lock().map_err(|_| StorageError::IoError("Failed to acquire events lock".to_string()))?;
        let mut metadata_store = self.metadata.lock().map_err(|_| StorageError::IoError("Failed to acquire metadata lock".to_string()))?;

        let event_id = event.event_id.to_string();
        events.insert(event_id.clone(), event.clone());

        let metadata = self.create_metadata("store_event");
        metadata_store.insert(event_id.clone(), metadata.clone());

        Ok(AdapterResult::new(event_id, metadata))
    }

    async fn get_item(&self, item_id: &str) -> Result<Option<AdapterResult<Item>>, StorageError> {
        let items = self.items.lock().map_err(|_| StorageError::IoError("Failed to acquire items lock".to_string()))?;
        let metadata_store = self.metadata.lock().map_err(|_| StorageError::IoError("Failed to acquire metadata lock".to_string()))?;

        if let Some(item) = items.get(item_id) {
            let metadata = metadata_store.get(item_id).cloned().unwrap_or_else(|| self.create_metadata("get_item"));
            Ok(Some(AdapterResult::new(item.clone(), metadata)))
        } else {
            Ok(None)
        }
    }

    async fn get_event(&self, event_id: &str) -> Result<Option<AdapterResult<Event>>, StorageError> {
        let events = self.events.lock().map_err(|_| StorageError::IoError("Failed to acquire events lock".to_string()))?;
        let metadata_store = self.metadata.lock().map_err(|_| StorageError::IoError("Failed to acquire metadata lock".to_string()))?;

        if let Some(event) = events.get(event_id) {
            let metadata = metadata_store.get(event_id).cloned().unwrap_or_else(|| self.create_metadata("get_event"));
            Ok(Some(AdapterResult::new(event.clone(), metadata)))
        } else {
            Ok(None)
        }
    }

    async fn get_item_events(&self, item_id: &str) -> Result<Vec<AdapterResult<Event>>, StorageError> {
        let events = self.events.lock().map_err(|_| StorageError::IoError("Failed to acquire events lock".to_string()))?;
        let metadata_store = self.metadata.lock().map_err(|_| StorageError::IoError("Failed to acquire metadata lock".to_string()))?;

        let mut results = Vec::new();
        for event in events.values() {
            if event.dfid == item_id {
                let event_id = event.event_id.to_string();
                let metadata = metadata_store.get(&event_id).cloned().unwrap_or_else(|| self.create_metadata("get_item_events"));
                results.push(AdapterResult::new(event.clone(), metadata));
            }
        }

        Ok(results)
    }

    async fn sync_status(&self) -> Result<SyncStatus, StorageError> {
        Ok(SyncStatus {
            adapter_type: AdapterType::LocalLocal,
            is_synced: true, // Local adapter is always synced
            pending_operations: 0,
            last_sync: Some(Utc::now()),
            error_count: 0,
            details: {
                let mut details = HashMap::new();
                let items = self.items.lock().map_err(|_| StorageError::IoError("Failed to acquire items lock".to_string()))?;
                let events = self.events.lock().map_err(|_| StorageError::IoError("Failed to acquire events lock".to_string()))?;

                details.insert("items_count".to_string(), serde_json::Value::Number(serde_json::Number::from(items.len())));
                details.insert("events_count".to_string(), serde_json::Value::Number(serde_json::Number::from(events.len())));
                details.insert("storage_type".to_string(), serde_json::Value::String("local_memory".to_string()));
                details
            },
        })
    }

    async fn health_check(&self) -> Result<bool, StorageError> {
        // Try to acquire both locks to verify adapter is functional
        let _items = self.items.lock().map_err(|_| StorageError::IoError("Failed to acquire items lock".to_string()))?;
        let _events = self.events.lock().map_err(|_| StorageError::IoError("Failed to acquire events lock".to_string()))?;
        Ok(true)
    }
}
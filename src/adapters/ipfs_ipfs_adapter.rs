use crate::adapters::base::*;
use crate::ipfs_client::IpfsClient;
use crate::storage::StorageError;
use crate::types::*;
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct IpfsIpfsAdapter {
    ipfs_client: Arc<IpfsClient>,
}

impl IpfsIpfsAdapter {
    pub fn new() -> Result<Self, StorageError> {
        // Get configuration from environment or use defaults
        let ipfs_endpoint =
            std::env::var("IPFS_ENDPOINT").unwrap_or_else(|_| "http://localhost:5001".to_string());

        // Initialize IPFS client (prefer Pinata if configured, fallback to local)
        let ipfs_client = if let (Ok(api_key), Ok(secret)) = (
            std::env::var("PINATA_API_KEY"),
            std::env::var("PINATA_SECRET_KEY"),
        ) {
            IpfsClient::with_pinata(api_key, secret).map_err(|e| {
                StorageError::ConnectionError(format!("Failed to configure Pinata: {e}"))
            })?
        } else {
            IpfsClient::with_endpoint(&ipfs_endpoint).map_err(|e| {
                StorageError::ConnectionError(format!("Failed to connect to IPFS: {e}"))
            })?
        };

        Ok(Self {
            ipfs_client: Arc::new(ipfs_client),
        })
    }

    fn create_metadata(&self, cid: &str) -> StorageMetadata {
        let now = Utc::now();
        StorageMetadata {
            adapter_type: AdapterType::IpfsIpfs,
            item_location: StorageLocation::IPFS {
                cid: cid.to_string(),
                pinned: true,
            },
            event_locations: vec![StorageLocation::IPFS {
                cid: cid.to_string(),
                pinned: true,
            }],
            created_at: now,
            updated_at: now,
        }
    }
}

#[async_trait]
impl StorageAdapter for IpfsIpfsAdapter {
    fn adapter_type(&self) -> AdapterType {
        AdapterType::IpfsIpfs
    }

    async fn store_item(&self, item: &Item) -> Result<AdapterResult<String>, StorageError> {
        // Upload item to IPFS and get CID
        let cid = self
            .ipfs_client
            .upload_json(item)
            .await
            .map_err(|e| StorageError::WriteError(format!("Failed to upload to IPFS: {e}")))?;

        // Create metadata with CID
        let metadata = self.create_metadata(&cid);

        Ok(AdapterResult::new(item.dfid.clone(), metadata))
    }

    async fn store_event(
        &self,
        event: &Event,
        _item_id: &str,
    ) -> Result<AdapterResult<String>, StorageError> {
        // Upload event to IPFS and get CID
        let cid = self.ipfs_client.upload_json(event).await.map_err(|e| {
            StorageError::WriteError(format!("Failed to upload event to IPFS: {e}"))
        })?;

        // Create metadata with CID
        let metadata = self.create_metadata(&cid);

        Ok(AdapterResult::new(event.event_id.to_string(), metadata))
    }

    async fn get_item(&self, item_id: &str) -> Result<Option<AdapterResult<Item>>, StorageError> {
        // For IPFS-only adapter, item_id should be the CID
        // Try to retrieve from IPFS
        match self.ipfs_client.get_json::<Item>(item_id).await {
            Ok(item) => {
                let metadata = self.create_metadata(item_id);
                Ok(Some(AdapterResult::new(item, metadata)))
            }
            Err(_) => Ok(None),
        }
    }

    async fn get_event(
        &self,
        event_id: &str,
    ) -> Result<Option<AdapterResult<Event>>, StorageError> {
        // For IPFS-only adapter, event_id should be the CID
        // Try to retrieve from IPFS
        match self.ipfs_client.get_json::<Event>(event_id).await {
            Ok(event) => {
                let metadata = self.create_metadata(event_id);
                Ok(Some(AdapterResult::new(event, metadata)))
            }
            Err(_) => Ok(None),
        }
    }

    async fn get_item_events(
        &self,
        _item_id: &str,
    ) -> Result<Vec<AdapterResult<Event>>, StorageError> {
        // IPFS-only adapter doesn't maintain an index of events per item
        // This would require a separate indexing mechanism
        Ok(Vec::new())
    }

    async fn sync_status(&self) -> Result<SyncStatus, StorageError> {
        let ipfs_connected = self.ipfs_client.health_check().await.unwrap_or(false);
        let node_info = self
            .ipfs_client
            .node_info()
            .await
            .unwrap_or_else(|_| "unknown".to_string());

        Ok(SyncStatus {
            adapter_type: AdapterType::IpfsIpfs,
            is_synced: ipfs_connected,
            pending_operations: 0,
            last_sync: if ipfs_connected {
                Some(Utc::now())
            } else {
                None
            },
            error_count: 0,
            details: {
                let mut details = HashMap::new();
                details.insert(
                    "implementation_status".to_string(),
                    serde_json::Value::String("production".to_string()),
                );
                details.insert(
                    "ipfs_node".to_string(),
                    serde_json::Value::String(node_info),
                );
                details.insert(
                    "ipfs_connected".to_string(),
                    serde_json::Value::Bool(ipfs_connected),
                );
                details
            },
        })
    }

    async fn health_check(&self) -> Result<bool, StorageError> {
        self.ipfs_client
            .health_check()
            .await
            .map_err(|e| StorageError::ConnectionError(format!("IPFS health check failed: {e}")))
    }
}

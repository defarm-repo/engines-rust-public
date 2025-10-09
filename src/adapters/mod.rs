pub mod base;
pub mod config;
pub mod local_local_adapter;
pub mod ipfs_ipfs_adapter;
pub mod stellar_testnet_ipfs_adapter;
pub mod stellar_mainnet_ipfs_adapter;
pub mod local_ipfs_adapter;
pub mod stellar_mainnet_stellar_mainnet_adapter;

pub use base::*;
pub use config::*;
pub use local_local_adapter::*;
pub use ipfs_ipfs_adapter::*;
pub use stellar_testnet_ipfs_adapter::*;
pub use stellar_mainnet_ipfs_adapter::*;
pub use local_ipfs_adapter::*;
pub use stellar_mainnet_stellar_mainnet_adapter::*;

use crate::types::*;
use crate::storage::StorageError;
use std::collections::HashMap;

#[derive(Debug)]
pub enum AdapterInstance {
    LocalLocal(LocalLocalAdapter),
    IpfsIpfs(IpfsIpfsAdapter),
    StellarTestnetIpfs(StellarTestnetIpfsAdapter),
    StellarMainnetIpfs(StellarMainnetIpfsAdapter),
    LocalIpfs(LocalIpfsAdapter),
    StellarMainnetStellarMainnet(StellarMainnetStellarMainnetAdapter),
}

impl AdapterInstance {
    pub fn adapter_type(&self) -> AdapterType {
        match self {
            AdapterInstance::LocalLocal(adapter) => adapter.adapter_type(),
            AdapterInstance::IpfsIpfs(adapter) => adapter.adapter_type(),
            AdapterInstance::StellarTestnetIpfs(adapter) => adapter.adapter_type(),
            AdapterInstance::StellarMainnetIpfs(adapter) => adapter.adapter_type(),
            AdapterInstance::LocalIpfs(adapter) => adapter.adapter_type(),
            AdapterInstance::StellarMainnetStellarMainnet(adapter) => adapter.adapter_type(),
        }
    }
}

#[async_trait::async_trait]
impl StorageAdapter for AdapterInstance {
    fn adapter_type(&self) -> AdapterType {
        match self {
            AdapterInstance::LocalLocal(adapter) => adapter.adapter_type(),
            AdapterInstance::IpfsIpfs(adapter) => adapter.adapter_type(),
            AdapterInstance::StellarTestnetIpfs(adapter) => adapter.adapter_type(),
            AdapterInstance::StellarMainnetIpfs(adapter) => adapter.adapter_type(),
            AdapterInstance::LocalIpfs(adapter) => adapter.adapter_type(),
            AdapterInstance::StellarMainnetStellarMainnet(adapter) => adapter.adapter_type(),
        }
    }

    async fn store_item(&self, item: &Item) -> Result<AdapterResult<String>, StorageError> {
        match self {
            AdapterInstance::LocalLocal(adapter) => adapter.store_item(item).await,
            AdapterInstance::IpfsIpfs(adapter) => adapter.store_item(item).await,
            AdapterInstance::StellarTestnetIpfs(adapter) => adapter.store_item(item).await,
            AdapterInstance::StellarMainnetIpfs(adapter) => adapter.store_item(item).await,
            AdapterInstance::LocalIpfs(adapter) => adapter.store_item(item).await,
            AdapterInstance::StellarMainnetStellarMainnet(adapter) => adapter.store_item(item).await,
        }
    }

    async fn store_event(&self, event: &Event, item_id: &str) -> Result<AdapterResult<String>, StorageError> {
        match self {
            AdapterInstance::LocalLocal(adapter) => adapter.store_event(event, item_id).await,
            AdapterInstance::IpfsIpfs(adapter) => adapter.store_event(event, item_id).await,
            AdapterInstance::StellarTestnetIpfs(adapter) => adapter.store_event(event, item_id).await,
            AdapterInstance::StellarMainnetIpfs(adapter) => adapter.store_event(event, item_id).await,
            AdapterInstance::LocalIpfs(adapter) => adapter.store_event(event, item_id).await,
            AdapterInstance::StellarMainnetStellarMainnet(adapter) => adapter.store_event(event, item_id).await,
        }
    }

    async fn get_item(&self, item_id: &str) -> Result<Option<AdapterResult<Item>>, StorageError> {
        match self {
            AdapterInstance::LocalLocal(adapter) => adapter.get_item(item_id).await,
            AdapterInstance::IpfsIpfs(adapter) => adapter.get_item(item_id).await,
            AdapterInstance::StellarTestnetIpfs(adapter) => adapter.get_item(item_id).await,
            AdapterInstance::StellarMainnetIpfs(adapter) => adapter.get_item(item_id).await,
            AdapterInstance::LocalIpfs(adapter) => adapter.get_item(item_id).await,
            AdapterInstance::StellarMainnetStellarMainnet(adapter) => adapter.get_item(item_id).await,
        }
    }

    async fn get_event(&self, event_id: &str) -> Result<Option<AdapterResult<Event>>, StorageError> {
        match self {
            AdapterInstance::LocalLocal(adapter) => adapter.get_event(event_id).await,
            AdapterInstance::IpfsIpfs(adapter) => adapter.get_event(event_id).await,
            AdapterInstance::StellarTestnetIpfs(adapter) => adapter.get_event(event_id).await,
            AdapterInstance::StellarMainnetIpfs(adapter) => adapter.get_event(event_id).await,
            AdapterInstance::LocalIpfs(adapter) => adapter.get_event(event_id).await,
            AdapterInstance::StellarMainnetStellarMainnet(adapter) => adapter.get_event(event_id).await,
        }
    }

    async fn get_item_events(&self, item_id: &str) -> Result<Vec<AdapterResult<Event>>, StorageError> {
        match self {
            AdapterInstance::LocalLocal(adapter) => adapter.get_item_events(item_id).await,
            AdapterInstance::IpfsIpfs(adapter) => adapter.get_item_events(item_id).await,
            AdapterInstance::StellarTestnetIpfs(adapter) => adapter.get_item_events(item_id).await,
            AdapterInstance::StellarMainnetIpfs(adapter) => adapter.get_item_events(item_id).await,
            AdapterInstance::LocalIpfs(adapter) => adapter.get_item_events(item_id).await,
            AdapterInstance::StellarMainnetStellarMainnet(adapter) => adapter.get_item_events(item_id).await,
        }
    }

    async fn sync_status(&self) -> Result<SyncStatus, StorageError> {
        match self {
            AdapterInstance::LocalLocal(adapter) => adapter.sync_status().await,
            AdapterInstance::IpfsIpfs(adapter) => adapter.sync_status().await,
            AdapterInstance::StellarTestnetIpfs(adapter) => adapter.sync_status().await,
            AdapterInstance::StellarMainnetIpfs(adapter) => adapter.sync_status().await,
            AdapterInstance::LocalIpfs(adapter) => adapter.sync_status().await,
            AdapterInstance::StellarMainnetStellarMainnet(adapter) => adapter.sync_status().await,
        }
    }

    async fn health_check(&self) -> Result<bool, StorageError> {
        match self {
            AdapterInstance::LocalLocal(adapter) => adapter.health_check().await,
            AdapterInstance::IpfsIpfs(adapter) => adapter.health_check().await,
            AdapterInstance::StellarTestnetIpfs(adapter) => adapter.health_check().await,
            AdapterInstance::StellarMainnetIpfs(adapter) => adapter.health_check().await,
            AdapterInstance::LocalIpfs(adapter) => adapter.health_check().await,
            AdapterInstance::StellarMainnetStellarMainnet(adapter) => adapter.health_check().await,
        }
    }
}

#[derive(Debug)]
pub struct AdapterRegistry {
    adapters: HashMap<AdapterType, AdapterInstance>,
    client_permissions: HashMap<String, Vec<AdapterType>>,
}

impl AdapterRegistry {
    pub fn new() -> Self {
        Self {
            adapters: HashMap::new(),
            client_permissions: HashMap::new(),
        }
    }

    pub fn register_adapter(&mut self, adapter: AdapterInstance) {
        let adapter_type = adapter.adapter_type();
        self.adapters.insert(adapter_type, adapter);
    }

    pub fn set_client_permissions(&mut self, client_id: String, adapters: Vec<AdapterType>) {
        self.client_permissions.insert(client_id, adapters);
    }

    pub fn get_available_adapters(&self, client_id: &str) -> Vec<AdapterType> {
        self.client_permissions
            .get(client_id)
            .cloned()
            .unwrap_or_else(|| vec![AdapterType::LocalLocal])
    }

    pub fn get_adapter(&self, adapter_type: &AdapterType) -> Option<&AdapterInstance> {
        self.adapters.get(adapter_type)
    }
}
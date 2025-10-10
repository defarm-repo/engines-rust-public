use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use chrono::Utc;
use crate::adapters::base::*;
use crate::types::*;
use crate::storage::StorageError;
use crate::ipfs_client::IpfsClient;
use crate::stellar_client::{StellarClient, StellarNetwork, TESTNET_IPCM_CONTRACT};

#[derive(Debug, Clone)]
pub struct StellarTestnetIpfsAdapter {
    stellar_client: Arc<StellarClient>,
    ipfs_client: Arc<IpfsClient>,
    contract_address: String,
    interface_address: String,
    source_account_identity: String,
}

impl StellarTestnetIpfsAdapter {
    pub fn new() -> Result<Self, StorageError> {
        // Legacy constructor for backward compatibility
        // This should only be used for testing
        Self::new_with_config(None)
    }

    pub fn new_with_config(config: Option<&AdapterConfig>) -> Result<Self, StorageError> {
        // If config provided, use it; otherwise fall back to env vars (for testing)
        let (ipfs_endpoint, contract_address, api_key, secret_key, stellar_secret, interface_address, source_account_identity) =
            if let Some(cfg) = config {
                // Extract from database config
                let ipfs_endpoint = cfg.connection_details.endpoint.clone();
                let api_key = cfg.connection_details.api_key.clone();
                let secret_key = cfg.connection_details.secret_key.clone();

                let contract_address = cfg.contract_configs.as_ref()
                    .and_then(|cc| cc.ipcm_contract.as_ref())
                    .map(|ci| ci.contract_address.clone())
                    .unwrap_or_else(|| TESTNET_IPCM_CONTRACT.to_string());

                // Extract from custom headers
                let stellar_secret = cfg.connection_details.custom_headers
                    .get("stellar_secret")
                    .cloned();
                let interface_address = cfg.connection_details.custom_headers
                    .get("interface_address")
                    .cloned()
                    .unwrap_or_else(|| std::env::var("DEFARM_OWNER_WALLET")
                        .unwrap_or_else(|_| "GANDYZQQ3OQBXHZQXJHZ7AQ2GDBFUQIR4ZLMUPD3P2B7PLIYQNFG54XQ".to_string()));
                let source_account_identity = cfg.connection_details.custom_headers
                    .get("source_account_identity")
                    .cloned()
                    .unwrap_or_else(|| "defarm-admin-testnet".to_string());

                (ipfs_endpoint, contract_address, api_key, secret_key, stellar_secret, interface_address, source_account_identity)
            } else {
                // Fall back to environment variables (legacy/testing)
                let ipfs_endpoint = std::env::var("IPFS_ENDPOINT")
                    .unwrap_or_else(|_| "http://localhost:5001".to_string());
                let contract_address = std::env::var("STELLAR_TESTNET_IPCM_CONTRACT")
                    .unwrap_or_else(|_| TESTNET_IPCM_CONTRACT.to_string());
                let api_key = std::env::var("PINATA_API_KEY").ok();
                let secret_key = std::env::var("PINATA_SECRET_KEY").ok();
                let stellar_secret = std::env::var("STELLAR_TESTNET_SECRET").ok();
                let interface_address = std::env::var("DEFARM_OWNER_WALLET")
                    .unwrap_or_else(|_| "GANDYZQQ3OQBXHZQXJHZ7AQ2GDBFUQIR4ZLMUPD3P2B7PLIYQNFG54XQ".to_string());
                let source_account_identity = "defarm-admin-testnet".to_string();

                (ipfs_endpoint, contract_address, api_key, secret_key, stellar_secret, interface_address, source_account_identity)
            };

        // Initialize IPFS client
        let ipfs_client = if let (Some(api_key), Some(secret)) = (api_key, secret_key) {
            IpfsClient::with_pinata(api_key, secret)
                .map_err(|e| StorageError::ConnectionError(format!("Failed to configure Pinata: {}", e)))?
        } else {
            IpfsClient::with_endpoint(&ipfs_endpoint)
                .map_err(|e| StorageError::ConnectionError(format!("Failed to connect to IPFS: {}", e)))?
        };

        // Initialize Stellar client
        let mut stellar_client = StellarClient::new(StellarNetwork::Testnet, contract_address.clone());

        // Configure with keypair if available
        if let Some(secret_key) = stellar_secret {
            stellar_client = stellar_client.with_keypair(&secret_key)
                .map_err(|e| StorageError::ConfigurationError(format!("Invalid Stellar keypair: {}", e)))?;
        }

        // Configure with interface and source account
        stellar_client = stellar_client
            .with_interface_address(interface_address.clone())
            .with_source_account(source_account_identity.clone());

        Ok(Self {
            stellar_client: Arc::new(stellar_client),
            ipfs_client: Arc::new(ipfs_client),
            contract_address,
            interface_address,
            source_account_identity,
        })
    }

    fn create_metadata(&self, stellar_tx: &str, ipfs_cid: &str) -> StorageMetadata {
        let now = Utc::now();
        StorageMetadata {
            adapter_type: AdapterType::StellarTestnetIpfs,
            item_location: StorageLocation::Stellar {
                transaction_id: stellar_tx.to_string(),
                contract_address: self.contract_address.clone(),
                asset_id: Some(ipfs_cid.to_string()),
            },
            event_locations: vec![StorageLocation::IPFS {
                cid: ipfs_cid.to_string(),
                pinned: true,
            }],
            created_at: now,
            updated_at: now,
        }
    }
}

#[async_trait]
impl StorageAdapter for StellarTestnetIpfsAdapter {
    fn adapter_type(&self) -> AdapterType {
        AdapterType::StellarTestnetIpfs
    }

    async fn store_item(&self, item: &Item) -> Result<AdapterResult<String>, StorageError> {
        // Step 1: Upload item to IPFS
        let cid = self.ipfs_client
            .upload_json(item)
            .await
            .map_err(|e| StorageError::WriteError(format!("Failed to upload to IPFS: {}", e)))?;

        // Step 2: Register CID on Stellar testnet blockchain using IPCM contract
        let tx_hash = self.stellar_client
            .update_ipcm(&item.dfid, &cid)
            .await
            .map_err(|e| StorageError::WriteError(format!("Failed to register on Stellar: {}", e)))?;

        // Step 3: Create metadata with both IPFS CID and Stellar transaction
        let metadata = self.create_metadata(&tx_hash, &cid);

        Ok(AdapterResult::new(item.dfid.clone(), metadata))
    }

    async fn store_event(&self, event: &Event, item_id: &str) -> Result<AdapterResult<String>, StorageError> {
        // Step 1: Upload event to IPFS
        let cid = self.ipfs_client
            .upload_json(event)
            .await
            .map_err(|e| StorageError::WriteError(format!("Failed to upload event to IPFS: {}", e)))?;

        // Step 2: Register event CID in IPCM contract with item reference
        let event_key = format!("event:{}:{}", item_id, event.event_id);
        let tx_hash = self.stellar_client
            .update_ipcm(&event_key, &cid)
            .await
            .map_err(|e| StorageError::WriteError(format!("Failed to register event on Stellar: {}", e)))?;

        // Step 3: Create metadata
        let metadata = self.create_metadata(&tx_hash, &cid);

        Ok(AdapterResult::new(event.event_id.to_string(), metadata))
    }

    async fn get_item(&self, item_id: &str) -> Result<Option<AdapterResult<Item>>, StorageError> {
        // Step 1: Get CID from Stellar contract
        let ipcm_entry = self.stellar_client
            .get_ipcm(item_id)
            .await
            .map_err(|e| StorageError::ReadError(format!("Failed to query Stellar: {}", e)))?;

        if let Some(entry) = ipcm_entry {
            // Step 2: Retrieve item from IPFS using CID
            let item = self.ipfs_client
                .get_json::<Item>(&entry.cid)
                .await
                .map_err(|e| StorageError::ReadError(format!("Failed to retrieve from IPFS: {}", e)))?;

            let metadata = self.create_metadata("read_only", &entry.cid);
            Ok(Some(AdapterResult::new(item, metadata)))
        } else {
            Ok(None)
        }
    }

    async fn get_event(&self, event_id: &str) -> Result<Option<AdapterResult<Event>>, StorageError> {
        let ipcm_entry = self.stellar_client
            .get_ipcm(event_id)
            .await
            .map_err(|e| StorageError::ReadError(format!("Failed to query Stellar: {}", e)))?;

        if let Some(entry) = ipcm_entry {
            let event = self.ipfs_client
                .get_json::<Event>(&entry.cid)
                .await
                .map_err(|e| StorageError::ReadError(format!("Failed to retrieve from IPFS: {}", e)))?;

            let metadata = self.create_metadata("read_only", &entry.cid);
            Ok(Some(AdapterResult::new(event, metadata)))
        } else {
            Ok(None)
        }
    }

    async fn get_item_events(&self, _item_id: &str) -> Result<Vec<AdapterResult<Event>>, StorageError> {
        Ok(Vec::new())
    }

    async fn sync_status(&self) -> Result<SyncStatus, StorageError> {
        let ipfs_connected = self.ipfs_client.health_check().await.unwrap_or(false);
        let stellar_connected = self.stellar_client.health_check().await.unwrap_or(false);

        let stellar_status = self.stellar_client
            .check_contract_status()
            .await
            .unwrap_or_else(|_| HashMap::new());

        let is_synced = ipfs_connected && stellar_connected;

        Ok(SyncStatus {
            adapter_type: AdapterType::StellarTestnetIpfs,
            is_synced,
            pending_operations: 0,
            last_sync: if is_synced { Some(Utc::now()) } else { None },
            error_count: 0,
            details: {
                let mut details = HashMap::new();
                details.insert("implementation_status".to_string(), serde_json::Value::String("production".to_string()));
                details.insert("stellar_network".to_string(), serde_json::Value::String("testnet".to_string()));
                details.insert("contract_address".to_string(), serde_json::Value::String(self.contract_address.clone()));
                details.insert("ipfs_connected".to_string(), serde_json::Value::Bool(ipfs_connected));
                details.insert("stellar_connected".to_string(), serde_json::Value::Bool(stellar_connected));

                for (key, value) in stellar_status {
                    details.insert(format!("stellar_{}", key), serde_json::Value::String(value));
                }

                details
            },
        })
    }

    async fn health_check(&self) -> Result<bool, StorageError> {
        let ipfs_health = self.ipfs_client.health_check().await.unwrap_or(false);
        let stellar_health = self.stellar_client.health_check().await.unwrap_or(false);

        Ok(ipfs_health && stellar_health)
    }
}

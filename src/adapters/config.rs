use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::types::AdapterType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterConfig {
    pub adapter_type: AdapterType,
    pub settings: HashMap<String, serde_json::Value>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientAdapterConfig {
    pub client_id: String,
    pub selected_adapter: AdapterType,
    pub available_adapters: Vec<AdapterType>,
    pub config: AdapterConfig,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StellarConfig {
    pub network: StellarNetwork,
    pub keypair: String,
    pub contract_address: String,
    pub fee_sponsor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StellarNetwork {
    Testnet,
    Mainnet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IPFSConfig {
    pub endpoint: String,
    pub api_key: Option<String>,
    pub pin_service: Option<String>,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthereumConfig {
    pub network: EthereumNetwork,
    pub rpc_endpoint: String,
    pub private_key: String,
    pub contract_address: String,
    pub gas_limit: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EthereumNetwork {
    Mainnet,
    Goerli,
    Sepolia,
    Polygon,
}

impl Default for AdapterConfig {
    fn default() -> Self {
        Self {
            adapter_type: AdapterType::IpfsIpfs,
            settings: HashMap::new(),
            enabled: true,
        }
    }
}

impl AdapterConfig {
    pub fn new(adapter_type: AdapterType) -> Self {
        Self {
            adapter_type,
            settings: HashMap::new(),
            enabled: true,
        }
    }

    pub fn with_setting(mut self, key: &str, value: serde_json::Value) -> Self {
        self.settings.insert(key.to_string(), value);
        self
    }

    pub fn get_setting<T>(&self, key: &str) -> Option<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        self.settings
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
}
use serde::{Deserialize, Serialize};
use chrono::Utc;
use std::collections::HashMap;
use std::time::Duration;
use std::rc::Rc;
use std::cell::RefCell;

// Soroban client imports
use soroban_client::{
    Server, Options,
    account::{Account, AccountBehavior},
    keypair::{Keypair, KeypairBehavior},
    network::{Networks, NetworkPassphrase},
    contract::{Contracts, ContractBehavior},
    transaction::{TransactionBuilder, TransactionBuilderBehavior, TransactionBehavior},
    soroban_rpc::TransactionStatus,
    xdr::{ScVal, ScString},
};

// Real contract addresses from .env configuration
pub const TESTNET_IPCM_CONTRACT: &str = "CAALVDSF7RLM7IRGE3GQKPRHWWZSPDSNHOBEIEDJU5MAM4I4PVFWJXLS";
pub const MAINNET_IPCM_CONTRACT: &str = "CBSIAY6QWRSRPXT2I2KP7TPFDH6G3BEPL4I7PPXTAXKQHTJYE5EC4P24";

#[derive(Debug, Clone)]
pub enum StellarNetwork {
    Testnet,
    Mainnet,
}

impl StellarNetwork {
    pub fn horizon_url(&self) -> &str {
        match self {
            StellarNetwork::Testnet => "https://horizon-testnet.stellar.org",
            StellarNetwork::Mainnet => "https://horizon.stellar.org",
        }
    }

    pub fn soroban_rpc_url(&self) -> &str {
        match self {
            StellarNetwork::Testnet => "https://soroban-testnet.stellar.org",
            StellarNetwork::Mainnet => "https://soroban-mainnet.stellar.org",
        }
    }

    pub fn network_passphrase(&self) -> &str {
        match self {
            StellarNetwork::Testnet => "Test SDF Network ; September 2015",
            StellarNetwork::Mainnet => "Public Global Stellar Network ; September 2015",
        }
    }
}

#[derive(Debug)]
pub enum StellarError {
    NetworkError(String),
    ContractError(String),
    SerializationError(String),
    SigningError(String),
    NotConfigured(String),
}

impl std::fmt::Display for StellarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StellarError::NetworkError(e) => write!(f, "Network error: {}", e),
            StellarError::ContractError(e) => write!(f, "Contract error: {}", e),
            StellarError::SerializationError(e) => write!(f, "Serialization error: {}", e),
            StellarError::SigningError(e) => write!(f, "Signing error: {}", e),
            StellarError::NotConfigured(e) => write!(f, "Not configured: {}", e),
        }
    }
}

impl std::error::Error for StellarError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcmEntry {
    pub dfid: String,
    pub cid: String,
    pub timestamp: i64,
    pub updated_by: String,
}

pub struct StellarClient {
    network: StellarNetwork,
    contract_address: String, // IPCM contract address
    nft_contract_address: Option<String>, // NFT minting contract address
    server: Server,
    keypair: Option<Keypair>,
    source_account: Option<String>, // Identity string for the source account
    http_client: reqwest::Client, // For read-only operations
}

impl std::fmt::Debug for StellarClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StellarClient")
            .field("network", &self.network)
            .field("contract_address", &self.contract_address)
            .field("has_keypair", &self.keypair.is_some())
            .field("source_account", &self.source_account)
            .finish()
    }
}

impl StellarClient {
    pub fn new(network: StellarNetwork, contract_address: String) -> Self {
        let server = Server::new(network.soroban_rpc_url(), Options::default())
            .expect("Failed to create Soroban RPC server");

        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            network,
            contract_address,
            nft_contract_address: None,
            server,
            keypair: None,
            source_account: None,
            http_client,
        }
    }

    pub fn with_nft_contract(mut self, nft_contract: String) -> Self {
        self.nft_contract_address = Some(nft_contract);
        self
    }

    pub fn with_keypair(mut self, secret_key: &str) -> Result<Self, StellarError> {
        let keypair = Keypair::from_secret(secret_key)
            .map_err(|e| StellarError::SigningError(format!("Invalid secret key: {:?}", e)))?;
        self.keypair = Some(keypair);
        Ok(self)
    }

    pub fn with_source_account(mut self, source_account: String) -> Self {
        self.source_account = Some(source_account);
        self
    }

    pub fn with_interface_address(mut self, _interface_address: String) -> Self {
        // Not needed with soroban-client, kept for API compatibility
        self
    }

    /// Get the NFT contract address if configured
    pub fn get_nft_contract_address(&self) -> Option<&str> {
        self.nft_contract_address.as_deref()
    }

    /// Update IPCM contract with new CID for a DFID using soroban-client
    pub async fn update_ipcm(&self, dfid: &str, cid: &str) -> Result<String, StellarError> {
        // Get keypair
        let keypair = self.keypair.as_ref()
            .ok_or_else(|| StellarError::NotConfigured("Keypair not configured".to_string()))?;

        // Get source account from network
        let source_account = self.server.get_account(&keypair.public_key())
            .await
            .map_err(|e| StellarError::NetworkError(format!("Failed to get account: {:?}", e)))?;

        // Create contract instance
        let contract = Contracts::new(&self.contract_address)
            .map_err(|e| StellarError::ContractError(format!("Invalid contract address: {:?}", e)))?;

        // Build ScVal arguments for the contract call
        // Assuming IPCM contract has: update(dfid: String, cid: String)
        let dfid_val = ScVal::String(ScString(dfid.try_into()
            .map_err(|e| StellarError::SerializationError(format!("Failed to convert dfid: {:?}", e)))?));
        let cid_val = ScVal::String(ScString(cid.try_into()
            .map_err(|e| StellarError::SerializationError(format!("Failed to convert cid: {:?}", e)))?));

        // Get network for transaction builder
        let network = match self.network {
            StellarNetwork::Testnet => Networks::testnet(),
            StellarNetwork::Mainnet => Networks::public(),
        };

        // Build transaction
        let tx = TransactionBuilder::new(Rc::new(RefCell::new(source_account)), network, None)
            .fee(1000u32) // Base fee, will be adjusted by prepare_transaction
            .add_operation(contract.call("update", Some(vec![dfid_val, cid_val])))
            .build();

        // Prepare transaction (simulate and assemble)
        let mut prepared_tx = self.server.prepare_transaction(&tx)
            .await
            .map_err(|e| StellarError::NetworkError(format!("Failed to prepare transaction: {:?}", e)))?;

        // Sign transaction
        prepared_tx.sign(&[keypair.clone()]);

        // Send transaction
        let response = self.server.send_transaction(prepared_tx)
            .await
            .map_err(|e| StellarError::NetworkError(format!("Failed to send transaction: {:?}", e)))?;

        let tx_hash = response.hash.clone();

        // Wait for transaction to complete
        match self.server.wait_transaction(&tx_hash, Duration::from_secs(30)).await {
            Ok(tx_result) if tx_result.status == TransactionStatus::Success => {
                tracing::info!(
                    "✅ IPCM updated successfully via soroban-client. Network: {:?}, TX: {}, DFID: {}, CID: {}",
                    self.network, tx_hash, dfid, cid
                );
                Ok(tx_hash)
            }
            Ok(tx_result) => {
                Err(StellarError::ContractError(format!("Transaction failed with status: {:?}", tx_result.status)))
            }
            Err(e) => {
                Err(StellarError::NetworkError(format!("Failed to wait for transaction: {:?}", e)))
            }
        }
    }

    /// Mint a new NFT with DFID as the token identifier using soroban-client
    /// This should only be called once per DFID (when first tokenized)
    pub async fn mint_nft(&self, dfid: &str, creator: &str, canonical_identifiers: Vec<String>, metadata: Option<serde_json::Value>) -> Result<String, StellarError> {
        // Ensure NFT contract is configured
        let nft_contract = self.nft_contract_address.as_ref()
            .ok_or_else(|| StellarError::NotConfigured("NFT contract address not configured".to_string()))?;

        // Get keypair
        let keypair = self.keypair.as_ref()
            .ok_or_else(|| StellarError::NotConfigured("Keypair not configured".to_string()))?;

        // Get source account from network
        let source_account = self.server.get_account(&keypair.public_key())
            .await
            .map_err(|e| StellarError::NetworkError(format!("Failed to get account: {:?}", e)))?;

        // Create contract instance for NFT contract
        let contract = Contracts::new(nft_contract)
            .map_err(|e| StellarError::ContractError(format!("Invalid NFT contract address: {:?}", e)))?;

        // Build metadata with canonical identifiers
        let metadata_str = metadata
            .map(|m| m.to_string())
            .unwrap_or_else(|| {
                let mut meta = serde_json::json!({
                    "dfid": dfid,
                    "creator": creator,
                    "minted_at": chrono::Utc::now().to_rfc3339()
                });

                // Add canonical identifiers (for IPCM key references)
                if !canonical_identifiers.is_empty() {
                    meta["canonical_identifiers"] = serde_json::json!(canonical_identifiers);
                }

                meta.to_string()
            });

        // Build ScVal arguments for the contract call
        // NFT contract mint function: mint(to: Address, token_id: String, metadata: String)
        let to_address_val = ScVal::String(ScString(keypair.public_key().as_str().try_into()
            .map_err(|e| StellarError::SerializationError(format!("Failed to convert address: {:?}", e)))?));
        let token_id_val = ScVal::String(ScString(dfid.try_into()
            .map_err(|e| StellarError::SerializationError(format!("Failed to convert token_id: {:?}", e)))?));
        let metadata_val = ScVal::String(ScString(metadata_str.as_str().try_into()
            .map_err(|e| StellarError::SerializationError(format!("Failed to convert metadata: {:?}", e)))?));

        // Get network for transaction builder
        let network = match self.network {
            StellarNetwork::Testnet => Networks::testnet(),
            StellarNetwork::Mainnet => Networks::public(),
        };

        // Build transaction
        let tx = TransactionBuilder::new(Rc::new(RefCell::new(source_account)), network, None)
            .fee(1000u32) // Base fee, will be adjusted by prepare_transaction
            .add_operation(contract.call("mint", Some(vec![to_address_val, token_id_val, metadata_val])))
            .build();

        // Prepare transaction (simulate and assemble)
        let mut prepared_tx = self.server.prepare_transaction(&tx)
            .await
            .map_err(|e| StellarError::NetworkError(format!("Failed to prepare NFT mint transaction: {:?}", e)))?;

        // Sign transaction
        prepared_tx.sign(&[keypair.clone()]);

        // Send transaction
        let response = self.server.send_transaction(prepared_tx)
            .await
            .map_err(|e| StellarError::NetworkError(format!("Failed to send NFT mint transaction: {:?}", e)))?;

        let tx_hash = response.hash.clone();

        // Wait for transaction to complete
        match self.server.wait_transaction(&tx_hash, Duration::from_secs(30)).await {
            Ok(tx_result) if tx_result.status == TransactionStatus::Success => {
                tracing::info!(
                    "✅ NFT minted successfully via soroban-client. Network: {:?}, TX: {}, DFID: {}, Creator: {}",
                    self.network, tx_hash, dfid, creator
                );
                Ok(tx_hash)
            }
            Ok(tx_result) => {
                Err(StellarError::ContractError(format!("NFT mint failed with status: {:?}", tx_result.status)))
            }
            Err(e) => {
                Err(StellarError::NetworkError(format!("Failed to wait for NFT mint transaction: {:?}", e)))
            }
        }
    }

    /// Get IPCM entry for a DFID
    pub async fn get_ipcm(&self, dfid: &str) -> Result<Option<IpcmEntry>, StellarError> {
        // Query contract state
        let contract_url = format!(
            "{}/contracts/{}/data/{}",
            self.network.horizon_url(),
            self.contract_address,
            dfid
        );

        let response = self.http_client.get(&contract_url)
            .send()
            .await
            .map_err(|e| StellarError::NetworkError(format!("Failed to query contract: {}", e)))?;

        if response.status().as_u16() == 404 {
            return Ok(None);
        }

        if !response.status().is_success() {
            return Err(StellarError::ContractError("Failed to query IPCM".to_string()));
        }

        let data: serde_json::Value = response.json()
            .await
            .map_err(|e| StellarError::SerializationError(format!("Failed to parse response: {}", e)))?;

        // Parse contract data
        let cid = data["value"]["cid"]
            .as_str()
            .ok_or_else(|| StellarError::SerializationError("No CID in response".to_string()))?
            .to_string();

        let timestamp = data["value"]["timestamp"]
            .as_i64()
            .unwrap_or_else(|| Utc::now().timestamp());

        let updated_by = data["value"]["updated_by"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        Ok(Some(IpcmEntry {
            dfid: dfid.to_string(),
            cid,
            timestamp,
            updated_by,
        }))
    }

    pub async fn health_check(&self) -> Result<bool, StellarError> {
        let url = self.network.horizon_url();
        let response = self.http_client.get(url)
            .send()
            .await
            .map_err(|e| StellarError::NetworkError(format!("Health check failed: {}", e)))?;

        Ok(response.status().is_success())
    }

    pub async fn check_contract_status(&self) -> Result<HashMap<String, String>, StellarError> {
        let contract_url = format!("{}/contracts/{}", self.network.horizon_url(), self.contract_address);

        let response = self.http_client.get(&contract_url)
            .send()
            .await
            .map_err(|e| StellarError::NetworkError(format!("Failed to check contract: {}", e)))?;

        let mut status = HashMap::new();
        status.insert("exists".to_string(), response.status().is_success().to_string());
        status.insert("contract_address".to_string(), self.contract_address.clone());
        status.insert("network".to_string(), format!("{:?}", self.network));

        Ok(status)
    }
}

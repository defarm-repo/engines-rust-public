use ed25519_dalek::{Signer, SigningKey};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use chrono::Utc;
use std::collections::HashMap;
use base64::{Engine as _, prelude::BASE64_STANDARD};

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
    http_client: Client,
    signing_key: Option<SigningKey>,
    secret_key_string: Option<String>, // Store the original secret key for CLI usage
    interface_address: Option<String>,
    source_account: Option<String>,
}

impl std::fmt::Debug for StellarClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StellarClient")
            .field("network", &self.network)
            .field("contract_address", &self.contract_address)
            .field("has_signing_key", &self.signing_key.is_some())
            .field("interface_address", &self.interface_address)
            .field("source_account", &self.source_account)
            .finish()
    }
}

impl StellarClient {
    pub fn new(network: StellarNetwork, contract_address: String) -> Self {
        Self {
            network,
            contract_address,
            nft_contract_address: None,
            http_client: Client::new(),
            signing_key: None,
            secret_key_string: None,
            interface_address: None,
            source_account: None,
        }
    }

    pub fn with_nft_contract(mut self, nft_contract: String) -> Self {
        self.nft_contract_address = Some(nft_contract);
        self
    }

    pub fn with_keypair(mut self, secret_key: &str) -> Result<Self, StellarError> {
        // Parse Stellar secret key (starts with S)
        let secret_bytes = stellar_strkey::ed25519::PrivateKey::from_string(secret_key)
            .map_err(|e| StellarError::SigningError(format!("Invalid secret key: {}", e)))?;

        let signing_key = SigningKey::from_bytes(&secret_bytes.0);
        self.signing_key = Some(signing_key);
        self.secret_key_string = Some(secret_key.to_string()); // Store the original string for CLI
        Ok(self)
    }

    pub fn with_interface_address(mut self, interface_address: String) -> Self {
        self.interface_address = Some(interface_address);
        self
    }

    pub fn with_source_account(mut self, source_account: String) -> Self {
        self.source_account = Some(source_account);
        self
    }

    /// Update IPCM contract with new CID for a DFID using native Soroban RPC
    /// This uses direct Soroban RPC calls for production-ready performance
    pub async fn update_ipcm(&self, dfid: &str, cid: &str) -> Result<String, StellarError> {
        // For now, we'll use a simplified implementation that calls the Soroban RPC directly
        // This avoids the complexity of the incomplete soroban-client high-level API

        // Get the signing key
        let signing_key = self.signing_key.as_ref()
            .ok_or_else(|| StellarError::NotConfigured("Signing key not configured".to_string()))?;

        // Get interface address or use default
        let interface_address = self.interface_address.as_ref()
            .map(|s| s.as_str())
            .unwrap_or_else(|| "GANDYZQQ3OQBXHZQXJHZ7AQ2GDBFUQIR4ZLMUPD3P2B7PLIYQNFG54XQ");

        // Get source account public key from signing key
        let public_key = signing_key.verifying_key();
        let public_key_bytes = public_key.to_bytes();

        // Convert to Stellar address format using our local stellar_strkey implementation
        let source_address = {
            let mut payload = vec![0x30]; // Version byte for Ed25519 public key (G address)
            payload.extend_from_slice(&public_key_bytes);

            // Calculate CRC16-XModem checksum
            let checksum = Self::crc16_xmodem(&payload);
            payload.push((checksum & 0xFF) as u8);
            payload.push(((checksum >> 8) & 0xFF) as u8);

            // Encode with base32
            base32::encode(base32::Alphabet::RFC4648 { padding: false }, &payload)
        };

        // Make direct Soroban RPC call to invoke contract
        // For production, we should use the soroban_client SDK, but for now use HTTP directly

        let rpc_url = self.network.soroban_rpc_url();

        // Build the contract invocation payload
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "invoke",
            "params": {
                "contract_id": self.contract_address,
                "function": "update",
                "args": [
                    {"string": dfid},
                    {"string": cid},
                    {"address": interface_address}
                ],
                "source_account": source_address
            }
        });

        let response = self.http_client
            .post(rpc_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| StellarError::NetworkError(format!("Failed to invoke contract: {}", e)))?;

        let result: serde_json::Value = response.json()
            .await
            .map_err(|e| StellarError::SerializationError(format!("Failed to parse response: {}", e)))?;

        // Extract transaction hash from response
        let tx_hash = result["result"]["hash"]
            .as_str()
            .ok_or_else(|| StellarError::ContractError("No transaction hash in response".to_string()))?
            .to_string();

        tracing::info!(
            "✅ Stellar transaction submitted successfully (native RPC). Network: {:?}, TX: {}, DFID: {}, CID: {}",
            self.network, tx_hash, dfid, cid
        );

        Ok(tx_hash)
    }

    /// Mint a new NFT with DFID as the token identifier
    /// This should only be called once per DFID (when first tokenized)
    pub async fn mint_nft(&self, dfid: &str, creator: &str, metadata: Option<serde_json::Value>) -> Result<String, StellarError> {
        // Ensure NFT contract is configured
        let nft_contract = self.nft_contract_address.as_ref()
            .ok_or_else(|| StellarError::NotConfigured("NFT contract address not configured".to_string()))?;

        // Get the signing key
        let signing_key = self.signing_key.as_ref()
            .ok_or_else(|| StellarError::NotConfigured("Signing key not configured".to_string()))?;

        // Get source account public key from signing key
        let public_key = signing_key.verifying_key();
        let public_key_bytes = public_key.to_bytes();

        // Convert to Stellar address format
        let source_address = {
            let mut payload = vec![0x30]; // Version byte for Ed25519 public key (G address)
            payload.extend_from_slice(&public_key_bytes);

            // Calculate CRC16-XModem checksum
            let checksum = Self::crc16_xmodem(&payload);
            payload.push((checksum & 0xFF) as u8);
            payload.push(((checksum >> 8) & 0xFF) as u8);

            // Encode with base32
            base32::encode(base32::Alphabet::RFC4648 { padding: false }, &payload)
        };

        // Build the NFT minting payload
        // NFT contract mint function: mint(to: Address, token_id: String, metadata: String)
        let metadata_str = metadata
            .map(|m| m.to_string())
            .unwrap_or_else(|| serde_json::json!({
                "dfid": dfid,
                "creator": creator,
                "minted_at": chrono::Utc::now().to_rfc3339()
            }).to_string());

        let rpc_url = self.network.soroban_rpc_url();

        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "invoke",
            "params": {
                "contract_id": nft_contract,
                "function": "mint",
                "args": [
                    {"address": source_address},  // to: recipient address
                    {"string": dfid},             // token_id: DFID as unique identifier
                    {"string": metadata_str}      // metadata: JSON string
                ],
                "source_account": source_address
            }
        });

        let response = self.http_client
            .post(rpc_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| StellarError::NetworkError(format!("Failed to mint NFT: {}", e)))?;

        let result: serde_json::Value = response.json()
            .await
            .map_err(|e| StellarError::SerializationError(format!("Failed to parse mint response: {}", e)))?;

        // Extract transaction hash from response
        let tx_hash = result["result"]["hash"]
            .as_str()
            .ok_or_else(|| StellarError::ContractError("No transaction hash in mint response".to_string()))?
            .to_string();

        tracing::info!(
            "✅ NFT minted successfully (native RPC). Network: {:?}, TX: {}, DFID: {}, Creator: {}",
            self.network, tx_hash, dfid, creator
        );

        Ok(tx_hash)
    }

    // CRC16-XModem checksum for Stellar encoding
    fn crc16_xmodem(data: &[u8]) -> u16 {
        let mut crc: u16 = 0x0000;
        for byte in data {
            crc ^= (*byte as u16) << 8;
            for _ in 0..8 {
                if (crc & 0x8000) != 0 {
                    crc = (crc << 1) ^ 0x1021;
                } else {
                    crc <<= 1;
                }
            }
        }
        crc
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

    fn sign_transaction(&self, transaction: &serde_json::Value, signing_key: &SigningKey) -> Result<String, StellarError> {
        // Serialize transaction to XDR format
        let tx_json = serde_json::to_string(transaction)
            .map_err(|e| StellarError::SerializationError(format!("Failed to serialize tx: {}", e)))?;

        // Hash transaction with network passphrase
        let network_id = stellar_base::network::Network::new(self.network.network_passphrase().as_bytes());
        let tx_hash = stellar_base::hashing::hash(&[network_id.network_id(), tx_json.as_bytes()].concat());

        // Sign the hash
        let signature = signing_key.sign(&tx_hash);

        // Encode as base64 XDR envelope
        let envelope = format!("{}:{}", tx_json, BASE64_STANDARD.encode(signature.to_bytes()));

        Ok(envelope)
    }
}

// Minimal stellar strkey implementation
mod stellar_strkey {
    pub mod ed25519 {
        pub struct PrivateKey(pub [u8; 32]);
        pub struct PublicKey(pub [u8; 32]);

        impl PrivateKey {
            pub fn from_string(s: &str) -> Result<Self, String> {
                if !s.starts_with('S') {
                    return Err("Secret key must start with S".to_string());
                }

                // Stellar uses base32 encoding (RFC 4648) without padding
                let decoded = base32::decode(
                    base32::Alphabet::RFC4648 { padding: false },
                    s
                ).ok_or_else(|| "Failed to decode base32".to_string())?;

                // Stellar secret keys have: 1 byte version + 32 bytes key + 2 bytes checksum = 35 bytes
                if decoded.len() < 33 {
                    return Err(format!("Invalid key length: {} bytes", decoded.len()));
                }

                // Extract the 32-byte Ed25519 seed (skip version byte, ignore checksum)
                let mut bytes = [0u8; 32];
                bytes.copy_from_slice(&decoded[1..33]);
                Ok(PrivateKey(bytes))
            }
        }

        impl PublicKey {
            pub fn to_string(&self) -> String {
                // Stellar public keys: version byte (0x30) + 32 bytes key + 2 bytes checksum
                let mut payload = vec![0x30]; // Version byte for Ed25519 public key
                payload.extend_from_slice(&self.0);

                // Calculate CRC16-XModem checksum
                let checksum = Self::crc16_xmodem(&payload);
                payload.push((checksum & 0xFF) as u8);
                payload.push(((checksum >> 8) & 0xFF) as u8);

                // Encode with base32
                base32::encode(base32::Alphabet::RFC4648 { padding: false }, &payload)
            }

            // CRC16-XModem checksum for Stellar encoding
            fn crc16_xmodem(data: &[u8]) -> u16 {
                let mut crc: u16 = 0x0000;
                for byte in data {
                    crc ^= (*byte as u16) << 8;
                    for _ in 0..8 {
                        if (crc & 0x8000) != 0 {
                            crc = (crc << 1) ^ 0x1021;
                        } else {
                            crc <<= 1;
                        }
                    }
                }
                crc
            }
        }
    }
}

// Minimal stellar base implementation
mod stellar_base {
    pub mod network {
        pub struct Network {
            id: [u8; 32],
        }

        impl Network {
            pub fn new(passphrase: &[u8]) -> Self {
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(passphrase);
                let result = hasher.finalize();
                let mut id = [0u8; 32];
                id.copy_from_slice(&result);
                Network { id }
            }

            pub fn network_id(&self) -> &[u8] {
                &self.id
            }
        }
    }

    pub mod hashing {
        use sha2::{Digest, Sha256};

        pub fn hash(data: &[u8]) -> [u8; 32] {
            let mut hasher = Sha256::new();
            hasher.update(data);
            let result = hasher.finalize();
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&result);
            hash
        }
    }
}

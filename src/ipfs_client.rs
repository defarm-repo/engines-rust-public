use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug)]
pub enum IpfsError {
    NetworkError(String),
    UploadError(String),
    RetrievalError(String),
    SerializationError(String),
    NotConfigured(String),
}

impl std::fmt::Display for IpfsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IpfsError::NetworkError(e) => write!(f, "Network error: {e}"),
            IpfsError::UploadError(e) => write!(f, "Upload error: {e}"),
            IpfsError::RetrievalError(e) => write!(f, "Retrieval error: {e}"),
            IpfsError::SerializationError(e) => write!(f, "Serialization error: {e}"),
            IpfsError::NotConfigured(e) => write!(f, "Not configured: {e}"),
        }
    }
}

impl std::error::Error for IpfsError {}

impl From<reqwest::Error> for IpfsError {
    fn from(e: reqwest::Error) -> Self {
        IpfsError::NetworkError(e.to_string())
    }
}

impl From<serde_json::Error> for IpfsError {
    fn from(e: serde_json::Error) -> Self {
        IpfsError::SerializationError(e.to_string())
    }
}

#[derive(Debug, Clone)]
pub enum IpfsClientType {
    Kubo { endpoint: String },
    Pinata { api_key: String, secret: String },
}

pub struct IpfsClient {
    client_type: IpfsClientType,
    http_client: Client,
}

impl std::fmt::Debug for IpfsClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IpfsClient")
            .field("client_type", &self.client_type)
            .finish()
    }
}

impl Clone for IpfsClient {
    fn clone(&self) -> Self {
        Self {
            client_type: self.client_type.clone(),
            http_client: Client::new(),
        }
    }
}

#[derive(Deserialize)]
struct PinataResponse {
    #[serde(rename = "IpfsHash")]
    ipfs_hash: String,
}

#[derive(Deserialize)]
struct KuboAddResponse {
    #[serde(rename = "Hash")]
    hash: String,
}

impl IpfsClient {
    /// Create client for local Kubo (IPFS) node
    pub fn with_endpoint(endpoint: &str) -> Result<Self, IpfsError> {
        Ok(Self {
            client_type: IpfsClientType::Kubo {
                endpoint: endpoint.to_string(),
            },
            http_client: Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .map_err(|e| {
                    IpfsError::NetworkError(format!("Failed to create HTTP client: {e}"))
                })?,
        })
    }

    /// Create client for Pinata service
    pub fn with_pinata(api_key: String, secret: String) -> Result<Self, IpfsError> {
        if api_key.is_empty() || secret.is_empty() {
            return Err(IpfsError::NotConfigured(
                "Pinata API key or secret is empty".to_string(),
            ));
        }

        Ok(Self {
            client_type: IpfsClientType::Pinata { api_key, secret },
            http_client: Client::builder()
                .timeout(Duration::from_secs(120))
                .build()
                .map_err(|e| {
                    IpfsError::NetworkError(format!("Failed to create HTTP client: {e}"))
                })?,
        })
    }

    /// Upload JSON data to IPFS and return CID
    pub async fn upload_json<T: Serialize>(&self, data: &T) -> Result<String, IpfsError> {
        let json_data = serde_json::to_string(data)?;

        match &self.client_type {
            IpfsClientType::Kubo { endpoint } => self.kubo_upload(&json_data, endpoint).await,
            IpfsClientType::Pinata { .. } => self.pinata_upload_json(data, None).await,
        }
    }

    /// Get JSON data from IPFS by CID
    pub async fn get_json<T: for<'de> Deserialize<'de>>(&self, cid: &str) -> Result<T, IpfsError> {
        let data = match &self.client_type {
            IpfsClientType::Kubo { endpoint } => self.kubo_get(cid, endpoint).await?,
            IpfsClientType::Pinata { .. } => {
                // Use public gateway for retrieval
                self.get_from_gateway(cid, "https://gateway.pinata.cloud")
                    .await?
            }
        };

        serde_json::from_str(&data)
            .map_err(|e| IpfsError::SerializationError(format!("Failed to deserialize JSON: {e}")))
    }

    /// Pin content (for Kubo, this is automatic; for Pinata, already pinned on upload)
    pub async fn pin(&self, cid: &str) -> Result<(), IpfsError> {
        match &self.client_type {
            IpfsClientType::Kubo { endpoint } => {
                let url = format!("{endpoint}/api/v0/pin/add?arg={cid}");
                let response = self.http_client.post(&url).send().await?;

                if !response.status().is_success() {
                    let error_text = response.text().await.unwrap_or_default();
                    return Err(IpfsError::UploadError(format!(
                        "Failed to pin: {error_text}"
                    )));
                }

                Ok(())
            }
            IpfsClientType::Pinata { .. } => {
                // Pinata pins automatically on upload
                Ok(())
            }
        }
    }

    pub async fn health_check(&self) -> Result<bool, IpfsError> {
        match &self.client_type {
            IpfsClientType::Kubo { endpoint } => {
                let url = format!("{endpoint}/api/v0/version");
                let response = self.http_client.post(&url).send().await?;
                Ok(response.status().is_success())
            }
            IpfsClientType::Pinata { api_key, secret } => {
                let url = "https://api.pinata.cloud/data/testAuthentication";
                let response = self
                    .http_client
                    .get(url)
                    .header("pinata_api_key", api_key)
                    .header("pinata_secret_api_key", secret)
                    .send()
                    .await?;
                Ok(response.status().is_success())
            }
        }
    }

    pub async fn node_info(&self) -> Result<String, IpfsError> {
        match &self.client_type {
            IpfsClientType::Kubo { endpoint } => {
                let url = format!("{endpoint}/api/v0/version");
                let response = self.http_client.post(&url).send().await?;
                let version: serde_json::Value = response.json().await?;
                Ok(format!(
                    "Kubo {}",
                    version["Version"].as_str().unwrap_or("unknown")
                ))
            }
            IpfsClientType::Pinata { .. } => Ok("Pinata Cloud".to_string()),
        }
    }

    // Private helper methods

    async fn kubo_upload(&self, json_data: &str, endpoint: &str) -> Result<String, IpfsError> {
        let url = format!("{endpoint}/api/v0/add");

        let form = reqwest::multipart::Form::new().text("file", json_data.to_string());

        let response = self.http_client.post(&url).multipart(form).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(IpfsError::UploadError(format!(
                "Kubo upload failed: {error_text}"
            )));
        }

        let result: KuboAddResponse = response.json().await?;
        Ok(result.hash)
    }

    pub async fn pinata_upload_json<T: serde::Serialize>(
        &self,
        data: &T,
        _name: Option<String>,
    ) -> Result<String, IpfsError> {
        let (api_key, secret) = match &self.client_type {
            IpfsClientType::Pinata { api_key, secret } => (api_key, secret),
            _ => {
                return Err(IpfsError::NotConfigured(
                    "Not configured for Pinata".to_string(),
                ))
            }
        };

        let url = "https://api.pinata.cloud/pinning/pinJSONToIPFS";

        let _json_data = serde_json::to_string(data)?;

        let response = self
            .http_client
            .post(url)
            .header("pinata_api_key", api_key)
            .header("pinata_secret_api_key", secret)
            .header("Content-Type", "application/json")
            .json(data)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(IpfsError::UploadError(format!(
                "Pinata upload failed ({status}): {error_text}"
            )));
        }

        let result: PinataResponse = response.json().await?;
        Ok(result.ipfs_hash)
    }

    async fn kubo_get(&self, cid: &str, endpoint: &str) -> Result<String, IpfsError> {
        let url = format!("{endpoint}/api/v0/cat?arg={cid}");

        let response = self.http_client.post(&url).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(IpfsError::RetrievalError(format!(
                "Failed to retrieve from Kubo: {error_text}"
            )));
        }

        Ok(response.text().await?)
    }

    async fn get_from_gateway(&self, cid: &str, gateway: &str) -> Result<String, IpfsError> {
        let url = format!("{gateway}/ipfs/{cid}");

        let response = self.http_client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(IpfsError::RetrievalError(format!(
                "Failed to retrieve from gateway: {}",
                response.status()
            )));
        }

        Ok(response.text().await?)
    }
}

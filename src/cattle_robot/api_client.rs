use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;

const MAX_RETRIES: u32 = 3;
const RETRY_DELAY_MS: u64 = 1000;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("HTTP request failed: {0}")]
    RequestFailed(String),

    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("API returned error: {status} - {message}")]
    ApiError { status: u16, message: String },

    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Rate limit exceeded")]
    RateLimited,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLocalItemRequest {
    pub enhanced_identifiers: Vec<EnhancedIdentifier>,
    pub enriched_data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedIdentifier {
    pub namespace: String,
    pub key: String,
    pub value: String,
    pub id_type: String, // "Canonical" or "Contextual"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLocalItemResponse {
    pub local_id: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushLocalRequest {
    pub local_id: String,
    pub requester_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushLocalResponse {
    pub dfid: String,
    pub operation_id: String,
    pub storage_metadata: Option<StorageMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageMetadata {
    pub adapter_type: String,
    pub cid: Option<String>,
    pub stellar_tx: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitResponse {
    pub id: String,
    pub name: String,
    pub owner_id: String,
}

pub struct RailwayApiClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl RailwayApiClient {
    pub fn new(base_url: String, api_key: String) -> Result<Self, ApiError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(ApiError::NetworkError)?;

        Ok(Self {
            client,
            base_url,
            api_key,
        })
    }

    /// Create a local item (step 1 of tokenization)
    pub async fn create_local_item(
        &self,
        identifiers: Vec<EnhancedIdentifier>,
        enriched_data: serde_json::Value,
    ) -> Result<CreateLocalItemResponse, ApiError> {
        let url = format!("{}/api/items/local", self.base_url);

        let request_body = CreateLocalItemRequest {
            enhanced_identifiers: identifiers,
            enriched_data,
        };

        let response = self
            .client
            .post(&url)
            .header("X-API-Key", &self.api_key)
            .json(&request_body)
            .send()
            .await
            .map_err(ApiError::NetworkError)?;

        let status = response.status();

        if status == StatusCode::TOO_MANY_REQUESTS {
            return Err(ApiError::RateLimited);
        }

        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ApiError::AuthFailed(error_text));
        }

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ApiError::ApiError {
                status: status.as_u16(),
                message: error_text,
            });
        }

        let response_body: CreateLocalItemResponse =
            response.json().await.map_err(ApiError::NetworkError)?;

        Ok(response_body)
    }

    /// Push local item to circuit for tokenization (step 2)
    pub async fn push_local_to_circuit(
        &self,
        circuit_id: &str,
        local_id: &str,
        requester_id: &str,
    ) -> Result<PushLocalResponse, ApiError> {
        let url = format!("{}/api/circuits/{}/push-local", self.base_url, circuit_id);

        let request_body = PushLocalRequest {
            local_id: local_id.to_string(),
            requester_id: requester_id.to_string(),
        };

        let mut retries = 0;
        loop {
            let response = self
                .client
                .post(&url)
                .header("X-API-Key", &self.api_key)
                .json(&request_body)
                .send()
                .await
                .map_err(ApiError::NetworkError)?;

            let status = response.status();

            if status == StatusCode::TOO_MANY_REQUESTS {
                if retries < MAX_RETRIES {
                    retries += 1;
                    tokio::time::sleep(Duration::from_millis(RETRY_DELAY_MS * retries as u64))
                        .await;
                    continue;
                }
                return Err(ApiError::RateLimited);
            }

            if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
                let error_text = response.text().await.unwrap_or_default();
                return Err(ApiError::AuthFailed(error_text));
            }

            if !status.is_success() {
                let error_text = response.text().await.unwrap_or_default();
                return Err(ApiError::ApiError {
                    status: status.as_u16(),
                    message: error_text,
                });
            }

            let response_body: PushLocalResponse =
                response.json().await.map_err(ApiError::NetworkError)?;

            return Ok(response_body);
        }
    }

    /// Create a new circuit for robot operations
    pub async fn create_circuit(
        &self,
        name: &str,
        description: &str,
    ) -> Result<CircuitResponse, ApiError> {
        let url = format!("{}/api/circuits", self.base_url);

        let request_body = json!({
            "name": name,
            "description": description,
            "default_namespace": "bovino",
            "auto_apply_namespace": true,
            "use_fingerprint": false,
            "require_approval_for_push": false,
            "require_approval_for_pull": false,
        });

        let response = self
            .client
            .post(&url)
            .header("X-API-Key", &self.api_key)
            .json(&request_body)
            .send()
            .await
            .map_err(ApiError::NetworkError)?;

        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ApiError::ApiError {
                status: status.as_u16(),
                message: error_text,
            });
        }

        let response_body: CircuitResponse =
            response.json().await.map_err(ApiError::NetworkError)?;

        Ok(response_body)
    }

    /// Configure circuit adapter (Stellar Testnet + IPFS)
    pub async fn configure_circuit_adapter(&self, circuit_id: &str) -> Result<(), ApiError> {
        let url = format!("{}/api/circuits/{}/adapter", self.base_url, circuit_id);

        let request_body = json!({
            "adapter_type": "StellarTestnetIpfs",
            "require_approval": false,
            "sponsor_adapter_access": true,
            "use_onchain_storage": false,
        });

        let response = self
            .client
            .put(&url)
            .header("X-API-Key", &self.api_key)
            .json(&request_body)
            .send()
            .await
            .map_err(ApiError::NetworkError)?;

        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ApiError::ApiError {
                status: status.as_u16(),
                message: error_text,
            });
        }

        Ok(())
    }

    /// Health check
    pub async fn health_check(&self) -> Result<bool, ApiError> {
        let url = format!("{}/health", self.base_url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(ApiError::NetworkError)?;

        Ok(response.status().is_success())
    }
}

impl EnhancedIdentifier {
    pub fn canonical(namespace: &str, key: &str, value: &str) -> Self {
        Self {
            namespace: namespace.to_string(),
            key: key.to_string(),
            value: value.to_string(),
            id_type: "Canonical".to_string(),
        }
    }

    pub fn contextual(namespace: &str, key: &str, value: &str) -> Self {
        Self {
            namespace: namespace.to_string(),
            key: key.to_string(),
            value: value.to_string(),
            id_type: "Contextual".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enhanced_identifier_creation() {
        let canonical = EnhancedIdentifier::canonical("bovino", "sisbov", "BR123456789012");
        assert_eq!(canonical.id_type, "Canonical");
        assert_eq!(canonical.namespace, "bovino");

        let contextual = EnhancedIdentifier::contextual("bovino", "owner", "hash:owner:abc123");
        assert_eq!(contextual.id_type, "Contextual");
    }
}

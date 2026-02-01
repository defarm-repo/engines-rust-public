use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DfidClientError {
    #[error("HTTP request failed: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("DFID service returned error: {0}")]
    ServiceError(String),

    #[error("Invalid response from DFID service")]
    InvalidResponse,

    #[error("DFID generation failed: {0}")]
    GenerationFailed(String),
}

#[derive(Clone)]
pub struct DfidClient {
    base_url: String,
    client: Client,
}

#[derive(Serialize)]
struct GenerateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    context: Option<String>,
    count: usize,
}

#[derive(Deserialize)]
struct GenerateResponse {
    dfids: Vec<String>,
}

#[derive(Deserialize)]
struct ValidateResponse {
    valid: bool,
    #[allow(dead_code)]
    checksum_ok: bool,
    #[allow(dead_code)]
    error: Option<String>,
}

impl DfidClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("Failed to build HTTP client"),
        }
    }

    pub async fn generate_dfid(&self, context: Option<String>) -> Result<String, DfidClientError> {
        let request = GenerateRequest { context, count: 1 };

        let response = self
            .client
            .post(format!("{}/dfid/generate", self.base_url))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(DfidClientError::ServiceError(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let mut generate_response: GenerateResponse = response.json().await?;

        generate_response
            .dfids
            .pop()
            .ok_or(DfidClientError::InvalidResponse)
    }

    pub async fn generate_batch(
        &self,
        count: usize,
        context: Option<String>,
    ) -> Result<Vec<String>, DfidClientError> {
        let request = GenerateRequest { context, count };

        let response = self
            .client
            .post(format!("{}/dfid/batch", self.base_url))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(DfidClientError::ServiceError(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let generate_response: GenerateResponse = response.json().await?;
        Ok(generate_response.dfids)
    }

    pub async fn validate_dfid(&self, dfid: &str) -> Result<bool, DfidClientError> {
        let response = self
            .client
            .get(format!("{}/dfid/{}/validate", self.base_url, dfid))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(DfidClientError::ServiceError(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let validate_response: ValidateResponse = response.json().await?;
        Ok(validate_response.valid)
    }

    pub async fn health_check(&self) -> Result<bool, DfidClientError> {
        let response = self
            .client
            .get(format!("{}/health", self.base_url))
            .send()
            .await?;

        Ok(response.status().is_success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires running DFID service
    async fn test_generate_dfid() {
        let client = DfidClient::new("http://localhost:3001".to_string());
        let dfid = client.generate_dfid(Some("test".to_string())).await;

        assert!(dfid.is_ok());
        let dfid = dfid.unwrap();
        assert!(dfid.starts_with("DFID-"));
    }

    #[tokio::test]
    #[ignore] // Requires running DFID service
    async fn test_generate_batch() {
        let client = DfidClient::new("http://localhost:3001".to_string());
        let dfids = client.generate_batch(10, None).await;

        assert!(dfids.is_ok());
        let dfids = dfids.unwrap();
        assert_eq!(dfids.len(), 10);

        // Verify uniqueness
        let unique: std::collections::HashSet<_> = dfids.iter().collect();
        assert_eq!(unique.len(), 10);
    }

    #[tokio::test]
    #[ignore] // Requires running DFID service
    async fn test_validate_dfid() {
        let client = DfidClient::new("http://localhost:3001".to_string());

        // Generate a valid DFID first
        let dfid = client.generate_dfid(None).await.unwrap();

        // Validate it
        let valid = client.validate_dfid(&dfid).await;
        assert!(valid.is_ok());
        assert!(valid.unwrap());

        // Test invalid DFID
        let invalid = client.validate_dfid("INVALID-DFID").await;
        assert!(invalid.is_ok());
        assert!(!invalid.unwrap());
    }

    #[tokio::test]
    #[ignore] // Requires running DFID service
    async fn test_health_check() {
        let client = DfidClient::new("http://localhost:3001".to_string());
        let health = client.health_check().await;

        assert!(health.is_ok());
        assert!(health.unwrap());
    }
}

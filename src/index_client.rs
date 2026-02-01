use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error as StdError;
use uuid::Uuid;

#[derive(Clone)]
pub struct IndexClient {
    base_url: String,
    client: Client,
}

#[derive(Debug, Serialize)]
pub struct RegisterLocationRequest {
    pub dfid: String,
    pub location: LocationInput,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LocationInput {
    Circuit {
        circuit_id: Uuid,
        circuit_name: String,
        url: String,
    },
    Blockchain {
        chain: String,
        tx_hash: String,
        url: String,
    },
    Ipfs {
        cid: String,
        url: String,
    },
    Registry {
        registry_name: String,
        registry_url: String,
    },
}

#[derive(Debug, Deserialize)]
pub struct RegisterLocationResponse {
    pub success: bool,
    pub location_id: Uuid,
    pub dfid: String,
    pub registered_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct GetLocationsResponse {
    pub dfid: String,
    pub locations: Vec<DfidLocation>,
    pub total_locations: usize,
}

#[derive(Debug, Deserialize)]
pub struct DfidLocation {
    pub location_id: Uuid,
    pub dfid: String,
    pub location_type: serde_json::Value,
    pub location_url: String,
    pub metadata: serde_json::Value,
    pub registered_by: String,
    pub registered_at: DateTime<Utc>,
    pub verified: bool,
    pub last_verified: Option<DateTime<Utc>>,
}

#[derive(Debug)]
pub enum IndexClientError {
    RequestFailed(String),
    ParseError(String),
    ServiceUnavailable,
}

impl std::fmt::Display for IndexClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexClientError::RequestFailed(msg) => write!(f, "Request failed: {}", msg),
            IndexClientError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            IndexClientError::ServiceUnavailable => write!(f, "Index service unavailable"),
        }
    }
}

impl StdError for IndexClientError {}

impl IndexClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: Client::new(),
        }
    }

    /// Register a new location for a DFID
    pub async fn register_location(
        &self,
        request: RegisterLocationRequest,
    ) -> Result<RegisterLocationResponse, IndexClientError> {
        let url = format!("{}/index/register", self.base_url);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| IndexClientError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(IndexClientError::ServiceUnavailable);
        }

        let result = response
            .json::<RegisterLocationResponse>()
            .await
            .map_err(|e| IndexClientError::ParseError(e.to_string()))?;

        Ok(result)
    }

    /// Get all locations for a DFID
    pub async fn get_locations(
        &self,
        dfid: &str,
    ) -> Result<GetLocationsResponse, IndexClientError> {
        let url = format!("{}/index/{}/locations", self.base_url, dfid);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| IndexClientError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(IndexClientError::ServiceUnavailable);
        }

        let result = response
            .json::<GetLocationsResponse>()
            .await
            .map_err(|e| IndexClientError::ParseError(e.to_string()))?;

        Ok(result)
    }

    /// Search for DFIDs by pattern
    pub async fn search_dfids(&self, query: &str) -> Result<Vec<String>, IndexClientError> {
        let url = format!("{}/index/search?q={}", self.base_url, query);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| IndexClientError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(IndexClientError::ServiceUnavailable);
        }

        #[derive(Deserialize)]
        struct SearchResponse {
            dfids: Vec<String>,
        }

        let result = response
            .json::<SearchResponse>()
            .await
            .map_err(|e| IndexClientError::ParseError(e.to_string()))?;

        Ok(result.dfids)
    }
}

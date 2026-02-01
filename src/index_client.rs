use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error as StdError;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Duration;
use uuid::Uuid;

#[derive(Clone)]
pub struct IndexClient {
    base_url: String,
    client: Client,
    retry_queue: Arc<Mutex<Vec<RetryEntry>>>,
}

#[derive(Clone, Debug)]
struct RetryEntry {
    request: RegisterLocationRequest,
    attempts: u32,
    last_attempt: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RegisterLocationRequest {
    pub dfid: String,
    pub location: LocationInput,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Clone, Debug, Serialize)]
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
            IndexClientError::RequestFailed(msg) => write!(f, "Request failed: {msg}"),
            IndexClientError::ParseError(msg) => write!(f, "Parse error: {msg}"),
            IndexClientError::ServiceUnavailable => write!(f, "Index service unavailable"),
        }
    }
}

impl StdError for IndexClientError {}

impl IndexClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap(),
            retry_queue: Arc::new(Mutex::new(Vec::new())),
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

    /// Register location with automatic retry queue on failure
    pub async fn register_location_with_retry(&self, request: RegisterLocationRequest) {
        match self.register_location(request.clone()).await {
            Ok(_) => {
                tracing::debug!("Successfully registered DFID {} in index", request.dfid);
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to register DFID {} in index: {}. Adding to retry queue.",
                    request.dfid,
                    e
                );

                let mut queue = self.retry_queue.lock().await;
                queue.push(RetryEntry {
                    request,
                    attempts: 0,
                    last_attempt: Utc::now(),
                });
            }
        }
    }

    /// Process retry queue (should be called periodically in background)
    pub async fn process_retry_queue(&self) {
        const MAX_RETRIES: u32 = 5;
        const RETRY_INTERVAL: Duration = Duration::from_secs(60);

        let mut queue = self.retry_queue.lock().await;
        let mut to_retry = Vec::new();
        let mut to_keep = Vec::new();

        for entry in queue.drain(..) {
            let elapsed = Utc::now().signed_duration_since(entry.last_attempt);

            // Only retry if enough time has passed
            if elapsed.num_seconds() >= RETRY_INTERVAL.as_secs() as i64 {
                to_retry.push(entry);
            } else {
                to_keep.push(entry);
            }
        }

        drop(queue); // Release lock while processing

        for mut entry in to_retry {
            entry.attempts += 1;
            entry.last_attempt = Utc::now();

            match self.register_location(entry.request.clone()).await {
                Ok(_) => {
                    tracing::info!(
                        "✅ Retry successful for DFID {} after {} attempts",
                        entry.request.dfid,
                        entry.attempts
                    );
                }
                Err(e) => {
                    if entry.attempts >= MAX_RETRIES {
                        tracing::error!(
                            "❌ Giving up on DFID {} after {} attempts: {}",
                            entry.request.dfid,
                            entry.attempts,
                            e
                        );
                    } else {
                        tracing::warn!(
                            "Retry {}/{} failed for DFID {}: {}",
                            entry.attempts,
                            MAX_RETRIES,
                            entry.request.dfid,
                            e
                        );
                        to_keep.push(entry);
                    }
                }
            }
        }

        // Put failed entries back in queue
        let mut queue = self.retry_queue.lock().await;
        queue.extend(to_keep);

        if !queue.is_empty() {
            tracing::debug!("Retry queue size: {}", queue.len());
        }
    }

    /// Get retry queue size (for monitoring)
    pub async fn get_retry_queue_size(&self) -> usize {
        self.retry_queue.lock().await.len()
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

    /// Spawn background task to process retry queue
    pub fn spawn_retry_processor(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                self.process_retry_queue().await;
            }
        });
    }
}

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DfidLocation {
    pub location_id: Uuid,
    pub dfid: String,
    pub location_type: LocationType,
    pub location_url: String,
    pub metadata: serde_json::Value,
    pub registered_by: String,
    pub registered_at: DateTime<Utc>,
    pub verified: bool,
    pub last_verified: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LocationType {
    Circuit {
        circuit_id: Uuid,
        circuit_name: String,
    },
    Blockchain {
        chain: String,
        tx_hash: String,
    },
    Ipfs {
        cid: String,
    },
    Registry {
        registry_name: String,
        registry_url: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterLocationRequest {
    pub dfid: String,
    pub location: LocationInput,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Serialize)]
pub struct GetLocationsResponse {
    pub dfid: String,
    pub locations: Vec<DfidLocation>,
    pub total_locations: usize,
}

#[derive(Debug, Serialize)]
pub struct RegisterLocationResponse {
    pub success: bool,
    pub location_id: Uuid,
    pub dfid: String,
    pub registered_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub query: String,
    pub dfids: Vec<String>,
    pub total: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum IndexError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("DFID not found: {0}")]
    DfidNotFound(String),

    #[error("Invalid location data: {0}")]
    InvalidLocation(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

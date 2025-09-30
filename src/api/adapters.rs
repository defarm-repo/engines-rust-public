use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use chrono::{DateTime, Utc};

use crate::adapters::{AdapterRegistry, AdapterInstance, LocalLocalAdapter, IpfsIpfsAdapter, StellarTestnetIpfsAdapter, StellarMainnetIpfsAdapter, LocalIpfsAdapter, StellarMainnetStellarMainnetAdapter, StorageAdapter};
use crate::types::{AdapterType, StorageBackendType};
use crate::api::shared_state::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct AdapterInfo {
    pub adapter_type: AdapterType,
    pub name: String,
    pub description: String,
    pub item_storage: StorageBackendType,
    pub event_storage: StorageBackendType,
    pub requires_blockchain: bool,
    pub available: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClientAdapterPreferences {
    pub client_id: String,
    pub selected_adapter: AdapterType,
    pub available_adapters: Vec<AdapterType>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct SelectAdapterRequest {
    pub adapter_type: AdapterType,
}

#[derive(Debug, Deserialize)]
pub struct AdapterQuery {
    pub client_id: Option<String>,
    pub tier: Option<String>,
}

async fn list_available_adapters(
    Query(params): Query<AdapterQuery>,
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<Value>, StatusCode> {
    // TODO: In a real implementation, client_id would come from authentication
    let client_id = params.client_id.unwrap_or_else(|| "default_client".to_string());

    // Get available adapters based on client tier/permissions
    let available_adapters = get_client_available_adapters(&client_id, params.tier.as_deref());

    let adapter_infos: Vec<AdapterInfo> = available_adapters.into_iter().map(|adapter_type| {
        let (item_storage, event_storage) = adapter_type.storage_locations();
        AdapterInfo {
            name: adapter_type.to_string(),
            description: adapter_type.description().to_string(),
            item_storage,
            event_storage,
            requires_blockchain: adapter_type.requires_blockchain(),
            available: true,
            adapter_type,
        }
    }).collect();

    Ok(Json(json!({
        "success": true,
        "adapters": adapter_infos,
        "client_id": client_id,
        "count": adapter_infos.len()
    })))
}

async fn select_adapter(
    Query(params): Query<AdapterQuery>,
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<SelectAdapterRequest>,
) -> Result<Json<Value>, StatusCode> {
    let client_id = params.client_id.unwrap_or_else(|| "default_client".to_string());

    // Validate that the client has access to this adapter
    let available_adapters = get_client_available_adapters(&client_id, params.tier.as_deref());

    if !available_adapters.contains(&request.adapter_type) {
        return Ok(Json(json!({
            "success": false,
            "error": "Adapter not available for this client tier",
            "adapter_type": request.adapter_type
        })));
    }

    // TODO: Store client adapter preference in database
    // For now, just return success

    Ok(Json(json!({
        "success": true,
        "message": "Adapter selected successfully",
        "client_id": client_id,
        "adapter_type": request.adapter_type,
        "description": request.adapter_type.description(),
        "updated_at": Utc::now()
    })))
}

async fn get_adapter_status(
    Path(adapter_type_str): Path<String>,
    Query(params): Query<AdapterQuery>,
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<Value>, StatusCode> {
    let adapter_type = match AdapterType::from_string(&adapter_type_str) {
        Ok(adapter_type) => adapter_type,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };

    // Create a temporary adapter instance to check status
    let adapter_instance = create_adapter_instance(&adapter_type);

    match adapter_instance.sync_status().await {
        Ok(status) => {
            Ok(Json(json!({
                "success": true,
                "adapter_type": adapter_type,
                "status": status
            })))
        }
        Err(e) => {
            Ok(Json(json!({
                "success": false,
                "adapter_type": adapter_type,
                "error": format!("Failed to get status: {}", e)
            })))
        }
    }
}

async fn health_check_adapter(
    Path(adapter_type_str): Path<String>,
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<Value>, StatusCode> {
    let adapter_type = match AdapterType::from_string(&adapter_type_str) {
        Ok(adapter_type) => adapter_type,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };

    let adapter_instance = create_adapter_instance(&adapter_type);

    match adapter_instance.health_check().await {
        Ok(healthy) => {
            Ok(Json(json!({
                "success": true,
                "adapter_type": adapter_type,
                "healthy": healthy,
                "checked_at": Utc::now()
            })))
        }
        Err(e) => {
            Ok(Json(json!({
                "success": false,
                "adapter_type": adapter_type,
                "healthy": false,
                "error": format!("Health check failed: {}", e),
                "checked_at": Utc::now()
            })))
        }
    }
}

async fn get_adapter_templates() -> Result<Json<Value>, StatusCode> {
    let templates = json!({
        "local-local": {
            "description": "Local storage only - for development and testing",
            "cost": "Free",
            "scalability": "Limited",
            "decentralization": "None",
            "use_case": "Development and testing"
        },
        "ipfs-ipfs": {
            "description": "Full IPFS storage - decentralized with no blockchain",
            "cost": "IPFS pinning costs",
            "scalability": "High",
            "decentralization": "Full",
            "use_case": "Decentralized storage without blockchain costs"
        },
        "stellar_testnet-ipfs": {
            "description": "Stellar testnet NFTs + IPFS events - for testing blockchain integration",
            "cost": "Free (testnet)",
            "scalability": "High",
            "decentralization": "Full",
            "use_case": "Testing blockchain integration"
        },
        "stellar_mainnet-ipfs": {
            "description": "Stellar mainnet NFTs + IPFS events - production blockchain + IPFS",
            "cost": "Stellar fees + IPFS costs",
            "scalability": "High",
            "decentralization": "Full",
            "use_case": "Production decentralized traceability"
        },
        "local-ipfs": {
            "description": "Local item storage + IPFS events - hybrid approach",
            "cost": "IPFS pinning costs only",
            "scalability": "Medium",
            "decentralization": "Partial",
            "use_case": "Cost-optimized hybrid solution"
        },
        "stellar_mainnet-stellar_mainnet": {
            "description": "Full Stellar mainnet storage - complete on-chain solution",
            "cost": "Full Stellar transaction fees",
            "scalability": "Medium",
            "decentralization": "Full",
            "use_case": "Maximum immutability and transparency"
        }
    });

    Ok(Json(json!({
        "success": true,
        "templates": templates
    })))
}

fn get_client_available_adapters(client_id: &str, tier: Option<&str>) -> Vec<AdapterType> {
    // TODO: Implement proper client tier and permission system
    match tier {
        Some("enterprise") => vec![
            AdapterType::LocalLocal,
            AdapterType::IpfsIpfs,
            AdapterType::StellarTestnetIpfs,
            AdapterType::StellarMainnetIpfs,
            AdapterType::LocalIpfs,
            AdapterType::StellarMainnetStellarMainnet,
        ],
        Some("professional") => vec![
            AdapterType::LocalLocal,
            AdapterType::IpfsIpfs,
            AdapterType::StellarTestnetIpfs,
            AdapterType::LocalIpfs,
        ],
        Some("basic") | None => vec![
            AdapterType::LocalLocal,
            AdapterType::LocalIpfs,
        ],
        _ => vec![AdapterType::LocalLocal],
    }
}

fn create_adapter_instance(adapter_type: &AdapterType) -> AdapterInstance {
    match adapter_type {
        AdapterType::LocalLocal => AdapterInstance::LocalLocal(LocalLocalAdapter::new()),
        AdapterType::IpfsIpfs => AdapterInstance::IpfsIpfs(IpfsIpfsAdapter::new()),
        AdapterType::StellarTestnetIpfs => AdapterInstance::StellarTestnetIpfs(StellarTestnetIpfsAdapter::new()),
        AdapterType::StellarMainnetIpfs => AdapterInstance::StellarMainnetIpfs(StellarMainnetIpfsAdapter::new()),
        AdapterType::LocalIpfs => AdapterInstance::LocalIpfs(LocalIpfsAdapter::new()),
        AdapterType::StellarMainnetStellarMainnet => AdapterInstance::StellarMainnetStellarMainnet(StellarMainnetStellarMainnetAdapter::new()),
        AdapterType::Custom(_) => AdapterInstance::LocalLocal(LocalLocalAdapter::new()), // Fallback
        _ => AdapterInstance::LocalLocal(LocalLocalAdapter::new()), // Fallback
    }
}

pub fn adapter_routes(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(list_available_adapters))
        .route("/select", post(select_adapter))
        .route("/templates", get(get_adapter_templates))
        .route("/:adapter_type/status", get(get_adapter_status))
        .route("/:adapter_type/health", get(health_check_adapter))
        .with_state(app_state)
}
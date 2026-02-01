use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::adapters::{
    AdapterInstance, IpfsIpfsAdapter, StellarMainnetIpfsAdapter, StellarTestnetIpfsAdapter,
    StorageAdapter,
};
use crate::api::auth::Claims;
use crate::api::shared_state::AppState;
use crate::storage::{StorageBackend, StorageError};
use crate::storage_helpers::{with_storage, StorageLockError};
use crate::types::{AdapterType, StorageBackendType, UserTier};

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

#[derive(Debug, Deserialize)]
pub struct RegisterDfidRequest {
    pub dfid: String,
    #[serde(default)]
    pub options: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct RegisterDfidResponse {
    pub success: bool,
    pub dfid: String,
    pub adapter_type: String,
    pub location: serde_json::Value,
    pub registered_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

async fn list_available_adapters(
    State(app_state): State<Arc<AppState>>,
    claims: Option<Extension<Claims>>,
    api_key_ctx: Option<Extension<crate::api_key_middleware::ApiKeyContext>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Auto-populate user_id from authenticated context (JWT or API key)
    let user_id = if let Some(Extension(claims)) = claims {
        claims.user_id.clone()
    } else if let Some(Extension(ctx)) = api_key_ctx {
        ctx.user_id.to_string()
    } else {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Authentication required. Use JWT token or API key."})),
        ));
    };

    // Get user account to check adapter permissions
    let user = with_storage(
        &app_state.shared_storage,
        "adapters.rs::list_available_adapters::read_user",
        |storage| {
            storage.get_user_account(&user_id)?.ok_or_else(|| {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "User not found",
                )) as Box<dyn std::error::Error>
            })
        },
    )
    .map_err(|e| match e {
        StorageLockError::Timeout => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "Service temporarily unavailable"})),
        ),
        StorageLockError::Other(msg) if msg.contains("not found") => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "User not found"})),
        ),
        StorageLockError::Other(msg) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to get user: {}", msg)})),
        ),
    })?;

    // Get available adapters based on user's custom config or tier defaults
    let available_adapters =
        get_client_available_adapters(user.available_adapters.clone(), &user.tier);

    let adapter_infos: Vec<AdapterInfo> = available_adapters
        .into_iter()
        .map(|adapter_type| {
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
        })
        .collect();

    Ok(Json(json!({
        "success": true,
        "adapters": adapter_infos,
        "user_id": user_id,
        "tier": format!("{:?}", user.tier),
        "using_custom_adapters": user.available_adapters.is_some(),
        "count": adapter_infos.len()
    })))
}

async fn select_adapter(
    State(app_state): State<Arc<AppState>>,
    claims: Option<Extension<Claims>>,
    api_key_ctx: Option<Extension<crate::api_key_middleware::ApiKeyContext>>,
    Json(request): Json<SelectAdapterRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Auto-populate user_id from authenticated context (JWT or API key)
    let user_id = if let Some(Extension(claims)) = claims {
        claims.user_id.clone()
    } else if let Some(Extension(ctx)) = api_key_ctx {
        ctx.user_id.to_string()
    } else {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Authentication required. Use JWT token or API key."})),
        ));
    };

    // Get user account to check adapter permissions
    let user = with_storage(
        &app_state.shared_storage,
        "adapters.rs::select_adapter::read_user",
        |storage| {
            storage.get_user_account(&user_id)?.ok_or_else(|| {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "User not found",
                )) as Box<dyn std::error::Error>
            })
        },
    )
    .map_err(|e| match e {
        StorageLockError::Timeout => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "Service temporarily unavailable"})),
        ),
        StorageLockError::Other(msg) if msg.contains("not found") => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "User not found"})),
        ),
        StorageLockError::Other(msg) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to get user: {}", msg)})),
        ),
    })?;

    // Validate that the user has access to this adapter
    let available_adapters =
        get_client_available_adapters(user.available_adapters.clone(), &user.tier);

    if !available_adapters.contains(&request.adapter_type) {
        return Ok(Json(json!({
            "success": false,
            "error": "Adapter not available for your account",
            "adapter_type": request.adapter_type,
            "available_adapters": available_adapters
        })));
    }

    // TODO: Store user's selected adapter preference in database
    // For now, just return success

    Ok(Json(json!({
        "success": true,
        "message": "Adapter selected successfully",
        "user_id": user_id,
        "adapter_type": request.adapter_type,
        "description": request.adapter_type.description(),
        "updated_at": Utc::now()
    })))
}

async fn get_adapter_status(
    Path(adapter_type_str): Path<String>,
    Query(_params): Query<AdapterQuery>,
    State(_app_state): State<Arc<AppState>>,
) -> Result<Json<Value>, StatusCode> {
    let adapter_type = match AdapterType::from_string(&adapter_type_str) {
        Ok(adapter_type) => adapter_type,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };

    // Create a temporary adapter instance to check status
    let adapter_instance = match create_adapter_instance(&adapter_type) {
        Ok(instance) => instance,
        Err(e) => {
            return Ok(Json(json!({
                "success": false,
                "adapter_type": adapter_type,
                "error": format!("Failed to create adapter: {}", e)
            })));
        }
    };

    match adapter_instance.sync_status().await {
        Ok(status) => Ok(Json(json!({
            "success": true,
            "adapter_type": adapter_type,
            "status": status
        }))),
        Err(e) => Ok(Json(json!({
            "success": false,
            "adapter_type": adapter_type,
            "error": format!("Failed to get status: {}", e)
        }))),
    }
}

async fn health_check_adapter(
    Path(adapter_type_str): Path<String>,
    State(_app_state): State<Arc<AppState>>,
) -> Result<Json<Value>, StatusCode> {
    let adapter_type = match AdapterType::from_string(&adapter_type_str) {
        Ok(adapter_type) => adapter_type,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };

    let adapter_instance = match create_adapter_instance(&adapter_type) {
        Ok(instance) => instance,
        Err(e) => {
            return Ok(Json(json!({
                "success": false,
                "adapter_type": adapter_type,
                "error": format!("Failed to create adapter: {}", e)
            })));
        }
    };

    match adapter_instance.health_check().await {
        Ok(healthy) => Ok(Json(json!({
            "success": true,
            "adapter_type": adapter_type,
            "healthy": healthy,
            "checked_at": Utc::now()
        }))),
        Err(e) => Ok(Json(json!({
            "success": false,
            "adapter_type": adapter_type,
            "healthy": false,
            "error": format!("Health check failed: {}", e),
            "checked_at": Utc::now()
        }))),
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

fn get_client_available_adapters(
    user_adapters: Option<Vec<AdapterType>>,
    tier: &UserTier,
) -> Vec<AdapterType> {
    // If user has custom adapters configured, use those
    if let Some(adapters) = user_adapters {
        return adapters;
    }

    // Otherwise, fall back to tier defaults
    match tier {
        UserTier::Admin | UserTier::Enterprise => vec![
            AdapterType::IpfsIpfs,
            AdapterType::StellarTestnetIpfs,
            AdapterType::StellarMainnetIpfs,
        ],
        UserTier::Professional => vec![AdapterType::IpfsIpfs, AdapterType::StellarTestnetIpfs],
        UserTier::Basic => vec![AdapterType::IpfsIpfs],
    }
}

/// Register an existing DFID on an adapter (ad-hoc registration outside circuits)
/// Allows users to anchor same DFID in multiple storage backends
async fn register_dfid_on_adapter(
    State(app_state): State<Arc<AppState>>,
    Path(adapter_type_str): Path<String>,
    claims: Option<Extension<Claims>>,
    api_key_ctx: Option<Extension<crate::api_key_middleware::ApiKeyContext>>,
    Json(payload): Json<RegisterDfidRequest>,
) -> Result<Json<RegisterDfidResponse>, (StatusCode, Json<Value>)> {
    // Auto-populate user_id from authenticated context (JWT or API key)
    let user_id = if let Some(Extension(claims)) = claims {
        claims.user_id.clone()
    } else if let Some(Extension(ctx)) = api_key_ctx {
        ctx.user_id.to_string()
    } else {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Authentication required. Use JWT token or API key."})),
        ));
    };

    // Parse adapter type from path parameter
    let adapter_type = AdapterType::from_string(&adapter_type_str).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "Invalid adapter type",
                "adapter_type": adapter_type_str,
                "supported_types": ["ipfs-ipfs", "stellar-testnet-ipfs", "stellar-mainnet-ipfs"]
            })),
        )
    })?;

    // Get user account to check adapter permissions
    let user = with_storage(
        &app_state.shared_storage,
        "adapters.rs::register_dfid::read_user",
        |storage| {
            storage.get_user_account(&user_id)?.ok_or_else(|| {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "User not found",
                )) as Box<dyn std::error::Error>
            })
        },
    )
    .map_err(|e| match e {
        StorageLockError::Timeout => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "Service temporarily unavailable"})),
        ),
        StorageLockError::Other(msg) if msg.contains("not found") => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "User not found"})),
        ),
        StorageLockError::Other(msg) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to get user: {}", msg)})),
        ),
    })?;

    // Validate that the user has access to this adapter
    let available_adapters =
        get_client_available_adapters(user.available_adapters.clone(), &user.tier);

    if !available_adapters.contains(&adapter_type) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({
                "error": "Adapter not available for your account tier",
                "adapter_type": adapter_type_str,
                "your_tier": format!("{:?}", user.tier),
                "available_adapters": available_adapters.iter().map(|a| a.to_string()).collect::<Vec<_>>(),
                "suggestion": "Upgrade your tier or request adapter access from administrator"
            })),
        ));
    }

    // Get the item with this DFID
    let item = with_storage(
        &app_state.shared_storage,
        "adapters.rs::register_dfid::get_item",
        |storage| {
            storage.get_item_by_dfid(&payload.dfid)?.ok_or_else(|| {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("DFID {} not found", payload.dfid),
                )) as Box<dyn std::error::Error>
            })
        },
    )
    .map_err(|e| match e {
        StorageLockError::Timeout => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "Service temporarily unavailable"})),
        ),
        StorageLockError::Other(msg) if msg.contains("not found") => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": format!("DFID {} not found in your workspace", payload.dfid),
                "suggestion": "Ensure the DFID exists before registering it on an adapter"
            })),
        ),
        StorageLockError::Other(msg) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to get item: {}", msg)})),
        ),
    })?;

    // Create adapter instance
    let adapter_instance = create_adapter_instance(&adapter_type).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": format!("Failed to create adapter: {}", e),
                "adapter_type": adapter_type_str
            })),
        )
    })?;

    // Register item on adapter (is_new_dfid = false since we're registering existing DFID)
    let adapter_result = adapter_instance
        .store_new_item(&item, false, &user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": format!("Failed to register on adapter: {}", e),
                    "adapter_type": adapter_type_str,
                    "dfid": payload.dfid
                })),
            )
        })?;

    // Extract location metadata
    let location = extract_storage_location_json(&adapter_result.metadata.item_location);

    Ok(Json(RegisterDfidResponse {
        success: true,
        dfid: payload.dfid,
        adapter_type: adapter_type_str,
        location,
        registered_at: Utc::now(),
        message: Some(format!(
            "DFID successfully registered on {} adapter",
            adapter_type.to_string()
        )),
    }))
}

/// Helper to convert StorageLocation to JSON
fn extract_storage_location_json(location: &crate::adapters::base::StorageLocation) -> serde_json::Value {
    use crate::adapters::base::StorageLocation;
    match location {
        StorageLocation::Local { id } => json!({
            "type": "local",
            "id": id
        }),
        StorageLocation::IPFS { cid, pinned } => json!({
            "type": "ipfs",
            "cid": cid,
            "pinned": pinned
        }),
        StorageLocation::Stellar {
            transaction_id,
            contract_address,
            asset_id,
        } => json!({
            "type": "stellar",
            "transaction_id": transaction_id,
            "contract_address": contract_address,
            "asset_id": asset_id
        }),
        StorageLocation::Ethereum {
            transaction_hash,
            contract_address,
            token_id,
        } => json!({
            "type": "ethereum",
            "transaction_hash": transaction_hash,
            "contract_address": contract_address,
            "token_id": token_id
        }),
        StorageLocation::Arweave { transaction_id } => json!({
            "type": "arweave",
            "transaction_id": transaction_id
        }),
    }
}

pub fn create_adapter_instance(
    adapter_type: &AdapterType,
) -> Result<AdapterInstance, StorageError> {
    match adapter_type {
        AdapterType::IpfsIpfs => Ok(AdapterInstance::IpfsIpfs(IpfsIpfsAdapter::new()?)),
        AdapterType::StellarTestnetIpfs => Ok(AdapterInstance::StellarTestnetIpfs(
            StellarTestnetIpfsAdapter::new()?,
        )),
        AdapterType::StellarMainnetIpfs => Ok(AdapterInstance::StellarMainnetIpfs(
            StellarMainnetIpfsAdapter::new()?,
        )),
        AdapterType::Custom(_) => Ok(AdapterInstance::IpfsIpfs(IpfsIpfsAdapter::new()?)), // Fallback to IpfsIpfs
        _ => Ok(AdapterInstance::IpfsIpfs(IpfsIpfsAdapter::new()?)), // Fallback to IpfsIpfs
    }
}

pub fn adapter_routes(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(list_available_adapters))
        .route("/select", post(select_adapter))
        .route("/templates", get(get_adapter_templates))
        .route("/:adapter_type/status", get(get_adapter_status))
        .route("/:adapter_type/health", get(health_check_adapter))
        .route("/:adapter_type/register", post(register_dfid_on_adapter))
        .with_state(app_state)
}

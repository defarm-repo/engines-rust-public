//! Snapshot API Endpoints
//!
//! REST API for Git-like state snapshot management.
//! Provides endpoints to query item and circuit snapshot history.

use axum::http::StatusCode;
use axum::{
    extract::{Path, Query, State},
    routing::get,
    Extension, Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

use crate::api::shared_state::AppState;
use crate::snapshot_types::{SnapshotEntityType, StateSnapshot};
use crate::storage::StorageBackend;

// ============================================================================
// REQUEST/RESPONSE TYPES
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct SnapshotQuery {
    /// Limit number of results
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct SnapshotResponse {
    pub snapshot_id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub version: u64,
    pub parent_hash: Option<String>,
    pub operation: String,
    pub operation_description: String,
    pub ipfs_cid: Option<String>,
    pub blockchain_tx: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub created_by: String,
    pub message: Option<String>,
    /// URLs for viewing snapshot data
    pub urls: SnapshotUrlsResponse,
}

#[derive(Debug, Serialize)]
pub struct SnapshotUrlsResponse {
    pub ipfs_url: Option<String>,
    pub stellar_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SnapshotListResponse {
    pub entity_id: String,
    pub entity_type: String,
    pub total_snapshots: usize,
    pub latest_version: u64,
    pub snapshots: Vec<SnapshotResponse>,
}

#[derive(Debug, Serialize)]
pub struct SnapshotDetailResponse {
    pub snapshot: SnapshotResponse,
    /// Full state at this snapshot (if available)
    pub state: Option<Value>,
}

impl From<&StateSnapshot> for SnapshotResponse {
    fn from(s: &StateSnapshot) -> Self {
        let urls = s.get_urls();
        Self {
            snapshot_id: s.snapshot_id.clone(),
            entity_type: s.entity_type.to_string(),
            entity_id: s.entity_id.clone(),
            version: s.version,
            parent_hash: s.parent_hash.clone(),
            operation: format!("{:?}", s.operation),
            operation_description: s.operation.description(),
            ipfs_cid: s.ipfs_cid.clone(),
            blockchain_tx: s.blockchain_tx.clone(),
            timestamp: s.timestamp,
            created_by: s.created_by.clone(),
            message: s.message.clone(),
            urls: SnapshotUrlsResponse {
                ipfs_url: urls.ipfs_url,
                stellar_url: urls.stellar_url,
            },
        }
    }
}

// ============================================================================
// ROUTER SETUP
// ============================================================================

pub fn create_snapshot_routes() -> Router<Arc<AppState>> {
    Router::new()
        // Item snapshots
        .route("/items/:dfid/snapshots", get(list_item_snapshots))
        .route(
            "/items/:dfid/snapshots/latest",
            get(get_latest_item_snapshot),
        )
        .route(
            "/items/:dfid/snapshots/:snapshot_id",
            get(get_item_snapshot),
        )
        // Circuit snapshots
        .route(
            "/circuits/:circuit_id/snapshots",
            get(list_circuit_snapshots),
        )
        .route(
            "/circuits/:circuit_id/snapshots/latest",
            get(get_latest_circuit_snapshot),
        )
        .route(
            "/circuits/:circuit_id/snapshots/:snapshot_id",
            get(get_circuit_snapshot),
        )
}

/// Public snapshot routes (no authentication required for public circuits)
pub fn create_public_snapshot_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/items/:dfid/snapshots", get(list_public_item_snapshots))
        .route(
            "/items/:dfid/snapshots/latest",
            get(get_latest_public_item_snapshot),
        )
}

// ============================================================================
// ITEM SNAPSHOT HANDLERS
// ============================================================================

/// List all snapshots for an item
async fn list_item_snapshots(
    State(state): State<Arc<AppState>>,
    claims: Option<Extension<crate::api::auth::Claims>>,
    api_key_ctx: Option<Extension<crate::api_key_middleware::ApiKeyContext>>,
    Path(dfid): Path<String>,
    Query(query): Query<SnapshotQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Authenticate
    let _user_id = match (&claims, &api_key_ctx) {
        (Some(Extension(claims)), _) => claims.user_id.clone(),
        (_, Some(Extension(ctx))) => ctx.user_id.to_string(),
        _ => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "success": false,
                    "error": "Authentication required"
                })),
            ))
        }
    };

    // Get snapshots from storage
    let storage = state.shared_storage.lock().unwrap();
    let snapshots = storage
        .get_snapshots_for_entity(SnapshotEntityType::Item, &dfid)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "error": format!("Failed to get snapshots: {}", e)
                })),
            )
        })?;

    // Apply pagination
    let total = snapshots.len();
    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(50);
    let snapshots: Vec<_> = snapshots.into_iter().skip(offset).take(limit).collect();

    let latest_version = snapshots.last().map(|s| s.version).unwrap_or(0);

    let response = SnapshotListResponse {
        entity_id: dfid,
        entity_type: "item".to_string(),
        total_snapshots: total,
        latest_version,
        snapshots: snapshots.iter().map(SnapshotResponse::from).collect(),
    };

    Ok(Json(json!({
        "success": true,
        "data": response
    })))
}

/// Get latest snapshot for an item
async fn get_latest_item_snapshot(
    State(state): State<Arc<AppState>>,
    claims: Option<Extension<crate::api::auth::Claims>>,
    api_key_ctx: Option<Extension<crate::api_key_middleware::ApiKeyContext>>,
    Path(dfid): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Authenticate
    let _user_id = match (&claims, &api_key_ctx) {
        (Some(Extension(claims)), _) => claims.user_id.clone(),
        (_, Some(Extension(ctx))) => ctx.user_id.to_string(),
        _ => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "success": false,
                    "error": "Authentication required"
                })),
            ))
        }
    };

    let storage = state.shared_storage.lock().unwrap();
    let snapshot = storage
        .get_latest_snapshot(SnapshotEntityType::Item, &dfid)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "error": format!("Failed to get snapshot: {}", e)
                })),
            )
        })?;

    match snapshot {
        Some(s) => {
            let response = SnapshotDetailResponse {
                snapshot: SnapshotResponse::from(&s),
                state: Some(s.state.clone()),
            };
            Ok(Json(json!({
                "success": true,
                "data": response
            })))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "error": "No snapshots found for this item"
            })),
        )),
    }
}

/// Get a specific snapshot by ID
async fn get_item_snapshot(
    State(state): State<Arc<AppState>>,
    claims: Option<Extension<crate::api::auth::Claims>>,
    api_key_ctx: Option<Extension<crate::api_key_middleware::ApiKeyContext>>,
    Path((dfid, snapshot_id)): Path<(String, String)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Authenticate
    let _user_id = match (&claims, &api_key_ctx) {
        (Some(Extension(claims)), _) => claims.user_id.clone(),
        (_, Some(Extension(ctx))) => ctx.user_id.to_string(),
        _ => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "success": false,
                    "error": "Authentication required"
                })),
            ))
        }
    };

    let storage = state.shared_storage.lock().unwrap();
    let snapshot = storage.get_snapshot(&snapshot_id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "error": format!("Failed to get snapshot: {}", e)
            })),
        )
    })?;

    match snapshot {
        Some(s) if s.entity_id == dfid && matches!(s.entity_type, SnapshotEntityType::Item) => {
            let response = SnapshotDetailResponse {
                snapshot: SnapshotResponse::from(&s),
                state: Some(s.state.clone()),
            };
            Ok(Json(json!({
                "success": true,
                "data": response
            })))
        }
        Some(_) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "error": "Snapshot not found for this item"
            })),
        )),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "error": "Snapshot not found"
            })),
        )),
    }
}

// ============================================================================
// CIRCUIT SNAPSHOT HANDLERS
// ============================================================================

/// List all snapshots for a circuit
async fn list_circuit_snapshots(
    State(state): State<Arc<AppState>>,
    claims: Option<Extension<crate::api::auth::Claims>>,
    api_key_ctx: Option<Extension<crate::api_key_middleware::ApiKeyContext>>,
    Path(circuit_id): Path<Uuid>,
    Query(query): Query<SnapshotQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Authenticate
    let _user_id = match (&claims, &api_key_ctx) {
        (Some(Extension(claims)), _) => claims.user_id.clone(),
        (_, Some(Extension(ctx))) => ctx.user_id.to_string(),
        _ => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "success": false,
                    "error": "Authentication required"
                })),
            ))
        }
    };

    let storage = state.shared_storage.lock().unwrap();
    let snapshots = storage
        .get_snapshots_for_entity(SnapshotEntityType::Circuit, &circuit_id.to_string())
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "error": format!("Failed to get snapshots: {}", e)
                })),
            )
        })?;

    let total = snapshots.len();
    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(50);
    let snapshots: Vec<_> = snapshots.into_iter().skip(offset).take(limit).collect();

    let latest_version = snapshots.last().map(|s| s.version).unwrap_or(0);

    let response = SnapshotListResponse {
        entity_id: circuit_id.to_string(),
        entity_type: "circuit".to_string(),
        total_snapshots: total,
        latest_version,
        snapshots: snapshots.iter().map(SnapshotResponse::from).collect(),
    };

    Ok(Json(json!({
        "success": true,
        "data": response
    })))
}

/// Get latest snapshot for a circuit
async fn get_latest_circuit_snapshot(
    State(state): State<Arc<AppState>>,
    claims: Option<Extension<crate::api::auth::Claims>>,
    api_key_ctx: Option<Extension<crate::api_key_middleware::ApiKeyContext>>,
    Path(circuit_id): Path<Uuid>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Authenticate
    let _user_id = match (&claims, &api_key_ctx) {
        (Some(Extension(claims)), _) => claims.user_id.clone(),
        (_, Some(Extension(ctx))) => ctx.user_id.to_string(),
        _ => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "success": false,
                    "error": "Authentication required"
                })),
            ))
        }
    };

    let storage = state.shared_storage.lock().unwrap();
    let snapshot = storage
        .get_latest_snapshot(SnapshotEntityType::Circuit, &circuit_id.to_string())
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "error": format!("Failed to get snapshot: {}", e)
                })),
            )
        })?;

    match snapshot {
        Some(s) => {
            let response = SnapshotDetailResponse {
                snapshot: SnapshotResponse::from(&s),
                state: Some(s.state.clone()),
            };
            Ok(Json(json!({
                "success": true,
                "data": response
            })))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "error": "No snapshots found for this circuit"
            })),
        )),
    }
}

/// Get a specific circuit snapshot by ID
async fn get_circuit_snapshot(
    State(state): State<Arc<AppState>>,
    claims: Option<Extension<crate::api::auth::Claims>>,
    api_key_ctx: Option<Extension<crate::api_key_middleware::ApiKeyContext>>,
    Path((circuit_id, snapshot_id)): Path<(Uuid, String)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Authenticate
    let _user_id = match (&claims, &api_key_ctx) {
        (Some(Extension(claims)), _) => claims.user_id.clone(),
        (_, Some(Extension(ctx))) => ctx.user_id.to_string(),
        _ => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "success": false,
                    "error": "Authentication required"
                })),
            ))
        }
    };

    let storage = state.shared_storage.lock().unwrap();
    let snapshot = storage.get_snapshot(&snapshot_id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "error": format!("Failed to get snapshot: {}", e)
            })),
        )
    })?;

    match snapshot {
        Some(s)
            if s.entity_id == circuit_id.to_string()
                && matches!(s.entity_type, SnapshotEntityType::Circuit) =>
        {
            let response = SnapshotDetailResponse {
                snapshot: SnapshotResponse::from(&s),
                state: Some(s.state.clone()),
            };
            Ok(Json(json!({
                "success": true,
                "data": response
            })))
        }
        Some(_) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "error": "Snapshot not found for this circuit"
            })),
        )),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "error": "Snapshot not found"
            })),
        )),
    }
}

// ============================================================================
// PUBLIC SNAPSHOT HANDLERS
// ============================================================================

/// List snapshots for an item in a public circuit (no auth required)
async fn list_public_item_snapshots(
    State(state): State<Arc<AppState>>,
    Path(dfid): Path<String>,
    Query(query): Query<SnapshotQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Check if item is in a public circuit
    let storage = state.shared_storage.lock().unwrap();

    // First verify the item exists
    let item = storage.get_item_by_dfid(&dfid).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "error": format!("Failed to get item: {}", e)
            })),
        )
    })?;

    if item.is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "error": "Item not found"
            })),
        ));
    }

    // Check if item is in a public circuit
    let circuits = storage.list_circuits().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "error": format!("Failed to list circuits: {}", e)
            })),
        )
    })?;

    let is_public = circuits.iter().any(|c| {
        c.public_settings
            .as_ref()
            .map(|ps| {
                matches!(
                    ps.access_mode,
                    crate::types::PublicAccessMode::Public
                        | crate::types::PublicAccessMode::Protected
                ) && ps.published_items.contains(&dfid)
            })
            .unwrap_or(false)
    });

    if !is_public {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "error": "Item not available publicly"
            })),
        ));
    }

    // Get snapshots
    let snapshots = storage
        .get_snapshots_for_entity(SnapshotEntityType::Item, &dfid)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "error": format!("Failed to get snapshots: {}", e)
                })),
            )
        })?;

    let total = snapshots.len();
    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(50);
    let snapshots: Vec<_> = snapshots.into_iter().skip(offset).take(limit).collect();

    let latest_version = snapshots.last().map(|s| s.version).unwrap_or(0);

    let response = SnapshotListResponse {
        entity_id: dfid,
        entity_type: "item".to_string(),
        total_snapshots: total,
        latest_version,
        snapshots: snapshots.iter().map(SnapshotResponse::from).collect(),
    };

    Ok(Json(json!({
        "success": true,
        "data": response
    })))
}

/// Get latest public item snapshot
async fn get_latest_public_item_snapshot(
    State(state): State<Arc<AppState>>,
    Path(dfid): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let storage = state.shared_storage.lock().unwrap();

    // Verify item exists
    let item = storage.get_item_by_dfid(&dfid).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "error": format!("Failed to get item: {}", e)
            })),
        )
    })?;

    if item.is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "error": "Item not found"
            })),
        ));
    }

    // Check if item is in a public circuit
    let circuits = storage.list_circuits().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "error": format!("Failed to list circuits: {}", e)
            })),
        )
    })?;

    let is_public = circuits.iter().any(|c| {
        c.public_settings
            .as_ref()
            .map(|ps| {
                matches!(
                    ps.access_mode,
                    crate::types::PublicAccessMode::Public
                        | crate::types::PublicAccessMode::Protected
                ) && ps.published_items.contains(&dfid)
            })
            .unwrap_or(false)
    });

    if !is_public {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "error": "Item not available publicly"
            })),
        ));
    }

    let snapshot = storage
        .get_latest_snapshot(SnapshotEntityType::Item, &dfid)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "error": format!("Failed to get snapshot: {}", e)
                })),
            )
        })?;

    match snapshot {
        Some(s) => {
            let response = SnapshotDetailResponse {
                snapshot: SnapshotResponse::from(&s),
                state: Some(s.state.clone()),
            };
            Ok(Json(json!({
                "success": true,
                "data": response
            })))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "error": "No snapshots found for this item"
            })),
        )),
    }
}

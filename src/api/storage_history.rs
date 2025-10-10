use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

use crate::adapters::StorageLocation;
use crate::types::{ItemStorageHistory, AdapterType};
use crate::api::shared_state::AppState;
use crate::api::adapters::create_adapter_instance;
use crate::storage::StorageBackend;

#[derive(Debug, Serialize, Deserialize)]
pub struct StorageHistoryResponse {
    pub dfid: String,
    pub storage_records: Vec<StorageRecordResponse>,
    pub current_primary: Option<StorageLocation>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StorageRecordResponse {
    pub storage_location: StorageLocation,
    pub stored_at: String,
    pub circuit_id: Option<Uuid>,
    pub user_id: String,
    pub operation_type: String,
    pub is_primary: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MigrateItemRequest {
    pub target_adapter_type: AdapterType,
    pub circuit_id: Uuid,
    pub user_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StorageLocationQuery {
    pub dfid: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetPrimaryStorageRequest {
    pub storage_location: StorageLocation,
}

impl From<ItemStorageHistory> for StorageHistoryResponse {
    fn from(history: ItemStorageHistory) -> Self {
        Self {
            dfid: history.dfid,
            storage_records: history.storage_records.into_iter().map(|record| {
                StorageRecordResponse {
                    storage_location: record.storage_location,
                    stored_at: record.stored_at.to_rfc3339(),
                    circuit_id: record.triggered_by_id.as_ref().and_then(|id| id.parse().ok()),
                    user_id: record.triggered_by_id.unwrap_or_default(),
                    operation_type: record.triggered_by,
                    is_primary: record.is_active,
                }
            }).collect(),
            current_primary: history.current_primary,
            created_at: history.created_at.to_rfc3339(),
            updated_at: history.updated_at.to_rfc3339(),
        }
    }
}

async fn get_item_storage_history(
    Path(dfid): Path<String>,
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<Value>, StatusCode> {
    match app_state.storage_history_manager.get_item_storage_history(&dfid).await {
        Ok(Some(history)) => {
            let response = StorageHistoryResponse::from(history);
            Ok(Json(json!({
                "success": true,
                "data": response
            })))
        }
        Ok(None) => {
            Ok(Json(json!({
                "success": false,
                "error": "Item not found",
                "dfid": dfid
            })))
        }
        Err(e) => {
            Ok(Json(json!({
                "success": false,
                "error": format!("Failed to get storage history: {}", e),
                "dfid": dfid
            })))
        }
    }
}

async fn get_all_storage_locations(
    Query(params): Query<StorageLocationQuery>,
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<Value>, StatusCode> {
    match app_state.storage_history_manager.get_all_storage_locations(&params.dfid).await {
        Ok(locations) => {
            Ok(Json(json!({
                "success": true,
                "dfid": params.dfid,
                "locations": locations,
                "count": locations.len()
            })))
        }
        Err(e) => {
            Ok(Json(json!({
                "success": false,
                "error": format!("Failed to get storage locations: {}", e),
                "dfid": params.dfid
            })))
        }
    }
}

async fn migrate_item_storage(
    Path(dfid): Path<String>,
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<MigrateItemRequest>,
) -> Result<Json<Value>, StatusCode> {
    let adapter_instance = match create_adapter_instance(&request.target_adapter_type) {
        Ok(instance) => instance,
        Err(e) => {
            return Ok(Json(json!({
                "success": false,
                "error": format!("Failed to create adapter: {}", e),
                "dfid": dfid
            })));
        }
    };

    match app_state.storage_history_manager.migrate_to_circuit_adapter(
        &dfid,
        &adapter_instance,
        request.circuit_id,
        &request.user_id,
    ).await {
        Ok(()) => {
            Ok(Json(json!({
                "success": true,
                "message": "Item migration initiated successfully",
                "dfid": dfid,
                "target_adapter": request.target_adapter_type,
                "circuit_id": request.circuit_id,
                "user_id": request.user_id
            })))
        }
        Err(e) => {
            Ok(Json(json!({
                "success": false,
                "error": format!("Failed to migrate item: {}", e),
                "dfid": dfid,
                "target_adapter": request.target_adapter_type
            })))
        }
    }
}

async fn set_primary_storage(
    Path(dfid): Path<String>,
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<SetPrimaryStorageRequest>,
) -> Result<Json<Value>, StatusCode> {
    match app_state.storage_history_manager.set_primary_storage(&dfid, request.storage_location.clone()).await {
        Ok(()) => {
            Ok(Json(json!({
                "success": true,
                "message": "Primary storage updated successfully",
                "dfid": dfid,
                "primary_storage": request.storage_location
            })))
        }
        Err(e) => {
            Ok(Json(json!({
                "success": false,
                "error": format!("Failed to set primary storage: {}", e),
                "dfid": dfid
            })))
        }
    }
}

async fn get_storage_statistics(
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<Value>, StatusCode> {
    let storage = app_state.shared_storage.lock().unwrap();

    // Get all items to iterate through their storage histories
    let items = match storage.list_items() {
        Ok(items) => items,
        Err(e) => {
            return Ok(Json(json!({
                "success": false,
                "error": format!("Failed to list items: {}", e)
            })));
        }
    };

    let mut total_items_tracked = 0;
    let mut total_storage_locations = 0;
    let mut adapter_distribution: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    let mut migration_count = 0;

    // Iterate through each item and get its storage history
    for item in items {
        if let Ok(Some(history)) = storage.get_storage_history(&item.dfid) {
            total_items_tracked += 1;
            total_storage_locations += history.storage_records.len();

            for record in &history.storage_records {
                let adapter_name = record.adapter_type.to_string();
                *adapter_distribution.entry(adapter_name).or_insert(0) += 1;

                // Count migrations (records triggered by circuit operations or explicit migrations)
                if record.triggered_by.contains("migration") || record.triggered_by.contains("circuit") {
                    migration_count += 1;
                }
            }
        }
    }

    Ok(Json(json!({
        "success": true,
        "statistics": {
            "total_items_tracked": total_items_tracked,
            "total_storage_locations": total_storage_locations,
            "adapter_distribution": adapter_distribution,
            "migration_count": migration_count,
            "generated_at": chrono::Utc::now()
        }
    })))
}

pub fn storage_history_routes(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/:dfid", get(get_item_storage_history))
        .route("/:dfid/locations", get(get_all_storage_locations))
        .route("/:dfid/migrate", post(migrate_item_storage))
        .route("/:dfid/primary", post(set_primary_storage))
        .route("/statistics", get(get_storage_statistics))
        .with_state(app_state)
}
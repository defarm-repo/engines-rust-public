use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put, delete},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::{ItemsEngine, InMemoryStorage, Item, ItemStatus, Identifier, PendingItem, PendingReason};
use crate::items_engine::ResolutionAction;
use crate::identifier_types::EnhancedIdentifier;
use crate::storage::StorageBackend;
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct IdentifierRequest {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EnhancedIdentifierRequest {
    pub namespace: String,
    pub key: String,
    pub value: String,
    pub id_type: String, // "Canonical" or "Contextual"
}

#[derive(Debug, Deserialize)]
pub struct CreateLocalItemRequest {
    pub identifiers: Option<Vec<IdentifierRequest>>,
    pub enhanced_identifiers: Option<Vec<EnhancedIdentifierRequest>>,
    pub enriched_data: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize)]
pub struct CreateLocalItemResponse {
    pub success: bool,
    pub data: LocalItemData,
}

#[derive(Debug, Serialize)]
pub struct LocalItemData {
    pub local_id: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct LidDfidMappingResponse {
    pub success: bool,
    pub data: LidDfidMappingData,
}

#[derive(Debug, Serialize)]
pub struct LidDfidMappingData {
    pub local_id: String,
    pub dfid: Option<String>,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateItemRequest {
    pub identifiers: Vec<IdentifierRequest>,
    pub enriched_data: Option<HashMap<String, serde_json::Value>>,
    pub source_entry: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateItemsBatchRequest {
    pub items: Vec<CreateItemRequest>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateItemRequest {
    pub enriched_data: Option<HashMap<String, serde_json::Value>>,
    pub identifiers: Option<Vec<IdentifierRequest>>,
}

#[derive(Debug, Deserialize)]
pub struct SplitItemRequest {
    pub identifiers_for_new_item: Vec<IdentifierRequest>,
}

#[derive(Debug, Serialize)]
pub struct SplitItemResponse {
    pub original_item: ItemResponse,
    pub new_item: ItemResponse,
}

#[derive(Debug, Deserialize)]
pub struct ItemQueryParams {
    pub identifier_key: Option<String>,
    pub identifier_value: Option<String>,
    pub status: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct ItemResponse {
    pub dfid: String,
    pub identifiers: Vec<IdentifierRequest>,
    pub enriched_data: HashMap<String, serde_json::Value>,
    pub creation_timestamp: i64,
    pub last_modified: i64,
    pub source_entries: Vec<String>,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct CreateItemsBatchResponse {
    pub success_count: usize,
    pub failed_count: usize,
    pub results: Vec<BatchItemResult>,
}

#[derive(Debug, Serialize)]
pub struct BatchItemResult {
    pub success: bool,
    pub item: Option<ItemResponse>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ItemStatsResponse {
    pub total_items: usize,
    pub active_items: usize,
    pub merged_items: usize,
    pub split_items: usize,
    pub archived_items: usize,
    pub average_confidence: f64,
}

#[derive(Debug, Deserialize)]
pub struct ShareItemRequest {
    pub recipient_user_id: String,
    pub permissions: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct PendingItemResponse {
    pub id: String,
    pub identifiers: Vec<IdentifierRequest>,
    pub enriched_data: Option<HashMap<String, serde_json::Value>>,
    pub source_entry: String,
    pub reason: String,
    pub reason_details: Option<String>,
    pub priority: u32,
    pub created_at: i64,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct ResolvePendingItemRequest {
    pub action: String, // "approve", "reject", or "modify"
    pub new_identifiers: Option<Vec<IdentifierRequest>>,
    pub new_enriched_data: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize)]
pub struct ResolvePendingItemResponse {
    pub success: bool,
    pub item: Option<ItemResponse>,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct PendingItemQueryParams {
    pub reason: Option<String>,
    pub priority_min: Option<u32>,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct ShareItemResponse {
    pub share_id: String,
    pub dfid: String,
    pub recipient_user_id: String,
    pub shared_at: i64,
}

#[derive(Debug, Serialize)]
pub struct SharedItemListResponse {
    pub share_id: String,
    pub item: ItemResponse,
    pub shared_by: String,
    pub shared_at: i64,
    pub permissions: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct SharedWithCheckResponse {
    pub is_shared: bool,
    pub share_id: Option<String>,
    pub shared_at: Option<i64>,
}

pub struct ItemState {
    pub engine: Arc<Mutex<ItemsEngine<InMemoryStorage>>>,
}

impl ItemState {
    pub fn new() -> Self {
        let storage = InMemoryStorage::new();
        Self {
            engine: Arc::new(Mutex::new(ItemsEngine::new(storage))),
        }
    }
}

use super::shared_state::AppState;

pub fn item_routes(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", post(create_item))
        .route("/local", post(create_local_item))
        .route("/mapping/:local_id", get(get_lid_dfid_mapping))
        .route("/batch", post(create_items_batch))
        .route("/", get(list_items))
        .route("/:dfid", get(get_item))
        .route("/:dfid", put(update_item))
        .route("/:dfid", delete(delete_item))
        .route("/:dfid/merge", post(merge_items))
        .route("/:dfid/split", post(split_item))
        .route("/:dfid/deprecate", put(deprecate_item))
        .route("/:dfid/share", post(share_item))
        .route("/:dfid/shared-with/:user_id", get(check_item_shared_with_user))
        .route("/search", get(search_items))
        .route("/stats", get(get_item_stats))
        .route("/identifier/:key/:value", get(get_items_by_identifier))
        .route("/shared-to/:user_id", get(get_shared_items_for_user))
        .route("/pending", get(list_pending_items))
        .route("/pending/:id", get(get_pending_item))
        .route("/pending/:id/resolve", post(resolve_pending_item))
        .route("/:dfid/storage-history", get(get_storage_history))
        .with_state(app_state)
}

fn parse_item_status(status_str: &str) -> Result<ItemStatus, String> {
    match status_str.to_lowercase().as_str() {
        "active" => Ok(ItemStatus::Active),
        "deprecated" => Ok(ItemStatus::Deprecated),
        "merged" => Ok(ItemStatus::Merged),
        "split" => Ok(ItemStatus::Split),
        _ => Err(format!("Invalid item status: {}", status_str)),
    }
}

fn item_to_response(item: Item) -> ItemResponse {
    ItemResponse {
        dfid: item.dfid,
        identifiers: item.identifiers
            .into_iter()
            .map(|id| IdentifierRequest { key: id.key, value: id.value })
            .collect(),
        enriched_data: item.enriched_data,
        creation_timestamp: item.creation_timestamp.timestamp(),
        last_modified: item.last_modified.timestamp(),
        source_entries: item.source_entries
            .into_iter()
            .map(|uuid| uuid.to_string())
            .collect(),
        status: format!("{:?}", item.status),
    }
}

async fn create_item(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateItemRequest>,
) -> Result<Json<ItemResponse>, (StatusCode, Json<Value>)> {
    let mut engine = state.items_engine.lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Items engine mutex poisoned"}))))?;

    let source_entry = uuid::Uuid::parse_str(&payload.source_entry)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid source entry UUID"}))))?;

    let identifiers: Vec<Identifier> = payload.identifiers
        .into_iter()
        .map(|id| Identifier::new(id.key, id.value))
        .collect();

    match engine.create_item_with_generated_dfid(identifiers, source_entry, payload.enriched_data) {
        Ok(item) => Ok(Json(item_to_response(item))),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({"error": format!("Failed to create item: {}", e)})))),
    }
}

async fn create_items_batch(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateItemsBatchRequest>,
) -> Result<Json<CreateItemsBatchResponse>, (StatusCode, Json<Value>)> {
    let mut engine = state.items_engine.lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Items engine mutex poisoned"}))))?;
    let mut results = Vec::new();
    let mut success_count = 0;
    let mut failed_count = 0;

    for item_request in payload.items {
        let result = match uuid::Uuid::parse_str(&item_request.source_entry) {
            Ok(source_entry) => {
                let identifiers: Vec<Identifier> = item_request.identifiers
                    .into_iter()
                    .map(|id| Identifier::new(id.key, id.value))
                    .collect();

                match engine.create_item_with_generated_dfid(identifiers, source_entry, item_request.enriched_data) {
                    Ok(item) => {
                        success_count += 1;
                        BatchItemResult {
                            success: true,
                            item: Some(item_to_response(item)),
                            error: None,
                        }
                    }
                    Err(e) => {
                        failed_count += 1;
                        BatchItemResult {
                            success: false,
                            item: None,
                            error: Some(format!("Failed to create item: {}", e)),
                        }
                    }
                }
            }
            Err(_) => {
                failed_count += 1;
                BatchItemResult {
                    success: false,
                    item: None,
                    error: Some("Invalid source entry UUID".to_string()),
                }
            }
        };
        results.push(result);
    }

    Ok(Json(CreateItemsBatchResponse {
        success_count,
        failed_count,
        results,
    }))
}

async fn get_item(
    State(state): State<Arc<AppState>>,
    Path(dfid): Path<String>,
) -> Result<Json<ItemResponse>, (StatusCode, Json<Value>)> {
    let engine = state.items_engine.lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Items engine mutex poisoned"}))))?;

    match engine.get_item(&dfid) {
        Ok(Some(item)) => Ok(Json(item_to_response(item))),
        Ok(None) => Err((StatusCode::NOT_FOUND, Json(json!({"error": "Item not found"})))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to get item: {}", e)})))),
    }
}

async fn update_item(
    State(state): State<Arc<AppState>>,
    Path(dfid): Path<String>,
    Json(payload): Json<UpdateItemRequest>,
) -> Result<Json<ItemResponse>, (StatusCode, Json<Value>)> {
    let mut engine = state.items_engine.lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Items engine mutex poisoned"}))))?;

    // Update enriched data if provided
    if let Some(enriched_data) = payload.enriched_data {
        let source_entry = uuid::Uuid::new_v4(); // Generate a new UUID for the enrichment
        match engine.enrich_item(&dfid, enriched_data, source_entry) {
            Ok(_) => {},
            Err(e) => return Err((StatusCode::BAD_REQUEST, Json(json!({"error": format!("Failed to enrich item: {}", e)})))),
        }
    }

    // Add identifiers if provided
    if let Some(identifier_requests) = payload.identifiers {
        let identifiers: Vec<Identifier> = identifier_requests
            .into_iter()
            .map(|id| Identifier::new(id.key, id.value))
            .collect();

        match engine.add_identifiers(&dfid, identifiers) {
            Ok(_) => {},
            Err(e) => return Err((StatusCode::BAD_REQUEST, Json(json!({"error": format!("Failed to add identifiers: {}", e)})))),
        }
    }

    // Return updated item
    match engine.get_item(&dfid) {
        Ok(Some(item)) => Ok(Json(item_to_response(item))),
        Ok(None) => Err((StatusCode::NOT_FOUND, Json(json!({"error": "Item not found after update"})))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to get updated item: {}", e)})))),
    }
}

async fn delete_item(
    State(state): State<Arc<AppState>>,
    Path(dfid): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let mut engine = state.items_engine.lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Items engine mutex poisoned"}))))?;

    match engine.deprecate_item(&dfid) {
        Ok(_) => Ok(Json(json!({"message": "Item deprecated successfully"}))),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({"error": format!("Failed to deprecate item: {}", e)})))),
    }
}

async fn list_items(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ItemQueryParams>,
) -> Result<Json<Vec<ItemResponse>>, (StatusCode, Json<Value>)> {
    let engine = state.items_engine.lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Items engine mutex poisoned"}))))?;

    match engine.list_items() {
        Ok(mut items) => {
            // Apply filters
            if let Some(status_str) = params.status {
                if let Ok(status) = parse_item_status(&status_str) {
                    items.retain(|item| item.status == status);
                }
            }

            if let Some(key) = &params.identifier_key {
                if let Some(value) = &params.identifier_value {
                    items.retain(|item| {
                        item.identifiers.iter().any(|id| id.key == *key && id.value == *value)
                    });
                } else {
                    items.retain(|item| item.identifiers.iter().any(|id| id.key == *key));
                }
            }

            // Apply limit
            if let Some(limit) = params.limit {
                items.truncate(limit);
            }

            let response: Vec<ItemResponse> = items
                .into_iter()
                .map(item_to_response)
                .collect();
            Ok(Json(response))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to list items: {}", e)})))),
    }
}

async fn merge_items(
    State(state): State<Arc<AppState>>,
    Path(primary_dfid): Path<String>,
    Json(secondary_dfid): Json<String>,
) -> Result<Json<ItemResponse>, (StatusCode, Json<Value>)> {
    let mut engine = state.items_engine.lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Items engine mutex poisoned"}))))?;

    match engine.merge_items(&primary_dfid, &secondary_dfid) {
        Ok(item) => Ok(Json(item_to_response(item))),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({"error": format!("Failed to merge items: {}", e)})))),
    }
}

async fn split_item(
    State(state): State<Arc<AppState>>,
    Path(dfid): Path<String>,
    Json(split_request): Json<SplitItemRequest>,
) -> Result<Json<SplitItemResponse>, (StatusCode, Json<Value>)> {
    let mut engine = state.items_engine.lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Items engine mutex poisoned"}))))?;

    let identifiers: Vec<Identifier> = split_request.identifiers_for_new_item
        .into_iter()
        .map(|id| Identifier::new(id.key, id.value))
        .collect();

    match engine.split_item_with_generated_dfid(&dfid, identifiers) {
        Ok((original_item, new_item)) => {
            Ok(Json(SplitItemResponse {
                original_item: item_to_response(original_item),
                new_item: item_to_response(new_item),
            }))
        }
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({"error": format!("Failed to split item: {}", e)})))),
    }
}

async fn deprecate_item(
    State(state): State<Arc<AppState>>,
    Path(dfid): Path<String>,
) -> Result<Json<ItemResponse>, (StatusCode, Json<Value>)> {
    let mut engine = state.items_engine.lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Items engine mutex poisoned"}))))?;

    match engine.deprecate_item(&dfid) {
        Ok(item) => Ok(Json(item_to_response(item))),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({"error": format!("Failed to deprecate item: {}", e)})))),
    }
}


async fn search_items(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ItemQueryParams>,
) -> Result<Json<Vec<ItemResponse>>, (StatusCode, Json<Value>)> {
    // Reuse list_items logic for search
    list_items(State(state), Query(params)).await
}

async fn get_item_stats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ItemStatsResponse>, (StatusCode, Json<Value>)> {
    let engine = state.items_engine.lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Items engine mutex poisoned"}))))?;

    match engine.list_items() {
        Ok(items) => {
            let total_items = items.len();
            let active_items = items.iter().filter(|i| matches!(i.status, ItemStatus::Active)).count();
            let merged_items = items.iter().filter(|i| matches!(i.status, ItemStatus::Merged)).count();
            let split_items = items.iter().filter(|i| matches!(i.status, ItemStatus::Split)).count();
            let deprecated_items = items.iter().filter(|i| matches!(i.status, ItemStatus::Deprecated)).count();

            let average_confidence = 0.0; // Not available in current Item struct

            Ok(Json(ItemStatsResponse {
                total_items,
                active_items,
                merged_items,
                split_items,
                archived_items: deprecated_items,
                average_confidence,
            }))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to get stats: {}", e)})))),
    }
}

async fn get_items_by_identifier(
    State(state): State<Arc<AppState>>,
    Path((key, value)): Path<(String, String)>,
) -> Result<Json<Vec<ItemResponse>>, (StatusCode, Json<Value>)> {
    let engine = state.items_engine.lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Items engine mutex poisoned"}))))?;
    let identifier = Identifier::new(key, value);

    match engine.find_items_by_identifier(&identifier) {
        Ok(items) => {
            let response: Vec<ItemResponse> = items
                .into_iter()
                .map(item_to_response)
                .collect();
            Ok(Json(response))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to get items by identifier: {}", e)})))),
    }
}

// Sharing endpoints
async fn share_item(
    State(state): State<Arc<AppState>>,
    Path(dfid): Path<String>,
    Json(payload): Json<ShareItemRequest>,
) -> Result<Json<ShareItemResponse>, (StatusCode, Json<Value>)> {
    let mut engine = state.items_engine.lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Items engine mutex poisoned"}))))?;

    // For now, use a placeholder for the shared_by user ID
    // In a real application, this would come from authentication
    let shared_by = "current_user".to_string();

    match engine.share_item(&dfid, shared_by, payload.recipient_user_id, payload.permissions) {
        Ok(share) => Ok(Json(ShareItemResponse {
            share_id: share.share_id,
            dfid: share.dfid,
            recipient_user_id: share.recipient_user_id,
            shared_at: share.shared_at.timestamp(),
        })),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({"error": format!("Failed to share item: {}", e)})))),
    }
}

async fn check_item_shared_with_user(
    State(state): State<Arc<AppState>>,
    Path((dfid, user_id)): Path<(String, String)>,
) -> Result<Json<SharedWithCheckResponse>, (StatusCode, Json<Value>)> {
    let engine = state.items_engine.lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Items engine mutex poisoned"}))))?;

    match engine.is_item_shared_with_user(&dfid, &user_id) {
        Ok(is_shared) => {
            if is_shared {
                // Get the share details
                if let Ok(shares) = engine.get_shares_for_item(&dfid) {
                    if let Some(share) = shares.iter().find(|s| s.recipient_user_id == user_id) {
                        return Ok(Json(SharedWithCheckResponse {
                            is_shared: true,
                            share_id: Some(share.share_id.clone()),
                            shared_at: Some(share.shared_at.timestamp()),
                        }));
                    }
                }
            }
            Ok(Json(SharedWithCheckResponse {
                is_shared: false,
                share_id: None,
                shared_at: None,
            }))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to check share status: {}", e)})))),
    }
}

pub async fn get_shared_items_for_user(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
) -> Result<Json<Vec<SharedItemListResponse>>, (StatusCode, Json<Value>)> {
    let engine = state.items_engine.lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Items engine mutex poisoned"}))))?;

    match engine.get_shares_for_user(&user_id) {
        Ok(shared_items) => {
            let response: Vec<SharedItemListResponse> = shared_items
                .into_iter()
                .map(|shared_item| SharedItemListResponse {
                    share_id: shared_item.share_id,
                    item: item_to_response(shared_item.item),
                    shared_by: shared_item.shared_by,
                    shared_at: shared_item.shared_at.timestamp(),
                    permissions: shared_item.permissions,
                })
                .collect();
            Ok(Json(response))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to get shared items: {}", e)})))),
    }
}

// Pending Items Handlers
async fn list_pending_items(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PendingItemQueryParams>,
) -> Result<Json<Vec<PendingItemResponse>>, (StatusCode, Json<Value>)> {
    let engine = state.items_engine.lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Items engine mutex poisoned"}))))?;

    match engine.get_pending_items() {
        Ok(pending_items) => {
            let mut filtered_items = pending_items;

            // Apply filters
            if let Some(reason_filter) = &params.reason {
                filtered_items.retain(|item| format!("{:?}", item.reason).contains(reason_filter));
            }

            if let Some(priority_min) = params.priority_min {
                filtered_items.retain(|item| item.priority >= priority_min);
            }

            // Sort by priority (descending)
            filtered_items.sort_by(|a, b| b.priority.cmp(&a.priority));

            // Apply limit
            if let Some(limit) = params.limit {
                filtered_items.truncate(limit);
            }

            let response: Vec<PendingItemResponse> = filtered_items
                .into_iter()
                .map(pending_item_to_response)
                .collect();

            Ok(Json(response))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to list pending items: {}", e)})))),
    }
}

async fn get_pending_item(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<PendingItemResponse>, (StatusCode, Json<Value>)> {
    let engine = state.items_engine.lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Items engine mutex poisoned"}))))?;

    let pending_id = match Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(_) => return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid pending item ID format"})))),
    };

    match engine.get_pending_item(&pending_id) {
        Ok(Some(pending_item)) => Ok(Json(pending_item_to_response(pending_item))),
        Ok(None) => Err((StatusCode::NOT_FOUND, Json(json!({"error": "Pending item not found"})))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to get pending item: {}", e)})))),
    }
}

async fn resolve_pending_item(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(payload): Json<ResolvePendingItemRequest>,
) -> Result<Json<ResolvePendingItemResponse>, (StatusCode, Json<Value>)> {
    let mut engine = state.items_engine.lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Items engine mutex poisoned"}))))?;

    let pending_id = match Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(_) => return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid pending item ID format"})))),
    };

    let resolution_action = match payload.action.as_str() {
        "approve" => ResolutionAction::Approve,
        "reject" => ResolutionAction::Reject,
        "modify" => {
            let identifiers = payload.new_identifiers
                .unwrap_or_default()
                .into_iter()
                .map(|req| Identifier::new(&req.key, &req.value))
                .collect();
            ResolutionAction::Modify(identifiers, payload.new_enriched_data)
        },
        _ => return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid action. Must be 'approve', 'reject', or 'modify'"})))),
    };

    match engine.resolve_pending_item(&pending_id, resolution_action) {
        Ok(Some(item)) => Ok(Json(ResolvePendingItemResponse {
            success: true,
            item: Some(item_to_response(item)),
            message: "Pending item resolved and created successfully".to_string(),
        })),
        Ok(None) => Ok(Json(ResolvePendingItemResponse {
            success: true,
            item: None,
            message: "Pending item resolved but not created (rejected or still pending)".to_string(),
        })),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to resolve pending item: {}", e)})))),
    }
}

// Utility function for converting PendingItem to response format
fn pending_item_to_response(pending_item: PendingItem) -> PendingItemResponse {
    PendingItemResponse {
        id: pending_item.pending_id.to_string(),
        identifiers: pending_item.identifiers
            .into_iter()
            .map(|id| IdentifierRequest { key: id.key, value: id.value })
            .collect(),
        enriched_data: pending_item.enriched_data,
        source_entry: pending_item.source_entry.to_string(),
        reason: format!("{:?}", pending_item.reason),
        reason_details: match &pending_item.reason {
            PendingReason::InvalidIdentifiers(details) => Some(details.clone()),
            PendingReason::ConflictingDFIDs { identifier, conflicting_dfids, .. } => {
                Some(format!("Identifier {}:{} maps to DFIDs: {}",
                    identifier.key, identifier.value, conflicting_dfids.join(", ")))
            },
            PendingReason::DataQualityIssue { details, .. } => Some(details.clone()),
            _ => None,
        },
        priority: pending_item.priority as u32,
        created_at: pending_item.created_at.timestamp(),
        metadata: pending_item.metadata.into_iter().map(|(k, v)| (k, v.to_string())).collect(),
    }
}

// Local item creation handlers
async fn create_local_item(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateLocalItemRequest>,
) -> Result<Json<CreateLocalItemResponse>, (StatusCode, Json<Value>)> {
    // Create item in in-memory storage (must not hold lock across await)
    let item = {
        let mut engine = state.items_engine.lock()
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Items engine mutex poisoned"}))))?;

        // Convert legacy identifiers
        let identifiers: Vec<Identifier> = payload.identifiers
            .unwrap_or_default()
            .into_iter()
            .map(|id| Identifier::new(id.key, id.value))
            .collect();

        // Convert enhanced identifiers
        let enhanced_identifiers: Vec<EnhancedIdentifier> = payload.enhanced_identifiers
            .unwrap_or_default()
            .into_iter()
            .filter_map(|req| {
                match req.id_type.as_str() {
                    "Canonical" => Some(EnhancedIdentifier::canonical(&req.namespace, &req.key, &req.value)),
                    "Contextual" => Some(EnhancedIdentifier::contextual(&req.namespace, &req.key, &req.value)),
                    _ => None,
                }
            })
            .collect();

        // Generate source entry
        let source_entry = Uuid::new_v4();

        engine.create_local_item(identifiers, enhanced_identifiers, payload.enriched_data, source_entry)
            .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error": format!("Failed to create local item: {}", e)}))))?
    }; // Lock dropped here

    // Write-through cache: Also persist to PostgreSQL if available
    let pg_lock = state.postgres_persistence.read().await;
    if let Some(pg) = &*pg_lock {
        if let Err(e) = pg.persist_item(&item).await {
            tracing::warn!("Failed to persist item to PostgreSQL: {}", e);
            // Don't fail the request - in-memory write succeeded
        }
    }
    drop(pg_lock);

    // Extract local_id from item
    let local_id = item.local_id.unwrap_or_else(|| {
        // Extract from temporary DFID format "LID-{uuid}"
        let dfid = &item.dfid;
        if dfid.starts_with("LID-") {
            Uuid::parse_str(&dfid[4..]).unwrap_or_else(|_| Uuid::new_v4())
        } else {
            Uuid::new_v4()
        }
    });

    Ok(Json(CreateLocalItemResponse {
        success: true,
        data: LocalItemData {
            local_id: local_id.to_string(),
            status: "LocalOnly".to_string(),
        },
    }))
}

async fn get_lid_dfid_mapping(
    State(state): State<Arc<AppState>>,
    Path(local_id_str): Path<String>,
) -> Result<Json<LidDfidMappingResponse>, (StatusCode, Json<Value>)> {
    let engine = state.items_engine.lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Items engine mutex poisoned"}))))?;

    let local_id = match Uuid::parse_str(&local_id_str) {
        Ok(uuid) => uuid,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Invalid local_id format"})),
            ))
        }
    };

    // Try to get item by LID
    match engine.get_item_by_lid(&local_id) {
        Ok(Some(item)) => {
            // Check if it's a local-only item or tokenized
            let (dfid, status) = if item.dfid.starts_with("LID-") {
                (None, "LocalOnly".to_string())
            } else {
                (Some(item.dfid), "Tokenized".to_string())
            };

            Ok(Json(LidDfidMappingResponse {
                success: true,
                data: LidDfidMappingData {
                    local_id: local_id_str,
                    dfid,
                    status,
                },
            }))
        }
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Local item not found"})),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to get mapping: {}", e)})),
        )),
    }
}

#[derive(Debug, Serialize)]
struct StorageHistoryResponse {
    success: bool,
    dfid: String,
    records: Vec<StorageRecordResponse>,
}

#[derive(Debug, Serialize)]
struct StorageRecordResponse {
    adapter_type: String,
    network: Option<String>,
    nft_mint_tx: Option<String>,
    ipcm_update_tx: Option<String>,
    ipfs_cid: Option<String>,
    ipfs_pinned: Option<bool>,
    nft_contract: Option<String>,
    storage_location: String,
    stored_at: String,
    triggered_by: String,
    is_active: bool,
}

/// GET /api/items/:dfid/storage-history
/// Retrieve all storage records for a DFID (NFT mint transactions, IPCM updates, IPFS CIDs)
async fn get_storage_history(
    Path(dfid): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<StorageHistoryResponse>, (StatusCode, Json<Value>)> {
    // Get storage history from shared storage
    let storage_guard = state.shared_storage.lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Storage mutex poisoned"}))))?;

    match storage_guard.get_storage_history(&dfid) {
        Ok(Some(history)) => {
            let records: Vec<StorageRecordResponse> = history.storage_records.iter().map(|record| {
                // Extract metadata
                let network = record.metadata.get("network")
                    .and_then(|v| v.as_str().map(String::from));
                let nft_mint_tx = record.metadata.get("nft_mint_tx")
                    .and_then(|v| v.as_str().map(String::from));
                let ipcm_update_tx = record.metadata.get("ipcm_update_tx")
                    .and_then(|v| v.as_str().map(String::from));
                let ipfs_cid = record.metadata.get("ipfs_cid")
                    .and_then(|v| v.as_str().map(String::from));
                let ipfs_pinned = record.metadata.get("ipfs_pinned")
                    .and_then(|v| v.as_bool());
                let nft_contract = record.metadata.get("nft_contract")
                    .and_then(|v| v.as_str().map(String::from));

                StorageRecordResponse {
                    adapter_type: format!("{:?}", record.adapter_type),
                    network,
                    nft_mint_tx,
                    ipcm_update_tx,
                    ipfs_cid,
                    ipfs_pinned,
                    nft_contract,
                    storage_location: format!("{:?}", record.storage_location),
                    stored_at: record.stored_at.to_rfc3339(),
                    triggered_by: record.triggered_by.clone(),
                    is_active: record.is_active,
                }
            }).collect();

            Ok(Json(StorageHistoryResponse {
                success: true,
                dfid: dfid.clone(),
                records,
            }))
        }
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "No storage history found for this DFID"})),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to retrieve storage history: {}", e)})),
        )),
    }
}
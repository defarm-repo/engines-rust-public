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

use crate::{ItemsEngine, InMemoryStorage, Item, ItemStatus, Identifier};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct IdentifierRequest {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateItemRequest {
    pub dfid: String,
    pub canonical_identifiers: Vec<IdentifierRequest>,
    pub enriched_data: Option<HashMap<String, serde_json::Value>>,
    pub source_entry: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateItemRequest {
    pub enriched_data: Option<HashMap<String, serde_json::Value>>,
    pub identifiers: Option<Vec<IdentifierRequest>>,
}

#[derive(Debug, Deserialize)]
pub struct SplitItemRequest {
    pub identifiers_for_new_item: Vec<IdentifierRequest>,
    pub new_dfid: String,
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
    pub canonical_identifiers: Vec<IdentifierRequest>,
    pub enriched_data: HashMap<String, serde_json::Value>,
    pub creation_timestamp: i64,
    pub last_modified: i64,
    pub source_entries: Vec<String>,
    pub status: String,
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

pub fn item_routes() -> Router {
    let state = Arc::new(ItemState::new());

    Router::new()
        .route("/", post(create_item))
        .route("/", get(list_items))
        .route("/:dfid", get(get_item))
        .route("/:dfid", put(update_item))
        .route("/:dfid", delete(delete_item))
        .route("/:dfid/merge", post(merge_items))
        .route("/:dfid/split", post(split_item))
        .route("/:dfid/deprecate", put(deprecate_item))
        .route("/search", get(search_items))
        .route("/stats", get(get_item_stats))
        .route("/identifier/:key/:value", get(get_items_by_identifier))
        .with_state(state)
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
        canonical_identifiers: item.canonical_identifiers
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
    State(state): State<Arc<ItemState>>,
    Json(payload): Json<CreateItemRequest>,
) -> Result<Json<ItemResponse>, (StatusCode, Json<Value>)> {
    let mut engine = state.engine.lock().unwrap();

    let source_entry = uuid::Uuid::parse_str(&payload.source_entry)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid source entry UUID"}))))?;

    let identifiers: Vec<Identifier> = payload.canonical_identifiers
        .into_iter()
        .map(|id| Identifier::new(id.key, id.value))
        .collect();

    match engine.create_item(payload.dfid, identifiers, source_entry) {
        Ok(item) => Ok(Json(item_to_response(item))),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({"error": format!("Failed to create item: {}", e)})))),
    }
}

async fn get_item(
    State(state): State<Arc<ItemState>>,
    Path(dfid): Path<String>,
) -> Result<Json<ItemResponse>, (StatusCode, Json<Value>)> {
    let engine = state.engine.lock().unwrap();

    match engine.get_item(&dfid) {
        Ok(Some(item)) => Ok(Json(item_to_response(item))),
        Ok(None) => Err((StatusCode::NOT_FOUND, Json(json!({"error": "Item not found"})))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to get item: {}", e)})))),
    }
}

async fn update_item(
    State(state): State<Arc<ItemState>>,
    Path(dfid): Path<String>,
    Json(payload): Json<UpdateItemRequest>,
) -> Result<Json<ItemResponse>, (StatusCode, Json<Value>)> {
    let mut engine = state.engine.lock().unwrap();

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
    State(state): State<Arc<ItemState>>,
    Path(dfid): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let mut engine = state.engine.lock().unwrap();

    match engine.deprecate_item(&dfid) {
        Ok(_) => Ok(Json(json!({"message": "Item deprecated successfully"}))),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({"error": format!("Failed to deprecate item: {}", e)})))),
    }
}

async fn list_items(
    State(state): State<Arc<ItemState>>,
    Query(params): Query<ItemQueryParams>,
) -> Result<Json<Vec<ItemResponse>>, (StatusCode, Json<Value>)> {
    let engine = state.engine.lock().unwrap();

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
                        item.canonical_identifiers.iter().any(|id| id.key == *key && id.value == *value)
                    });
                } else {
                    items.retain(|item| item.canonical_identifiers.iter().any(|id| id.key == *key));
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
    State(state): State<Arc<ItemState>>,
    Path(primary_dfid): Path<String>,
    Json(secondary_dfid): Json<String>,
) -> Result<Json<ItemResponse>, (StatusCode, Json<Value>)> {
    let mut engine = state.engine.lock().unwrap();

    match engine.merge_items(&primary_dfid, &secondary_dfid) {
        Ok(item) => Ok(Json(item_to_response(item))),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({"error": format!("Failed to merge items: {}", e)})))),
    }
}

async fn split_item(
    State(state): State<Arc<ItemState>>,
    Path(dfid): Path<String>,
    Json(split_request): Json<SplitItemRequest>,
) -> Result<Json<SplitItemResponse>, (StatusCode, Json<Value>)> {
    let mut engine = state.engine.lock().unwrap();

    let identifiers: Vec<Identifier> = split_request.identifiers_for_new_item
        .into_iter()
        .map(|id| Identifier::new(id.key, id.value))
        .collect();

    match engine.split_item(&dfid, identifiers, split_request.new_dfid) {
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
    State(state): State<Arc<ItemState>>,
    Path(dfid): Path<String>,
) -> Result<Json<ItemResponse>, (StatusCode, Json<Value>)> {
    let mut engine = state.engine.lock().unwrap();

    match engine.deprecate_item(&dfid) {
        Ok(item) => Ok(Json(item_to_response(item))),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({"error": format!("Failed to deprecate item: {}", e)})))),
    }
}


async fn search_items(
    State(state): State<Arc<ItemState>>,
    Query(params): Query<ItemQueryParams>,
) -> Result<Json<Vec<ItemResponse>>, (StatusCode, Json<Value>)> {
    // Reuse list_items logic for search
    list_items(State(state), Query(params)).await
}

async fn get_item_stats(
    State(state): State<Arc<ItemState>>,
) -> Result<Json<ItemStatsResponse>, (StatusCode, Json<Value>)> {
    let engine = state.engine.lock().unwrap();

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
    State(state): State<Arc<ItemState>>,
    Path((key, value)): Path<(String, String)>,
) -> Result<Json<Vec<ItemResponse>>, (StatusCode, Json<Value>)> {
    let engine = state.engine.lock().unwrap();
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
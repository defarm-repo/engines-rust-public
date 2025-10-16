use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::{Identifier, InMemoryStorage, ReceiptEngine, ReceiptError};

#[derive(Debug, Deserialize)]
pub struct CreateReceiptRequest {
    pub data: String, // Base64 encoded data
    pub identifiers: Vec<IdentifierRequest>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct IdentifierRequest {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Serialize)]
pub struct ReceiptResponse {
    pub id: String,
    pub hash: String,
    pub timestamp: i64,
    pub data_size: usize,
    pub identifiers: Vec<IdentifierRequest>,
}

#[derive(Debug, Serialize)]
pub struct VerificationResponse {
    pub is_valid: bool,
    pub receipt_id: String,
    pub original_hash: String,
    pub provided_hash: String,
    pub timestamp: i64,
}

pub struct ReceiptState {
    pub engine: Arc<Mutex<ReceiptEngine<InMemoryStorage>>>,
}

impl ReceiptState {
    pub fn new() -> Self {
        Self {
            engine: Arc::new(Mutex::new(ReceiptEngine::new(InMemoryStorage::new()))),
        }
    }
}

pub fn receipt_routes() -> Router {
    let state = Arc::new(ReceiptState::new());

    Router::new()
        .route("/", post(create_receipt))
        .route("/:id", get(get_receipt))
        .route("/:id/verify", post(verify_receipt))
        .route("/search/identifier", post(search_by_identifier))
        .route("/search/key/:key", get(search_by_key))
        .route("/search/value/:value", get(search_by_value))
        .route("/list", get(list_receipts))
        .with_state(state)
}

async fn create_receipt(
    State(state): State<Arc<ReceiptState>>,
    Json(payload): Json<CreateReceiptRequest>,
) -> Result<Json<ReceiptResponse>, (StatusCode, Json<Value>)> {
    // Decode base64 data
    let data = general_purpose::STANDARD
        .decode(&payload.data)
        .map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Invalid base64 data"})),
            )
        })?;

    // Convert identifiers
    let identifiers: Vec<Identifier> = payload
        .identifiers
        .into_iter()
        .map(|id| Identifier::new(id.key, id.value))
        .collect();

    if identifiers.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "At least one identifier is required"})),
        ));
    }

    let mut engine = state.engine.lock().unwrap();

    match engine.process_data(&data, identifiers.clone()) {
        Ok(receipt) => {
            let response = ReceiptResponse {
                id: receipt.id.to_string(),
                hash: receipt.hash,
                timestamp: receipt.timestamp.timestamp(),
                data_size: receipt.data_size,
                identifiers: receipt
                    .identifiers
                    .into_iter()
                    .map(|id| IdentifierRequest {
                        key: id.key,
                        value: id.value,
                    })
                    .collect(),
            };
            Ok(Json(response))
        }
        Err(ReceiptError::NoIdentifiers) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "At least one identifier is required"})),
        )),
        Err(ReceiptError::StorageError(e)) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Storage error: {}", e)})),
        )),
    }
}

async fn get_receipt(
    State(state): State<Arc<ReceiptState>>,
    Path(id): Path<String>,
) -> Result<Json<ReceiptResponse>, (StatusCode, Json<Value>)> {
    let receipt_id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid receipt ID format"})),
        )
    })?;

    let engine = state.engine.lock().unwrap();

    match engine.get_receipt(&receipt_id) {
        Ok(Some(receipt)) => {
            let response = ReceiptResponse {
                id: receipt.id.to_string(),
                hash: receipt.hash,
                timestamp: receipt.timestamp.timestamp(),
                data_size: receipt.data_size,
                identifiers: receipt
                    .identifiers
                    .into_iter()
                    .map(|id| IdentifierRequest {
                        key: id.key,
                        value: id.value,
                    })
                    .collect(),
            };
            Ok(Json(response))
        }
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Receipt not found"})),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Storage error: {}", e)})),
        )),
    }
}

async fn verify_receipt(
    State(state): State<Arc<ReceiptState>>,
    Path(id): Path<String>,
    Json(payload): Json<Value>,
) -> Result<Json<VerificationResponse>, (StatusCode, Json<Value>)> {
    let receipt_id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid receipt ID format"})),
        )
    })?;

    let data_b64 = payload
        .get("data")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Missing data field"})),
            )
        })?;

    let data = general_purpose::STANDARD.decode(data_b64).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid base64 data"})),
        )
    })?;

    let engine = state.engine.lock().unwrap();

    match engine.verify_data(&receipt_id, &data) {
        Ok(is_valid) => {
            if let Ok(Some(receipt)) = engine.get_receipt(&receipt_id) {
                // Calculate hash of provided data for comparison
                let provided_hash = blake3::hash(&data).to_hex().to_string();

                Ok(Json(VerificationResponse {
                    is_valid,
                    receipt_id: receipt_id.to_string(),
                    original_hash: receipt.hash,
                    provided_hash,
                    timestamp: receipt.timestamp.timestamp(),
                }))
            } else {
                Err((
                    StatusCode::NOT_FOUND,
                    Json(json!({"error": "Receipt not found"})),
                ))
            }
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Verification error: {}", e)})),
        )),
    }
}

async fn search_by_identifier(
    State(state): State<Arc<ReceiptState>>,
    Json(payload): Json<IdentifierRequest>,
) -> Result<Json<Vec<ReceiptResponse>>, (StatusCode, Json<Value>)> {
    let identifier = Identifier::new(payload.key, payload.value);
    let engine = state.engine.lock().unwrap();

    match engine.find_receipts_by_identifier(&identifier) {
        Ok(receipts) => {
            let response: Vec<ReceiptResponse> = receipts
                .into_iter()
                .map(|receipt| ReceiptResponse {
                    id: receipt.id.to_string(),
                    hash: receipt.hash,
                    timestamp: receipt.timestamp.timestamp(),
                    data_size: receipt.data_size,
                    identifiers: receipt
                        .identifiers
                        .into_iter()
                        .map(|id| IdentifierRequest {
                            key: id.key,
                            value: id.value,
                        })
                        .collect(),
                })
                .collect();
            Ok(Json(response))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Search error: {}", e)})),
        )),
    }
}

async fn search_by_key(
    State(state): State<Arc<ReceiptState>>,
    Path(key): Path<String>,
) -> Result<Json<Vec<ReceiptResponse>>, (StatusCode, Json<Value>)> {
    let engine = state.engine.lock().unwrap();

    match engine.find_receipts_by_key(&key) {
        Ok(receipts) => {
            let response: Vec<ReceiptResponse> = receipts
                .into_iter()
                .map(|receipt| ReceiptResponse {
                    id: receipt.id.to_string(),
                    hash: receipt.hash,
                    timestamp: receipt.timestamp.timestamp(),
                    data_size: receipt.data_size,
                    identifiers: receipt
                        .identifiers
                        .into_iter()
                        .map(|id| IdentifierRequest {
                            key: id.key,
                            value: id.value,
                        })
                        .collect(),
                })
                .collect();
            Ok(Json(response))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Search error: {}", e)})),
        )),
    }
}

async fn search_by_value(
    State(state): State<Arc<ReceiptState>>,
    Path(value): Path<String>,
) -> Result<Json<Vec<ReceiptResponse>>, (StatusCode, Json<Value>)> {
    let engine = state.engine.lock().unwrap();

    match engine.find_receipts_by_value(&value) {
        Ok(receipts) => {
            let response: Vec<ReceiptResponse> = receipts
                .into_iter()
                .map(|receipt| ReceiptResponse {
                    id: receipt.id.to_string(),
                    hash: receipt.hash,
                    timestamp: receipt.timestamp.timestamp(),
                    data_size: receipt.data_size,
                    identifiers: receipt
                        .identifiers
                        .into_iter()
                        .map(|id| IdentifierRequest {
                            key: id.key,
                            value: id.value,
                        })
                        .collect(),
                })
                .collect();
            Ok(Json(response))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Search error: {}", e)})),
        )),
    }
}

async fn list_receipts(
    State(state): State<Arc<ReceiptState>>,
) -> Result<Json<Vec<ReceiptResponse>>, (StatusCode, Json<Value>)> {
    let engine = state.engine.lock().unwrap();

    match engine.list_receipts() {
        Ok(receipts) => {
            let response: Vec<ReceiptResponse> = receipts
                .into_iter()
                .map(|receipt| ReceiptResponse {
                    id: receipt.id.to_string(),
                    hash: receipt.hash,
                    timestamp: receipt.timestamp.timestamp(),
                    data_size: receipt.data_size,
                    identifiers: receipt
                        .identifiers
                        .into_iter()
                        .map(|id| IdentifierRequest {
                            key: id.key,
                            value: id.value,
                        })
                        .collect(),
                })
                .collect();
            Ok(Json(response))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Storage error: {}", e)})),
        )),
    }
}

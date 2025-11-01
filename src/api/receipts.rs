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
use std::sync::Arc;
use uuid::Uuid;

use crate::api::items::{build_identifiers, IdentifierRequest};
use crate::api::shared_state::AppState;
use crate::storage_helpers::{with_lock_mut, StorageLockError};

#[derive(Debug, Deserialize)]
pub struct CreateReceiptRequest {
    pub data: String, // Base64 encoded data
    pub identifiers: Vec<IdentifierRequest>,
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

pub fn receipt_routes(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", post(create_receipt))
        .route("/:id", get(get_receipt))
        .route("/:id/verify", post(verify_receipt))
        .route("/search/identifier", post(search_by_identifier))
        .route("/search/key/:key", get(search_by_key))
        .route("/search/value/:value", get(search_by_value))
        .route("/list", get(list_receipts))
        .with_state(app_state)
}

async fn create_receipt(
    State(state): State<Arc<AppState>>,
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
    let identifiers = build_identifiers(payload.identifiers).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("Invalid identifier payload: {}", e)})),
        )
    })?;

    if identifiers.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "At least one identifier is required"})),
        ));
    }

    let receipt = with_lock_mut(
        &state.receipt_engine,
        "receipts::create_receipt::process_data",
        |engine| {
            engine
                .process_data(&data, identifiers.clone())
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        },
    )
    .map_err(|e| match e {
        StorageLockError::Timeout => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "Service temporarily unavailable, please retry"})),
        ),
        StorageLockError::Other(msg) => {
            if msg.contains("At least one identifier is required") {
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({"error": "At least one identifier is required"})),
                )
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": format!("Storage error: {}", msg)})),
                )
            }
        }
    })?;

    let response = ReceiptResponse {
        id: receipt.id.to_string(),
        hash: receipt.hash,
        timestamp: receipt.timestamp.timestamp(),
        data_size: receipt.data_size,
        identifiers: receipt
            .identifiers
            .into_iter()
            .map(|id| IdentifierRequest::from_identifier(&id))
            .collect(),
    };
    Ok(Json(response))
}

async fn get_receipt(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ReceiptResponse>, (StatusCode, Json<Value>)> {
    let receipt_id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid receipt ID format"})),
        )
    })?;

    let receipt_opt = with_lock_mut(
        &state.receipt_engine,
        "receipts::get_receipt::get_receipt",
        |engine| {
            engine
                .get_receipt(&receipt_id)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        },
    )
    .map_err(|e| match e {
        StorageLockError::Timeout => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "Service temporarily unavailable, please retry"})),
        ),
        StorageLockError::Other(msg) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Storage error: {}", msg)})),
        ),
    })?;

    match receipt_opt {
        Some(receipt) => {
            let response = ReceiptResponse {
                id: receipt.id.to_string(),
                hash: receipt.hash,
                timestamp: receipt.timestamp.timestamp(),
                data_size: receipt.data_size,
                identifiers: receipt
                    .identifiers
                    .into_iter()
                    .map(|id| IdentifierRequest::from_identifier(&id))
                    .collect(),
            };
            Ok(Json(response))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Receipt not found"})),
        )),
    }
}

async fn verify_receipt(
    State(state): State<Arc<AppState>>,
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

    let (is_valid, receipt_opt) = with_lock_mut(
        &state.receipt_engine,
        "receipts::verify_receipt::verify_and_get",
        |engine| {
            let is_valid = engine
                .verify_data(&receipt_id, &data)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            let receipt_opt = engine
                .get_receipt(&receipt_id)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            Ok((is_valid, receipt_opt))
        },
    )
    .map_err(|e| match e {
        StorageLockError::Timeout => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "Service temporarily unavailable, please retry"})),
        ),
        StorageLockError::Other(msg) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Verification error: {}", msg)})),
        ),
    })?;

    match receipt_opt {
        Some(receipt) => {
            // Calculate hash of provided data for comparison
            let provided_hash = blake3::hash(&data).to_hex().to_string();

            Ok(Json(VerificationResponse {
                is_valid,
                receipt_id: receipt_id.to_string(),
                original_hash: receipt.hash,
                provided_hash,
                timestamp: receipt.timestamp.timestamp(),
            }))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Receipt not found"})),
        )),
    }
}

async fn search_by_identifier(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<IdentifierRequest>,
) -> Result<Json<Vec<ReceiptResponse>>, (StatusCode, Json<Value>)> {
    let identifier = payload.into_identifier().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("Invalid identifier payload: {}", e)})),
        )
    })?;

    let receipts = with_lock_mut(
        &state.receipt_engine,
        "receipts::search_by_identifier::find",
        |engine| {
            engine
                .find_receipts_by_identifier(&identifier)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        },
    )
    .map_err(|e| match e {
        StorageLockError::Timeout => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "Service temporarily unavailable, please retry"})),
        ),
        StorageLockError::Other(msg) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Search error: {}", msg)})),
        ),
    })?;

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
                .map(|id| IdentifierRequest::from_identifier(&id))
                .collect(),
        })
        .collect();
    Ok(Json(response))
}

async fn search_by_key(
    State(state): State<Arc<AppState>>,
    Path(key): Path<String>,
) -> Result<Json<Vec<ReceiptResponse>>, (StatusCode, Json<Value>)> {
    let receipts = with_lock_mut(
        &state.receipt_engine,
        "receipts::search_by_key::find",
        |engine| {
            engine
                .find_receipts_by_key(&key)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        },
    )
    .map_err(|e| match e {
        StorageLockError::Timeout => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "Service temporarily unavailable, please retry"})),
        ),
        StorageLockError::Other(msg) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Search error: {}", msg)})),
        ),
    })?;

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
                .map(|id| IdentifierRequest::from_identifier(&id))
                .collect(),
        })
        .collect();
    Ok(Json(response))
}

async fn search_by_value(
    State(state): State<Arc<AppState>>,
    Path(value): Path<String>,
) -> Result<Json<Vec<ReceiptResponse>>, (StatusCode, Json<Value>)> {
    let receipts = with_lock_mut(
        &state.receipt_engine,
        "receipts::search_by_value::find",
        |engine| {
            engine
                .find_receipts_by_value(&value)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        },
    )
    .map_err(|e| match e {
        StorageLockError::Timeout => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "Service temporarily unavailable, please retry"})),
        ),
        StorageLockError::Other(msg) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Search error: {}", msg)})),
        ),
    })?;

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
                .map(|id| IdentifierRequest::from_identifier(&id))
                .collect(),
        })
        .collect();
    Ok(Json(response))
}

async fn list_receipts(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ReceiptResponse>>, (StatusCode, Json<Value>)> {
    let receipts = with_lock_mut(
        &state.receipt_engine,
        "receipts::list_receipts::list",
        |engine| {
            engine
                .list_receipts()
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        },
    )
    .map_err(|e| match e {
        StorageLockError::Timeout => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "Service temporarily unavailable, please retry"})),
        ),
        StorageLockError::Other(msg) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Storage error: {}", msg)})),
        ),
    })?;

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
                .map(|id| IdentifierRequest::from_identifier(&id))
                .collect(),
        })
        .collect();
    Ok(Json(response))
}

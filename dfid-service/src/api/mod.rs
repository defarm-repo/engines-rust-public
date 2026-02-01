use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

use crate::engine::DfidEngine;
use crate::metrics::{DFIDS_GENERATED_TOTAL, DFIDS_VALIDATED_TOTAL, CURRENT_SEQUENCE};

pub struct AppState {
    pub engine: Arc<DfidEngine>,
}

#[derive(Deserialize, ToSchema)]
pub struct GenerateRequest {
    /// Optional context for DFID generation (reserved for future use)
    #[serde(default)]
    pub context: Option<String>,
    /// Number of DFIDs to generate (1-10000, default: 1)
    #[serde(default = "default_count")]
    pub count: usize,
}

fn default_count() -> usize {
    1
}

#[derive(Serialize, ToSchema)]
pub struct GenerateResponse {
    /// List of generated DFIDs
    pub dfids: Vec<String>,
    /// DFID format version
    pub format_version: String,
    /// ISO 8601 timestamp of generation
    pub generated_at: String,
}

#[derive(Serialize, ToSchema)]
pub struct ValidateResponse {
    /// Whether the DFID is valid
    pub valid: bool,
}

#[derive(Serialize, ToSchema)]
pub struct HealthResponse {
    /// Service health status
    pub status: String,
    /// Current sequence number for today
    pub current_sequence: u64,
}

/// Generate one or more DFIDs
#[utoipa::path(
    post,
    path = "/dfid/generate",
    request_body = GenerateRequest,
    responses(
        (status = 200, description = "DFIDs generated successfully", body = GenerateResponse)
    ),
    tag = "dfid"
)]
pub async fn generate_dfid(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<GenerateRequest>,
) -> impl IntoResponse {
    let count = payload.count.clamp(1, 1000);

    let dfids = if count == 1 {
        vec![state.engine.generate_dfid()]
    } else {
        (0..count).map(|_| state.engine.generate_dfid()).collect()
    };

    // Update metrics
    DFIDS_GENERATED_TOTAL.inc_by(count as u64);
    CURRENT_SEQUENCE.set(state.engine.get_current_sequence() as i64);

    let response = GenerateResponse {
        dfids,
        format_version: "1.0".to_string(),
        generated_at: chrono::Utc::now().to_rfc3339(),
    };

    (StatusCode::OK, Json(response))
}

/// Generate a batch of DFIDs (up to 10,000)
#[utoipa::path(
    post,
    path = "/dfid/batch",
    request_body = GenerateRequest,
    responses(
        (status = 200, description = "Batch DFIDs generated successfully", body = GenerateResponse)
    ),
    tag = "dfid"
)]
pub async fn generate_batch(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<GenerateRequest>,
) -> impl IntoResponse {
    let count = payload.count.clamp(1, 10000);

    let dfids: Vec<String> = (0..count).map(|_| state.engine.generate_dfid()).collect();

    // Update metrics
    DFIDS_GENERATED_TOTAL.inc_by(count as u64);
    CURRENT_SEQUENCE.set(state.engine.get_current_sequence() as i64);

    let response = GenerateResponse {
        dfids,
        format_version: "1.0".to_string(),
        generated_at: chrono::Utc::now().to_rfc3339(),
    };

    (StatusCode::OK, Json(response))
}

/// Validate a DFID format and checksum
#[utoipa::path(
    get,
    path = "/dfid/{id}/validate",
    params(
        ("id" = String, Path, description = "DFID to validate")
    ),
    responses(
        (status = 200, description = "Validation result", body = ValidateResponse)
    ),
    tag = "dfid"
)]
pub async fn validate_dfid(
    State(state): State<Arc<AppState>>,
    Path(dfid): Path<String>,
) -> impl IntoResponse {
    let valid = state.engine.validate_dfid(&dfid).unwrap_or(false);

    // Update metrics
    DFIDS_VALIDATED_TOTAL.inc();

    let response = ValidateResponse { valid };
    (StatusCode::OK, Json(response))
}

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse)
    ),
    tag = "health"
)]
pub async fn health_check(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let response = HealthResponse {
        status: "healthy".to_string(),
        current_sequence: state.engine.get_current_sequence(),
    };

    (StatusCode::OK, Json(response))
}

/// Prometheus metrics endpoint
pub async fn metrics() -> impl IntoResponse {
    let metrics = crate::metrics::encode_metrics();
    (StatusCode::OK, metrics)
}

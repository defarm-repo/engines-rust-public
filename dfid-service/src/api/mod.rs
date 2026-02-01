use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::engine::DfidEngine;

pub struct AppState {
    pub engine: Arc<DfidEngine>,
}

#[derive(Deserialize)]
pub struct GenerateRequest {
    #[serde(default)]
    pub context: Option<String>,
    #[serde(default = "default_count")]
    pub count: usize,
}

fn default_count() -> usize {
    1
}

#[derive(Serialize)]
pub struct GenerateResponse {
    pub dfids: Vec<String>,
    pub format_version: String,
    pub generated_at: String,
}

#[derive(Serialize)]
pub struct ValidateResponse {
    pub valid: bool,
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub current_sequence: u64,
}

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

    let response = GenerateResponse {
        dfids,
        format_version: "1.0".to_string(),
        generated_at: chrono::Utc::now().to_rfc3339(),
    };

    (StatusCode::OK, Json(response))
}

pub async fn generate_batch(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<GenerateRequest>,
) -> impl IntoResponse {
    let count = payload.count.clamp(1, 10000);

    let dfids: Vec<String> = (0..count).map(|_| state.engine.generate_dfid()).collect();

    let response = GenerateResponse {
        dfids,
        format_version: "1.0".to_string(),
        generated_at: chrono::Utc::now().to_rfc3339(),
    };

    (StatusCode::OK, Json(response))
}

pub async fn validate_dfid(
    State(state): State<Arc<AppState>>,
    Path(dfid): Path<String>,
) -> impl IntoResponse {
    let valid = state.engine.validate_dfid(&dfid);

    let response = ValidateResponse { valid };
    (StatusCode::OK, Json(response))
}

pub async fn health_check(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let response = HealthResponse {
        status: "healthy".to_string(),
        current_sequence: state.engine.get_current_sequence(),
    };

    (StatusCode::OK, Json(response))
}

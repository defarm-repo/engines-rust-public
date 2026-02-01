use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use crate::engine::IndexEngine;
use crate::types::{IndexError, RegisterLocationRequest};

pub struct AppState {
    pub engine: IndexEngine,
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

/// GET /index/:dfid/locations
/// Get all known locations for a DFID
async fn get_dfid_locations(
    State(state): State<Arc<AppState>>,
    Path(dfid): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match state.engine.get_locations(&dfid).await {
        Ok(response) => Ok(Json(json!(response))),
        Err(IndexError::DfidNotFound(msg)) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": msg,
                "dfid": dfid
            })),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": format!("Failed to get locations: {}", e)
            })),
        )),
    }
}

/// POST /index/register
/// Register a new location for a DFID
async fn register_location(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RegisterLocationRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // TODO: Add authentication - for now, accept any registration
    let registered_by = "anonymous";

    match state.engine.register_location(request, registered_by).await {
        Ok(response) => Ok(Json(json!(response))),
        Err(IndexError::InvalidLocation(msg)) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": msg
            })),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": format!("Failed to register location: {}", e)
            })),
        )),
    }
}

/// GET /index/search?q=DFID-...
/// Search for DFIDs by partial match
async fn search_dfids(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match state.engine.search_dfids(&params.q).await {
        Ok(response) => Ok(Json(json!(response))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": format!("Search failed: {}", e)
            })),
        )),
    }
}

/// GET /index/stats
/// Get index statistics
async fn get_stats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match state.engine.get_stats().await {
        Ok(stats) => Ok(Json(json!({
            "success": true,
            "stats": stats
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": format!("Failed to get stats: {}", e)
            })),
        )),
    }
}

/// GET /health
/// Health check endpoint
async fn health_check() -> Json<serde_json::Value> {
    Json(json!({
        "status": "healthy",
        "service": "index-service",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/index/:dfid/locations", get(get_dfid_locations))
        .route("/index/register", post(register_location))
        .route("/index/search", get(search_dfids))
        .route("/index/stats", get(get_stats))
        .with_state(state)
}

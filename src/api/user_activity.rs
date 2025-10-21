use super::shared_state::AppState;
use crate::types::{
    UserActivity, UserActivityCategory, UserActivityFilters, UserActivityType, UserResourceType,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post},
    Router,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct RecordActivityRequest {
    pub user_id: String,
    pub workspace_id: String,
    pub activity_type: UserActivityType,
    pub category: UserActivityCategory,
    pub resource_type: UserResourceType,
    pub resource_id: String,
    pub action: String,
    pub description: String,
    #[serde(default)]
    pub metadata: serde_json::Value,
    #[serde(default = "default_success")]
    pub success: bool,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

fn default_success() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct ActivityQueryParams {
    pub category: Option<UserActivityCategory>,
    pub activity_type: Option<UserActivityType>,
    pub resource_type: Option<UserResourceType>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub search_query: Option<String>,
    pub user_id: Option<String>,
    pub page: Option<usize>,
    pub per_page: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct StatsQueryParams {
    #[serde(default = "default_period_days")]
    pub period_days: i64,
}

fn default_period_days() -> i64 {
    30
}

#[derive(Debug, Deserialize)]
pub struct CleanupRequest {
    pub before_date: DateTime<Utc>,
}

pub fn user_activity_routes(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(list_activities).post(record_activity))
        .route("/stats", get(get_activity_stats))
        .route("/cleanup", delete(cleanup_old_activities))
        .route("/:activity_id", get(get_activity_by_id))
        .with_state(app_state)
}

/// Record a new user activity
async fn record_activity(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RecordActivityRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let engine = state.activity_engine.lock().unwrap();

    // Create the activity
    let activity = UserActivity {
        activity_id: uuid::Uuid::new_v4().to_string(),
        user_id: payload.user_id,
        workspace_id: payload.workspace_id,
        timestamp: Utc::now(),
        activity_type: payload.activity_type,
        category: payload.category,
        resource_type: payload.resource_type,
        resource_id: payload.resource_id,
        action: payload.action,
        description: payload.description,
        metadata: payload.metadata,
        success: payload.success,
        ip_address: payload.ip_address,
        user_agent: payload.user_agent,
    };

    match engine.record_activity(&activity) {
        Ok(()) => Ok(Json(json!({
            "success": true,
            "data": {
                "activity_id": activity.activity_id,
                "timestamp": activity.timestamp,
            }
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to record activity: {}", e)})),
        )),
    }
}

/// List activities with filtering and pagination
async fn list_activities(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ActivityQueryParams>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let engine = state.activity_engine.lock().unwrap();

    let filters = UserActivityFilters {
        category: params.category,
        activity_type: params.activity_type,
        resource_type: params.resource_type,
        start_date: params.start_date,
        end_date: params.end_date,
        search_query: params.search_query,
        user_id: params.user_id,
        page: params.page,
        per_page: params.per_page,
    };

    match engine.list_activities(&filters) {
        Ok(response) => Ok(Json(json!({
            "success": true,
            "data": {
                "activities": response.activities,
                "total": response.total,
                "page": response.page,
                "per_page": response.per_page,
                "total_pages": response.total_pages,
            }
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to list activities: {}", e)})),
        )),
    }
}

/// Get a specific activity by ID
async fn get_activity_by_id(
    State(state): State<Arc<AppState>>,
    Path(activity_id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let engine = state.activity_engine.lock().unwrap();

    match engine.get_activity(&activity_id) {
        Ok(Some(activity)) => Ok(Json(json!({
            "success": true,
            "data": activity,
        }))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Activity not found"})),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to get activity: {}", e)})),
        )),
    }
}

/// Get activity statistics for a period
async fn get_activity_stats(
    State(state): State<Arc<AppState>>,
    Query(params): Query<StatsQueryParams>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let engine = state.activity_engine.lock().unwrap();

    match engine.get_stats(params.period_days) {
        Ok(stats) => Ok(Json(json!({
            "success": true,
            "data": stats,
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to get stats: {}", e)})),
        )),
    }
}

/// Delete activities older than a specific date
async fn cleanup_old_activities(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CleanupRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let engine = state.activity_engine.lock().unwrap();

    match engine.cleanup_old_activities(payload.before_date) {
        Ok(deleted_count) => Ok(Json(json!({
            "success": true,
            "data": {
                "deleted_count": deleted_count,
            }
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to cleanup activities: {}", e)})),
        )),
    }
}

use super::shared_state::AppState;
use crate::api::circuits::{activity_to_response, ActivityResponse};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde_json::{json, Value};
use std::sync::Arc;

pub fn activity_routes(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(get_all_activities))
        .route("/user/:user_id", get(get_user_activities))
        .with_state(app_state)
}

async fn get_all_activities(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ActivityResponse>>, (StatusCode, Json<Value>)> {
    let engine = state.circuits_engine.write().await;

    match engine.get_all_activities() {
        Ok(activities) => {
            let response: Vec<ActivityResponse> =
                activities.into_iter().map(activity_to_response).collect();
            Ok(Json(response))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to get activities: {}", e)})),
        )),
    }
}

async fn get_user_activities(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
) -> Result<Json<Vec<ActivityResponse>>, (StatusCode, Json<Value>)> {
    let engine = state.circuits_engine.write().await;

    match engine.get_activities_for_user(&user_id) {
        Ok(activities) => {
            let response: Vec<ActivityResponse> =
                activities.into_iter().map(activity_to_response).collect();
            Ok(Json(response))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to get user activities: {}", e)})),
        )),
    }
}

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use super::shared_state::AppState;
use crate::{Event, EventType, EventVisibility};

#[derive(Debug, Deserialize)]
pub struct CreateEventRequest {
    pub dfid: String,
    pub event_type: String,
    pub source: String,
    pub visibility: String,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize)]
pub struct EventResponse {
    pub event_id: String,
    pub dfid: String,
    pub event_type: String,
    pub timestamp: i64,
    pub source: String,
    pub metadata: HashMap<String, serde_json::Value>,
    pub is_encrypted: bool,
    pub visibility: String,
}

#[derive(Debug, Deserialize)]
pub struct EventQueryParams {
    pub start_date: Option<i64>,
    pub end_date: Option<i64>,
    pub event_type: Option<String>,
    pub visibility: Option<String>,
}

pub fn event_routes(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", post(create_event))
        .route("/item/:dfid", get(get_events_for_item))
        .route("/type/:event_type", get(get_events_by_type))
        .route("/visibility/:visibility", get(get_events_by_visibility))
        .route("/timeline", get(get_events_timeline))
        .route("/public", get(get_public_events))
        .route("/private", get(get_private_events))
        .route("/:event_id", get(get_event))
        .route("/:event_id/metadata", post(add_event_metadata))
        .with_state(app_state)
}

fn parse_event_type(event_type_str: &str) -> Result<EventType, String> {
    match event_type_str.to_lowercase().as_str() {
        "created" => Ok(EventType::Created),
        "enriched" => Ok(EventType::Enriched),
        "merged" => Ok(EventType::Merged),
        "split" => Ok(EventType::Split),
        "pushedtocircuit" => Ok(EventType::PushedToCircuit),
        "pulledfromcircuit" => Ok(EventType::PulledFromCircuit),
        "updated" => Ok(EventType::Updated),
        "statuschanged" => Ok(EventType::StatusChanged),
        _ => Err(format!("Invalid event type: {}", event_type_str)),
    }
}

fn parse_event_visibility(visibility_str: &str) -> Result<EventVisibility, String> {
    match visibility_str.to_lowercase().as_str() {
        "public" => Ok(EventVisibility::Public),
        "private" => Ok(EventVisibility::Private),
        "circuitonly" => Ok(EventVisibility::CircuitOnly),
        _ => Err(format!("Invalid visibility: {}", visibility_str)),
    }
}

fn event_to_response(event: Event) -> EventResponse {
    EventResponse {
        event_id: event.event_id.to_string(),
        dfid: event.dfid,
        event_type: format!("{:?}", event.event_type),
        timestamp: event.timestamp.timestamp(),
        source: event.source,
        metadata: event.metadata,
        is_encrypted: event.is_encrypted,
        visibility: format!("{:?}", event.visibility),
    }
}

async fn create_event(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateEventRequest>,
) -> Result<Json<EventResponse>, (StatusCode, Json<Value>)> {
    let event_type = parse_event_type(&payload.event_type)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error": e}))))?;

    let visibility = parse_event_visibility(&payload.visibility)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error": e}))))?;

    let mut engine = state.events_engine.lock().unwrap();

    match engine.create_event(payload.dfid, event_type, payload.source, visibility) {
        Ok(mut event) => {
            // Add metadata if provided
            if let Some(metadata) = payload.metadata {
                for (key, value) in metadata {
                    engine
                        .add_event_metadata(
                            &event.event_id,
                            [(key, value)].iter().cloned().collect(),
                        )
                        .map_err(|e| {
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(json!({"error": format!("Failed to add metadata: {}", e)})),
                            )
                        })?;
                }
                // Refresh event data
                event = engine.get_event(&event.event_id)
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to retrieve updated event: {}", e)}))))?
                    .ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Event not found after creation"}))))?;
            }

            Ok(Json(event_to_response(event)))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to create event: {}", e)})),
        )),
    }
}

async fn get_events_for_item(
    State(state): State<Arc<AppState>>,
    Path(dfid): Path<String>,
) -> Result<Json<Vec<EventResponse>>, (StatusCode, Json<Value>)> {
    let engine = state.events_engine.lock().unwrap();

    match engine.get_events_for_item(&dfid) {
        Ok(events) => {
            let response: Vec<EventResponse> = events.into_iter().map(event_to_response).collect();
            Ok(Json(response))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to get events: {}", e)})),
        )),
    }
}

async fn get_events_by_type(
    State(state): State<Arc<AppState>>,
    Path(event_type_str): Path<String>,
) -> Result<Json<Vec<EventResponse>>, (StatusCode, Json<Value>)> {
    let event_type = parse_event_type(&event_type_str)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error": e}))))?;

    let engine = state.events_engine.lock().unwrap();

    match engine.get_events_by_type(event_type) {
        Ok(events) => {
            let response: Vec<EventResponse> = events.into_iter().map(event_to_response).collect();
            Ok(Json(response))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to get events: {}", e)})),
        )),
    }
}

async fn get_events_by_visibility(
    State(state): State<Arc<AppState>>,
    Path(visibility_str): Path<String>,
) -> Result<Json<Vec<EventResponse>>, (StatusCode, Json<Value>)> {
    let visibility = parse_event_visibility(&visibility_str)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error": e}))))?;

    let engine = state.events_engine.lock().unwrap();

    match engine.get_events_by_visibility(visibility) {
        Ok(events) => {
            let response: Vec<EventResponse> = events.into_iter().map(event_to_response).collect();
            Ok(Json(response))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to get events: {}", e)})),
        )),
    }
}

async fn get_events_timeline(
    State(state): State<Arc<AppState>>,
    Query(params): Query<EventQueryParams>,
) -> Result<Json<Vec<EventResponse>>, (StatusCode, Json<Value>)> {
    let engine = state.events_engine.lock().unwrap();

    match (params.start_date, params.end_date) {
        (Some(start), Some(end)) => {
            let start_dt = chrono::DateTime::from_timestamp(start, 0).ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({"error": "Invalid start_date timestamp"})),
                )
            })?;
            let end_dt = chrono::DateTime::from_timestamp(end, 0).ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({"error": "Invalid end_date timestamp"})),
                )
            })?;

            match engine.get_events_in_time_range(start_dt, end_dt) {
                Ok(events) => {
                    let response: Vec<EventResponse> =
                        events.into_iter().map(event_to_response).collect();
                    Ok(Json(response))
                }
                Err(e) => Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": format!("Failed to get events: {}", e)})),
                )),
            }
        }
        _ => {
            // Return all events if no time range specified
            match engine.list_all_events() {
                Ok(events) => {
                    let response: Vec<EventResponse> =
                        events.into_iter().map(event_to_response).collect();
                    Ok(Json(response))
                }
                Err(e) => Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": format!("Failed to get events: {}", e)})),
                )),
            }
        }
    }
}

async fn get_public_events(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<EventResponse>>, (StatusCode, Json<Value>)> {
    let engine = state.events_engine.lock().unwrap();

    match engine.get_public_events() {
        Ok(events) => {
            let response: Vec<EventResponse> = events.into_iter().map(event_to_response).collect();
            Ok(Json(response))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to get events: {}", e)})),
        )),
    }
}

async fn get_private_events(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<EventResponse>>, (StatusCode, Json<Value>)> {
    let engine = state.events_engine.lock().unwrap();

    match engine.get_private_events() {
        Ok(events) => {
            let response: Vec<EventResponse> = events.into_iter().map(event_to_response).collect();
            Ok(Json(response))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to get events: {}", e)})),
        )),
    }
}

async fn get_event(
    State(state): State<Arc<AppState>>,
    Path(event_id): Path<String>,
) -> Result<Json<EventResponse>, (StatusCode, Json<Value>)> {
    let event_uuid = Uuid::parse_str(&event_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid event ID format"})),
        )
    })?;

    let engine = state.events_engine.lock().unwrap();

    match engine.get_event(&event_uuid) {
        Ok(Some(event)) => Ok(Json(event_to_response(event))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Event not found"})),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to get event: {}", e)})),
        )),
    }
}

async fn add_event_metadata(
    State(state): State<Arc<AppState>>,
    Path(event_id): Path<String>,
    Json(metadata): Json<HashMap<String, serde_json::Value>>,
) -> Result<Json<EventResponse>, (StatusCode, Json<Value>)> {
    let event_uuid = Uuid::parse_str(&event_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid event ID format"})),
        )
    })?;

    let mut engine = state.events_engine.lock().unwrap();

    match engine.add_event_metadata(&event_uuid, metadata) {
        Ok(event) => Ok(Json(event_to_response(event))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to add metadata: {}", e)})),
        )),
    }
}

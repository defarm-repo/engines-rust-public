use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Extension, Router,
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
    // Note: 'source' field removed - now auto-populated from authentication context
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

/// Response for event creation with deduplication info
#[derive(Debug, Serialize)]
pub struct CreateEventResponse {
    pub event_id: String,
    pub dfid: String,
    pub event_type: String,
    pub timestamp: i64,
    pub source: String,
    pub metadata: HashMap<String, serde_json::Value>,
    pub is_encrypted: bool,
    pub visibility: String,
    /// True if this event was deduplicated (already existed)
    pub was_deduplicated: bool,
    /// If deduplicated, the ID of the original event
    pub original_event_id: Option<String>,
    /// Content hash used for deduplication
    pub content_hash: String,
}

#[derive(Debug, Deserialize)]
pub struct EventQueryParams {
    pub start_date: Option<i64>,
    pub end_date: Option<i64>,
    pub event_type: Option<String>,
    pub visibility: Option<String>,
}

/// Request for creating a local event (no DFID yet)
#[derive(Debug, Deserialize)]
pub struct CreateLocalEventRequest {
    pub event_type: String,
    pub visibility: String,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Response for local event creation
#[derive(Debug, Serialize)]
pub struct LocalEventResponse {
    pub event_id: String,
    pub local_event_id: String,
    pub event_type: String,
    pub timestamp: i64,
    pub source: String,
    pub metadata: HashMap<String, serde_json::Value>,
    pub visibility: String,
    pub is_local: bool,
}

pub fn event_routes(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", post(create_event))
        .route("/local", post(create_local_event))
        .route("/local/:local_event_id", get(get_local_event))
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
        _ => Err(format!("Invalid event type: {event_type_str}")),
    }
}

fn parse_event_visibility(visibility_str: &str) -> Result<EventVisibility, String> {
    match visibility_str.to_lowercase().as_str() {
        "public" => Ok(EventVisibility::Public),
        "private" => Ok(EventVisibility::Private),
        "circuitonly" => Ok(EventVisibility::CircuitOnly),
        _ => Err(format!("Invalid visibility: {visibility_str}")),
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
    claims: Option<Extension<crate::api::auth::Claims>>,
    api_key_ctx: Option<Extension<crate::api_key_middleware::ApiKeyContext>>,
    Json(payload): Json<CreateEventRequest>,
) -> Result<Json<CreateEventResponse>, (StatusCode, Json<Value>)> {
    let event_type = parse_event_type(&payload.event_type)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error": e}))))?;

    let visibility = parse_event_visibility(&payload.visibility)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error": e}))))?;

    // Auto-populate source from authenticated context (JWT or API key)
    let source = if let Some(Extension(claims)) = claims {
        claims.user_id.clone()
    } else if let Some(Extension(ctx)) = api_key_ctx {
        ctx.user_id.to_string()
    } else {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Authentication required. Use JWT token or API key."})),
        ));
    };

    let mut engine = state.events_engine.write().await;

    // Use create_event_with_metadata for automatic deduplication
    let metadata = payload.metadata.unwrap_or_default();

    match engine.create_event_with_metadata(payload.dfid, event_type, source, visibility, metadata)
    {
        Ok(result) => {
            let event = result.event.clone();

            // Only persist to PostgreSQL if this is a NEW event (not deduplicated)
            if !result.was_deduplicated {
                drop(engine);

                let event_clone = event.clone();
                let state_clone = Arc::clone(&state);
                tokio::spawn(async move {
                    let pg_lock = state_clone.postgres_persistence.read().await;
                    if let Some(pg) = &*pg_lock {
                        if let Err(e) = pg.persist_event(&event_clone).await {
                            tracing::warn!(
                                "Failed to persist event {} to PostgreSQL: {}",
                                event_clone.event_id,
                                e
                            );
                        }
                    }
                });
            } else {
                drop(engine);
                tracing::info!(
                    "Event deduplicated - returning existing event {}",
                    event.event_id
                );
            }

            Ok(Json(CreateEventResponse {
                event_id: event.event_id.to_string(),
                dfid: event.dfid.clone(),
                event_type: format!("{:?}", event.event_type),
                timestamp: event.timestamp.timestamp(),
                source: event.source.clone(),
                metadata: event.metadata.clone(),
                is_encrypted: event.is_encrypted,
                visibility: format!("{:?}", event.visibility),
                was_deduplicated: result.was_deduplicated,
                original_event_id: result.original_event_id.map(|id| id.to_string()),
                content_hash: event.content_hash.clone(),
            }))
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
    let engine = state.events_engine.write().await;

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

    let engine = state.events_engine.write().await;

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

    let engine = state.events_engine.write().await;

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
    let engine = state.events_engine.write().await;

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
    let engine = state.events_engine.write().await;

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
    let engine = state.events_engine.write().await;

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

    let engine = state.events_engine.write().await;

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

    let mut engine = state.events_engine.write().await;

    match engine.add_event_metadata(&event_uuid, metadata) {
        Ok(event) => Ok(Json(event_to_response(event))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to add metadata: {}", e)})),
        )),
    }
}

/// Create a local event (without DFID yet)
/// Local events are stored with a temporary DFID until pushed to a circuit
async fn create_local_event(
    State(state): State<Arc<AppState>>,
    claims: Option<Extension<crate::api::auth::Claims>>,
    api_key_ctx: Option<Extension<crate::api_key_middleware::ApiKeyContext>>,
    Json(payload): Json<CreateLocalEventRequest>,
) -> Result<Json<LocalEventResponse>, (StatusCode, Json<Value>)> {
    let event_type = parse_event_type(&payload.event_type)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error": e}))))?;

    let visibility = parse_event_visibility(&payload.visibility)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error": e}))))?;

    // Auto-populate source from authenticated context (JWT or API key)
    let source = if let Some(Extension(claims)) = claims {
        claims.user_id.clone()
    } else if let Some(Extension(ctx)) = api_key_ctx {
        ctx.user_id.to_string()
    } else {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Authentication required. Use JWT token or API key."})),
        ));
    };

    let mut engine = state.events_engine.write().await;
    let metadata = payload.metadata.unwrap_or_default();

    match engine.create_local_event(event_type, source, visibility, metadata) {
        Ok(result) => {
            let event = result.event;
            let local_event_id = event
                .local_event_id
                .map(|id| id.to_string())
                .unwrap_or_default();

            Ok(Json(LocalEventResponse {
                event_id: event.event_id.to_string(),
                local_event_id,
                event_type: format!("{:?}", event.event_type),
                timestamp: event.timestamp.timestamp(),
                source: event.source,
                metadata: event.metadata,
                visibility: format!("{:?}", event.visibility),
                is_local: event.is_local,
            }))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to create local event: {}", e)})),
        )),
    }
}

/// Get a local event by its local_event_id
async fn get_local_event(
    State(state): State<Arc<AppState>>,
    Path(local_event_id): Path<String>,
) -> Result<Json<LocalEventResponse>, (StatusCode, Json<Value>)> {
    let local_event_uuid = Uuid::parse_str(&local_event_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid local_event_id format"})),
        )
    })?;

    let engine = state.events_engine.read().await;

    match engine.get_local_event(&local_event_uuid) {
        Ok(Some(event)) => {
            let local_event_id = event
                .local_event_id
                .map(|id| id.to_string())
                .unwrap_or_default();

            Ok(Json(LocalEventResponse {
                event_id: event.event_id.to_string(),
                local_event_id,
                event_type: format!("{:?}", event.event_type),
                timestamp: event.timestamp.timestamp(),
                source: event.source,
                metadata: event.metadata,
                visibility: format!("{:?}", event.visibility),
                is_local: event.is_local,
            }))
        }
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Local event not found"})),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to get local event: {}", e)})),
        )),
    }
}

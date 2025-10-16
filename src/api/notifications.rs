use axum::{
    extract::{
        ws::{Message, WebSocket},
        Extension, Path, Query, State, WebSocketUpgrade,
    },
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, patch},
    Json, Router,
};
use futures::{sink::SinkExt, stream::StreamExt};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::interval;
use tracing::{error, info, warn};

use crate::api::auth::Claims;
use crate::api::shared_state::AppState;

#[derive(Debug, Clone, Serialize)]
pub struct NotificationMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub notification: crate::types::Notification,
}

// WebSocket connection manager
pub type NotificationSender = broadcast::Sender<NotificationMessage>;

#[derive(Debug, Deserialize)]
pub struct NotificationQuery {
    pub since: Option<i64>,
    pub limit: Option<usize>,
    pub unread_only: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct WebSocketQuery {
    pub token: String,
}

// REST API routes (protected by JWT middleware)
pub fn notifications_rest_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_notifications))
        .route("/unread-count", get(get_unread_count))
        .route("/:id/read", patch(mark_notification_read))
        .route("/:id", delete(delete_notification))
        .route("/mark-all-read", patch(mark_all_read))
}

// WebSocket route (NOT protected by middleware - verifies token manually from query param)
pub fn notifications_ws_route(notification_tx: NotificationSender) -> Router<Arc<AppState>> {
    Router::new().route(
        "/ws",
        get({
            let tx = notification_tx.clone();
            move |ws, state, query| websocket_handler(ws, state, query, tx.clone())
        }),
    )
}

// GET /api/notifications - Get user's notifications
async fn get_notifications(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<NotificationQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let notification_engine = state.notification_engine.lock().unwrap();

    let since = params
        .since
        .map(|ts| chrono::DateTime::from_timestamp(ts, 0).unwrap_or_else(chrono::Utc::now));

    let notifications = notification_engine
        .get_user_notifications(
            &claims.user_id,
            since,
            params.limit,
            params.unread_only.unwrap_or(false),
        )
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Failed to get notifications: {}", e)})),
            )
        })?;

    Ok(Json(json!({
        "success": true,
        "data": notifications,
        "count": notifications.len()
    })))
}

// GET /api/notifications/unread-count - Get count of unread notifications
async fn get_unread_count(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let notification_engine = state.notification_engine.lock().unwrap();

    let count = notification_engine
        .get_unread_count(&claims.user_id)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Failed to get unread count: {}", e)})),
            )
        })?;

    Ok(Json(json!({
        "success": true,
        "unread_count": count
    })))
}

// PATCH /api/notifications/:id/read - Mark notification as read
async fn mark_notification_read(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let notification_engine = state.notification_engine.lock().unwrap();

    let notification = notification_engine
        .mark_as_read(&id, &claims.user_id)
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("Failed to mark notification as read: {}", e)})),
            )
        })?;

    Ok(Json(json!({
        "success": true,
        "data": notification
    })))
}

// DELETE /api/notifications/:id - Delete notification
async fn delete_notification(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let notification_engine = state.notification_engine.lock().unwrap();

    notification_engine
        .delete_notification(&id, &claims.user_id)
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("Failed to delete notification: {}", e)})),
            )
        })?;

    Ok(Json(json!({
        "success": true,
        "message": "Notification deleted successfully"
    })))
}

// PATCH /api/notifications/mark-all-read - Mark all notifications as read
async fn mark_all_read(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let notification_engine = state.notification_engine.lock().unwrap();

    let count = notification_engine
        .mark_all_as_read(&claims.user_id)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Failed to mark all as read: {}", e)})),
            )
        })?;

    Ok(Json(json!({
        "success": true,
        "marked_read": count
    })))
}

// WebSocket handler for real-time notifications
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Query(query): Query<WebSocketQuery>,
    notification_tx: NotificationSender,
) -> Response {
    info!("WebSocket upgrade request received");

    // Verify JWT token from query parameter
    let claims = match decode::<Claims>(
        &query.token,
        &DecodingKey::from_secret(state.jwt_secret.as_ref()),
        &Validation::default(),
    ) {
        Ok(token_data) => {
            info!(
                "WebSocket token verified for user: {}",
                token_data.claims.user_id
            );
            token_data.claims
        }
        Err(e) => {
            error!("WebSocket token verification failed: {}", e);
            // Return HTTP 401 Unauthorized instead of upgrading
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid authentication token"})),
            )
                .into_response();
        }
    };

    info!(
        "WebSocket connection established for user: {}",
        claims.user_id
    );
    ws.on_upgrade(move |socket| handle_socket(socket, state, claims.user_id, notification_tx))
}

async fn handle_socket(
    socket: WebSocket,
    state: Arc<AppState>,
    user_id: String,
    notification_tx: NotificationSender,
) {
    info!("WebSocket handler started for user: {}", user_id);

    let (mut sender, mut receiver) = socket.split();
    let mut rx = notification_tx.subscribe();

    // Create a ping interval (every 30 seconds)
    let mut ping_interval = interval(Duration::from_secs(30));

    // Send initial connection message
    let welcome_msg = json!({
        "type": "connected",
        "message": "WebSocket connected to notifications",
        "user_id": user_id
    });

    if let Err(e) = sender.send(Message::Text(welcome_msg.to_string())).await {
        error!("Failed to send welcome message to {}: {}", user_id, e);
        return;
    }

    info!("Welcome message sent to user: {}", user_id);

    // Send initial unread count
    let unread_count = {
        if let Ok(notification_engine) = state.notification_engine.lock() {
            notification_engine.get_unread_count(&user_id).ok()
        } else {
            None
        }
    };

    if let Some(count) = unread_count {
        let count_msg = json!({
            "type": "unread_count",
            "count": count
        });
        let _ = sender.send(Message::Text(count_msg.to_string())).await;
    }

    // Spawn a task to handle incoming WebSocket messages from client
    let user_id_clone = user_id.clone();
    let state_clone = state.clone();
    let client_msg_task = tokio::spawn(async move {
        while let Some(msg_result) = receiver.next().await {
            match msg_result {
                Ok(Message::Text(text)) => {
                    // Handle client messages (e.g., mark as read, requests)
                    if let Ok(request) = serde_json::from_str::<Value>(&text) {
                        handle_client_message(request, &state_clone, &user_id_clone).await;
                    }
                }
                Ok(Message::Close(frame)) => {
                    info!("Client {} initiated close: {:?}", user_id_clone, frame);
                    break;
                }
                Ok(Message::Pong(_)) => {
                    // Received pong response - connection is alive
                }
                Ok(Message::Ping(_data)) => {
                    // Client sent ping - we should have auto-responded with pong
                    info!("Received ping from {}", user_id_clone);
                }
                Err(e) => {
                    error!("WebSocket error for user {}: {}", user_id_clone, e);
                    break;
                }
                _ => {}
            }
        }
        info!("Client message handler ending for user: {}", user_id_clone);
    });

    // Main loop: send notifications and heartbeat
    loop {
        tokio::select! {
            // Handle ping interval
            _ = ping_interval.tick() => {
                if sender.send(Message::Ping(vec![])).await.is_err() {
                    warn!("Failed to send ping to {}, connection likely closed", user_id);
                    break;
                }
            }

            // Handle broadcast notifications
            result = rx.recv() => {
                match result {
                    Ok(notification_msg) => {
                        // Only send notifications for this user
                        if notification_msg.notification.user_id == user_id {
                            let msg = json!({
                                "type": "notification",
                                "data": notification_msg.notification
                            });

                            if let Err(e) = sender.send(Message::Text(msg.to_string())).await {
                                warn!("Failed to send notification to {}: {}", user_id, e);
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("Client {} lagged by {} messages", user_id, n);
                        // Send a lag notification to client
                        let lag_msg = json!({
                            "type": "lag",
                            "message": format!("Connection lagged by {} messages", n),
                            "missed_count": n
                        });
                        let _ = sender.send(Message::Text(lag_msg.to_string())).await;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        warn!("Notification broadcast channel closed");
                        break;
                    }
                }
            }
        }
    }

    // Cleanup
    client_msg_task.abort();
    info!("WebSocket connection closed for user: {}", user_id);

    // Send close frame
    let _ = sender.send(Message::Close(None)).await;
}

async fn handle_client_message(request: Value, state: &Arc<AppState>, user_id: &str) {
    if let Some(action) = request.get("action").and_then(|v| v.as_str()) {
        match action {
            "mark_read" => {
                if let Some(notification_id) =
                    request.get("notification_id").and_then(|v| v.as_str())
                {
                    if let Ok(notification_engine) = state.notification_engine.lock() {
                        match notification_engine.mark_as_read(notification_id, user_id) {
                            Ok(_) => info!(
                                "Marked notification {} as read for user {}",
                                notification_id, user_id
                            ),
                            Err(e) => warn!("Failed to mark notification as read: {}", e),
                        }
                    }
                }
            }
            "mark_all_read" => {
                if let Ok(notification_engine) = state.notification_engine.lock() {
                    match notification_engine.mark_all_as_read(user_id) {
                        Ok(count) => info!(
                            "Marked {} notifications as read for user {}",
                            count, user_id
                        ),
                        Err(e) => warn!("Failed to mark all as read: {}", e),
                    }
                }
            }
            "ping" => {
                // Client sent ping via text message (not WebSocket ping frame)
                info!("Received application-level ping from {}", user_id);
            }
            _ => {
                warn!("Unknown action received from {}: {}", user_id, action);
            }
        }
    }
}

// Helper function to broadcast notification to all connected WebSocket clients
pub async fn broadcast_notification(
    tx: &NotificationSender,
    notification: crate::types::Notification,
) -> Result<(), Box<dyn std::error::Error>> {
    let msg = NotificationMessage {
        msg_type: "notification".to_string(),
        notification,
    };

    match tx.send(msg) {
        Ok(count) => {
            info!("Broadcasted notification to {} connected clients", count);
            Ok(())
        }
        Err(_) => {
            warn!("No active WebSocket connections to broadcast notification");
            Ok(())
        }
    }
}

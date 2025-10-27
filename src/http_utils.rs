// HTTP utility functions for consistent response handling

use axum::{
    http::{header::RETRY_AFTER, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// Returns a 503 Service Unavailable response with Retry-After: 1 header
/// Use this for transient failures like lock timeouts that clients should retry
pub fn svc_unavailable_retry(msg: &str) -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        [(RETRY_AFTER, "1")],
        Json(json!({ "error": msg })),
    )
        .into_response()
}

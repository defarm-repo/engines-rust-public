use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::sync::Arc;
use jsonwebtoken::{decode, DecodingKey, Validation};

use crate::api::auth::Claims;
use crate::api::shared_state::AppState;

/// JWT authentication middleware
/// Extracts and verifies JWT token from Authorization header
/// Injects Claims into request extensions on success
pub async fn jwt_auth_middleware(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    // Extract JWT token from Authorization header
    let token = extract_jwt_token(&request)
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Missing authentication token"})),
            )
        })?;

    // Verify and decode token using jwt_secret from AppState
    let claims = decode::<Claims>(
        &token,
        &DecodingKey::from_secret(state.jwt_secret.as_ref()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|e| {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": format!("Invalid token: {}", e)})),
        )
    })?;

    // Insert claims into request extensions
    request.extensions_mut().insert(claims);

    // Continue to next handler
    Ok(next.run(request).await)
}

/// Extract JWT token from Authorization header (Bearer token)
fn extract_jwt_token(request: &Request) -> Option<String> {
    let auth_header = request
        .headers()
        .get("Authorization")?
        .to_str()
        .ok()?;

    // Support "Bearer <token>" format
    if auth_header.starts_with("Bearer ") {
        Some(auth_header[7..].to_string())
    } else {
        None
    }
}

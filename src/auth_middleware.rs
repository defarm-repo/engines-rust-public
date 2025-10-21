use axum::{
    async_trait,
    extract::{FromRequestParts, Request, State},
    http::{request::Parts, StatusCode},
    middleware::Next,
    response::Response,
    Json,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde_json::json;
use std::sync::Arc;

use crate::api::auth::Claims;
use crate::api::shared_state::AppState;

/// Extractor for authenticated user ID from JWT claims or API key
/// Use this in handlers to get the authenticated user's ID automatically
/// Supports both JWT token authentication and API key authentication
pub struct AuthenticatedUser(pub String);

#[async_trait]
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<serde_json::Value>);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Try to extract JWT claims first
        if let Some(claims) = parts.extensions.get::<Claims>() {
            return Ok(AuthenticatedUser(claims.user_id.clone()));
        }

        // Try to extract API key context
        if let Some(api_key_ctx) = parts
            .extensions
            .get::<crate::api_key_middleware::ApiKeyContext>()
        {
            return Ok(AuthenticatedUser(api_key_ctx.user_id.to_string()));
        }

        // Neither JWT nor API key found
        Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Authentication required. Use JWT token or API key."})),
        ))
    }
}

/// JWT authentication middleware
/// Extracts and verifies JWT token from Authorization header
/// Injects Claims into request extensions on success
pub async fn jwt_auth_middleware(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    // Extract JWT token from Authorization header
    let token = extract_jwt_token(&request).ok_or_else(|| {
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
    let auth_header = request.headers().get("Authorization")?.to_str().ok()?;

    // Support "Bearer <token>" format
    auth_header.strip_prefix("Bearer ").map(|s| s.to_string())
}

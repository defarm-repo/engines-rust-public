use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::api_key_engine::{ApiKeyEngine, ApiKeyError, ApiKeyPermissions, OrganizationType};
use crate::api_key_storage::ApiKeyStorage;
use crate::logging::LoggingEngine;
use crate::rate_limiter::{RateLimitConfig, RateLimiter};

#[derive(Clone)]
pub struct ApiKeyContext {
    pub api_key_id: Uuid,
    pub user_id: Uuid,
    pub organization_type: OrganizationType,
    pub permissions: ApiKeyPermissions,
    pub rate_limit_per_hour: u32,
}

// Extension trait to add API key context to request extensions
pub trait ApiKeyContextExt {
    fn api_key_context(&self) -> Option<&ApiKeyContext>;
}

impl ApiKeyContextExt for Request {
    fn api_key_context(&self) -> Option<&ApiKeyContext> {
        self.extensions().get::<ApiKeyContext>()
    }
}

#[derive(Clone)]
pub struct ApiKeyMiddlewareState<S: ApiKeyStorage> {
    pub engine: Arc<ApiKeyEngine>,
    pub storage: Arc<S>,
    pub rate_limiter: Arc<RateLimiter>,
    pub logging: Arc<Mutex<LoggingEngine>>,
}

impl<S: ApiKeyStorage> ApiKeyMiddlewareState<S> {
    pub fn new(
        engine: Arc<ApiKeyEngine>,
        storage: Arc<S>,
        rate_limiter: Arc<RateLimiter>,
        logging: Arc<Mutex<LoggingEngine>>,
    ) -> Self {
        Self {
            engine,
            storage,
            rate_limiter,
            logging,
        }
    }
}

/// Extract API key from request headers
fn extract_api_key(headers: &HeaderMap) -> Option<String> {
    // Try X-API-Key header first
    if let Some(key) = headers.get("x-api-key") {
        if let Ok(key_str) = key.to_str() {
            return Some(key_str.to_string());
        }
    }

    // Try Authorization header with Bearer scheme
    if let Some(auth) = headers.get("authorization") {
        if let Ok(auth_str) = auth.to_str() {
            if let Some(bearer_token) = auth_str.strip_prefix("Bearer ") {
                return Some(bearer_token.to_string());
            }
        }
    }

    None
}

/// Extract client IP address from request
fn extract_client_ip(headers: &HeaderMap) -> Option<IpAddr> {
    // Try X-Forwarded-For header first (for proxied requests)
    if let Some(forwarded) = headers.get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            if let Some(first_ip) = forwarded_str.split(',').next() {
                if let Ok(ip) = first_ip.trim().parse() {
                    return Some(ip);
                }
            }
        }
    }

    // Try X-Real-IP header
    if let Some(real_ip) = headers.get("x-real-ip") {
        if let Ok(ip_str) = real_ip.to_str() {
            if let Ok(ip) = ip_str.parse() {
                return Some(ip);
            }
        }
    }

    None
}

/// Create error response
fn error_response(status: StatusCode, error: &str, message: &str) -> Response {
    (
        status,
        Json(json!({
            "error": error,
            "message": message
        })),
    )
        .into_response()
}

/// API key authentication middleware
pub async fn api_key_auth_middleware<S: ApiKeyStorage + 'static>(
    State(state): State<ApiKeyMiddlewareState<S>>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, Response> {
    // Extract API key from headers
    let api_key = extract_api_key(&headers).ok_or_else(|| {
        error_response(
            StatusCode::UNAUTHORIZED,
            "missing_api_key",
            "API key required. Provide it in X-API-Key header or Authorization Bearer token",
        )
    })?;

    // Hash the key and look it up
    let key_hash = state.engine.hash_key(&api_key);

    let stored_key = state
        .storage
        .get_api_key_by_hash(&key_hash)
        .await
        .map_err(|_| {
            if let Ok(mut logger) = state.logging.lock() {
                logger.warn(
                    "api_key_middleware",
                    "invalid_key",
                    format!("Invalid API key provided with hash {key_hash}"),
                );
            }
            error_response(
                StatusCode::UNAUTHORIZED,
                "invalid_api_key",
                "The provided API key is invalid or has been revoked",
            )
        })?;

    // Validate the key
    state
        .engine
        .validate_key(&api_key, &stored_key)
        .map_err(|err| {
            let (status, error_code, message) = match err {
                ApiKeyError::Inactive => (
                    StatusCode::UNAUTHORIZED,
                    "api_key_inactive",
                    "This API key has been deactivated",
                ),
                ApiKeyError::Expired => (
                    StatusCode::UNAUTHORIZED,
                    "api_key_expired",
                    "This API key has expired",
                ),
                ApiKeyError::InvalidFormat => (
                    StatusCode::BAD_REQUEST,
                    "invalid_format",
                    "Invalid API key format",
                ),
                _ => (
                    StatusCode::UNAUTHORIZED,
                    "validation_failed",
                    "API key validation failed",
                ),
            };

            if let Ok(mut logger) = state.logging.lock() {
                logger.warn(
                    "api_key_middleware",
                    "validation_failed",
                    format!("{} (API key ID: {})", message, stored_key.id),
                );
            }

            error_response(status, error_code, message)
        })?;

    // Check IP restrictions if configured
    if let Some(client_ip) = extract_client_ip(&headers) {
        state
            .engine
            .check_ip_allowed(&stored_key, client_ip)
            .map_err(|_| {
                if let Ok(mut logger) = state.logging.lock() {
                    logger.warn(
                        "api_key_middleware",
                        "ip_not_allowed",
                        format!(
                            "Request from unauthorized IP {} for API key {}",
                            client_ip, stored_key.id
                        ),
                    );
                }
                error_response(
                    StatusCode::FORBIDDEN,
                    "ip_not_allowed",
                    "Your IP address is not authorized to use this API key",
                )
            })?;
    }

    // Check endpoint restrictions
    let endpoint = request.uri().path().to_string();
    if !state.engine.check_endpoint_allowed(&stored_key, &endpoint) {
        if let Ok(mut logger) = state.logging.lock() {
            logger.warn(
                "api_key_middleware",
                "endpoint_not_allowed",
                format!(
                    "Endpoint {} not allowed for API key {}",
                    endpoint, stored_key.id
                ),
            );
        }
        return Err(error_response(
            StatusCode::FORBIDDEN,
            "endpoint_not_allowed",
            "This API key is not authorized to access this endpoint",
        ));
    }

    // Check rate limits
    let rate_config = RateLimitConfig::new(stored_key.rate_limit_per_hour);
    let rate_result = state
        .rate_limiter
        .check_rate_limit(stored_key.id, &rate_config)
        .map_err(|_| {
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "rate_limit_error",
                "Failed to check rate limit",
            )
        })?;

    if !rate_result.allowed {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "error": "rate_limit_exceeded",
                "message": format!(
                    "Rate limit of {} requests per hour exceeded",
                    rate_result.limit
                ),
                "limit": rate_result.limit,
                "remaining": rate_result.remaining,
                "reset_at": rate_result.reset_at,
                "retry_after": rate_result.retry_after_seconds
            })),
        )
            .into_response());
    }

    // Record the request for rate limiting
    state
        .rate_limiter
        .record_request(stored_key.id)
        .map_err(|_| {
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "rate_limit_error",
                "Failed to record request",
            )
        })?;

    // Update usage statistics (fire and forget)
    let storage_clone = state.storage.clone();
    let key_id = stored_key.id;
    tokio::spawn(async move {
        let _ = storage_clone.record_usage(key_id).await;
    });

    // Add API key context to request extensions
    let context = ApiKeyContext {
        api_key_id: stored_key.id,
        user_id: stored_key.created_by,
        organization_type: stored_key.organization_type,
        permissions: stored_key.permissions,
        rate_limit_per_hour: stored_key.rate_limit_per_hour,
    };

    request.extensions_mut().insert(context);

    if let Ok(mut logger) = state.logging.lock() {
        logger.info(
            "api_key_middleware",
            "request_authenticated",
            format!(
                "Request authenticated for API key {} on endpoint {}",
                stored_key.id, endpoint
            ),
        );
    }

    Ok(next.run(request).await)
}

/// Middleware to require specific permissions
pub fn require_permission(
    permission: &'static str,
) -> impl Fn(
    Request,
    Next,
)
    -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, Response>> + Send>>
       + Clone {
    move |request: Request, next: Next| {
        Box::pin(async move {
            let context = request.extensions().get::<ApiKeyContext>().ok_or_else(|| {
                error_response(
                    StatusCode::UNAUTHORIZED,
                    "not_authenticated",
                    "Authentication required",
                )
            })?;

            if !context.permissions.has_permission(permission) {
                return Err(error_response(
                    StatusCode::FORBIDDEN,
                    "insufficient_permissions",
                    &format!("This operation requires '{permission}' permission"),
                ));
            }

            Ok(next.run(request).await)
        })
    }
}

/// Middleware to require specific organization type
pub fn require_organization_type(
    allowed_types: &'static [OrganizationType],
) -> impl Fn(
    Request,
    Next,
)
    -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, Response>> + Send>>
       + Clone {
    move |request: Request, next: Next| {
        Box::pin(async move {
            let context = request.extensions().get::<ApiKeyContext>().ok_or_else(|| {
                error_response(
                    StatusCode::UNAUTHORIZED,
                    "not_authenticated",
                    "Authentication required",
                )
            })?;

            if !allowed_types.contains(&context.organization_type) {
                let allowed_str: Vec<String> =
                    allowed_types.iter().map(|t| t.to_string()).collect();
                return Err(error_response(
                    StatusCode::FORBIDDEN,
                    "organization_type_not_allowed",
                    &format!(
                        "This endpoint requires organization type: {}",
                        allowed_str.join(" or ")
                    ),
                ));
            }

            Ok(next.run(request).await)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api_key_engine::{CreateApiKeyRequest, OrganizationType};
    use crate::api_key_storage::InMemoryApiKeyStorage;
    #[allow(dead_code)]
    async fn create_test_setup() -> (ApiKeyMiddlewareState<InMemoryApiKeyStorage>, String, Uuid) {
        let logging = Arc::new(Mutex::new(LoggingEngine::new()));
        let engine = Arc::new(ApiKeyEngine::new());
        let storage = Arc::new(InMemoryApiKeyStorage::new());
        let rate_limiter = Arc::new(RateLimiter::new());

        let user_id = Uuid::new_v4();
        let (key, _, _) = engine.generate_key();

        let request = CreateApiKeyRequest {
            name: "Test Key".to_string(),
            created_by: user_id,
            organization_type: OrganizationType::Producer,
            organization_id: None,
            permissions: Some(ApiKeyPermissions::read_write()),
            allowed_endpoints: None,
            rate_limit_per_hour: Some(100),
            expires_in_days: None,
            notes: None,
            allowed_ips: None,
        };

        let mut api_key = engine.create_api_key(request);
        api_key.key_hash = engine.hash_key(&key);

        storage.create_api_key(api_key).await.unwrap();

        let state = ApiKeyMiddlewareState::new(engine, storage, rate_limiter, logging);

        (state, key, user_id)
    }

    #[tokio::test]
    async fn test_extract_api_key_from_header() {
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", "test_key".parse().unwrap());

        let key = extract_api_key(&headers);
        assert_eq!(key, Some("test_key".to_string()));
    }

    #[tokio::test]
    async fn test_extract_api_key_from_bearer() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer test_key".parse().unwrap());

        let key = extract_api_key(&headers);
        assert_eq!(key, Some("test_key".to_string()));
    }

    #[tokio::test]
    async fn test_extract_client_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "192.168.1.1".parse().unwrap());

        let ip = extract_client_ip(&headers);
        assert_eq!(ip, Some("192.168.1.1".parse().unwrap()));
    }
}

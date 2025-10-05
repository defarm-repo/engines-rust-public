use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::sync::Arc;
use uuid::Uuid;

use crate::api::shared_state::AppState;
use crate::api_key_engine::{
    ApiKeyEngine, ApiKeyMetadata, ApiKeyPermissions, CreateApiKeyRequest, OrganizationType,
};
use crate::api_key_middleware::ApiKeyContext;
use crate::api_key_storage::ApiKeyStorage;

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyPayload {
    pub name: String,
    pub organization_type: OrganizationType,
    pub organization_id: Option<Uuid>,
    pub permissions: Option<ApiKeyPermissions>,
    pub allowed_endpoints: Option<Vec<String>>,
    pub rate_limit_per_hour: Option<u32>,
    pub expires_in_days: Option<i64>,
    pub notes: Option<String>,
    pub allowed_ips: Option<Vec<IpAddr>>,
}

#[derive(Debug, Serialize)]
pub struct CreateApiKeyResponse {
    pub api_key: String,
    pub metadata: ApiKeyMetadata,
    pub warning: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateApiKeyPayload {
    pub name: Option<String>,
    pub permissions: Option<ApiKeyPermissions>,
    pub is_active: Option<bool>,
    pub rate_limit_per_hour: Option<u32>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListApiKeysQuery {
    pub include_inactive: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct ApiKeyListItem {
    pub metadata: ApiKeyMetadata,
}

#[derive(Debug, Serialize)]
pub struct UsageStatsResponse {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub avg_response_time_ms: f64,
    pub last_used_at: Option<String>,
    pub daily_usage: Vec<DailyUsageItem>,
}

#[derive(Debug, Serialize)]
pub struct DailyUsageItem {
    pub date: String,
    pub requests: u64,
    pub errors: u64,
}

/// Create a new API key
pub async fn create_api_key<S: ApiKeyStorage + 'static>(
    State(state): State<Arc<AppState<S>>>,
    context: ApiKeyContext,
    Json(payload): Json<CreateApiKeyPayload>,
) -> Result<Json<CreateApiKeyResponse>, (StatusCode, String)> {
    // Generate the API key
    let (full_key, _, _) = state.api_key_engine.generate_key();

    let request = CreateApiKeyRequest {
        name: payload.name,
        created_by: context.user_id,
        organization_type: payload.organization_type,
        organization_id: payload.organization_id,
        permissions: payload.permissions,
        allowed_endpoints: payload.allowed_endpoints,
        rate_limit_per_hour: payload.rate_limit_per_hour,
        expires_in_days: payload.expires_in_days,
        notes: payload.notes,
        allowed_ips: payload.allowed_ips,
    };

    let mut api_key = state.api_key_engine.create_api_key(request);
    api_key.key_hash = state.api_key_engine.hash_key(&full_key);

    // Store the API key
    let stored_key = state
        .api_key_storage
        .create_api_key(api_key)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if let Ok(mut logger) = state.logging.lock() {
        logger.info(
            "api_keys",
            "key_created",
            format!("New API key created: {} by user {}", stored_key.id, context.user_id),
        );
    }

    Ok(Json(CreateApiKeyResponse {
        api_key: full_key,
        metadata: stored_key.into(),
        warning: "Save this API key securely. You won't be able to see it again.".to_string(),
    }))
}

/// List all API keys for the authenticated user
pub async fn list_api_keys<S: ApiKeyStorage + 'static>(
    State(state): State<Arc<AppState<S>>>,
    context: ApiKeyContext,
    Query(query): Query<ListApiKeysQuery>,
) -> Result<Json<Vec<ApiKeyListItem>>, (StatusCode, String)> {
    let api_keys = state
        .api_key_storage
        .get_user_api_keys(context.user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let include_inactive = query.include_inactive.unwrap_or(false);

    let items: Vec<ApiKeyListItem> = api_keys
        .into_iter()
        .filter(|key| include_inactive || key.is_active)
        .map(|key| ApiKeyListItem {
            metadata: key.into(),
        })
        .collect();

    Ok(Json(items))
}

/// Get a specific API key by ID
pub async fn get_api_key<S: ApiKeyStorage + 'static>(
    State(state): State<Arc<AppState<S>>>,
    context: ApiKeyContext,
    Path(key_id): Path<Uuid>,
) -> Result<Json<ApiKeyMetadata>, (StatusCode, String)> {
    let api_key = state
        .api_key_storage
        .get_api_key(key_id)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    // Verify ownership
    if api_key.created_by != context.user_id && !context.permissions.admin {
        return Err((
            StatusCode::FORBIDDEN,
            "You don't have permission to access this API key".to_string(),
        ));
    }

    Ok(Json(api_key.into()))
}

/// Update an API key
pub async fn update_api_key<S: ApiKeyStorage + 'static>(
    State(state): State<Arc<AppState<S>>>,
    context: ApiKeyContext,
    Path(key_id): Path<Uuid>,
    Json(payload): Json<UpdateApiKeyPayload>,
) -> Result<Json<ApiKeyMetadata>, (StatusCode, String)> {
    let mut api_key = state
        .api_key_storage
        .get_api_key(key_id)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    // Verify ownership
    if api_key.created_by != context.user_id && !context.permissions.admin {
        return Err((
            StatusCode::FORBIDDEN,
            "You don't have permission to update this API key".to_string(),
        ));
    }

    // Apply updates
    if let Some(name) = payload.name {
        api_key.name = name;
    }
    if let Some(permissions) = payload.permissions {
        api_key.permissions = permissions;
    }
    if let Some(is_active) = payload.is_active {
        api_key.is_active = is_active;
    }
    if let Some(rate_limit) = payload.rate_limit_per_hour {
        api_key.rate_limit_per_hour = rate_limit;
    }
    if let Some(notes) = payload.notes {
        api_key.notes = Some(notes);
    }

    let updated_key = state
        .api_key_storage
        .update_api_key(api_key)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if let Ok(mut logger) = state.logging.lock() {
        logger.info(
            "api_keys",
            "key_updated",
            format!("API key updated: {} by user {}", key_id, context.user_id),
        );
    }

    Ok(Json(updated_key.into()))
}

/// Delete an API key
pub async fn delete_api_key<S: ApiKeyStorage + 'static>(
    State(state): State<Arc<AppState<S>>>,
    context: ApiKeyContext,
    Path(key_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let api_key = state
        .api_key_storage
        .get_api_key(key_id)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    // Verify ownership
    if api_key.created_by != context.user_id && !context.permissions.admin {
        return Err((
            StatusCode::FORBIDDEN,
            "You don't have permission to delete this API key".to_string(),
        ));
    }

    state
        .api_key_storage
        .delete_api_key(key_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if let Ok(mut logger) = state.logging.lock() {
        logger.info(
            "api_keys",
            "key_deleted",
            format!("API key deleted: {} by user {}", key_id, context.user_id),
        );
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Revoke an API key (set as inactive)
pub async fn revoke_api_key<S: ApiKeyStorage + 'static>(
    State(state): State<Arc<AppState<S>>>,
    context: ApiKeyContext,
    Path(key_id): Path<Uuid>,
) -> Result<Json<ApiKeyMetadata>, (StatusCode, String)> {
    let mut api_key = state
        .api_key_storage
        .get_api_key(key_id)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    // Verify ownership
    if api_key.created_by != context.user_id && !context.permissions.admin {
        return Err((
            StatusCode::FORBIDDEN,
            "You don't have permission to revoke this API key".to_string(),
        ));
    }

    api_key.is_active = false;

    let updated_key = state
        .api_key_storage
        .update_api_key(api_key)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if let Ok(mut logger) = state.logging.lock() {
        logger.info(
            "api_keys",
            "key_revoked",
            format!("API key revoked: {} by user {}", key_id, context.user_id),
        );
    }

    Ok(Json(updated_key.into()))
}

/// Get usage statistics for an API key
pub async fn get_usage_stats<S: ApiKeyStorage + 'static>(
    State(state): State<Arc<AppState<S>>>,
    context: ApiKeyContext,
    Path(key_id): Path<Uuid>,
    Query(query): Query<UsageStatsQuery>,
) -> Result<Json<UsageStatsResponse>, (StatusCode, String)> {
    let api_key = state
        .api_key_storage
        .get_api_key(key_id)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    // Verify ownership
    if api_key.created_by != context.user_id && !context.permissions.admin {
        return Err((
            StatusCode::FORBIDDEN,
            "You don't have permission to view usage stats for this API key".to_string(),
        ));
    }

    let days = query.days.unwrap_or(7);
    let stats = state
        .api_key_storage
        .get_usage_stats(key_id, days)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(UsageStatsResponse {
        total_requests: stats.total_requests,
        successful_requests: stats.successful_requests,
        failed_requests: stats.failed_requests,
        avg_response_time_ms: stats.avg_response_time_ms,
        last_used_at: stats.last_used_at.map(|dt| dt.to_rfc3339()),
        daily_usage: stats
            .daily_usage
            .into_iter()
            .map(|d| DailyUsageItem {
                date: d.date,
                requests: d.requests,
                errors: d.errors,
            })
            .collect(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct UsageStatsQuery {
    pub days: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api_key_storage::InMemoryApiKeyStorage;
    use crate::logging::LoggingEngine;
    use crate::rate_limiter::RateLimiter;

    fn create_test_state() -> Arc<AppState<InMemoryApiKeyStorage>> {
        let logging = Arc::new(LoggingEngine::new());
        let api_key_engine = Arc::new(ApiKeyEngine::new());
        let api_key_storage = Arc::new(InMemoryApiKeyStorage::new());
        let rate_limiter = Arc::new(RateLimiter::new());

        Arc::new(AppState {
            logging,
            api_key_engine,
            api_key_storage,
            rate_limiter,
        })
    }

    fn create_test_context() -> ApiKeyContext {
        ApiKeyContext {
            api_key_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            organization_type: OrganizationType::Producer,
            permissions: ApiKeyPermissions::admin(),
            rate_limit_per_hour: 100,
        }
    }

    #[tokio::test]
    async fn test_create_api_key() {
        let state = create_test_state();
        let context = create_test_context();

        let payload = CreateApiKeyPayload {
            name: "Test Key".to_string(),
            organization_type: OrganizationType::Producer,
            organization_id: None,
            permissions: Some(ApiKeyPermissions::read_write()),
            allowed_endpoints: None,
            rate_limit_per_hour: Some(100),
            expires_in_days: None,
            notes: None,
            allowed_ips: None,
        };

        let result = create_api_key(State(state), context, Json(payload)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert!(response.api_key.starts_with("dfm_"));
        assert_eq!(response.metadata.name, "Test Key");
    }

    #[tokio::test]
    async fn test_list_api_keys() {
        let state = create_test_state();
        let context = create_test_context();

        // Create a key first
        let payload = CreateApiKeyPayload {
            name: "Test Key".to_string(),
            organization_type: OrganizationType::Producer,
            organization_id: None,
            permissions: None,
            allowed_endpoints: None,
            rate_limit_per_hour: None,
            expires_in_days: None,
            notes: None,
            allowed_ips: None,
        };

        create_api_key(State(state.clone()), context.clone(), Json(payload))
            .await
            .unwrap();

        // List keys
        let result = list_api_keys(
            State(state),
            context,
            Query(ListApiKeysQuery {
                include_inactive: None,
            }),
        )
        .await;

        assert!(result.is_ok());
        let keys = result.unwrap().0;
        assert_eq!(keys.len(), 1);
    }
}

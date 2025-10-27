use axum::{
    async_trait,
    extract::{FromRequestParts, Path, Query, State},
    http::{request::Parts, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::sync::Arc;
use uuid::Uuid;

use crate::api::auth::Claims;
use crate::api::shared_state::AppState;
use crate::api_key_engine::{
    ApiKeyMetadata, ApiKeyPermissions, CreateApiKeyRequest, OrganizationType,
};
use crate::api_key_storage::ApiKeyStorage;
use crate::storage_helpers::{with_lock_mut, StorageLockError};

/// Helper to extract authenticated user from JWT
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: String,
    pub workspace_id: Option<String>,
}

#[async_trait]
impl FromRequestParts<Arc<AppState>> for AuthUser {
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let claims = parts
            .extensions
            .get::<Claims>()
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    "Authentication required".to_string(),
                )
            })?
            .clone();

        Ok(AuthUser {
            user_id: claims.user_id,
            workspace_id: claims.workspace_id,
        })
    }
}

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
    #[serde(with = "crate::safe_json_numbers::u64_safe")]
    pub total_requests: u64,
    #[serde(with = "crate::safe_json_numbers::u64_safe")]
    pub successful_requests: u64,
    #[serde(with = "crate::safe_json_numbers::u64_safe")]
    pub failed_requests: u64,
    pub avg_response_time_ms: f64,
    pub last_used_at: Option<String>,
    pub daily_usage: Vec<DailyUsageItem>,
}

#[derive(Debug, Serialize)]
pub struct DailyUsageItem {
    pub date: String,
    #[serde(with = "crate::safe_json_numbers::u64_safe")]
    pub requests: u64,
    #[serde(with = "crate::safe_json_numbers::u64_safe")]
    pub errors: u64,
}

/// Create a new API key
pub async fn create_api_key(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Json(payload): Json<CreateApiKeyPayload>,
) -> Result<Json<CreateApiKeyResponse>, (StatusCode, String)> {
    // Parse user_id as UUID
    let user_uuid = Uuid::parse_str(&auth.user_id).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Invalid user ID format".to_string(),
        )
    })?;

    // Generate the API key
    let (full_key, _, _) = state.api_key_engine.generate_key();

    let request = CreateApiKeyRequest {
        name: payload.name,
        created_by: user_uuid,
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

    let log_result = with_lock_mut(
        &state.logging,
        "api_keys.rs::create_api_key::log_create",
        |logger| {
            logger.info(
                "api_keys",
                "key_created",
                format!(
                    "New API key created: {} by user {}",
                    stored_key.id, auth.user_id
                ),
            );
            Ok(())
        },
    );
    if let Err(StorageLockError::Timeout) = log_result {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "Service temporarily unavailable".to_string(),
        ));
    }

    Ok(Json(CreateApiKeyResponse {
        api_key: full_key,
        metadata: stored_key.into(),
        warning: "Save this API key securely. You won't be able to see it again.".to_string(),
    }))
}

/// List all API keys for the authenticated user
pub async fn list_api_keys(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Query(query): Query<ListApiKeysQuery>,
) -> Result<Json<Vec<ApiKeyListItem>>, (StatusCode, String)> {
    // Parse user_id as UUID
    let user_uuid = Uuid::parse_str(&auth.user_id).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Invalid user ID format".to_string(),
        )
    })?;

    let api_keys = state
        .api_key_storage
        .get_user_api_keys(user_uuid)
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
pub async fn get_api_key(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(key_id): Path<Uuid>,
) -> Result<Json<ApiKeyMetadata>, (StatusCode, String)> {
    // Parse user_id as UUID
    let user_uuid = Uuid::parse_str(&auth.user_id).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Invalid user ID format".to_string(),
        )
    })?;

    let api_key = state
        .api_key_storage
        .get_api_key(key_id)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    // Verify ownership
    if api_key.created_by != user_uuid {
        return Err((
            StatusCode::FORBIDDEN,
            "You don't have permission to access this API key".to_string(),
        ));
    }

    Ok(Json(api_key.into()))
}

/// Update an API key
pub async fn update_api_key(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(key_id): Path<Uuid>,
    Json(payload): Json<UpdateApiKeyPayload>,
) -> Result<Json<ApiKeyMetadata>, (StatusCode, String)> {
    // Parse user_id as UUID
    let user_uuid = Uuid::parse_str(&auth.user_id).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Invalid user ID format".to_string(),
        )
    })?;

    let mut api_key = state
        .api_key_storage
        .get_api_key(key_id)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    // Verify ownership
    if api_key.created_by != user_uuid {
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

    let log_result = with_lock_mut(
        &state.logging,
        "api_keys.rs::update_api_key::log_update",
        |logger| {
            logger.info(
                "api_keys",
                "key_updated",
                format!("API key updated: {} by user {}", key_id, auth.user_id),
            );
            Ok(())
        },
    );
    if let Err(StorageLockError::Timeout) = log_result {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "Service temporarily unavailable".to_string(),
        ));
    }

    Ok(Json(updated_key.into()))
}

/// Delete an API key
pub async fn delete_api_key(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(key_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    // Parse user_id as UUID
    let user_uuid = Uuid::parse_str(&auth.user_id).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Invalid user ID format".to_string(),
        )
    })?;

    let api_key = state
        .api_key_storage
        .get_api_key(key_id)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    // Verify ownership
    if api_key.created_by != user_uuid {
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

    let log_result = with_lock_mut(
        &state.logging,
        "api_keys.rs::delete_api_key::log_delete",
        |logger| {
            logger.info(
                "api_keys",
                "key_deleted",
                format!("API key deleted: {} by user {}", key_id, auth.user_id),
            );
            Ok(())
        },
    );
    if let Err(StorageLockError::Timeout) = log_result {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "Service temporarily unavailable".to_string(),
        ));
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Revoke an API key (set as inactive)
pub async fn revoke_api_key(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(key_id): Path<Uuid>,
) -> Result<Json<ApiKeyMetadata>, (StatusCode, String)> {
    // Parse user_id as UUID
    let user_uuid = Uuid::parse_str(&auth.user_id).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Invalid user ID format".to_string(),
        )
    })?;

    let mut api_key = state
        .api_key_storage
        .get_api_key(key_id)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    // Verify ownership
    if api_key.created_by != user_uuid {
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

    let log_result = with_lock_mut(
        &state.logging,
        "api_keys.rs::revoke_api_key::log_revoke",
        |logger| {
            logger.info(
                "api_keys",
                "key_revoked",
                format!("API key revoked: {} by user {}", key_id, auth.user_id),
            );
            Ok(())
        },
    );
    if let Err(StorageLockError::Timeout) = log_result {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "Service temporarily unavailable".to_string(),
        ));
    }

    Ok(Json(updated_key.into()))
}

/// Get usage statistics for an API key
pub async fn get_usage_stats(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(key_id): Path<Uuid>,
    Query(query): Query<UsageStatsQuery>,
) -> Result<Json<UsageStatsResponse>, (StatusCode, String)> {
    // Parse user_id as UUID
    let user_uuid = Uuid::parse_str(&auth.user_id).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Invalid user ID format".to_string(),
        )
    })?;

    let api_key = state
        .api_key_storage
        .get_api_key(key_id)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    // Verify ownership
    if api_key.created_by != user_uuid {
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

/// Create API key routes
pub fn api_key_routes() -> axum::Router<Arc<AppState>> {
    use axum::routing::{get, post};

    axum::Router::new()
        .route("/", get(list_api_keys).post(create_api_key))
        .route(
            "/:key_id",
            get(get_api_key)
                .patch(update_api_key)
                .delete(delete_api_key),
        )
        .route("/:key_id/revoke", post(revoke_api_key))
        .route("/:key_id/usage", get(get_usage_stats))
}

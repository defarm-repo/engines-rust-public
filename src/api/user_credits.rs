use axum::{
    extract::{Query, Request, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::api::shared_state::AppState;
use crate::api::auth::Claims;
use crate::credit_manager::CreditEngine;
use crate::tier_permission_system::TierPermissionSystem;
use crate::storage::StorageBackend;
use crate::types::{CreditTransaction, CreditCosts, UserAccount, UserTier};

// ============================================================================
// REQUEST/RESPONSE TYPES
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct CreditHistoryQuery {
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct CreditHistoryResponse {
    pub transactions: Vec<CreditTransactionResponse>,
    pub total_credits: i64,
}

#[derive(Debug, Serialize)]
pub struct CreditTransactionResponse {
    pub transaction_id: String,
    pub amount: i64,
    pub transaction_type: String,
    pub description: String,
    pub timestamp: String,
    pub balance_after: i64,
}

#[derive(Debug, Serialize)]
pub struct OperationCostsResponse {
    pub item_creation: i64,
    pub circuit_operation: i64,
    pub storage_migration: i64,
    pub audit_export: i64,
    pub premium_adapter_usage: i64,
    pub api_request: i64,
    pub tier: String,
}

#[derive(Debug, Serialize)]
pub struct UserProfileResponse {
    pub user_id: String,
    pub username: String,
    pub email: String,
    pub tier: String,
    pub credits: i64,
    pub status: String,
    pub is_admin: bool,
    pub created_at: String,
    pub subscription: Option<SubscriptionResponse>,
    pub limits: TierLimitsResponse,
}

#[derive(Debug, Serialize)]
pub struct TierLimitsResponse {
    pub max_items_per_month: Option<i64>,
    pub max_circuits: Option<i64>,
    pub max_storage_locations: Option<i64>,
    pub max_api_requests_per_hour: Option<i64>,
    pub max_workspace_members: Option<i64>,
    pub can_use_premium_adapters: bool,
    pub max_audit_retention_days: i64,
    pub priority_support: bool,
}

#[derive(Debug, Serialize)]
pub struct SubscriptionResponse {
    pub plan_id: String,
    pub status: String,
    pub started_at: String,
    pub expires_at: Option<String>,
    pub auto_renew: bool,
}

// ============================================================================
// AUTHENTICATION HELPER
// ============================================================================

/// Extract authenticated user_id from JWT Claims in request
fn get_authenticated_user_id(request: &Request) -> Result<String, (StatusCode, Json<Value>)> {
    request
        .extensions()
        .get::<Claims>()
        .map(|claims| claims.user_id.clone())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Missing authentication"})),
            )
        })
}

// ============================================================================
// HANDLERS
// ============================================================================

/// GET /users/me/credits/history
/// Get the authenticated user's credit transaction history
pub async fn get_my_credit_history(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CreditHistoryQuery>,
    request: Request,
) -> Result<Json<CreditHistoryResponse>, (StatusCode, Json<Value>)> {
    let user_id = get_authenticated_user_id(&request)?;
    let limit = query.limit.unwrap_or(50);

    // Get user account
    let storage = state.shared_storage.lock().unwrap();
    let user = storage
        .get_user_account(&user_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(json!({"error": "User not found"}))))?;

    // Get credit transactions
    let transactions = storage
        .get_credit_transactions(&user_id, Some(limit))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;

    let transaction_responses: Vec<CreditTransactionResponse> = transactions
        .into_iter()
        .map(|t| CreditTransactionResponse {
            transaction_id: t.transaction_id,
            amount: t.amount,
            transaction_type: format!("{:?}", t.transaction_type),
            description: t.description,
            timestamp: t.timestamp.to_rfc3339(),
            balance_after: t.balance_after,
        })
        .collect();

    Ok(Json(CreditHistoryResponse {
        transactions: transaction_responses,
        total_credits: user.credits,
    }))
}

/// GET /users/me/credits/costs
/// Get operation costs for the authenticated user's tier
pub async fn get_my_operation_costs(
    State(state): State<Arc<AppState>>,
    request: Request,
) -> Result<Json<OperationCostsResponse>, (StatusCode, Json<Value>)> {
    let user_id = get_authenticated_user_id(&request)?;

    // Get user account
    let storage = state.shared_storage.lock().unwrap();
    let user = storage
        .get_user_account(&user_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(json!({"error": "User not found"}))))?;

    // Get credit costs for user's tier
    let credit_costs = CreditCosts::for_tier(&user.tier);

    Ok(Json(OperationCostsResponse {
        item_creation: credit_costs.item_creation,
        circuit_operation: credit_costs.circuit_operation,
        storage_migration: credit_costs.storage_migration,
        audit_export: credit_costs.audit_export,
        premium_adapter_usage: credit_costs.premium_adapter_usage,
        api_request: credit_costs.api_request,
        tier: format!("{:?}", user.tier),
    }))
}

/// GET /users/me/profile
/// Get the authenticated user's profile information
pub async fn get_my_profile(
    State(state): State<Arc<AppState>>,
    request: Request,
) -> Result<Json<UserProfileResponse>, (StatusCode, Json<Value>)> {
    let user_id = get_authenticated_user_id(&request)?;

    // Get user account
    let storage = state.shared_storage.lock().unwrap();
    let user = storage
        .get_user_account(&user_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(json!({"error": "User not found"}))))?;

    let subscription = user.subscription.as_ref().map(|sub| SubscriptionResponse {
        plan_id: sub.plan_id.clone(),
        status: format!("{:?}", sub.status),
        started_at: sub.started_at.to_rfc3339(),
        expires_at: sub.expires_at.map(|dt| dt.to_rfc3339()),
        auto_renew: sub.auto_renew,
    });

    let limits = TierLimitsResponse {
        max_items_per_month: user.limits.max_items_per_month,
        max_circuits: user.limits.max_circuits,
        max_storage_locations: user.limits.max_storage_locations,
        max_api_requests_per_hour: user.limits.max_api_requests_per_hour,
        max_workspace_members: user.limits.max_workspace_members,
        can_use_premium_adapters: user.limits.can_use_premium_adapters,
        max_audit_retention_days: user.limits.max_audit_retention_days,
        priority_support: user.limits.priority_support,
    };

    Ok(Json(UserProfileResponse {
        user_id: user.user_id.clone(),
        username: user.username.clone(),
        email: user.email.clone(),
        tier: format!("{:?}", user.tier),
        credits: user.credits,
        status: format!("{:?}", user.status),
        is_admin: user.is_admin,
        created_at: user.created_at.to_rfc3339(),
        subscription,
        limits,
    }))
}

/// GET /users/me/credits/balance
/// Get the authenticated user's current credit balance
pub async fn get_my_credit_balance(
    State(state): State<Arc<AppState>>,
    request: Request,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let user_id = get_authenticated_user_id(&request)?;

    // Get user account
    let storage = state.shared_storage.lock().unwrap();
    let user = storage
        .get_user_account(&user_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(json!({"error": "User not found"}))))?;

    Ok(Json(json!({
        "credits": user.credits,
        "tier": format!("{:?}", user.tier),
        "last_updated": user.updated_at.to_rfc3339(),
    })))
}

// ============================================================================
// ROUTER
// ============================================================================

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/users/me/credits/history", get(get_my_credit_history))
        .route("/users/me/credits/costs", get(get_my_operation_costs))
        .route("/users/me/credits/balance", get(get_my_credit_balance))
        .route("/users/me/profile", get(get_my_profile))
        .route("/credit/users/current", get(get_my_profile))  // Frontend compatibility alias
}

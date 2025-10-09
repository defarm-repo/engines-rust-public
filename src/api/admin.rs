use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put},
    Extension,
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::api::shared_state::AppState;
use crate::api::auth::Claims;
use crate::storage::StorageBackend;
use crate::types::{
    UserAccount, UserTier, AccountStatus, CreditTransactionType,
    AdminAction, AdminActionType, TierLimits, AdapterType,
    AdapterConnectionDetails, ContractConfigs
};
use crate::credit_manager::CreditEngine;
use crate::adapter_manager::{AdapterManager};
use crate::logging::LoggingEngine;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Verify that the authenticated user is an admin
fn verify_admin(claims: &Claims, app_state: &Arc<AppState>) -> Result<(), (StatusCode, Json<Value>)> {
    let storage = app_state.shared_storage.lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Storage mutex poisoned"}))))?;
    let user = storage
        .get_user_account(&claims.user_id)
        .map_err(|e| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Database error: {}", e)}))
        ))?
        .ok_or_else(|| (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "User not found"}))
        ))?;

    if !user.is_admin {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Admin privileges required"}))
        ));
    }

    Ok(())
}

// ============================================================================
// REQUEST/RESPONSE TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    pub tier: UserTier,
    pub initial_credits: Option<i64>,
    pub workspace_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateUserRequest {
    pub tier: Option<UserTier>,
    pub status: Option<AccountStatus>,
    pub credits: Option<i64>,
    pub is_admin: Option<bool>,
    pub available_adapters: Option<Vec<AdapterType>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreditAdjustmentRequest {
    pub amount: i64,
    pub reason: String,
    pub operation_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BulkCreditGrantRequest {
    pub user_ids: Vec<String>,
    pub amount: i64,
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserSearchQuery {
    pub username: Option<String>,
    pub email: Option<String>,
    pub tier: Option<UserTier>,
    pub status: Option<AccountStatus>,
    pub is_admin: Option<bool>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserResponse {
    pub user_id: String,
    pub username: String,
    pub email: String,
    pub tier: UserTier,
    pub status: AccountStatus,
    pub credits: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
    pub is_admin: bool,
    pub workspace_id: Option<String>,
    pub limits: TierLimits,
    pub available_adapters: Option<Vec<AdapterType>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdminDashboardStats {
    pub total_users: u64,
    pub users_by_tier: std::collections::HashMap<String, u64>,
    pub users_by_status: std::collections::HashMap<String, u64>,
    pub total_credits_issued: i64,
    pub total_credits_consumed: i64,
    pub active_users_last_30_days: u64,
    pub new_users_last_30_days: u64,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateAdapterConfigRequest {
    pub name: String,
    pub description: String,
    pub adapter_type: AdapterType,
    pub connection_details: AdapterConnectionDetails,
    pub contract_configs: Option<ContractConfigs>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateAdapterConfigRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub connection_details: Option<AdapterConnectionDetails>,
    pub contract_configs: Option<ContractConfigs>,
    pub is_active: Option<bool>,
}

// ============================================================================
// HANDLER FUNCTIONS
// ============================================================================

async fn create_user(
    State(app_state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<CreateUserRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    verify_admin(&claims, &app_state)?;
    let admin_user_id = claims.user_id;

    let user_id = Uuid::new_v4().to_string();
    let password_hash = format!("hash_{}", request.password); // TODO: Use proper password hashing

    let user = UserAccount {
        user_id: user_id.clone(),
        username: request.username.clone(),
        email: request.email.clone(),
        password_hash,
        tier: request.tier.clone(),
        status: AccountStatus::Active,
        credits: request.initial_credits.unwrap_or(0),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_login: None,
        subscription: None,
        limits: TierLimits::for_tier(&request.tier),
        is_admin: false,
        workspace_id: request.workspace_id.clone(),
        available_adapters: None, // Use tier defaults
    };

    let mut storage = app_state.shared_storage.lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Storage mutex poisoned"}))))?;

    // Check if username or email already exists
    if storage.get_user_by_username(&request.username).unwrap_or(None).is_some() {
        return Ok(Json(json!({
            "success": false,
            "error": "Username already exists"
        })));
    }

    if storage.get_user_by_email(&request.email).unwrap_or(None).is_some() {
        return Ok(Json(json!({
            "success": false,
            "error": "Email already exists"
        })));
    }

    storage.store_user_account(&user).map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to store user"}))))?;

    // Record admin action
    let admin_action = AdminAction {
        action_id: Uuid::new_v4().to_string(),
        admin_user_id,
        action_type: AdminActionType::UserCreated,
        target_user_id: Some(user_id.clone()),
        target_resource_id: None,
        details: {
            let mut map = std::collections::HashMap::new();
            map.insert("username".to_string(), serde_json::json!(request.username));
            map.insert("tier".to_string(), serde_json::json!(format!("{:?}", request.tier)));
            map
        },
        timestamp: Utc::now(),
        ip_address: None,
    };

    storage.record_admin_action(&admin_action).map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to record admin action"}))))?;

    Ok(Json(json!({
        "success": true,
        "message": "User created successfully",
        "user_id": user_id,
        "username": request.username
    })))
}

async fn get_user(
    Path(user_id): Path<String>,
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let storage = app_state.shared_storage.lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Storage mutex poisoned"}))))?;

    match storage.get_user_account(&user_id).map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Database error"}))))? {
        Some(user) => {
            let response = UserResponse {
                user_id: user.user_id,
                username: user.username,
                email: user.email,
                tier: user.tier,
                status: user.status,
                credits: user.credits,
                created_at: user.created_at,
                updated_at: user.updated_at,
                last_login: user.last_login,
                is_admin: user.is_admin,
                workspace_id: user.workspace_id,
                limits: user.limits,
                available_adapters: user.available_adapters,
            };

            Ok(Json(json!({
                "success": true,
                "user": response
            })))
        }
        None => Ok(Json(json!({
            "success": false,
            "error": "User not found"
        })))
    }
}

async fn update_user(
    Path(user_id): Path<String>,
    State(app_state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<UpdateUserRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    verify_admin(&claims, &app_state)?;
    let admin_user_id = claims.user_id;
    let mut storage = app_state.shared_storage.lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Storage mutex poisoned"}))))?;

    let mut user = match storage.get_user_account(&user_id).map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Database error"}))))? {
        Some(user) => user,
        None => return Ok(Json(json!({
            "success": false,
            "error": "User not found"
        })))
    };

    let mut changes = Vec::new();

    if let Some(tier) = request.tier {
        let old_tier = user.tier.clone();
        if old_tier != tier {
            user.tier = tier.clone();
            user.limits = TierLimits::for_tier(&tier);
            changes.push(format!("tier: {:?} -> {:?}", old_tier, tier));
        }
    }

    if let Some(status) = request.status {
        let old_status = user.status.clone();
        if old_status != status {
            user.status = status.clone();
            changes.push(format!("status: {:?} -> {:?}", old_status, status));
        }
    }

    if let Some(credits) = request.credits {
        let old_credits = user.credits;
        if old_credits != credits {
            user.credits = credits;
            changes.push(format!("credits: {} -> {}", old_credits, credits));
        }
    }

    if let Some(is_admin) = request.is_admin {
        let old_admin = user.is_admin;
        if old_admin != is_admin {
            user.is_admin = is_admin;
            changes.push(format!("is_admin: {} -> {}", old_admin, is_admin));
        }
    }

    if let Some(available_adapters) = request.available_adapters {
        let old_adapters = user.available_adapters.clone();
        if old_adapters != Some(available_adapters.clone()) {
            let old_str = old_adapters
                .as_ref()
                .map(|adapters| adapters.iter().map(|a| a.to_string()).collect::<Vec<_>>().join(", "))
                .unwrap_or_else(|| "tier defaults".to_string());
            let new_str = available_adapters.iter().map(|a| a.to_string()).collect::<Vec<_>>().join(", ");
            user.available_adapters = Some(available_adapters);
            changes.push(format!("adapters: {} -> {}", old_str, new_str));
        }
    }

    user.updated_at = Utc::now();

    storage.update_user_account(&user).map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to update user"}))))?;

    // Record admin action
    let admin_action = AdminAction {
        action_id: Uuid::new_v4().to_string(),
        admin_user_id: admin_user_id.clone(),
        action_type: AdminActionType::UserUpdated,
        target_user_id: Some(user_id.clone()),
        target_resource_id: None,
        details: {
            let mut map = std::collections::HashMap::new();
            map.insert("username".to_string(), serde_json::json!(user.username));
            map.insert("changes".to_string(), serde_json::json!(changes.join(", ")));
            map
        },
        timestamp: Utc::now(),
        ip_address: None,
    };

    storage.record_admin_action(&admin_action).map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to record admin action"}))))?;

    // Send notification to affected user
    drop(storage);  // Release storage lock before notification
    if let Ok(notification_engine) = app_state.notification_engine.lock() {
        // Get admin username for notification
        let admin_username = {
            let storage = app_state.shared_storage.lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Storage mutex poisoned"}))))?;
            storage.get_user_account(&admin_user_id)
                .ok()
                .flatten()
                .map(|u| u.username)
                .unwrap_or_else(|| "Admin".to_string())
        };

        if let Ok(notification) = notification_engine.create_account_updated_notification(
            &user_id,
            &admin_username,
            &changes.join(", "),
        ) {
            // Broadcast via WebSocket
            let _ = app_state.notification_tx.send(crate::api::notifications::NotificationMessage {
                msg_type: "notification".to_string(),
                notification,
            });
        }
    }

    Ok(Json(json!({
        "success": true,
        "message": "User updated successfully",
        "changes": changes
    })))
}

async fn freeze_user(
    Path(user_id): Path<String>,
    State(app_state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    verify_admin(&claims, &app_state)?;
    let admin_user_id = claims.user_id;
    let mut storage = app_state.shared_storage.lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Storage mutex poisoned"}))))?;

    let mut user = match storage.get_user_account(&user_id).map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Database error"}))))? {
        Some(user) => user,
        None => return Ok(Json(json!({
            "success": false,
            "error": "User not found"
        })))
    };

    // Get admin username for notification
    let admin_username = storage.get_user_account(&admin_user_id)
        .ok()
        .flatten()
        .map(|u| u.username)
        .unwrap_or_else(|| "Admin".to_string());

    // Change status to Suspended instead of deleting
    user.status = AccountStatus::Suspended;
    user.updated_at = Utc::now();

    storage.update_user_account(&user).map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to freeze user"}))))?;

    // Record admin action
    let admin_action = AdminAction {
        action_id: Uuid::new_v4().to_string(),
        admin_user_id: admin_user_id.clone(),
        action_type: AdminActionType::UserDeleted,  // Keeping same action type for audit compatibility
        target_user_id: Some(user_id.clone()),
        target_resource_id: None,
        details: {
            let mut map = std::collections::HashMap::new();
            map.insert("username".to_string(), serde_json::json!(user.username));
            map.insert("action".to_string(), serde_json::json!("frozen"));
            map
        },
        timestamp: Utc::now(),
        ip_address: None,
    };

    storage.record_admin_action(&admin_action).map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to record admin action"}))))?;

    drop(storage);  // Release storage lock

    // Send notification to affected user AFTER freezing
    if let Ok(notification_engine) = app_state.notification_engine.lock() {
        if let Ok(notification) = notification_engine.create_account_frozen_notification(
            &user_id,
            &admin_username,
            "Account has been frozen by administrator",
        ) {
            // Broadcast via WebSocket
            let _ = app_state.notification_tx.send(crate::api::notifications::NotificationMessage {
                msg_type: "notification".to_string(),
                notification,
            });
        }
    }

    Ok(Json(json!({
        "success": true,
        "message": "User frozen successfully"
    })))
}

async fn unfreeze_user(
    Path(user_id): Path<String>,
    State(app_state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    verify_admin(&claims, &app_state)?;
    let admin_user_id = claims.user_id;
    let mut storage = app_state.shared_storage.lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Storage mutex poisoned"}))))?;

    let mut user = match storage.get_user_account(&user_id).map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Database error"}))))? {
        Some(user) => user,
        None => return Ok(Json(json!({
            "success": false,
            "error": "User not found"
        })))
    };

    // Get admin username for notification
    let admin_username = storage.get_user_account(&admin_user_id)
        .ok()
        .flatten()
        .map(|u| u.username)
        .unwrap_or_else(|| "Admin".to_string());

    // Change status to Active
    user.status = AccountStatus::Active;
    user.updated_at = Utc::now();

    storage.update_user_account(&user).map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to unfreeze user"}))))?;

    // Record admin action
    let admin_action = AdminAction {
        action_id: Uuid::new_v4().to_string(),
        admin_user_id: admin_user_id.clone(),
        action_type: AdminActionType::UserUpdated,
        target_user_id: Some(user_id.clone()),
        target_resource_id: None,
        details: {
            let mut map = std::collections::HashMap::new();
            map.insert("username".to_string(), serde_json::json!(user.username));
            map.insert("action".to_string(), serde_json::json!("unfrozen"));
            map
        },
        timestamp: Utc::now(),
        ip_address: None,
    };

    storage.record_admin_action(&admin_action).map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to record admin action"}))))?;

    drop(storage);  // Release storage lock

    // Send notification to affected user
    if let Ok(notification_engine) = app_state.notification_engine.lock() {
        if let Ok(notification) = notification_engine.create_account_unfrozen_notification(
            &user_id,
            &admin_username,
        ) {
            // Broadcast via WebSocket
            let _ = app_state.notification_tx.send(crate::api::notifications::NotificationMessage {
                msg_type: "notification".to_string(),
                notification,
            });
        }
    }

    Ok(Json(json!({
        "success": true,
        "message": "User unfrozen successfully"
    })))
}

async fn list_users(
    Query(query): Query<UserSearchQuery>,
    State(app_state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    verify_admin(&claims, &app_state)?;
    let storage = app_state.shared_storage.lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Storage mutex poisoned"}))))?;

    let users = storage.list_user_accounts().map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Database error"}))))?;

    // Apply filters
    let filtered_users: Vec<UserResponse> = users
        .into_iter()
        .filter(|user| {
            if let Some(ref username) = query.username {
                if !user.username.contains(username) {
                    return false;
                }
            }
            if let Some(ref email) = query.email {
                if !user.email.contains(email) {
                    return false;
                }
            }
            if let Some(ref tier) = query.tier {
                if user.tier != *tier {
                    return false;
                }
            }
            if let Some(ref status) = query.status {
                if user.status != *status {
                    return false;
                }
            }
            if let Some(is_admin) = query.is_admin {
                if user.is_admin != is_admin {
                    return false;
                }
            }
            true
        })
        .skip(query.offset.unwrap_or(0))
        .take(query.limit.unwrap_or(50))
        .map(|user| UserResponse {
            user_id: user.user_id,
            username: user.username,
            email: user.email,
            tier: user.tier,
            status: user.status,
            credits: user.credits,
            created_at: user.created_at,
            updated_at: user.updated_at,
            last_login: user.last_login,
            is_admin: user.is_admin,
            workspace_id: user.workspace_id,
            limits: user.limits,
            available_adapters: user.available_adapters,
        })
        .collect();

    Ok(Json(json!({
        "success": true,
        "users": filtered_users,
        "count": filtered_users.len()
    })))
}

async fn adjust_user_credits(
    Path(user_id): Path<String>,
    State(app_state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<CreditAdjustmentRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    verify_admin(&claims, &app_state)?;
    let admin_user_id = claims.user_id;
    let credit_engine = CreditEngine::new(Arc::clone(&app_state.shared_storage));

    let description = format!("{} (Admin: {})", request.reason, request.amount);

    match credit_engine.add_credits(&user_id, request.amount, &description).await {
        Ok(()) => {
            // Record admin action
            let mut storage = app_state.shared_storage.lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Storage mutex poisoned"}))))?;
            let admin_action = AdminAction {
                action_id: Uuid::new_v4().to_string(),
                admin_user_id: admin_user_id.clone(),
                action_type: AdminActionType::CreditsAdjusted,
                target_user_id: Some(user_id.clone()),
                target_resource_id: None,
                details: {
                    let mut map = std::collections::HashMap::new();
                    map.insert("amount".to_string(), serde_json::json!(request.amount));
                    map.insert("reason".to_string(), serde_json::json!(request.reason));
                    map
                },
                timestamp: Utc::now(),
                ip_address: None,
            };

            storage.record_admin_action(&admin_action).map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to record admin action"}))))?;

            // Get updated user balance and admin username for notification
            let (new_balance, admin_username) = {
                let user = storage.get_user_account(&user_id)
                    .ok()
                    .flatten()
                    .ok_or_else(|| (StatusCode::NOT_FOUND, Json(json!({"error": "User not found"}))))?;
                let admin = storage.get_user_account(&admin_user_id)
                    .ok()
                    .flatten()
                    .map(|u| u.username)
                    .unwrap_or_else(|| "Admin".to_string());
                (user.credits, admin)
            };

            drop(storage);  // Release storage lock before notification

            // Send notification to affected user
            if let Ok(notification_engine) = app_state.notification_engine.lock() {
                if let Ok(notification) = notification_engine.create_credits_adjusted_notification(
                    &user_id,
                    &admin_username,
                    request.amount,
                    &request.reason,
                    new_balance,
                ) {
                    // Broadcast via WebSocket
                    let _ = app_state.notification_tx.send(crate::api::notifications::NotificationMessage {
                        msg_type: "notification".to_string(),
                        notification,
                    });
                }
            }

            Ok(Json(json!({
                "success": true,
                "message": "Credits adjusted successfully",
                "amount": request.amount
            })))
        }
        Err(e) => Ok(Json(json!({
            "success": false,
            "error": format!("Failed to adjust credits: {}", e)
        })))
    }
}

async fn bulk_grant_credits(
    State(app_state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<BulkCreditGrantRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    verify_admin(&claims, &app_state)?;
    let admin_user_id = claims.user_id;
    let credit_engine = CreditEngine::new(Arc::clone(&app_state.shared_storage));

    let mut successful = Vec::new();
    let mut failed = Vec::new();

    for user_id in &request.user_ids {
        match credit_engine.add_credits(user_id, request.amount, &request.reason).await {
            Ok(()) => successful.push(user_id.clone()),
            Err(e) => failed.push((user_id.clone(), e.to_string())),
        }
    }

    // Record admin action
    let mut storage = app_state.shared_storage.lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Storage mutex poisoned"}))))?;
    let admin_action = AdminAction {
        action_id: Uuid::new_v4().to_string(),
        admin_user_id,
        action_type: AdminActionType::BulkCreditsGranted,
        target_user_id: None,
        target_resource_id: None,
        details: {
            let mut map = std::collections::HashMap::new();
            map.insert("amount".to_string(), serde_json::json!(request.amount));
            map.insert("user_count".to_string(), serde_json::json!(request.user_ids.len()));
            map.insert("reason".to_string(), serde_json::json!(request.reason));
            map.insert("successful_count".to_string(), serde_json::json!(successful.len()));
            map.insert("failed_count".to_string(), serde_json::json!(failed.len()));
            map
        },
        timestamp: Utc::now(),
        ip_address: None,
    };

    storage.record_admin_action(&admin_action).map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to record admin action"}))))?;

    Ok(Json(json!({
        "success": true,
        "message": "Bulk credit grant completed",
        "successful": successful,
        "failed": failed,
        "total_users": request.user_ids.len(),
        "successful_count": successful.len(),
        "failed_count": failed.len()
    })))
}

async fn get_user_credit_history(
    Path(user_id): Path<String>,
    Query(params): Query<std::collections::HashMap<String, String>>,
    State(app_state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    verify_admin(&claims, &app_state)?;
    let credit_engine = CreditEngine::new(Arc::clone(&app_state.shared_storage));

    let limit = params.get("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(50);

    match credit_engine.get_credit_history(&user_id, Some(limit)).await {
        Ok(transactions) => Ok(Json(json!({
            "success": true,
            "user_id": user_id,
            "transactions": transactions,
            "count": transactions.len()
        }))),
        Err(e) => Ok(Json(json!({
            "success": false,
            "error": format!("Failed to get credit history: {}", e)
        })))
    }
}

async fn get_admin_dashboard_stats(
    State(app_state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    verify_admin(&claims, &app_state)?;
    let storage = app_state.shared_storage.lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Storage mutex poisoned"}))))?;

    let users = storage.list_user_accounts().map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Database error"}))))?;

    let mut users_by_tier = std::collections::HashMap::new();
    let mut users_by_status = std::collections::HashMap::new();
    let mut active_users_last_30_days = 0;
    let mut new_users_last_30_days = 0;

    let thirty_days_ago = Utc::now() - chrono::Duration::days(30);

    for user in &users {
        // Count by tier
        let tier_str = user.tier.as_str().to_string();
        *users_by_tier.entry(tier_str).or_insert(0) += 1;

        // Count by status
        let status_str = format!("{:?}", user.status);
        *users_by_status.entry(status_str).or_insert(0) += 1;

        // Count active users (logged in within 30 days)
        if let Some(last_login) = user.last_login {
            if last_login > thirty_days_ago {
                active_users_last_30_days += 1;
            }
        }

        // Count new users (created within 30 days)
        if user.created_at > thirty_days_ago {
            new_users_last_30_days += 1;
        }
    }

    // Calculate credit statistics
    let credit_transactions = storage.get_credit_transactions_by_operation("").unwrap_or_default();
    let total_credits_issued: i64 = credit_transactions.iter()
        .filter(|t| matches!(t.transaction_type, CreditTransactionType::Purchase | CreditTransactionType::Grant | CreditTransactionType::Subscription))
        .map(|t| t.amount)
        .sum();

    let total_credits_consumed: i64 = credit_transactions.iter()
        .filter(|t| matches!(t.transaction_type, CreditTransactionType::Consumption))
        .map(|t| -t.amount) // Consumption amounts are negative
        .sum();

    let stats = AdminDashboardStats {
        total_users: users.len() as u64,
        users_by_tier,
        users_by_status,
        total_credits_issued,
        total_credits_consumed,
        active_users_last_30_days,
        new_users_last_30_days,
        generated_at: Utc::now(),
    };

    Ok(Json(json!({
        "success": true,
        "stats": stats
    })))
}

async fn get_admin_actions(
    Query(params): Query<std::collections::HashMap<String, String>>,
    State(app_state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    verify_admin(&claims, &app_state)?;
    let storage = app_state.shared_storage.lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Storage mutex poisoned"}))))?;

    let admin_id = params.get("admin_id").map(|s| s.as_str());
    let limit = params.get("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);

    match storage.get_admin_actions(admin_id, Some(limit)) {
        Ok(actions) => Ok(Json(json!({
            "success": true,
            "actions": actions,
            "count": actions.len()
        }))),
        Err(e) => Ok(Json(json!({
            "success": false,
            "error": format!("Failed to get admin actions: {}", e)
        })))
    }
}

// ============================================================================
// ADAPTER CONFIGURATION HANDLERS
// ============================================================================

async fn create_adapter_config(
    State(app_state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<CreateAdapterConfigRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    verify_admin(&claims, &app_state)?;

    // Create logger for adapter manager
    let logger = LoggingEngine::new();
    let mut adapter_manager = AdapterManager::new(Arc::clone(&app_state.shared_storage), logger);

    match adapter_manager.create_adapter_config(
        request.name,
        request.description,
        request.adapter_type,
        request.connection_details,
        request.contract_configs,
        claims.user_id,
    ) {
        Ok(config) => {
            Ok(Json(json!({
                "success": true,
                "message": "Adapter configuration created successfully",
                "config": config
            })))
        }
        Err(e) => {
            Ok(Json(json!({
                "success": false,
                "error": e.to_string()
            })))
        }
    }
}

async fn list_adapter_configs(
    Query(params): Query<std::collections::HashMap<String, String>>,
    State(app_state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    verify_admin(&claims, &app_state)?;

    let logger = LoggingEngine::new();
    let adapter_manager = AdapterManager::new(Arc::clone(&app_state.shared_storage), logger);

    let active_only = params.get("active_only")
        .and_then(|s| s.parse::<bool>().ok())
        .unwrap_or(false);

    match adapter_manager.list_adapters(active_only) {
        Ok(configs) => {
            Ok(Json(json!({
                "success": true,
                "configs": configs,
                "count": configs.len()
            })))
        }
        Err(e) => {
            Ok(Json(json!({
                "success": false,
                "error": e.to_string()
            })))
        }
    }
}

async fn get_adapter_config(
    Path(config_id): Path<String>,
    State(app_state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    verify_admin(&claims, &app_state)?;

    let config_uuid = Uuid::parse_str(&config_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid UUID format"}))))?;

    let logger = LoggingEngine::new();
    let adapter_manager = AdapterManager::new(Arc::clone(&app_state.shared_storage), logger);

    match adapter_manager.get_adapter_config(&config_uuid) {
        Ok(config) => {
            Ok(Json(json!({
                "success": true,
                "config": config
            })))
        }
        Err(e) => {
            Ok(Json(json!({
                "success": false,
                "error": e.to_string()
            })))
        }
    }
}

async fn update_adapter_config(
    Path(config_id): Path<String>,
    State(app_state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<UpdateAdapterConfigRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    verify_admin(&claims, &app_state)?;

    let config_uuid = Uuid::parse_str(&config_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid UUID format"}))))?;

    let logger = LoggingEngine::new();
    let mut adapter_manager = AdapterManager::new(Arc::clone(&app_state.shared_storage), logger);

    match adapter_manager.update_adapter_config(
        &config_uuid,
        request.name,
        request.description,
        request.connection_details,
        request.contract_configs,
        request.is_active,
    ) {
        Ok(config) => {
            Ok(Json(json!({
                "success": true,
                "message": "Adapter configuration updated successfully",
                "config": config
            })))
        }
        Err(e) => {
            Ok(Json(json!({
                "success": false,
                "error": e.to_string()
            })))
        }
    }
}

async fn delete_adapter_config(
    Path(config_id): Path<String>,
    State(app_state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    verify_admin(&claims, &app_state)?;

    let config_uuid = Uuid::parse_str(&config_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid UUID format"}))))?;

    let logger = LoggingEngine::new();
    let mut adapter_manager = AdapterManager::new(Arc::clone(&app_state.shared_storage), logger);

    match adapter_manager.delete_adapter_config(&config_uuid) {
        Ok(()) => {
            Ok(Json(json!({
                "success": true,
                "message": "Adapter configuration deleted successfully"
            })))
        }
        Err(e) => {
            Ok(Json(json!({
                "success": false,
                "error": e.to_string()
            })))
        }
    }
}

async fn set_default_adapter(
    Path(config_id): Path<String>,
    State(app_state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    verify_admin(&claims, &app_state)?;

    let config_uuid = Uuid::parse_str(&config_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid UUID format"}))))?;

    let logger = LoggingEngine::new();
    let mut adapter_manager = AdapterManager::new(Arc::clone(&app_state.shared_storage), logger);

    match adapter_manager.set_default_adapter(&config_uuid) {
        Ok(()) => {
            Ok(Json(json!({
                "success": true,
                "message": "Default adapter set successfully"
            })))
        }
        Err(e) => {
            Ok(Json(json!({
                "success": false,
                "error": e.to_string()
            })))
        }
    }
}

// TODO: Implement test_adapter_config endpoint
// This endpoint needs special handling for async adapter testing
// For now, testing can be done programmatically using AdapterManager::test_adapter()
// async fn test_adapter_config(
//     State(app_state): State<Arc<AppState>>,
//     Extension(claims): Extension<Claims>,
//     Path(config_id): Path<String>,
// ) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
//     verify_admin(&claims, &app_state)?;
//     let config_uuid = Uuid::parse_str(&config_id)
//         .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid UUID format"}))))?;
//     let logger = LoggingEngine::new();
//     let mut adapter_manager = AdapterManager::new(Arc::clone(&app_state.shared_storage), logger);
//     match adapter_manager.test_adapter(&config_uuid).await {
//         Ok(result) => Ok(Json(json!({"success": true, "test_result": result}))),
//         Err(e) => Ok(Json(json!({"success": false, "error": e.to_string()})))
//     }
// }

// ============================================================================
// ROUTER SETUP
// ============================================================================

pub fn admin_routes() -> Router<Arc<AppState>> {
    Router::new()
        // User management
        .route("/users", get(list_users).post(create_user))
        .route("/users/:user_id", get(get_user).put(update_user))
        .route("/users/:user_id/freeze", put(freeze_user))
        .route("/users/:user_id/unfreeze", put(unfreeze_user))
        .route("/users/:user_id/credits", post(adjust_user_credits))
        .route("/users/:user_id/credits/history", get(get_user_credit_history))
        .route("/users/credits/bulk-grant", post(bulk_grant_credits))

        // Dashboard and monitoring
        .route("/dashboard/stats", get(get_admin_dashboard_stats))
        .route("/actions", get(get_admin_actions))

        // Adapter configuration management
        .route("/adapters", get(list_adapter_configs).post(create_adapter_config))
        .route("/adapters/:config_id", get(get_adapter_config).put(update_adapter_config).delete(delete_adapter_config))
        .route("/adapters/:config_id/set-default", post(set_default_adapter))
        // TODO: Add test endpoint once async handler issue is resolved
        // .route("/adapters/:config_id/test", post(test_adapter_config))
}
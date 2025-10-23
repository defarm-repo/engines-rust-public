use crate::auth_middleware::AuthenticatedUser;
use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, patch, post, put},
    Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex, MutexGuard};
use uuid::Uuid;

use crate::api::auth::Claims;
use crate::api::items::{build_identifiers, IdentifierRequest};
use crate::identifier_types::CircuitAliasConfig;
use crate::storage::StorageBackend;
use crate::types::{
    Activity, AdapterType, BatchPushItemResult, BatchPushResult, CircuitItem, CircuitPermissions,
    CustomRole, Permission, PublicSettings, UserActivity, UserActivityCategory, UserActivityType,
    UserResourceType,
};
use crate::{Circuit, CircuitOperation, CircuitsEngine, InMemoryStorage, ItemsEngine, MemberRole};

#[derive(Debug, Deserialize)]
pub struct CreateCircuitAdapterConfigRequest {
    pub adapter_type: Option<AdapterType>,
    pub requires_approval: bool,
    pub auto_migrate_existing: bool,
    pub sponsor_adapter_access: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateCircuitRequest {
    pub name: String,
    pub description: String,
    // owner_id is now extracted from JWT token automatically
    pub adapter_config: Option<CreateCircuitAdapterConfigRequest>,
    pub alias_config: Option<CircuitAliasConfig>,
    pub allow_public_visibility: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct AddMemberRequest {
    pub member_id: String,
    pub role: String,
    // Note: requester_id is now extracted automatically from JWT token
}

#[derive(Debug, Deserialize)]
pub struct CircuitOperationRequest {
    // Note: requester_id is now extracted automatically from JWT token
}

#[derive(Debug, Deserialize)]
pub struct ApproveOperationRequest {
    pub approver_id: String,
}

#[derive(Debug, Deserialize)]
pub struct RejectOperationRequest {
    pub rejecter_id: String,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PendingItemPreview {
    pub identifiers: Vec<IdentifierRequest>,
    pub enriched_data: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct PendingItemResponse {
    pub pending_id: String,
    pub dfid: String,
    pub pushed_by: String,
    pub pushed_at: i64,
    pub status: String,
    pub item_preview: Option<PendingItemPreview>,
}

#[derive(Debug, Deserialize)]
pub struct ApproveItemRequest {
    pub admin_id: String,
}

#[derive(Debug, Serialize)]
pub struct ApproveItemData {
    pub circuit_id: String,
    pub dfid: String,
    pub pushed_by: String,
    pub pushed_at: i64,
    pub approved_by: String,
    pub approved_at: i64,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct ApproveItemResponse {
    pub success: bool,
    pub message: String,
    pub data: ApproveItemData,
}

#[derive(Debug, Deserialize)]
pub struct RejectItemRequest {
    pub admin_id: String,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RejectItemResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct JoinCircuitRequest {
    pub message: Option<String>,
    // Note: requester_id is now extracted automatically from JWT token
}

#[derive(Debug, Deserialize)]
pub struct ApproveJoinRequest {
    pub admin_id: String,
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct RejectJoinRequest {
    pub admin_id: String,
}

#[derive(Debug, Serialize)]
pub struct JoinRequestResponse {
    pub requester_id: String,
    pub requested_at: i64,
    pub message: Option<String>,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct CircuitListQuery {
    pub user_id: Option<String>,
    pub include_public: Option<bool>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCircuitRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub permissions: Option<UpdateCircuitPermissions>,
    // Note: requester_id is now extracted automatically from JWT token
}

#[derive(Debug, Deserialize)]
pub struct UpdateCircuitPermissions {
    pub require_approval_for_push: Option<bool>,
    pub require_approval_for_pull: Option<bool>,
    pub allow_public_visibility: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCustomRoleRequest {
    pub role_name: String,
    pub permissions: Vec<String>,
    pub description: String,
    pub color: Option<String>,
    // Note: requester_id is now extracted automatically from JWT token
}

#[derive(Debug, Deserialize)]
pub struct UpdateCustomRoleRequest {
    pub permissions: Option<Vec<String>>,
    pub description: Option<String>,
    pub color: Option<String>,
    // Note: requester_id is now extracted automatically from JWT token
}

#[derive(Debug, Deserialize)]
pub struct AssignRoleRequest {
    pub role: String,
    pub custom_role_name: Option<String>,
    // Note: requester_id is now extracted automatically from JWT token
}

#[derive(Debug, Serialize)]
pub struct CustomRoleResponse {
    pub role_id: String,
    pub role_name: String,
    pub permissions: Vec<String>,
    pub description: String,
    pub color: Option<String>,
    pub member_count: usize,
    pub created_timestamp: i64,
    pub created_by: String,
    pub is_default: bool,
}

#[derive(Debug, Serialize)]
pub struct CircuitResponse {
    pub circuit_id: String,
    pub name: String,
    pub description: String,
    pub owner_id: String,
    pub created_timestamp: i64,
    pub last_modified: i64,
    pub members: Vec<CircuitMemberResponse>,
    pub permissions: CircuitPermissionsResponse,
    pub status: String,
    pub pending_requests: Vec<JoinRequestResponse>,
    pub custom_roles: Vec<CustomRoleResponse>,
    pub public_settings: Option<PublicSettings>,
}

#[derive(Debug, Serialize)]
pub struct CircuitMemberResponse {
    pub member_id: String,
    pub role: String,
    pub custom_role_name: Option<String>,
    pub permissions: Vec<String>,
    pub joined_timestamp: i64,
}

#[derive(Debug, Serialize)]
pub struct CircuitPermissionsResponse {
    pub require_approval_for_push: bool,
    pub require_approval_for_pull: bool,
    pub allow_public_visibility: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePublicSettingsRequest {
    pub public_settings: PublicSettingsRequest,
    // Note: requester_id is now extracted automatically from JWT token
}

#[derive(Debug, Deserialize)]
pub struct PublicSettingsRequest {
    pub access_mode: String,
    pub scheduled_date: Option<String>,
    pub access_password: Option<String>,
    pub public_name: Option<String>,
    pub public_description: Option<String>,
    pub primary_color: Option<String>,
    pub secondary_color: Option<String>,
    pub logo_url: Option<String>,
    pub tagline: Option<String>,
    pub footer_text: Option<String>,
    pub published_items: Option<Vec<String>>,
    pub auto_approve_members: Option<bool>,
    pub auto_publish_pushed_items: Option<bool>,
    pub show_encrypted_events: Option<bool>,
    pub required_event_types: Option<String>,
    pub data_quality_rules: Option<String>,
    pub export_permissions: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct JoinPublicCircuitRequest {
    pub access_password: Option<String>,
    pub message: Option<String>,
    // Note: requester_id is now extracted automatically from JWT token
}

#[derive(Debug, Serialize)]
pub struct PublicJoinResponse {
    pub success: bool,
    pub message: String,
    pub requires_approval: bool,
}

#[derive(Debug, Serialize)]
pub struct ActivityResponse {
    pub activity_id: String,
    pub activity_type: String,
    pub circuit_id: String,
    pub circuit_name: String,
    pub item_dfids: Vec<String>,
    pub user_id: String,
    pub timestamp: i64,
    pub status: String,
    pub details: ActivityDetailsResponse,
}

#[derive(Debug, Serialize)]
pub struct ActivityDetailsResponse {
    pub items_count: usize,
    pub success_count: usize,
    pub failed_items: Vec<String>,
    pub enrichment_matches: Vec<String>,
    pub permissions: Option<Vec<String>>,
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CircuitItemResponse {
    pub dfid: String,
    pub circuit_id: String,
    pub circuit_name: String,
    pub pushed_by: String,
    pub pushed_at: i64,
    pub permissions: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct CircuitItemWithEventsResponse {
    #[serde(flatten)]
    pub item: CircuitItemResponse,
    pub events: Vec<crate::types::Event>,
}

#[derive(Debug, Deserialize)]
pub struct CircuitItemsQuery {
    #[serde(default)]
    pub include_events: bool,
}

#[derive(Debug, Deserialize)]
pub struct BatchPushItem {
    pub dfid: String,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct BatchPushRequest {
    pub items: Vec<BatchPushItem>,
    // Note: requester_id is now extracted automatically from JWT token
}

#[derive(Debug, Deserialize)]
pub struct PushLocalItemRequest {
    pub local_id: String,
    pub identifiers: Option<Vec<IdentifierRequest>>,
    pub enriched_data: Option<std::collections::HashMap<String, serde_json::Value>>,
    // Note: requester_id is now extracted automatically from JWT token
    // No need to include it in the request body anymore
}

#[derive(Debug, Serialize)]
pub struct PushLocalItemResponse {
    pub success: bool,
    pub data: PushLocalItemData,
}

#[derive(Debug, Serialize)]
pub struct PushLocalItemData {
    pub dfid: String,
    pub status: String, // "NewItemCreated" or "ExistingItemEnriched"
    pub operation_id: String,
    pub local_id: String,
}

fn spawn_persist_circuit(state: &Arc<AppState>, circuit: Circuit) {
    let state_clone = Arc::clone(state);
    tokio::spawn(async move {
        let pg_lock = state_clone.postgres_persistence.read().await;
        if let Some(pg) = &*pg_lock {
            if let Err(e) = pg.persist_circuit(&circuit).await {
                tracing::warn!(
                    "Failed to persist circuit {} to PostgreSQL: {}",
                    circuit.circuit_id,
                    e
                );
            }
        }
    });
}

fn fetch_circuit(state: &Arc<AppState>, circuit_id: &Uuid) -> Option<Circuit> {
    state
        .shared_storage
        .lock()
        .ok()
        .and_then(|storage| storage.get_circuit(circuit_id).ok()?.map(|c| c))
}

fn fetch_circuit_operation(
    state: &Arc<AppState>,
    operation_id: &Uuid,
) -> Option<CircuitOperation> {
    state
        .shared_storage
        .lock()
        .ok()
        .and_then(|storage| storage.get_circuit_operation(operation_id).ok()?.map(|op| op))
}

fn spawn_persist_circuit_operation(state: &Arc<AppState>, operation: CircuitOperation) {
    let state_clone = Arc::clone(state);
    tokio::spawn(async move {
        let pg_lock = state_clone.postgres_persistence.read().await;
        if let Some(pg) = &*pg_lock {
            if let Err(e) = pg.persist_circuit_operation(&operation).await {
                tracing::warn!(
                    "Failed to persist circuit operation {} to PostgreSQL: {}",
                    operation.operation_id,
                    e
                );
            }
        }
    });
}

#[derive(Debug, Serialize)]
pub struct CircuitOperationResponse {
    pub operation_id: String,
    pub circuit_id: String,
    pub dfid: String,
    pub operation_type: String,
    pub requester_id: String,
    pub timestamp: i64,
    pub status: String,
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct GetAdapterConfigResponse {
    pub adapter_type: Option<String>,
    pub sponsor_adapter_access: bool,
    pub requires_approval: bool,
    pub auto_migrate_existing: bool,
    pub configured_by: String,
    pub configured_at: String,
}

#[derive(Debug, Deserialize)]
pub struct SetAdapterConfigRequest {
    pub adapter_type: Option<String>,
    pub auto_migrate_existing: bool,
    pub requires_approval: bool,
    pub sponsor_adapter_access: bool,
}

pub struct CircuitState {
    pub engine: Arc<Mutex<CircuitsEngine<InMemoryStorage>>>,
}

impl Default for CircuitState {
    fn default() -> Self {
        Self::new()
    }
}

impl CircuitState {
    pub fn new() -> Self {
        let storage = Arc::new(Mutex::new(InMemoryStorage::new()));
        Self {
            engine: Arc::new(Mutex::new(CircuitsEngine::new(storage))),
        }
    }
}

use super::shared_state::AppState;
use crate::types::{HttpMethod, PostActionTrigger, WebhookAuthType, WebhookConfig};

// ============================================================================
// WEBHOOK CONFIGURATION TYPES
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct UpdatePostActionSettingsRequest {
    pub enabled: bool,
    pub trigger_events: Vec<String>, // Serialized PostActionTrigger
    pub include_storage_details: bool,
    pub include_item_metadata: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateWebhookRequest {
    pub name: String,
    pub url: String,
    pub method: Option<String>, // "POST", "PUT", "PATCH"
    pub headers: Option<std::collections::HashMap<String, String>>,
    pub auth_type: Option<String>, // "None", "BearerToken", "ApiKey", "BasicAuth", "CustomHeader"
    pub auth_credentials: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWebhookRequest {
    pub name: Option<String>,
    pub url: Option<String>,
    pub method: Option<String>,
    pub headers: Option<std::collections::HashMap<String, String>>,
    pub auth_type: Option<String>,
    pub auth_credentials: Option<String>,
    pub enabled: Option<bool>,
}

pub fn circuit_routes(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", post(create_circuit))
        .route("/", get(list_circuits))
        .route("/:id", get(get_circuit))
        .route("/:id", patch(update_circuit))
        .route("/:id", put(update_circuit))
        .route("/:id/members", post(add_member))
        .route("/:id/push/:dfid", post(push_item))
        .route("/:id/push-local", post(push_local_item))
        .route("/:id/pull/:dfid", post(pull_item))
        .route("/:id/operations", get(get_circuit_operations))
        .route("/:id/operations/pending", get(get_pending_operations))
        .route("/operations/:operation_id/approve", post(approve_operation))
        .route("/:id/deactivate", put(deactivate_circuit))
        .route("/:id/requests", post(request_to_join_circuit))
        .route("/:id/requests/pending", get(get_pending_join_requests))
        .route(
            "/:id/requests/:requester_id/approve",
            post(approve_join_request),
        )
        .route(
            "/:id/requests/:requester_id/reject",
            post(reject_join_request),
        )
        .route("/:id/roles", post(create_custom_role))
        .route("/:id/roles", get(get_custom_roles))
        .route("/:id/roles/:role_name", put(update_custom_role))
        .route("/:id/roles/:role_name", delete(delete_custom_role))
        .route("/:id/members/:user_id", patch(assign_member_role))
        .route("/:id/public-settings", put(update_public_settings))
        .route("/:id/public", get(get_public_circuit))
        .route("/:id/public/join", post(join_public_circuit))
        .route("/:id/activities", get(get_circuit_activities))
        .route("/:id/items", get(get_circuit_items))
        .route("/:id/push/batch", post(batch_push_items))
        .route("/:id/pending-items", get(get_circuit_pending_items))
        .route(
            "/:id/pending-items/:pending_id/approve",
            post(approve_pending_item),
        )
        .route(
            "/:id/pending-items/:pending_id/reject",
            post(reject_pending_item),
        )
        .route("/:id/adapter", get(get_circuit_adapter_config))
        .route("/:id/adapter", put(set_circuit_adapter_config))
        // Webhook configuration routes
        .route("/:id/post-actions", get(get_post_action_settings))
        .route("/:id/post-actions", put(update_post_action_settings))
        .route("/:id/post-actions/webhooks", post(create_webhook))
        .route("/:id/post-actions/webhooks/:webhook_id", get(get_webhook))
        .route(
            "/:id/post-actions/webhooks/:webhook_id",
            put(update_webhook),
        )
        .route(
            "/:id/post-actions/webhooks/:webhook_id",
            delete(delete_webhook),
        )
        .route(
            "/:id/post-actions/webhooks/:webhook_id/test",
            post(test_webhook),
        )
        .route("/:id/post-actions/deliveries", get(get_webhook_deliveries))
        .route("/list", get(list_circuits))
        .route("/member/:member_id", get(get_circuits_for_member))
        .with_state(app_state)
}

fn parse_member_role(role_str: &str) -> Result<MemberRole, String> {
    match role_str.to_lowercase().as_str() {
        "owner" => Ok(MemberRole::Owner),
        "admin" => Ok(MemberRole::Admin),
        "member" => Ok(MemberRole::Member),
        "viewer" => Ok(MemberRole::Viewer),
        _ => Err(format!("Invalid member role: {role_str}")),
    }
}

fn lock_circuits_engine<'a>(
    state: &'a Arc<AppState>,
) -> Result<MutexGuard<'a, CircuitsEngine<InMemoryStorage>>, (StatusCode, Json<Value>)> {
    state.circuits_engine.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Circuits engine mutex poisoned"})),
        )
    })
}

#[allow(clippy::type_complexity)]
fn lock_items_engine<'a>(
    state: &'a Arc<AppState>,
) -> Result<MutexGuard<'a, ItemsEngine<Arc<Mutex<InMemoryStorage>>>>, (StatusCode, Json<Value>)> {
    state.items_engine.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Items engine mutex poisoned"})),
        )
    })
}

fn parse_permission(permission_str: &str) -> Result<Permission, String> {
    match permission_str.to_lowercase().as_str() {
        "push" => Ok(Permission::Push),
        "pull" => Ok(Permission::Pull),
        "invite" => Ok(Permission::Invite),
        "managemembers" => Ok(Permission::ManageMembers),
        "managepermissions" => Ok(Permission::ManagePermissions),
        "manageroles" => Ok(Permission::ManageRoles),
        "delete" => Ok(Permission::Delete),
        "certify" => Ok(Permission::Certify),
        "audit" => Ok(Permission::Audit),
        _ => Err(format!("Invalid permission: {permission_str}")),
    }
}

fn circuit_to_response(circuit: Circuit) -> CircuitResponse {
    let role_counts = circuit.get_member_count_by_role();

    CircuitResponse {
        circuit_id: circuit.circuit_id.to_string(),
        name: circuit.name,
        description: circuit.description,
        owner_id: circuit.owner_id,
        created_timestamp: circuit.created_timestamp.timestamp(),
        last_modified: circuit.last_modified.timestamp(),
        members: circuit
            .members
            .into_iter()
            .map(|member| CircuitMemberResponse {
                member_id: member.member_id,
                role: format!("{:?}", member.role),
                custom_role_name: member.custom_role_name,
                permissions: member
                    .permissions
                    .into_iter()
                    .map(|p| format!("{p:?}"))
                    .collect(),
                joined_timestamp: member.joined_timestamp.timestamp(),
            })
            .collect(),
        permissions: CircuitPermissionsResponse {
            require_approval_for_push: circuit.permissions.require_approval_for_push,
            require_approval_for_pull: circuit.permissions.require_approval_for_pull,
            allow_public_visibility: circuit.permissions.allow_public_visibility,
        },
        status: format!("{:?}", circuit.status),
        pending_requests: circuit
            .pending_requests
            .into_iter()
            .filter(|req| matches!(req.status, crate::types::JoinRequestStatus::Pending))
            .map(|req| JoinRequestResponse {
                requester_id: req.requester_id,
                requested_at: req.requested_at.timestamp(),
                message: req.message,
                status: format!("{:?}", req.status),
            })
            .collect(),
        custom_roles: circuit
            .custom_roles
            .into_iter()
            .map(|role| {
                let member_count = role_counts.get(&role.role_name).copied().unwrap_or(0);
                custom_role_to_response(role, member_count)
            })
            .collect(),
        public_settings: circuit.public_settings,
    }
}

fn custom_role_to_response(role: CustomRole, member_count: usize) -> CustomRoleResponse {
    CustomRoleResponse {
        role_id: role.role_id.to_string(),
        role_name: role.role_name,
        permissions: role
            .permissions
            .into_iter()
            .map(|p| format!("{p:?}"))
            .collect(),
        description: role.description,
        color: role.color,
        member_count,
        created_timestamp: role.created_timestamp.timestamp(),
        created_by: role.created_by,
        is_default: role.is_default,
    }
}

fn operation_to_response(operation: CircuitOperation) -> CircuitOperationResponse {
    CircuitOperationResponse {
        operation_id: operation.operation_id.to_string(),
        circuit_id: operation.circuit_id.to_string(),
        dfid: operation.dfid,
        operation_type: format!("{:?}", operation.operation_type),
        requester_id: operation.requester_id,
        timestamp: operation.timestamp.timestamp(),
        status: format!("{:?}", operation.status),
        metadata: operation.metadata,
    }
}

pub fn activity_to_response(activity: Activity) -> ActivityResponse {
    ActivityResponse {
        activity_id: activity.activity_id,
        activity_type: format!("{:?}", activity.activity_type),
        circuit_id: activity.circuit_id.to_string(),
        circuit_name: activity.circuit_name,
        item_dfids: activity.item_dfids,
        user_id: activity.user_id,
        timestamp: activity.timestamp.timestamp(),
        status: format!("{:?}", activity.status),
        details: ActivityDetailsResponse {
            items_count: activity.details.items_affected,
            success_count: activity.details.items_affected,
            failed_items: Vec::new(),
            enrichment_matches: Vec::new(),
            permissions: None,
            error_message: activity.details.error_message,
        },
    }
}

fn circuit_item_to_response(item: CircuitItem) -> CircuitItemResponse {
    CircuitItemResponse {
        dfid: item.dfid,
        circuit_id: item.circuit_id.to_string(),
        circuit_name: format!("Circuit {}", item.circuit_id), // Placeholder since we don't have circuit name
        pushed_by: item.pushed_by,
        pushed_at: item.pushed_at.timestamp(),
        permissions: Some(item.permissions),
    }
}

async fn create_circuit(
    State(state): State<Arc<AppState>>,
    AuthenticatedUser(owner_id): AuthenticatedUser,
    Json(payload): Json<CreateCircuitRequest>,
) -> Result<Json<CircuitResponse>, (StatusCode, Json<Value>)> {
    // Create circuit in in-memory storage (must not hold lock across await)
    let circuit = {
        let mut engine = lock_circuits_engine(&state)?;

        // First create circuit without adapter_config (owner_id from JWT)
        let circuit = engine
            .create_circuit(
                payload.name,
                payload.description,
                owner_id.clone(),
                None,
                payload.alias_config,
            )
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": format!("Failed to create circuit: {}", e)})),
                )
            })?;

        // Set adapter config if provided
        if let Some(adapter_req) = payload.adapter_config {
            engine
                .set_circuit_adapter_config(
                    &circuit.circuit_id,
                    &owner_id,
                    adapter_req.adapter_type,
                    adapter_req.auto_migrate_existing,
                    adapter_req.requires_approval,
                    adapter_req.sponsor_adapter_access,
                )
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": format!("Failed to set adapter config: {}", e)})),
                    )
                })?;
        }

        // Set public visibility if requested
        if let Some(allow_public) = payload.allow_public_visibility {
            if allow_public {
                let updated_permissions = CircuitPermissions {
                    require_approval_for_push: circuit.permissions.require_approval_for_push,
                    require_approval_for_pull: circuit.permissions.require_approval_for_pull,
                    allow_public_visibility: true,
                };

                engine
                    .update_circuit(
                        &circuit.circuit_id,
                        None,
                        None,
                        Some(updated_permissions),
                        &owner_id,
                    )
                    .map_err(|e| {
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(
                                json!({"error": format!("Failed to set public visibility: {}", e)}),
                            ),
                        )
                    })?;
            }
        }

        // Get updated circuit
        engine
            .get_circuit(&circuit.circuit_id)
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": format!("Failed to get circuit: {}", e)})),
                )
            })?
            .ok_or((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Circuit not found after creation"})),
            ))?
    };

    // Write-through cache: Also persist to PostgreSQL if available
    let pg_lock = state.postgres_persistence.read().await;
    if let Some(pg) = &*pg_lock {
        tracing::info!(
            "🔄 Attempting to persist circuit {} to PostgreSQL...",
            circuit.circuit_id
        );
        match pg.persist_circuit(&circuit).await {
            Ok(()) => {
                tracing::info!(
                    "✅ Successfully persisted circuit {} to PostgreSQL",
                    circuit.circuit_id
                );
            }
            Err(e) => {
                tracing::error!(
                    "❌ CRITICAL: Failed to persist circuit {} to PostgreSQL: {}",
                    circuit.circuit_id,
                    e
                );
                // Don't fail the request - in-memory write succeeded
            }
        }
    } else {
        tracing::warn!(
            "⚠️  PostgreSQL not available, circuit {} only in memory!",
            circuit.circuit_id
        );
    }
    drop(pg_lock);

    // Record user activity
    let circuit_id_str = circuit.circuit_id.to_string();
    let user_activity = UserActivity {
        activity_id: Uuid::new_v4().to_string(),
        user_id: owner_id.clone(),
        workspace_id: "default-workspace".to_string(), // TODO: Get from context
        timestamp: Utc::now(),
        activity_type: UserActivityType::Create,
        category: UserActivityCategory::Circuits,
        resource_type: UserResourceType::Circuit,
        resource_id: circuit_id_str.clone(),
        action: "create_circuit".to_string(),
        description: format!("Created circuit: {}", circuit.name),
        metadata: serde_json::Value::Null,
        success: true,
        ip_address: None, // TODO: Extract from request
        user_agent: None, // TODO: Extract from request
    };

    if let Ok(engine) = state.activity_engine.lock() {
        if let Err(e) = engine.record_activity(&user_activity) {
            tracing::warn!(
                "Failed to record user activity {}: {}",
                user_activity.activity_id,
                e
            );
        }
    }

    Ok(Json(circuit_to_response(circuit)))
}

async fn get_circuit(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<CircuitResponse>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid circuit ID format"})),
        )
    })?;

    let engine = lock_circuits_engine(&state)?;

    match engine.get_circuit(&circuit_id) {
        Ok(Some(circuit)) => Ok(Json(circuit_to_response(circuit))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Circuit not found"})),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to get circuit: {}", e)})),
        )),
    }
}

async fn add_member(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    AuthenticatedUser(requester_id): AuthenticatedUser,
    Json(payload): Json<AddMemberRequest>,
) -> Result<Json<CircuitResponse>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid circuit ID format"})),
        )
    })?;

    let role = parse_member_role(&payload.role)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error": e}))))?;

    let mut engine = lock_circuits_engine(&state)?;

    match engine.add_member_to_circuit(&circuit_id, payload.member_id, role, &requester_id) {
        Ok(circuit) => Ok(Json(circuit_to_response(circuit))),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("Failed to add member: {}", e)})),
        )),
    }
}

async fn push_item(
    State(state): State<Arc<AppState>>,
    Path((id, dfid)): Path<(String, String)>,
    AuthenticatedUser(requester_id): AuthenticatedUser,
    Json(_payload): Json<CircuitOperationRequest>,
) -> Result<Json<CircuitOperationResponse>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid circuit ID format"})),
        )
    })?;

    let engine_clone = Arc::clone(&state.circuits_engine);

    let operation = {
        let mut engine = engine_clone.lock().map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Circuits engine mutex poisoned"})),
            )
        })?;

        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                engine
                    .push_item_to_circuit(&dfid, &circuit_id, &requester_id)
                    .await
            })
        })
        .map_err(|e| {
            let status_code = match e {
                crate::circuits_engine::CircuitsError::PermissionDenied(_) => StatusCode::FORBIDDEN,
                crate::circuits_engine::CircuitsError::AdapterPermissionDenied(_) => {
                    StatusCode::FORBIDDEN
                }
                crate::circuits_engine::CircuitsError::ValidationError(_) => {
                    StatusCode::BAD_REQUEST
                }
                crate::circuits_engine::CircuitsError::CircuitNotFound
                | crate::circuits_engine::CircuitsError::NotFound => StatusCode::NOT_FOUND,
                crate::circuits_engine::CircuitsError::ItemNotFound => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            (status_code, Json(json!({"error": e.to_string()})))
        })?
    };

    Ok(Json(operation_to_response(operation)))
}

async fn pull_item(
    State(state): State<Arc<AppState>>,
    Path((id, dfid)): Path<(String, String)>,
    AuthenticatedUser(requester_id): AuthenticatedUser,
    Json(_payload): Json<CircuitOperationRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid circuit ID format"})),
        )
    })?;

    let (item, operation) = {
        let mut engine = lock_circuits_engine(&state)?;
        engine
            .pull_item_from_circuit(&dfid, &circuit_id, &requester_id)
            .map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({"error": format!("Failed to pull item: {}", e)})),
                )
            })?
    };

    // Fetch all events for this item
    let events = {
        let engine = lock_circuits_engine(&state)?;
        engine
            .get_events_for_item(&dfid)
            .unwrap_or_else(|_| Vec::new())
    };

    Ok(Json(json!({
        "item": {
            "dfid": item.dfid,
            "identifiers": item.identifiers,
            "enriched_data": item.enriched_data,
            "creation_timestamp": item.creation_timestamp.timestamp(),
            "last_modified": item.last_modified.timestamp(),
            "source_entries": item.source_entries,
            "confidence_score": item.confidence_score,
            "status": format!("{:?}", item.status)
        },
        "events": events,
        "events_count": events.len(),
        "operation": operation_to_response(operation)
    })))
}

async fn push_local_item(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    AuthenticatedUser(requester_id): AuthenticatedUser,
    Json(payload): Json<PushLocalItemRequest>,
) -> Result<Json<PushLocalItemResponse>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid circuit ID format"})),
        )
    })?;

    let PushLocalItemRequest {
        local_id: local_id_str,
        identifiers: identifier_requests,
        enriched_data,
    } = payload;

    let local_id = Uuid::parse_str(&local_id_str).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid local_id format"})),
        )
    })?;

    // Convert identifier requests into unified Identifier type
    let identifiers = build_identifiers(identifier_requests.unwrap_or_default()).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("Invalid identifier payload: {}", e)})),
        )
    })?;

    // Clone the Arc so we can use it after dropping the lock
    let engine_clone = Arc::clone(&state.circuits_engine);

    // Call the push_local_item_to_circuit method
    // We need to use tokio::task::spawn_blocking or refactor to not hold mutex across await
    // For now, let's try to make the call without holding the lock by using interior mutability
    let result = {
        let mut engine = engine_clone.lock().map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Circuits engine mutex poisoned"})),
            )
        })?;

        // Make the async call - this will fail with std::sync::Mutex
        // We need to use tokio::task::block_in_place to allow this
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                engine
                    .push_local_item_to_circuit(
                        &local_id,
                        identifiers,
                        enriched_data,
                        &circuit_id,
                        &requester_id, // Extracted from JWT token automatically
                    )
                    .await
            })
        })
        .map_err(|e| {
            let status_code = match e {
                crate::circuits_engine::CircuitsError::PermissionDenied(_) => StatusCode::FORBIDDEN,
                crate::circuits_engine::CircuitsError::ValidationError(_) => {
                    StatusCode::BAD_REQUEST
                }
                crate::circuits_engine::CircuitsError::CircuitNotFound => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            (status_code, Json(json!({"error": format!("{}", e)})))
        })?
    };

    // Capture the item with its latest DFID for persistence outside of the async lock scope
    let item_to_persist = {
        if let Ok(storage_guard) = state.shared_storage.lock() {
            match storage_guard.get_item_by_dfid(&result.dfid) {
                Ok(Some(item)) => Some(item),
                Ok(None) => None,
                Err(e) => {
                    tracing::warn!(
                        "Failed to fetch item {} from shared storage for persistence: {}",
                        result.dfid,
                        e
                    );
                    None
                }
            }
        } else {
            tracing::warn!(
                "Failed to acquire shared storage lock while preparing persistence for {}",
                result.dfid
            );
            None
        }
    };

    let operation_to_persist = {
        if let Ok(storage_guard) = state.shared_storage.lock() {
            match storage_guard.get_circuit_operation(&result.operation_id) {
                Ok(Some(operation)) => Some(operation),
                Ok(None) => None,
                Err(e) => {
                    tracing::warn!(
                        "Failed to fetch circuit operation {} for persistence: {}",
                        result.operation_id,
                        e
                    );
                    None
                }
            }
        } else {
            tracing::warn!(
                "Failed to acquire shared storage lock while preparing operation persistence for {}",
                result.operation_id
            );
            None
        }
    };

    // Write-through cache: Persist to PostgreSQL if available
    let pg_lock = state.postgres_persistence.read().await;
    if let Some(pg) = &*pg_lock {
        // Ensure PostgreSQL is connected before attempting persistence
        if let Err(e) = pg.wait_for_connection(10).await {
            tracing::warn!(
                "⚠️  PostgreSQL not ready for persistence operations: {}. Data will be in-memory only.",
                e
            );
        } else {
            // Persist latest item snapshot
            if let Some(item) = item_to_persist.clone() {
                if let Err(e) = pg.persist_item(&item).await {
                    tracing::warn!(
                        "Failed to persist tokenized item {} to PostgreSQL: {}",
                        item.dfid,
                        e
                    );
                } else {
                    tracing::debug!("✅ Persisted tokenized item {} to PostgreSQL", item.dfid);
                }
            } else {
                tracing::warn!(
                    "Item {} not found in shared storage after tokenization; skipping item persistence",
                    result.dfid
                );
            }

            // Persist LID-DFID mapping
            if let Err(e) = pg
                .persist_lid_dfid_mapping(&result.local_id, &result.dfid)
                .await
            {
                tracing::warn!("Failed to persist LID-DFID mapping to PostgreSQL: {}", e);
            }

            // Persist storage records (transaction hashes, CIDs, etc.)
            // Extract records first to avoid holding lock across await
            let records_to_persist = {
                if let Ok(storage_guard) = state.shared_storage.lock() {
                    if let Ok(Some(storage_history)) =
                        storage_guard.get_storage_history(&result.dfid)
                    {
                        Some(storage_history.storage_records.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            if let Some(records) = records_to_persist {
                for record in &records {
                    // Persist storage record to PostgreSQL
                    if let Err(e) = pg.persist_storage_record(&result.dfid, record).await {
                        tracing::warn!("Failed to persist storage record to PostgreSQL: {}", e);
                        continue; // Skip timeline creation if storage record persist fails
                    }

                    tracing::debug!(
                        "✅ Persisted storage record for {} to PostgreSQL with {} metadata entries",
                        result.dfid,
                        record.metadata.len()
                    );

                    // Log all metadata keys and values for debugging
                    tracing::info!(
                        "📋 Storage record metadata for {}: {:?}",
                        result.dfid,
                        record.metadata
                    );

                    // ============================================================
                    // DUAL-WRITE: Persist CID timeline entry to PostgreSQL
                    // ============================================================
                    // Extract CID, transaction hash, and network from storage record metadata
                    // Looking for specific keys that adapter should have returned
                    let ipfs_cid = record.metadata.get("ipfs_cid").and_then(|v| v.as_str());
                    let ipcm_tx = record
                        .metadata
                        .get("ipcm_update_tx")
                        .and_then(|v| v.as_str());

                    tracing::info!(
                        "🔍 Extracted from metadata - ipfs_cid: {:?}, ipcm_update_tx: {:?}",
                        ipfs_cid,
                        ipcm_tx
                    );

                    if let (Some(cid), Some(ipcm_tx)) = (ipfs_cid, ipcm_tx) {
                        tracing::info!(
                            "✅ Found CID timeline data for {}: CID={}, TX={}",
                            result.dfid,
                            cid,
                            ipcm_tx
                        );
                        let network = record
                            .metadata
                            .get("network")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");

                        let blockchain_timestamp = chrono::Utc::now().timestamp();

                        if let Err(e) = pg
                            .add_cid_to_timeline(
                                &result.dfid,
                                cid,
                                ipcm_tx,
                                blockchain_timestamp,
                                network,
                            )
                            .await
                        {
                            tracing::warn!(
                                "⚠️  Failed to add CID to timeline (non-fatal): {} -> {} ({})",
                                result.dfid,
                                cid,
                                e
                            );
                        } else {
                            tracing::info!(
                            "✅ Added CID to timeline (PostgreSQL): {} -> {} (TX: {}, network: {})",
                            result.dfid,
                            cid,
                            ipcm_tx,
                            network
                        );
                        }
                    } else {
                        tracing::warn!(
                            "⚠️  Missing CID timeline data for {} - ipfs_cid: {:?}, ipcm_update_tx: {:?}. Available metadata keys: {:?}",
                            result.dfid,
                            ipfs_cid,
                            ipcm_tx,
                            record.metadata.keys().collect::<Vec<_>>()
                        );
                    }
                }
            }

            if let Some(operation) = operation_to_persist {
                if let Err(e) = pg.persist_circuit_operation(&operation).await {
                    tracing::warn!(
                        "Failed to persist circuit operation {} to PostgreSQL: {}",
                        operation.operation_id,
                        e
                    );
                }
            }
        } // End of else block (PostgreSQL is connected)
    } // End of if let Some(pg)
    drop(pg_lock);

    let status_str = match result.status {
        crate::circuits_engine::PushStatus::NewItemCreated => "NewItemCreated",
        crate::circuits_engine::PushStatus::ExistingItemEnriched => "ExistingItemEnriched",
        crate::circuits_engine::PushStatus::ConflictDetected { .. } => "ConflictDetected",
    };

    Ok(Json(PushLocalItemResponse {
        success: true,
        data: PushLocalItemData {
            dfid: result.dfid,
            status: status_str.to_string(),
            operation_id: result.operation_id.to_string(),
            local_id: result.local_id.to_string(),
        },
    }))
}

async fn get_circuit_operations(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<CircuitOperationResponse>>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid circuit ID format"})),
        )
    })?;

    let engine = lock_circuits_engine(&state)?;

    match engine.get_circuit_operations(&circuit_id) {
        Ok(operations) => {
            let response: Vec<CircuitOperationResponse> =
                operations.into_iter().map(operation_to_response).collect();
            Ok(Json(response))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to get operations: {}", e)})),
        )),
    }
}

async fn get_pending_operations(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<CircuitOperationResponse>>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid circuit ID format"})),
        )
    })?;

    let engine = lock_circuits_engine(&state)?;

    match engine.get_pending_operations(&circuit_id) {
        Ok(operations) => {
            let response: Vec<CircuitOperationResponse> =
                operations.into_iter().map(operation_to_response).collect();
            Ok(Json(response))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to get pending operations: {}", e)})),
        )),
    }
}

async fn approve_operation(
    State(state): State<Arc<AppState>>,
    Path(operation_id): Path<String>,
    Json(payload): Json<ApproveOperationRequest>,
) -> Result<Json<CircuitOperationResponse>, (StatusCode, Json<Value>)> {
    let operation_uuid = Uuid::parse_str(&operation_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid operation ID format"})),
        )
    })?;

    let mut engine = lock_circuits_engine(&state)?;

    match engine.approve_operation(&operation_uuid, &payload.approver_id) {
        Ok(operation) => {
            let operation_clone = operation.clone();
            drop(engine);
            spawn_persist_circuit_operation(&state, operation_clone);
            Ok(Json(operation_to_response(operation)))
        }
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("Failed to approve operation: {}", e)})),
        )),
    }
}

async fn deactivate_circuit(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    AuthenticatedUser(requester_id): AuthenticatedUser,
    Json(_payload): Json<CircuitOperationRequest>,
) -> Result<Json<CircuitResponse>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid circuit ID format"})),
        )
    })?;

    let mut engine = lock_circuits_engine(&state)?;

    match engine.deactivate_circuit(&circuit_id, &requester_id) {
        Ok(circuit) => {
            // Clone circuit for PostgreSQL persistence
            let circuit_clone = circuit.clone();

            // Release engine lock before async operations
            drop(engine);

            // PostgreSQL persistence happens asynchronously in background
            let pg = state.postgres_persistence.clone();
            tokio::spawn(async move {
                let pg_lock = pg.read().await;
                if let Some(pg_instance) = &*pg_lock {
                    if let Err(e) = pg_instance.persist_circuit(&circuit_clone).await {
                        tracing::warn!(
                            "Failed to persist deactivated circuit to PostgreSQL: {}",
                            e
                        );
                    } else {
                        tracing::info!(
                            "Circuit {} deactivation persisted to PostgreSQL",
                            circuit_clone.circuit_id
                        );
                    }
                }
            });

            Ok(Json(circuit_to_response(circuit)))
        }
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("Failed to deactivate circuit: {}", e)})),
        )),
    }
}

async fn list_circuits(
    State(state): State<Arc<AppState>>,
    Query(params): Query<CircuitListQuery>,
) -> Result<Json<Vec<CircuitResponse>>, (StatusCode, Json<Value>)> {
    let engine = lock_circuits_engine(&state)?;

    match engine.list_circuits() {
        Ok(mut circuits) => {
            // Apply permission-based filtering
            if let Some(user_id) = &params.user_id {
                circuits.retain(|circuit| {
                    // Include circuits where user is a member
                    let is_member = circuit.is_member(user_id);

                    // Include public circuits if requested
                    let is_public = params.include_public.unwrap_or(true)
                        && circuit.permissions.allow_public_visibility;

                    is_member || is_public
                });
            } else if !params.include_public.unwrap_or(false) {
                // If no user_id provided and not requesting public, return empty list for security
                circuits = Vec::new();
            } else {
                // Only show public circuits
                circuits.retain(|circuit| circuit.permissions.allow_public_visibility);
            }

            // Apply status filter
            if let Some(status_str) = &params.status {
                circuits.retain(|circuit| {
                    format!("{:?}", circuit.status).to_lowercase() == status_str.to_lowercase()
                });
            }

            let response: Vec<CircuitResponse> =
                circuits.into_iter().map(circuit_to_response).collect();
            Ok(Json(response))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to list circuits: {}", e)})),
        )),
    }
}

async fn get_circuits_for_member(
    State(state): State<Arc<AppState>>,
    Path(member_id): Path<String>,
) -> Result<Json<Vec<CircuitResponse>>, (StatusCode, Json<Value>)> {
    let engine = lock_circuits_engine(&state)?;

    match engine.get_circuits_for_member(&member_id) {
        Ok(circuits) => {
            let response: Vec<CircuitResponse> =
                circuits.into_iter().map(circuit_to_response).collect();
            Ok(Json(response))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to get circuits for member: {}", e)})),
        )),
    }
}

async fn request_to_join_circuit(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    AuthenticatedUser(requester_id): AuthenticatedUser,
    Json(payload): Json<JoinCircuitRequest>,
) -> Result<Json<CircuitResponse>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid circuit ID format"})),
        )
    })?;

    let mut engine = lock_circuits_engine(&state)?;

    match engine.request_to_join_circuit(&circuit_id, &requester_id, payload.message.clone()) {
        Ok(circuit) => {
            // Send notifications to circuit owner and admins
            if let Ok(notification_engine) = state.notification_engine.lock() {
                let circuit_name = circuit.name.clone();
                let requester_id_clone = requester_id.clone();
                let message_ref = payload.message.as_deref();

                // Notify owner
                if let Ok(notification) = notification_engine.create_join_request_notification(
                    &circuit.owner_id,
                    &requester_id_clone,
                    &circuit_id.to_string(),
                    &circuit_name,
                    message_ref,
                ) {
                    let _ = state.notification_tx.send(
                        crate::api::notifications::NotificationMessage {
                            msg_type: "notification".to_string(),
                            notification: notification.clone(),
                        },
                    );
                }

                // Notify admins with ManageMembers permission
                for member in &circuit.members {
                    if member.member_id != circuit.owner_id
                        && member
                            .permissions
                            .contains(&crate::types::Permission::ManageMembers)
                    {
                        if let Ok(notification) = notification_engine
                            .create_join_request_notification(
                                &member.member_id,
                                &requester_id_clone,
                                &circuit_id.to_string(),
                                &circuit_name,
                                message_ref,
                            )
                        {
                            let _ = state.notification_tx.send(
                                crate::api::notifications::NotificationMessage {
                                    msg_type: "notification".to_string(),
                                    notification: notification.clone(),
                                },
                            );
                        }
                    }
                }
            }

            Ok(Json(circuit_to_response(circuit)))
        }
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("Failed to submit join request: {}", e)})),
        )),
    }
}

async fn get_pending_join_requests(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<JoinRequestResponse>>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid circuit ID format"})),
        )
    })?;

    let engine = lock_circuits_engine(&state)?;

    match engine.get_pending_join_requests(&circuit_id) {
        Ok(requests) => {
            let response: Vec<JoinRequestResponse> = requests
                .into_iter()
                .map(|req| JoinRequestResponse {
                    requester_id: req.requester_id,
                    requested_at: req.requested_at.timestamp(),
                    message: req.message,
                    status: format!("{:?}", req.status),
                })
                .collect();
            Ok(Json(response))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to get pending requests: {}", e)})),
        )),
    }
}

async fn approve_join_request(
    State(state): State<Arc<AppState>>,
    Path((id, requester_id)): Path<(String, String)>,
    Json(payload): Json<ApproveJoinRequest>,
) -> Result<Json<CircuitResponse>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid circuit ID format"})),
        )
    })?;

    let role = parse_member_role(&payload.role)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error": e}))))?;

    let mut engine = lock_circuits_engine(&state)?;

    match engine.approve_join_request(&circuit_id, &requester_id, &payload.admin_id, role) {
        Ok(circuit) => {
            // Clone circuit for PostgreSQL persistence
            let circuit_clone = circuit.clone();

            // Release engine lock before async operations
            drop(engine);

            // PostgreSQL persistence happens asynchronously in background
            let pg = state.postgres_persistence.clone();
            tokio::spawn(async move {
                let pg_lock = pg.read().await;
                if let Some(pg_instance) = &*pg_lock {
                    if let Err(e) = pg_instance.persist_circuit(&circuit_clone).await {
                        tracing::warn!(
                            "Failed to persist join request approval to PostgreSQL: {}",
                            e
                        );
                    } else {
                        tracing::info!(
                            "Circuit {} join approval persisted to PostgreSQL",
                            circuit_clone.circuit_id
                        );
                    }
                }
            });

            // Create and broadcast notification to the requester
            if let Ok(notification_engine) = state.notification_engine.lock() {
                if let Ok(notification) = notification_engine.create_join_approved_notification(
                    &requester_id,
                    &circuit_id.to_string(),
                    &circuit.name,
                    &payload.admin_id,
                    &payload.role,
                ) {
                    // Broadcast via WebSocket
                    let _ = state.notification_tx.send(
                        crate::api::notifications::NotificationMessage {
                            msg_type: "notification".to_string(),
                            notification: notification.clone(),
                        },
                    );
                }
            }
            Ok(Json(circuit_to_response(circuit)))
        }
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("Failed to approve join request: {}", e)})),
        )),
    }
}

async fn reject_join_request(
    State(state): State<Arc<AppState>>,
    Path((id, requester_id)): Path<(String, String)>,
    Json(payload): Json<RejectJoinRequest>,
) -> Result<Json<CircuitResponse>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid circuit ID format"})),
        )
    })?;

    let mut engine = lock_circuits_engine(&state)?;

    match engine.reject_join_request(&circuit_id, &requester_id, &payload.admin_id) {
        Ok(circuit) => {
            // Clone circuit for PostgreSQL persistence
            let circuit_clone = circuit.clone();

            // Release engine lock before async operations
            drop(engine);

            // PostgreSQL persistence happens asynchronously in background
            let pg = state.postgres_persistence.clone();
            tokio::spawn(async move {
                let pg_lock = pg.read().await;
                if let Some(pg_instance) = &*pg_lock {
                    if let Err(e) = pg_instance.persist_circuit(&circuit_clone).await {
                        tracing::warn!(
                            "Failed to persist join request rejection to PostgreSQL: {}",
                            e
                        );
                    } else {
                        tracing::info!(
                            "Circuit {} join rejection persisted to PostgreSQL",
                            circuit_clone.circuit_id
                        );
                    }
                }
            });

            // Create and broadcast notification to the requester
            if let Ok(notification_engine) = state.notification_engine.lock() {
                if let Ok(notification) = notification_engine.create_join_rejected_notification(
                    &requester_id,
                    &circuit_id.to_string(),
                    &circuit.name,
                    &payload.admin_id,
                ) {
                    // Broadcast via WebSocket
                    let _ = state.notification_tx.send(
                        crate::api::notifications::NotificationMessage {
                            msg_type: "notification".to_string(),
                            notification: notification.clone(),
                        },
                    );
                }
            }
            Ok(Json(circuit_to_response(circuit)))
        }
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("Failed to reject join request: {}", e)})),
        )),
    }
}

async fn update_circuit(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    AuthenticatedUser(requester_id): AuthenticatedUser,
    Json(payload): Json<UpdateCircuitRequest>,
) -> Result<Json<CircuitResponse>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid circuit ID format"})),
        )
    })?;

    let mut engine = lock_circuits_engine(&state)?;

    // Get current circuit to preserve existing values
    let current_circuit = engine
        .get_circuit(&circuit_id)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Failed to get circuit: {}", e)})),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Circuit not found"})),
            )
        })?;

    // Convert permissions if provided, preserving existing values for unspecified fields
    let permissions = if let Some(update_perms) = payload.permissions {
        Some(CircuitPermissions {
            require_approval_for_push: update_perms
                .require_approval_for_push
                .unwrap_or(current_circuit.permissions.require_approval_for_push),
            require_approval_for_pull: update_perms
                .require_approval_for_pull
                .unwrap_or(current_circuit.permissions.require_approval_for_pull),
            allow_public_visibility: update_perms
                .allow_public_visibility
                .unwrap_or(current_circuit.permissions.allow_public_visibility),
        })
    } else {
        None
    };

    match engine.update_circuit(
        &circuit_id,
        payload.name,
        payload.description,
        permissions,
        &requester_id,
    ) {
        Ok(circuit) => {
            // Clone circuit for PostgreSQL persistence
            let circuit_clone = circuit.clone();

            // Release engine lock before async operations
            drop(engine);

            // PostgreSQL persistence happens asynchronously in background
            let pg = state.postgres_persistence.clone();
            tokio::spawn(async move {
                let pg_lock = pg.read().await;
                if let Some(pg_instance) = &*pg_lock {
                    if let Err(e) = pg_instance.persist_circuit(&circuit_clone).await {
                        tracing::warn!("Failed to persist circuit update to PostgreSQL: {}", e);
                    } else {
                        tracing::info!(
                            "Circuit {} update persisted to PostgreSQL",
                            circuit_clone.circuit_id
                        );
                    }
                }
            });

            Ok(Json(circuit_to_response(circuit)))
        }
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("Failed to update circuit: {}", e)})),
        )),
    }
}

async fn create_custom_role(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    AuthenticatedUser(requester_id): AuthenticatedUser,
    Json(payload): Json<CreateCustomRoleRequest>,
) -> Result<Json<CustomRoleResponse>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid circuit ID format"})),
        )
    })?;

    // Parse permissions
    let permissions: Result<Vec<Permission>, String> = payload
        .permissions
        .into_iter()
        .map(|p| parse_permission(&p))
        .collect();

    let permissions =
        permissions.map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error": e}))))?;

    let mut engine = lock_circuits_engine(&state)?;

    match engine.create_custom_role(
        &circuit_id,
        payload.role_name,
        permissions,
        payload.description,
        payload.color,
        &requester_id,
    ) {
        Ok(custom_role) => {
            let role_counts = engine
                .get_circuit(&circuit_id)
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": format!("Failed to get circuit: {}", e)})),
                    )
                })?
                .ok_or_else(|| {
                    (
                        StatusCode::NOT_FOUND,
                        Json(json!({"error": "Circuit not found"})),
                    )
                })?
                .get_member_count_by_role();

            let member_count = role_counts
                .get(&custom_role.role_name)
                .copied()
                .unwrap_or(0);
            Ok(Json(custom_role_to_response(custom_role, member_count)))
        }
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("Failed to create custom role: {}", e)})),
        )),
    }
}

async fn get_custom_roles(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<CustomRoleResponse>>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid circuit ID format"})),
        )
    })?;

    let engine = lock_circuits_engine(&state)?;

    match engine.get_custom_roles(&circuit_id) {
        Ok(roles) => {
            let circuit = engine
                .get_circuit(&circuit_id)
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": format!("Failed to get circuit: {}", e)})),
                    )
                })?
                .ok_or_else(|| {
                    (
                        StatusCode::NOT_FOUND,
                        Json(json!({"error": "Circuit not found"})),
                    )
                })?;

            let role_counts = circuit.get_member_count_by_role();

            let response: Vec<CustomRoleResponse> = roles
                .into_iter()
                .map(|role| {
                    let member_count = role_counts.get(&role.role_name).copied().unwrap_or(0);
                    custom_role_to_response(role, member_count)
                })
                .collect();

            Ok(Json(response))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to get custom roles: {}", e)})),
        )),
    }
}

async fn update_custom_role(
    State(state): State<Arc<AppState>>,
    Path((id, role_name)): Path<(String, String)>,
    AuthenticatedUser(requester_id): AuthenticatedUser,
    Json(payload): Json<UpdateCustomRoleRequest>,
) -> Result<Json<CustomRoleResponse>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid circuit ID format"})),
        )
    })?;

    // Parse permissions if provided
    let permissions = if let Some(perm_strings) = payload.permissions {
        let parsed: Result<Vec<Permission>, String> = perm_strings
            .into_iter()
            .map(|p| parse_permission(&p))
            .collect();
        Some(parsed.map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error": e}))))?)
    } else {
        None
    };

    let mut engine = lock_circuits_engine(&state)?;

    match engine.update_custom_role(
        &circuit_id,
        &role_name,
        permissions,
        payload.description,
        payload.color,
        &requester_id,
    ) {
        Ok(updated_role) => {
            let role_counts = engine
                .get_circuit(&circuit_id)
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": format!("Failed to get circuit: {}", e)})),
                    )
                })?
                .ok_or_else(|| {
                    (
                        StatusCode::NOT_FOUND,
                        Json(json!({"error": "Circuit not found"})),
                    )
                })?
                .get_member_count_by_role();

            let member_count = role_counts
                .get(&updated_role.role_name)
                .copied()
                .unwrap_or(0);
            Ok(Json(custom_role_to_response(updated_role, member_count)))
        }
        Err(e) => {
            let status = match e.to_string().as_str() {
                s if s.contains("Permission denied") => StatusCode::FORBIDDEN,
                s if s.contains("not found") => StatusCode::NOT_FOUND,
                s if s.contains("Cannot update") => StatusCode::BAD_REQUEST,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            Err((
                status,
                Json(json!({"error": format!("Failed to update custom role: {}", e)})),
            ))
        }
    }
}

async fn delete_custom_role(
    State(state): State<Arc<AppState>>,
    Path((id, role_name)): Path<(String, String)>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid circuit ID format"})),
        )
    })?;

    let mut engine = lock_circuits_engine(&state)?;

    match engine.remove_custom_role(&circuit_id, &role_name, &claims.user_id) {
        Ok(_) => Ok(Json(json!({
            "success": true,
            "message": format!("Custom role '{}' deleted successfully", role_name)
        }))),
        Err(e) => {
            let status = match e.to_string().as_str() {
                s if s.contains("Permission denied") => StatusCode::FORBIDDEN,
                s if s.contains("not found") => StatusCode::NOT_FOUND,
                s if s.contains("Cannot delete") || s.contains("in use") => StatusCode::BAD_REQUEST,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            Err((
                status,
                Json(json!({"error": format!("Failed to delete custom role: {}", e)})),
            ))
        }
    }
}

async fn assign_member_role(
    State(state): State<Arc<AppState>>,
    Path((id, user_id)): Path<(String, String)>,
    AuthenticatedUser(requester_id): AuthenticatedUser,
    Json(payload): Json<AssignRoleRequest>,
) -> Result<Json<CircuitResponse>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid circuit ID format"})),
        )
    })?;

    let mut engine = lock_circuits_engine(&state)?;

    // If a custom role is specified, assign it
    let role_name = payload.custom_role_name.unwrap_or(payload.role);

    match engine.assign_member_custom_role(&circuit_id, &user_id, &role_name, &requester_id) {
        Ok(circuit) => Ok(Json(circuit_to_response(circuit))),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("Failed to assign role: {}", e)})),
        )),
    }
}

async fn update_public_settings(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    AuthenticatedUser(requester_id): AuthenticatedUser,
    Json(payload): Json<UpdatePublicSettingsRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "validation_error",
                "message": "Invalid circuit ID format",
                "details": {
                    "field": "circuit_id",
                    "issue": "Must be a valid UUID"
                }
            })),
        )
    })?;

    // Validate and parse access mode
    let access_mode = match payload.public_settings.access_mode.to_lowercase().as_str() {
        "public" => crate::types::PublicAccessMode::Public,
        "protected" => crate::types::PublicAccessMode::Protected,
        "scheduled" => crate::types::PublicAccessMode::Scheduled,
        _ => {
            return Err((
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(json!({
                    "error": "validation_error",
                    "message": "Invalid access mode",
                    "details": {
                        "field": "access_mode",
                        "provided_value": payload.public_settings.access_mode,
                        "allowed_values": ["public", "protected", "scheduled"],
                        "issue": "access_mode must be one of: public, protected, scheduled"
                    }
                })),
            ))
        }
    };

    // Validate scheduled date if access mode is scheduled
    if matches!(access_mode, crate::types::PublicAccessMode::Scheduled)
        && payload.public_settings.scheduled_date.is_none()
    {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({
                "error": "validation_error",
                "message": "Scheduled date is required when access_mode is 'scheduled'",
                "details": {
                    "field": "scheduled_date",
                    "required": true,
                    "issue": "scheduled_date must be provided when access_mode is 'scheduled'"
                }
            })),
        ));
    }

    // Parse and validate scheduled date if provided
    let scheduled_date = if let Some(date_str) = payload.public_settings.scheduled_date {
        Some(
            chrono::DateTime::parse_from_rfc3339(&date_str)
                .map_err(|e| {
                    (
                        StatusCode::UNPROCESSABLE_ENTITY,
                        Json(json!({
                            "error": "validation_error",
                            "message": "Invalid scheduled date format",
                            "details": {
                                "field": "scheduled_date",
                                "provided_value": date_str,
                                "expected_format": "RFC3339 (e.g., '2025-01-01T00:00:00Z')",
                                "issue": format!("Failed to parse date: {}", e)
                            }
                        })),
                    )
                })?
                .with_timezone(&chrono::Utc),
        )
    } else {
        None
    };

    // Validate password if access mode is protected
    if matches!(access_mode, crate::types::PublicAccessMode::Protected)
        && payload.public_settings.access_password.is_none()
    {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({
                "error": "validation_error",
                "message": "Password is required when access_mode is 'protected'",
                "details": {
                    "field": "access_password",
                    "required": true,
                    "issue": "access_password must be provided when access_mode is 'protected'"
                }
            })),
        ));
    }

    // Parse and validate export permissions
    let export_permissions = if let Some(export_str) = payload.public_settings.export_permissions {
        match export_str.to_lowercase().as_str() {
            "admin" => Some(crate::types::ExportPermissionLevel::Admin),
            "members" => Some(crate::types::ExportPermissionLevel::Members),
            "public" => Some(crate::types::ExportPermissionLevel::Public),
            _ => {
                return Err((
                    StatusCode::UNPROCESSABLE_ENTITY,
                    Json(json!({
                        "error": "validation_error",
                        "message": "Invalid export permission level",
                        "details": {
                            "field": "export_permissions",
                            "provided_value": export_str,
                            "allowed_values": ["admin", "members", "public"],
                            "issue": "export_permissions must be one of: admin, members, public"
                        }
                    })),
                ))
            }
        }
    } else {
        None
    };

    let public_settings = crate::types::PublicSettings {
        access_mode,
        scheduled_date,
        access_password: payload.public_settings.access_password,
        public_name: payload.public_settings.public_name,
        public_description: payload.public_settings.public_description,
        primary_color: payload.public_settings.primary_color,
        secondary_color: payload.public_settings.secondary_color,
        logo_url: payload.public_settings.logo_url,
        tagline: payload.public_settings.tagline,
        footer_text: payload.public_settings.footer_text,
        published_items: payload.public_settings.published_items.unwrap_or_default(),
        auto_approve_members: payload
            .public_settings
            .auto_approve_members
            .unwrap_or(false),
        auto_publish_pushed_items: payload
            .public_settings
            .auto_publish_pushed_items
            .unwrap_or(false),
        show_encrypted_events: payload
            .public_settings
            .show_encrypted_events
            .unwrap_or(false),
        required_event_types: payload.public_settings.required_event_types,
        data_quality_rules: payload.public_settings.data_quality_rules,
        export_permissions,
    };

    let mut engine = lock_circuits_engine(&state)?;
    match engine.update_public_settings(&circuit_id, public_settings, &requester_id) {
        Ok(circuit) => {
            // Clone circuit for PostgreSQL persistence
            let circuit_clone = circuit.clone();

            // Release engine lock before async operations
            drop(engine);

            // PostgreSQL persistence happens asynchronously in background
            let pg = state.postgres_persistence.clone();
            tokio::spawn(async move {
                let pg_lock = pg.read().await;
                if let Some(pg_instance) = &*pg_lock {
                    if let Err(e) = pg_instance.persist_circuit(&circuit_clone).await {
                        tracing::warn!(
                            "Failed to persist public settings update to PostgreSQL: {}",
                            e
                        );
                    } else {
                        tracing::info!(
                            "Circuit {} public settings persisted to PostgreSQL",
                            circuit_clone.circuit_id
                        );
                    }
                }
            });

            Ok(Json(json!({
                "success": true,
                "data": circuit_to_response(circuit)
            })))
        }
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "update_failed",
                "message": format!("Failed to update public settings: {}", e)
            })),
        )),
    }
}

async fn get_public_circuit(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid circuit ID format"})),
        )
    })?;

    let engine = lock_circuits_engine(&state)?;
    match engine.get_public_circuit_info(&circuit_id) {
        Ok(Some(public_info)) => Ok(Json(json!({
            "success": true,
            "data": {
                "circuit_id": public_info.circuit_id.to_string(),
                "public_name": public_info.public_name,
                "public_description": public_info.public_description,
                "primary_color": public_info.primary_color,
                "secondary_color": public_info.secondary_color,
                "logo_url": public_info.logo_url,
                "tagline": public_info.tagline,
                "footer_text": public_info.footer_text,
                "member_count": public_info.member_count,
                "access_mode": format!("{:?}", public_info.access_mode).to_lowercase(),
                "requires_password": public_info.requires_password,
                "is_currently_accessible": public_info.is_currently_accessible,
                "published_items": public_info.published_items,
                "auto_publish_pushed_items": public_info.auto_publish_pushed_items,
                "recent_activity": []
            }
        }))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Circuit is not publicly accessible"})),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to get public circuit info: {}", e)})),
        )),
    }
}

async fn join_public_circuit(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    AuthenticatedUser(requester_id): AuthenticatedUser,
    Json(payload): Json<JoinPublicCircuitRequest>,
) -> Result<Json<PublicJoinResponse>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid circuit ID format"})),
        )
    })?;

    let mut engine = lock_circuits_engine(&state)?;
    match engine.join_public_circuit(
        &circuit_id,
        &requester_id,
        payload.access_password,
        payload.message,
    ) {
        Ok((requires_approval, message)) => Ok(Json(PublicJoinResponse {
            success: true,
            message,
            requires_approval,
        })),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("Failed to join circuit: {}", e)})),
        )),
    }
}

async fn get_circuit_activities(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<ActivityResponse>>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid circuit ID format"})),
        )
    })?;

    let engine = lock_circuits_engine(&state)?;

    match engine.get_activities_for_circuit(&circuit_id) {
        Ok(activities) => {
            let response: Vec<ActivityResponse> =
                activities.into_iter().map(activity_to_response).collect();
            Ok(Json(response))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to get activities: {}", e)})),
        )),
    }
}

async fn get_circuit_items(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query): Query<CircuitItemsQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid circuit ID format"})),
        )
    })?;

    let items = {
        let engine = lock_circuits_engine(&state)?;
        engine.get_circuit_items(&circuit_id).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Failed to get circuit items: {}", e)})),
            )
        })?
    };

    if query.include_events {
        // Fetch events for each item
        let mut items_with_events = Vec::new();

        for item in items {
            let dfid = item.dfid.clone();
            let item_response = circuit_item_to_response(item);

            // Fetch events for this DFID
            let events = {
                let engine = lock_circuits_engine(&state)?;
                engine
                    .get_events_for_item(&dfid)
                    .unwrap_or_else(|_| Vec::new())
            };

            items_with_events.push(CircuitItemWithEventsResponse {
                item: item_response,
                events,
            });
        }

        Ok(Json(serde_json::to_value(items_with_events).map_err(
            |e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": format!("Failed to serialize response: {}", e)})),
                )
            },
        )?))
    } else {
        // Return items without events (original behavior)
        let response: Vec<CircuitItemResponse> =
            items.into_iter().map(circuit_item_to_response).collect();
        Ok(Json(serde_json::to_value(response).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Failed to serialize response: {}", e)})),
            )
        })?))
    }
}

async fn batch_push_items(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    AuthenticatedUser(requester_id): AuthenticatedUser,
    Json(payload): Json<BatchPushRequest>,
) -> Result<Json<BatchPushResult>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid circuit ID format"})),
        )
    })?;

    let mut results = Vec::new();
    let mut success_count = 0;
    let mut failed_count = 0;

    // Process each item sequentially using block_in_place
    for item in payload.items.iter() {
        let dfid = item.dfid.clone();
        let circuit_id_copy = circuit_id;
        let requester_id_clone = requester_id.clone();
        let engine_arc = state.circuits_engine.clone();

        let result = tokio::task::block_in_place(|| {
            let mut engine = engine_arc.lock().map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": "Engine mutex poisoned"})),
                )
            })?;

            tokio::runtime::Handle::current()
                .block_on(async {
                    engine
                        .push_item_to_circuit(&dfid, &circuit_id_copy, &requester_id_clone)
                        .await
                })
                .map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(json!({"error": format!("Push failed: {}", e)})),
                    )
                })
        });

        match result {
            Ok(_) => {
                success_count += 1;
                results.push(BatchPushItemResult {
                    dfid: item.dfid.clone(),
                    success: true,
                    error_message: None,
                });
            }
            Err(e) => {
                failed_count += 1;
                results.push(BatchPushItemResult {
                    dfid: item.dfid.clone(),
                    success: false,
                    error_message: Some(format!("{:?}", e.1)),
                });
            }
        }
    }

    Ok(Json(BatchPushResult {
        success_count,
        failed_count,
        results,
    }))
}

async fn get_circuit_pending_items(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid circuit ID format"})),
        )
    })?;

    let circuits_engine = lock_circuits_engine(&state)?;

    // Get pending operations for this circuit
    let pending_operations = circuits_engine
        .get_pending_operations(&circuit_id)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Failed to get pending operations: {}", e)})),
            )
        })?;

    drop(circuits_engine); // Release lock before fetching items

    let mut pending_items = Vec::new();

    for operation in pending_operations {
        // Try to fetch the item to build preview
        let item_preview = {
            let items_engine = lock_items_engine(&state)?;
            match items_engine.get_item(&operation.dfid) {
                Ok(Some(item)) => Some(PendingItemPreview {
                    identifiers: item
                        .identifiers
                        .into_iter()
                        .map(|identifier| IdentifierRequest::from_identifier(&identifier))
                        .collect(),
                    enriched_data: serde_json::to_value(item.enriched_data)
                        .unwrap_or(serde_json::json!({})),
                }),
                _ => None,
            }
        };

        pending_items.push(PendingItemResponse {
            pending_id: operation.operation_id.to_string(),
            dfid: operation.dfid,
            pushed_by: operation.requester_id,
            pushed_at: operation.timestamp.timestamp(),
            status: format!("{:?}", operation.status),
            item_preview,
        });
    }

    Ok(Json(json!({
        "success": true,
        "data": pending_items
    })))
}

async fn approve_pending_item(
    State(state): State<Arc<AppState>>,
    Path((circuit_id, pending_id)): Path<(String, String)>,
    Json(payload): Json<ApproveItemRequest>,
) -> Result<Json<ApproveItemResponse>, (StatusCode, Json<Value>)> {
    let _circuit_id = Uuid::parse_str(&circuit_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid circuit ID format"})),
        )
    })?;

    let operation_id = Uuid::parse_str(&pending_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid pending ID format"})),
        )
    })?;

    let mut circuits_engine = lock_circuits_engine(&state)?;

    let approved_operation = circuits_engine
        .approve_operation(&operation_id, &payload.admin_id)
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("Failed to approve operation: {}", e)})),
            )
        })?;

    Ok(Json(ApproveItemResponse {
        success: true,
        message: "Item approved and added to circuit".to_string(),
        data: ApproveItemData {
            circuit_id: approved_operation.circuit_id.to_string(),
            dfid: approved_operation.dfid,
            pushed_by: approved_operation.requester_id.clone(),
            pushed_at: approved_operation.timestamp.timestamp(),
            approved_by: payload.admin_id,
            approved_at: Utc::now().timestamp(),
            status: "active".to_string(),
        },
    }))
}

async fn reject_pending_item(
    State(state): State<Arc<AppState>>,
    Path((circuit_id, pending_id)): Path<(String, String)>,
    Json(payload): Json<RejectItemRequest>,
) -> Result<Json<RejectItemResponse>, (StatusCode, Json<Value>)> {
    let _circuit_id = Uuid::parse_str(&circuit_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid circuit ID format"})),
        )
    })?;

    let operation_id = Uuid::parse_str(&pending_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid pending ID format"})),
        )
    })?;

    let mut circuits_engine = lock_circuits_engine(&state)?;

    circuits_engine
        .reject_operation(&operation_id, &payload.admin_id, payload.reason.clone())
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("Failed to reject operation: {}", e)})),
            )
        })?;

    drop(circuits_engine);

    let operation_to_persist = {
        if let Ok(storage_guard) = state.shared_storage.lock() {
            match storage_guard.get_circuit_operation(&operation_id) {
                Ok(Some(operation)) => Some(operation),
                Ok(None) => {
                    tracing::warn!(
                        "Circuit operation {} missing after rejection; skipping persistence",
                        operation_id
                    );
                    None
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to fetch circuit operation {} for persistence: {}",
                        operation_id,
                        e
                    );
                    None
                }
            }
        } else {
            tracing::warn!(
                "Failed to acquire shared storage lock while preparing persistence for operation {}",
                operation_id
            );
            None
        }
    };

    if let Some(operation) = operation_to_persist {
        spawn_persist_circuit_operation(&state, operation);
    }

    Ok(Json(RejectItemResponse {
        success: true,
        message: "Item rejected".to_string(),
    }))
}

async fn get_circuit_adapter_config(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<GetAdapterConfigResponse>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid circuit ID format"})),
        )
    })?;

    let engine = lock_circuits_engine(&state)?;

    match engine.get_circuit(&circuit_id) {
        Ok(Some(circuit)) => {
            if let Some(adapter_config) = circuit.adapter_config {
                // Convert AdapterType enum to hyphenated string format
                // If adapter_type is None, return "none"
                let adapter_type_str = adapter_config
                    .adapter_type
                    .map(|adapter| adapter_type_to_string(&adapter))
                    .or(Some("none".to_string()));

                Ok(Json(GetAdapterConfigResponse {
                    adapter_type: adapter_type_str,
                    sponsor_adapter_access: adapter_config.sponsor_adapter_access,
                    requires_approval: adapter_config.requires_approval,
                    auto_migrate_existing: adapter_config.auto_migrate_existing,
                    configured_by: adapter_config.configured_by,
                    configured_at: adapter_config.configured_at.to_rfc3339(),
                }))
            } else {
                // This branch should never be reached now that circuits are initialized with default adapter_config
                // But keep it for backward compatibility with old circuits
                Ok(Json(GetAdapterConfigResponse {
                    adapter_type: Some("none".to_string()),
                    sponsor_adapter_access: false,
                    requires_approval: false,
                    auto_migrate_existing: false,
                    configured_by: "system".to_string(),
                    configured_at: chrono::Utc::now().to_rfc3339(),
                }))
            }
        }
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Circuit not found"})),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to get circuit: {}", e)})),
        )),
    }
}

/// Helper function to convert AdapterType enum to hyphenated string format
fn adapter_type_to_string(adapter: &AdapterType) -> String {
    adapter.to_string()
}

async fn set_circuit_adapter_config(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<SetAdapterConfigRequest>,
) -> Result<Json<GetAdapterConfigResponse>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid circuit ID format"})),
        )
    })?;

    // Parse adapter_type string to AdapterType enum
    let adapter_type = if let Some(adapter_str) = payload.adapter_type {
        Some(AdapterType::from_string(&adapter_str).map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("Invalid adapter type: {}", e)})),
            )
        })?)
    } else {
        None
    };

    let mut engine = lock_circuits_engine(&state)?;

    match engine.set_circuit_adapter_config(
        &circuit_id,
        &claims.user_id,
        adapter_type,
        payload.auto_migrate_existing,
        payload.requires_approval,
        payload.sponsor_adapter_access,
    ) {
        Ok(adapter_config) => {
            if let Ok(Some(circuit)) = engine.get_circuit(&circuit_id) {
                spawn_persist_circuit(&state, circuit.clone());
            }

            // Convert AdapterType enum to hyphenated string format
            let adapter_type_str = adapter_config
                .adapter_type
                .map(|adapter| adapter_type_to_string(&adapter));

            Ok(Json(GetAdapterConfigResponse {
                adapter_type: adapter_type_str,
                sponsor_adapter_access: adapter_config.sponsor_adapter_access,
                requires_approval: adapter_config.requires_approval,
                auto_migrate_existing: adapter_config.auto_migrate_existing,
                configured_by: adapter_config.configured_by,
                configured_at: adapter_config.configured_at.to_rfc3339(),
            }))
        }
        Err(e) => {
            let status = match e.to_string().as_str() {
                s if s.contains("Permission denied") || s.contains("does not have access") => {
                    StatusCode::FORBIDDEN
                }
                s if s.contains("not found") => StatusCode::NOT_FOUND,
                s if s.contains("Invalid") => StatusCode::BAD_REQUEST,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            Err((status, Json(json!({"error": e.to_string()}))))
        }
    }
}

// ============================================================================
// WEBHOOK CONFIGURATION HANDLERS
// ============================================================================

async fn get_post_action_settings(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid circuit ID"})),
        )
    })?;

    let storage = state.shared_storage.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Storage mutex poisoned"})),
        )
    })?;
    let circuit = storage
        .get_circuit(&circuit_id)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Circuit not found"})),
        ))?;

    // Only owner and admins can view post-action settings
    if !circuit.has_permission(&claims.user_id, &Permission::ManagePermissions) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Permission denied"})),
        ));
    }

    Ok(Json(json!({
        "success": true,
        "data": circuit.post_action_settings
    })))
}

async fn update_post_action_settings(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<UpdatePostActionSettingsRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid circuit ID"})),
        )
    })?;

    let mut storage = state.shared_storage.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Storage mutex poisoned"})),
        )
    })?;
    let mut circuit = storage
        .get_circuit(&circuit_id)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Circuit not found"})),
        ))?;

    // Only owner and admins can modify settings
    if !circuit.has_permission(&claims.user_id, &Permission::ManagePermissions) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Permission denied"})),
        ));
    }

    // Parse trigger events
    let trigger_events: Vec<PostActionTrigger> = request
        .trigger_events
        .iter()
        .filter_map(|s| match s.as_str() {
            "item_pushed" => Some(PostActionTrigger::ItemPushed),
            "item_approved" => Some(PostActionTrigger::ItemApproved),
            "item_tokenized" => Some(PostActionTrigger::ItemTokenized),
            "item_published" => Some(PostActionTrigger::ItemPublished),
            _ => None,
        })
        .collect();

    // Update settings
    let mut settings = circuit.post_action_settings.unwrap_or_default();
    settings.enabled = request.enabled;
    settings.trigger_events = trigger_events;
    settings.include_storage_details = request.include_storage_details;
    settings.include_item_metadata = request.include_item_metadata;

    circuit.post_action_settings = Some(settings.clone());
    storage.store_circuit(&circuit).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
    })?;

    Ok(Json(json!({
        "success": true,
        "message": "Post-action settings updated",
        "data": settings
    })))
}

async fn create_webhook(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<CreateWebhookRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid circuit ID"})),
        )
    })?;

    // Validate webhook URL
    use crate::storage::InMemoryStorage;
    use crate::webhook_engine::WebhookEngine;
    WebhookEngine::<InMemoryStorage>::validate_webhook_url(&request.url).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": e.to_string()})),
        )
    })?;

    let mut storage = state.shared_storage.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Storage mutex poisoned"})),
        )
    })?;
    let mut circuit = storage
        .get_circuit(&circuit_id)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Circuit not found"})),
        ))?;

    // Only owner and admins can create webhooks
    if !circuit.has_permission(&claims.user_id, &Permission::ManagePermissions) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Permission denied"})),
        ));
    }

    // Create webhook config
    let mut webhook = WebhookConfig::new(request.name, request.url);

    if let Some(method_str) = request.method {
        webhook.method = match method_str.to_uppercase().as_str() {
            "POST" => HttpMethod::Post,
            "PUT" => HttpMethod::Put,
            "PATCH" => HttpMethod::Patch,
            _ => HttpMethod::Post,
        };
    }

    if let Some(headers) = request.headers {
        webhook.headers = headers;
    }

    if let Some(auth_type_str) = request.auth_type {
        webhook.auth_type = match auth_type_str.as_str() {
            "BearerToken" => WebhookAuthType::BearerToken,
            "ApiKey" => WebhookAuthType::ApiKey,
            "BasicAuth" => WebhookAuthType::BasicAuth,
            "CustomHeader" => WebhookAuthType::CustomHeader,
            _ => WebhookAuthType::None,
        };
    }

    webhook.auth_credentials = request.auth_credentials;
    webhook.enabled = request.enabled.unwrap_or(true);

    // Add webhook to circuit
    let mut settings = circuit.post_action_settings.unwrap_or_default();
    settings.webhooks.push(webhook.clone());
    circuit.post_action_settings = Some(settings);

    storage.store_circuit(&circuit).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
    })?;

    let persisted_circuit = circuit.clone();
    drop(storage);
    spawn_persist_circuit(&state, persisted_circuit);

    Ok(Json(json!({
        "success": true,
        "message": "Webhook created successfully",
        "data": webhook
    })))
}

async fn get_webhook(
    State(state): State<Arc<AppState>>,
    Path((circuit_id, webhook_id)): Path<(String, String)>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let circuit_uuid = Uuid::parse_str(&circuit_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid circuit ID"})),
        )
    })?;

    let webhook_uuid = Uuid::parse_str(&webhook_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid webhook ID"})),
        )
    })?;

    let storage = state.shared_storage.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Storage mutex poisoned"})),
        )
    })?;
    let circuit = storage
        .get_circuit(&circuit_uuid)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Circuit not found"})),
        ))?;

    if !circuit.has_permission(&claims.user_id, &Permission::ManagePermissions) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Permission denied"})),
        ));
    }

    let settings = circuit.post_action_settings.ok_or((
        StatusCode::NOT_FOUND,
        Json(json!({"error": "Post-action settings not configured"})),
    ))?;

    let webhook = settings
        .webhooks
        .iter()
        .find(|w| w.id == webhook_uuid)
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Webhook not found"})),
        ))?;

    Ok(Json(json!({
        "success": true,
        "data": webhook
    })))
}

async fn update_webhook(
    State(state): State<Arc<AppState>>,
    Path((circuit_id, webhook_id)): Path<(String, String)>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<UpdateWebhookRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let circuit_uuid = Uuid::parse_str(&circuit_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid circuit ID"})),
        )
    })?;

    let webhook_uuid = Uuid::parse_str(&webhook_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid webhook ID"})),
        )
    })?;

    let mut storage = state.shared_storage.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Storage mutex poisoned"})),
        )
    })?;
    let mut circuit = storage
        .get_circuit(&circuit_uuid)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Circuit not found"})),
        ))?;

    if !circuit.has_permission(&claims.user_id, &Permission::ManagePermissions) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Permission denied"})),
        ));
    }

    let mut settings = circuit.post_action_settings.ok_or((
        StatusCode::NOT_FOUND,
        Json(json!({"error": "Post-action settings not configured"})),
    ))?;

    let webhook = settings
        .webhooks
        .iter_mut()
        .find(|w| w.id == webhook_uuid)
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Webhook not found"})),
        ))?;

    // Update webhook fields
    if let Some(name) = request.name {
        webhook.name = name;
    }
    if let Some(url) = request.url {
        use crate::storage::InMemoryStorage;
        use crate::webhook_engine::WebhookEngine;
        WebhookEngine::<InMemoryStorage>::validate_webhook_url(&url).map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": e.to_string()})),
            )
        })?;
        webhook.url = url;
    }
    if let Some(enabled) = request.enabled {
        webhook.enabled = enabled;
    }

    webhook.updated_at = Utc::now();
    circuit.post_action_settings = Some(settings);

    storage.store_circuit(&circuit).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
    })?;

    let persisted_circuit = circuit.clone();
    drop(storage);
    spawn_persist_circuit(&state, persisted_circuit);

    Ok(Json(json!({
        "success": true,
        "message": "Webhook updated successfully"
    })))
}

async fn delete_webhook(
    State(state): State<Arc<AppState>>,
    Path((circuit_id, webhook_id)): Path<(String, String)>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let circuit_uuid = Uuid::parse_str(&circuit_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid circuit ID"})),
        )
    })?;

    let webhook_uuid = Uuid::parse_str(&webhook_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid webhook ID"})),
        )
    })?;

    let mut storage = state.shared_storage.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Storage mutex poisoned"})),
        )
    })?;
    let mut circuit = storage
        .get_circuit(&circuit_uuid)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Circuit not found"})),
        ))?;

    if !circuit.has_permission(&claims.user_id, &Permission::ManagePermissions) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Permission denied"})),
        ));
    }

    let mut settings = circuit.post_action_settings.ok_or((
        StatusCode::NOT_FOUND,
        Json(json!({"error": "Post-action settings not configured"})),
    ))?;

    settings.webhooks.retain(|w| w.id != webhook_uuid);
    circuit.post_action_settings = Some(settings);

    storage.store_circuit(&circuit).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
    })?;

    let persisted_circuit = circuit.clone();
    drop(storage);
    spawn_persist_circuit(&state, persisted_circuit);

    Ok(Json(json!({
        "success": true,
        "message": "Webhook deleted successfully"
    })))
}

async fn test_webhook(
    State(state): State<Arc<AppState>>,
    Path((circuit_id, webhook_id)): Path<(String, String)>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let circuit_uuid = Uuid::parse_str(&circuit_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid circuit ID"})),
        )
    })?;

    let webhook_uuid = Uuid::parse_str(&webhook_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid webhook ID"})),
        )
    })?;

    let storage = state.shared_storage.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Storage mutex poisoned"})),
        )
    })?;
    let circuit = storage
        .get_circuit(&circuit_uuid)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Circuit not found"})),
        ))?;

    if !circuit.has_permission(&claims.user_id, &Permission::ManagePermissions) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Permission denied"})),
        ));
    }

    let settings = circuit.post_action_settings.ok_or((
        StatusCode::NOT_FOUND,
        Json(json!({"error": "Post-action settings not configured"})),
    ))?;

    let webhook = settings
        .webhooks
        .iter()
        .find(|w| w.id == webhook_uuid)
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Webhook not found"})),
        ))?;

    // Create test payload
    let _test_payload = json!({
        "event_type": "webhook_test",
        "circuit_id": circuit_id,
        "circuit_name": circuit.name,
        "timestamp": Utc::now().to_rfc3339(),
        "test": true,
        "message": "This is a test webhook delivery from DeFarm"
    });

    // Test webhook delivery (send test payload)
    Ok(Json(json!({
        "success": true,
        "message": "Webhook test initiated",
        "webhook": {
            "id": webhook.id,
            "name": webhook.name,
            "url": webhook.url
        }
    })))
}

async fn get_webhook_deliveries(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid circuit ID"})),
        )
    })?;

    let storage = state.shared_storage.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Storage mutex poisoned"})),
        )
    })?;
    let circuit = storage
        .get_circuit(&circuit_id)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Circuit not found"})),
        ))?;

    if !circuit.has_permission(&claims.user_id, &Permission::ManagePermissions) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Permission denied"})),
        ));
    }

    let limit = params.get("limit").and_then(|s| s.parse::<usize>().ok());

    let deliveries = storage
        .get_webhook_deliveries_by_circuit(&circuit_id, limit)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
        })?;

    Ok(Json(json!({
        "success": true,
        "data": deliveries,
        "count": deliveries.len()
    })))
}

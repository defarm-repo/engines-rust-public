use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, patch, post, put},
    Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::{CircuitsEngine, InMemoryStorage, MemberRole, Circuit, CircuitOperation};
use crate::types::{Activity, BatchPushResult, CircuitItem, CircuitPermissions, Permission, CustomRole, PublicSettings};

#[derive(Debug, Deserialize)]
pub struct CreateCircuitRequest {
    pub name: String,
    pub description: String,
    pub owner_id: String,
}

#[derive(Debug, Deserialize)]
pub struct AddMemberRequest {
    pub member_id: String,
    pub role: String,
    pub requester_id: String,
}

#[derive(Debug, Deserialize)]
pub struct CircuitOperationRequest {
    pub requester_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ApproveOperationRequest {
    pub approver_id: String,
}

#[derive(Debug, Deserialize)]
pub struct JoinCircuitRequest {
    pub requester_id: String,
    pub message: Option<String>,
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
    pub requester_id: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCircuitPermissions {
    pub default_push: Option<bool>,
    pub default_pull: Option<bool>,
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
    pub requester_id: String,
}

#[derive(Debug, Deserialize)]
pub struct AssignRoleRequest {
    pub role: String,
    pub custom_role_name: Option<String>,
    pub requester_id: String,
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
    pub default_push: bool,
    pub default_pull: bool,
    pub require_approval_for_push: bool,
    pub require_approval_for_pull: bool,
    pub allow_public_visibility: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePublicSettingsRequest {
    pub requester_id: String,
    pub public_settings: PublicSettingsRequest,
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
    pub requester_id: String,
    pub access_password: Option<String>,
    pub message: Option<String>,
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

#[derive(Debug, Deserialize)]
pub struct BatchPushRequest {
    pub dfids: Vec<String>,
    pub requester_id: String,
    pub permissions: Option<Vec<String>>,
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

pub struct CircuitState {
    pub engine: Arc<Mutex<CircuitsEngine<InMemoryStorage>>>,
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

pub fn circuit_routes(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", post(create_circuit))
        .route("/", get(list_circuits))
        .route("/:id", get(get_circuit))
        .route("/:id", patch(update_circuit))
        .route("/:id", put(update_circuit))
        .route("/:id/members", post(add_member))
        .route("/:id/push/:dfid", post(push_item))
        .route("/:id/pull/:dfid", post(pull_item))
        .route("/:id/operations", get(get_circuit_operations))
        .route("/:id/operations/pending", get(get_pending_operations))
        .route("/operations/:operation_id/approve", post(approve_operation))
        .route("/:id/deactivate", put(deactivate_circuit))
        .route("/:id/requests", post(request_to_join_circuit))
        .route("/:id/requests/pending", get(get_pending_join_requests))
        .route("/:id/requests/:requester_id/approve", post(approve_join_request))
        .route("/:id/requests/:requester_id/reject", post(reject_join_request))
        .route("/:id/roles", post(create_custom_role))
        .route("/:id/roles", get(get_custom_roles))
        .route("/:id/members/:user_id", patch(assign_member_role))
        .route("/:id/public-settings", put(update_public_settings))
        .route("/:id/public", get(get_public_circuit))
        .route("/:id/public/join", post(join_public_circuit))
        .route("/:id/activities", get(get_circuit_activities))
        .route("/:id/items", get(get_circuit_items))
        .route("/:id/push/batch", post(batch_push_items))
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
        _ => Err(format!("Invalid member role: {}", role_str)),
    }
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
        _ => Err(format!("Invalid permission: {}", permission_str)),
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
        members: circuit.members
            .into_iter()
            .map(|member| CircuitMemberResponse {
                member_id: member.member_id,
                role: format!("{:?}", member.role),
                custom_role_name: member.custom_role_name,
                permissions: member.permissions
                    .into_iter()
                    .map(|p| format!("{:?}", p))
                    .collect(),
                joined_timestamp: member.joined_timestamp.timestamp(),
            })
            .collect(),
        permissions: CircuitPermissionsResponse {
            default_push: circuit.permissions.default_push,
            default_pull: circuit.permissions.default_pull,
            require_approval_for_push: circuit.permissions.require_approval_for_push,
            require_approval_for_pull: circuit.permissions.require_approval_for_pull,
            allow_public_visibility: circuit.permissions.allow_public_visibility,
        },
        status: format!("{:?}", circuit.status),
        pending_requests: circuit.pending_requests
            .into_iter()
            .map(|req| JoinRequestResponse {
                requester_id: req.requester_id,
                requested_at: req.requested_at.timestamp(),
                message: req.message,
                status: format!("{:?}", req.status),
            })
            .collect(),
        custom_roles: circuit.custom_roles
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
        permissions: role.permissions
            .into_iter()
            .map(|p| format!("{:?}", p))
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
    Json(payload): Json<CreateCircuitRequest>,
) -> Result<Json<CircuitResponse>, (StatusCode, Json<Value>)> {
    let mut engine = state.circuits_engine.lock().unwrap();

    match engine.create_circuit(payload.name, payload.description, payload.owner_id) {
        Ok(circuit) => Ok(Json(circuit_to_response(circuit))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to create circuit: {}", e)})))),
    }
}

async fn get_circuit(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<CircuitResponse>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid circuit ID format"}))))?;

    let engine = state.circuits_engine.lock().unwrap();

    match engine.get_circuit(&circuit_id) {
        Ok(Some(circuit)) => Ok(Json(circuit_to_response(circuit))),
        Ok(None) => Err((StatusCode::NOT_FOUND, Json(json!({"error": "Circuit not found"})))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to get circuit: {}", e)})))),
    }
}

async fn add_member(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(payload): Json<AddMemberRequest>,
) -> Result<Json<CircuitResponse>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid circuit ID format"}))))?;

    let role = parse_member_role(&payload.role)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error": e}))))?;

    let mut engine = state.circuits_engine.lock().unwrap();

    match engine.add_member_to_circuit(&circuit_id, payload.member_id, role, &payload.requester_id) {
        Ok(circuit) => Ok(Json(circuit_to_response(circuit))),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({"error": format!("Failed to add member: {}", e)})))),
    }
}

async fn push_item(
    State(state): State<Arc<AppState>>,
    Path((id, dfid)): Path<(String, String)>,
    Json(payload): Json<CircuitOperationRequest>,
) -> Result<Json<CircuitOperationResponse>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid circuit ID format"}))))?;

    let mut engine = state.circuits_engine.lock().unwrap();

    match engine.push_item_to_circuit(&dfid, &circuit_id, &payload.requester_id) {
        Ok(operation) => Ok(Json(operation_to_response(operation))),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({"error": format!("Failed to push item: {}", e)})))),
    }
}

async fn pull_item(
    State(state): State<Arc<AppState>>,
    Path((id, dfid)): Path<(String, String)>,
    Json(payload): Json<CircuitOperationRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid circuit ID format"}))))?;

    let mut engine = state.circuits_engine.lock().unwrap();

    match engine.pull_item_from_circuit(&dfid, &circuit_id, &payload.requester_id) {
        Ok((item, operation)) => {
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
                "operation": operation_to_response(operation)
            })))
        }
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({"error": format!("Failed to pull item: {}", e)})))),
    }
}

async fn get_circuit_operations(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<CircuitOperationResponse>>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid circuit ID format"}))))?;

    let engine = state.circuits_engine.lock().unwrap();

    match engine.get_circuit_operations(&circuit_id) {
        Ok(operations) => {
            let response: Vec<CircuitOperationResponse> = operations
                .into_iter()
                .map(operation_to_response)
                .collect();
            Ok(Json(response))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to get operations: {}", e)})))),
    }
}

async fn get_pending_operations(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<CircuitOperationResponse>>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid circuit ID format"}))))?;

    let engine = state.circuits_engine.lock().unwrap();

    match engine.get_pending_operations(&circuit_id) {
        Ok(operations) => {
            let response: Vec<CircuitOperationResponse> = operations
                .into_iter()
                .map(operation_to_response)
                .collect();
            Ok(Json(response))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to get pending operations: {}", e)})))),
    }
}

async fn approve_operation(
    State(state): State<Arc<AppState>>,
    Path(operation_id): Path<String>,
    Json(payload): Json<ApproveOperationRequest>,
) -> Result<Json<CircuitOperationResponse>, (StatusCode, Json<Value>)> {
    let operation_uuid = Uuid::parse_str(&operation_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid operation ID format"}))))?;

    let mut engine = state.circuits_engine.lock().unwrap();

    match engine.approve_operation(&operation_uuid, &payload.approver_id) {
        Ok(operation) => Ok(Json(operation_to_response(operation))),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({"error": format!("Failed to approve operation: {}", e)})))),
    }
}

async fn deactivate_circuit(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(payload): Json<CircuitOperationRequest>,
) -> Result<Json<CircuitResponse>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid circuit ID format"}))))?;

    let mut engine = state.circuits_engine.lock().unwrap();

    match engine.deactivate_circuit(&circuit_id, &payload.requester_id) {
        Ok(circuit) => Ok(Json(circuit_to_response(circuit))),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({"error": format!("Failed to deactivate circuit: {}", e)})))),
    }
}

async fn list_circuits(
    State(state): State<Arc<AppState>>,
    Query(params): Query<CircuitListQuery>,
) -> Result<Json<Vec<CircuitResponse>>, (StatusCode, Json<Value>)> {
    let engine = state.circuits_engine.lock().unwrap();

    match engine.list_circuits() {
        Ok(mut circuits) => {
            // Apply permission-based filtering
            if let Some(user_id) = &params.user_id {
                circuits = circuits.into_iter().filter(|circuit| {
                    // Include circuits where user is a member
                    let is_member = circuit.is_member(user_id);

                    // Include public circuits if requested
                    let is_public = params.include_public.unwrap_or(true) &&
                                   circuit.permissions.allow_public_visibility;

                    is_member || is_public
                }).collect();
            } else if !params.include_public.unwrap_or(false) {
                // If no user_id provided and not requesting public, return empty list for security
                circuits = Vec::new();
            } else {
                // Only show public circuits
                circuits = circuits.into_iter().filter(|circuit| {
                    circuit.permissions.allow_public_visibility
                }).collect();
            }

            // Apply status filter
            if let Some(status_str) = &params.status {
                circuits = circuits.into_iter().filter(|circuit| {
                    format!("{:?}", circuit.status).to_lowercase() == status_str.to_lowercase()
                }).collect();
            }

            let response: Vec<CircuitResponse> = circuits
                .into_iter()
                .map(circuit_to_response)
                .collect();
            Ok(Json(response))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to list circuits: {}", e)})))),
    }
}

async fn get_circuits_for_member(
    State(state): State<Arc<AppState>>,
    Path(member_id): Path<String>,
) -> Result<Json<Vec<CircuitResponse>>, (StatusCode, Json<Value>)> {
    let engine = state.circuits_engine.lock().unwrap();

    match engine.get_circuits_for_member(&member_id) {
        Ok(circuits) => {
            let response: Vec<CircuitResponse> = circuits
                .into_iter()
                .map(circuit_to_response)
                .collect();
            Ok(Json(response))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to get circuits for member: {}", e)})))),
    }
}

async fn request_to_join_circuit(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(payload): Json<JoinCircuitRequest>,
) -> Result<Json<CircuitResponse>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid circuit ID format"}))))?;

    let mut engine = state.circuits_engine.lock().unwrap();

    match engine.request_to_join_circuit(&circuit_id, &payload.requester_id, payload.message) {
        Ok(circuit) => Ok(Json(circuit_to_response(circuit))),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({"error": format!("Failed to submit join request: {}", e)})))),
    }
}

async fn get_pending_join_requests(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<JoinRequestResponse>>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid circuit ID format"}))))?;

    let engine = state.circuits_engine.lock().unwrap();

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
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to get pending requests: {}", e)})))),
    }
}

async fn approve_join_request(
    State(state): State<Arc<AppState>>,
    Path((id, requester_id)): Path<(String, String)>,
    Json(payload): Json<ApproveJoinRequest>,
) -> Result<Json<CircuitResponse>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid circuit ID format"}))))?;

    let role = parse_member_role(&payload.role)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error": e}))))?;

    let mut engine = state.circuits_engine.lock().unwrap();

    match engine.approve_join_request(&circuit_id, &requester_id, &payload.admin_id, role) {
        Ok(circuit) => Ok(Json(circuit_to_response(circuit))),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({"error": format!("Failed to approve join request: {}", e)})))),
    }
}

async fn reject_join_request(
    State(state): State<Arc<AppState>>,
    Path((id, requester_id)): Path<(String, String)>,
    Json(payload): Json<RejectJoinRequest>,
) -> Result<Json<CircuitResponse>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid circuit ID format"}))))?;

    let mut engine = state.circuits_engine.lock().unwrap();

    match engine.reject_join_request(&circuit_id, &requester_id, &payload.admin_id) {
        Ok(circuit) => Ok(Json(circuit_to_response(circuit))),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({"error": format!("Failed to reject join request: {}", e)})))),
    }
}

async fn update_circuit(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateCircuitRequest>,
) -> Result<Json<CircuitResponse>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid circuit ID format"}))))?;

    let mut engine = state.circuits_engine.lock().unwrap();

    // Get current circuit to preserve existing values
    let current_circuit = engine.get_circuit(&circuit_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to get circuit: {}", e)}))))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(json!({"error": "Circuit not found"}))))?;

    // Convert permissions if provided, preserving existing values for unspecified fields
    let permissions = if let Some(update_perms) = payload.permissions {
        Some(CircuitPermissions {
            default_push: update_perms.default_push.unwrap_or(current_circuit.permissions.default_push),
            default_pull: update_perms.default_pull.unwrap_or(current_circuit.permissions.default_pull),
            require_approval_for_push: update_perms.require_approval_for_push.unwrap_or(current_circuit.permissions.require_approval_for_push),
            require_approval_for_pull: update_perms.require_approval_for_pull.unwrap_or(current_circuit.permissions.require_approval_for_pull),
            allow_public_visibility: update_perms.allow_public_visibility.unwrap_or(current_circuit.permissions.allow_public_visibility),
        })
    } else {
        None
    };

    match engine.update_circuit(&circuit_id, payload.name, payload.description, permissions, &payload.requester_id) {
        Ok(circuit) => Ok(Json(circuit_to_response(circuit))),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({"error": format!("Failed to update circuit: {}", e)})))),
    }
}

async fn create_custom_role(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(payload): Json<CreateCustomRoleRequest>,
) -> Result<Json<CustomRoleResponse>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid circuit ID format"}))))?;

    // Parse permissions
    let permissions: Result<Vec<Permission>, String> = payload.permissions
        .into_iter()
        .map(|p| parse_permission(&p))
        .collect();

    let permissions = permissions
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error": e}))))?;

    let mut engine = state.circuits_engine.lock().unwrap();

    match engine.create_custom_role(&circuit_id, payload.role_name, permissions, payload.description, payload.color, &payload.requester_id) {
        Ok(custom_role) => {
            let role_counts = engine.get_circuit(&circuit_id)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to get circuit: {}", e)}))))?
                .ok_or_else(|| (StatusCode::NOT_FOUND, Json(json!({"error": "Circuit not found"}))))?
                .get_member_count_by_role();

            let member_count = role_counts.get(&custom_role.role_name).copied().unwrap_or(0);
            Ok(Json(custom_role_to_response(custom_role, member_count)))
        },
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({"error": format!("Failed to create custom role: {}", e)})))),
    }
}

async fn get_custom_roles(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<CustomRoleResponse>>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid circuit ID format"}))))?;

    let engine = state.circuits_engine.lock().unwrap();

    match engine.get_custom_roles(&circuit_id) {
        Ok(roles) => {
            let circuit = engine.get_circuit(&circuit_id)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to get circuit: {}", e)}))))?
                .ok_or_else(|| (StatusCode::NOT_FOUND, Json(json!({"error": "Circuit not found"}))))?;

            let role_counts = circuit.get_member_count_by_role();

            let response: Vec<CustomRoleResponse> = roles
                .into_iter()
                .map(|role| {
                    let member_count = role_counts.get(&role.role_name).copied().unwrap_or(0);
                    custom_role_to_response(role, member_count)
                })
                .collect();

            Ok(Json(response))
        },
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to get custom roles: {}", e)})))),
    }
}

async fn assign_member_role(
    State(state): State<Arc<AppState>>,
    Path((id, user_id)): Path<(String, String)>,
    Json(payload): Json<AssignRoleRequest>,
) -> Result<Json<CircuitResponse>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid circuit ID format"}))))?;

    let mut engine = state.circuits_engine.lock().unwrap();

    // If a custom role is specified, assign it
    let role_name = payload.custom_role_name.unwrap_or(payload.role);

    match engine.assign_member_custom_role(&circuit_id, &user_id, &role_name, &payload.requester_id) {
        Ok(circuit) => Ok(Json(circuit_to_response(circuit))),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({"error": format!("Failed to assign role: {}", e)})))),
    }
}

async fn update_public_settings(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(payload): Json<UpdatePublicSettingsRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid circuit ID format"}))))?;

    // Parse access mode
    let access_mode = match payload.public_settings.access_mode.to_lowercase().as_str() {
        "public" => crate::types::PublicAccessMode::Public,
        "protected" => crate::types::PublicAccessMode::Protected,
        "scheduled" => crate::types::PublicAccessMode::Scheduled,
        _ => return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid access mode"})))),
    };

    // Parse scheduled date if provided
    let scheduled_date = if let Some(date_str) = payload.public_settings.scheduled_date {
        Some(chrono::DateTime::parse_from_rfc3339(&date_str)
            .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid scheduled date format"}))))?
            .with_timezone(&chrono::Utc))
    } else {
        None
    };

    // Parse export permissions
    let export_permissions = if let Some(export_str) = payload.public_settings.export_permissions {
        match export_str.to_lowercase().as_str() {
            "admin" => Some(crate::types::ExportPermissionLevel::Admin),
            "members" => Some(crate::types::ExportPermissionLevel::Members),
            "public" => Some(crate::types::ExportPermissionLevel::Public),
            _ => return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid export permission level"})))),
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
        published_items: payload.public_settings.published_items.unwrap_or_default(),
        auto_approve_members: payload.public_settings.auto_approve_members.unwrap_or(false),
        auto_publish_pushed_items: payload.public_settings.auto_publish_pushed_items.unwrap_or(false),
        show_encrypted_events: payload.public_settings.show_encrypted_events.unwrap_or(false),
        required_event_types: payload.public_settings.required_event_types,
        data_quality_rules: payload.public_settings.data_quality_rules,
        export_permissions,
    };

    let mut engine = state.circuits_engine.lock().unwrap();
    match engine.update_public_settings(&circuit_id, public_settings, &payload.requester_id) {
        Ok(circuit) => Ok(Json(json!({
            "success": true,
            "data": circuit_to_response(circuit)
        }))),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({"error": format!("Failed to update public settings: {}", e)})))),
    }
}

async fn get_public_circuit(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid circuit ID format"}))))?;

    let engine = state.circuits_engine.lock().unwrap();
    match engine.get_public_circuit_info(&circuit_id) {
        Ok(Some(public_info)) => Ok(Json(json!({
            "success": true,
            "data": {
                "circuit_id": public_info.circuit_id.to_string(),
                "public_name": public_info.public_name,
                "public_description": public_info.public_description,
                "primary_color": public_info.primary_color,
                "secondary_color": public_info.secondary_color,
                "member_count": public_info.member_count,
                "access_mode": format!("{:?}", public_info.access_mode).to_lowercase(),
                "requires_password": public_info.requires_password,
                "is_currently_accessible": public_info.is_currently_accessible,
                "published_items": public_info.published_items,
                "auto_publish_pushed_items": public_info.auto_publish_pushed_items,
                "recent_activity": []
            }
        }))),
        Ok(None) => Err((StatusCode::NOT_FOUND, Json(json!({"error": "Circuit is not publicly accessible"})))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to get public circuit info: {}", e)})))),
    }
}

async fn join_public_circuit(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(payload): Json<JoinPublicCircuitRequest>,
) -> Result<Json<PublicJoinResponse>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid circuit ID format"}))))?;

    let mut engine = state.circuits_engine.lock().unwrap();
    match engine.join_public_circuit(&circuit_id, &payload.requester_id, payload.access_password, payload.message) {
        Ok((requires_approval, message)) => Ok(Json(PublicJoinResponse {
            success: true,
            message,
            requires_approval,
        })),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({"error": format!("Failed to join circuit: {}", e)})))),
    }
}

async fn get_circuit_activities(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<ActivityResponse>>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid circuit ID format"}))))?;

    let engine = state.circuits_engine.lock().unwrap();

    match engine.get_activities_for_circuit(&circuit_id) {
        Ok(activities) => {
            let response: Vec<ActivityResponse> = activities
                .into_iter()
                .map(activity_to_response)
                .collect();
            Ok(Json(response))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to get activities: {}", e)})))),
    }
}

async fn get_circuit_items(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<CircuitItemResponse>>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid circuit ID format"}))))?;

    let engine = state.circuits_engine.lock().unwrap();

    match engine.get_circuit_items(&circuit_id) {
        Ok(items) => {
            let response: Vec<CircuitItemResponse> = items
                .into_iter()
                .map(circuit_item_to_response)
                .collect();
            Ok(Json(response))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to get circuit items: {}", e)})))),
    }
}

async fn batch_push_items(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(payload): Json<BatchPushRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid circuit ID format"}))))?;

    let mut engine = state.circuits_engine.lock().unwrap();

    match engine.batch_push_items(&payload.dfids, &circuit_id, &payload.requester_id, payload.permissions) {
        Ok(result) => {
            Ok(Json(json!({
                "success_count": result.success_count,
                "failed_count": result.failed_count,
                "results": result.results
            })))
        }
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({"error": format!("Failed to batch push items: {}", e)})))),
    }
}
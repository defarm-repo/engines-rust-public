use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::{CircuitsEngine, InMemoryStorage, MemberRole, Circuit, CircuitOperation};

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
}

#[derive(Debug, Serialize)]
pub struct CircuitMemberResponse {
    pub member_id: String,
    pub role: String,
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

pub fn circuit_routes() -> Router {
    let state = Arc::new(CircuitState::new());

    Router::new()
        .route("/", post(create_circuit))
        .route("/:id", get(get_circuit))
        .route("/:id/members", post(add_member))
        .route("/:id/push/:dfid", post(push_item))
        .route("/:id/pull/:dfid", post(pull_item))
        .route("/:id/operations", get(get_circuit_operations))
        .route("/:id/operations/pending", get(get_pending_operations))
        .route("/operations/:operation_id/approve", post(approve_operation))
        .route("/:id/deactivate", put(deactivate_circuit))
        .route("/list", get(list_circuits))
        .route("/member/:member_id", get(get_circuits_for_member))
        .with_state(state)
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

fn circuit_to_response(circuit: Circuit) -> CircuitResponse {
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

async fn create_circuit(
    State(state): State<Arc<CircuitState>>,
    Json(payload): Json<CreateCircuitRequest>,
) -> Result<Json<CircuitResponse>, (StatusCode, Json<Value>)> {
    let mut engine = state.engine.lock().unwrap();

    match engine.create_circuit(payload.name, payload.description, payload.owner_id) {
        Ok(circuit) => Ok(Json(circuit_to_response(circuit))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to create circuit: {}", e)})))),
    }
}

async fn get_circuit(
    State(state): State<Arc<CircuitState>>,
    Path(id): Path<String>,
) -> Result<Json<CircuitResponse>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid circuit ID format"}))))?;

    let engine = state.engine.lock().unwrap();

    match engine.get_circuit(&circuit_id) {
        Ok(Some(circuit)) => Ok(Json(circuit_to_response(circuit))),
        Ok(None) => Err((StatusCode::NOT_FOUND, Json(json!({"error": "Circuit not found"})))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to get circuit: {}", e)})))),
    }
}

async fn add_member(
    State(state): State<Arc<CircuitState>>,
    Path(id): Path<String>,
    Json(payload): Json<AddMemberRequest>,
) -> Result<Json<CircuitResponse>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid circuit ID format"}))))?;

    let role = parse_member_role(&payload.role)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error": e}))))?;

    let mut engine = state.engine.lock().unwrap();

    match engine.add_member_to_circuit(&circuit_id, payload.member_id, role, &payload.requester_id) {
        Ok(circuit) => Ok(Json(circuit_to_response(circuit))),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({"error": format!("Failed to add member: {}", e)})))),
    }
}

async fn push_item(
    State(state): State<Arc<CircuitState>>,
    Path((id, dfid)): Path<(String, String)>,
    Json(payload): Json<CircuitOperationRequest>,
) -> Result<Json<CircuitOperationResponse>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid circuit ID format"}))))?;

    let mut engine = state.engine.lock().unwrap();

    match engine.push_item_to_circuit(&dfid, &circuit_id, &payload.requester_id) {
        Ok(operation) => Ok(Json(operation_to_response(operation))),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({"error": format!("Failed to push item: {}", e)})))),
    }
}

async fn pull_item(
    State(state): State<Arc<CircuitState>>,
    Path((id, dfid)): Path<(String, String)>,
    Json(payload): Json<CircuitOperationRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid circuit ID format"}))))?;

    let mut engine = state.engine.lock().unwrap();

    match engine.pull_item_from_circuit(&dfid, &circuit_id, &payload.requester_id) {
        Ok((item, operation)) => {
            Ok(Json(json!({
                "item": {
                    "dfid": item.dfid,
                    "canonical_identifiers": item.canonical_identifiers,
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
    State(state): State<Arc<CircuitState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<CircuitOperationResponse>>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid circuit ID format"}))))?;

    let engine = state.engine.lock().unwrap();

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
    State(state): State<Arc<CircuitState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<CircuitOperationResponse>>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid circuit ID format"}))))?;

    let engine = state.engine.lock().unwrap();

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
    State(state): State<Arc<CircuitState>>,
    Path(operation_id): Path<String>,
    Json(payload): Json<ApproveOperationRequest>,
) -> Result<Json<CircuitOperationResponse>, (StatusCode, Json<Value>)> {
    let operation_uuid = Uuid::parse_str(&operation_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid operation ID format"}))))?;

    let mut engine = state.engine.lock().unwrap();

    match engine.approve_operation(&operation_uuid, &payload.approver_id) {
        Ok(operation) => Ok(Json(operation_to_response(operation))),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({"error": format!("Failed to approve operation: {}", e)})))),
    }
}

async fn deactivate_circuit(
    State(state): State<Arc<CircuitState>>,
    Path(id): Path<String>,
    Json(payload): Json<CircuitOperationRequest>,
) -> Result<Json<CircuitResponse>, (StatusCode, Json<Value>)> {
    let circuit_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid circuit ID format"}))))?;

    let mut engine = state.engine.lock().unwrap();

    match engine.deactivate_circuit(&circuit_id, &payload.requester_id) {
        Ok(circuit) => Ok(Json(circuit_to_response(circuit))),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({"error": format!("Failed to deactivate circuit: {}", e)})))),
    }
}

async fn list_circuits(
    State(state): State<Arc<CircuitState>>,
) -> Result<Json<Vec<CircuitResponse>>, (StatusCode, Json<Value>)> {
    let engine = state.engine.lock().unwrap();

    match engine.list_circuits() {
        Ok(circuits) => {
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
    State(state): State<Arc<CircuitState>>,
    Path(member_id): Path<String>,
) -> Result<Json<Vec<CircuitResponse>>, (StatusCode, Json<Value>)> {
    let engine = state.engine.lock().unwrap();

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
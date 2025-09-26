use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put, delete},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use chrono::Utc;

#[derive(Debug, Deserialize)]
pub struct CreateWorkspaceRequest {
    pub name: String,
    pub description: String,
    pub owner_id: String,
    pub settings: Option<WorkspaceSettingsRequest>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWorkspaceRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub settings: Option<WorkspaceSettingsRequest>,
}

#[derive(Debug, Deserialize)]
pub struct WorkspaceSettingsRequest {
    pub default_circuit_permissions: Option<HashMap<String, bool>>,
    pub default_event_visibility: Option<String>,
    pub encryption_enabled: Option<bool>,
    pub retention_policy_days: Option<u32>,
    pub max_members: Option<u32>,
    pub allow_public_circuits: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct AddMemberRequest {
    pub user_id: String,
    pub role: String,
    pub requester_id: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMemberRequest {
    pub role: String,
    pub requester_id: String,
}

#[derive(Debug, Serialize)]
pub struct WorkspaceResponse {
    pub workspace_id: String,
    pub name: String,
    pub description: String,
    pub owner_id: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub settings: WorkspaceSettingsResponse,
    pub members: Vec<WorkspaceMemberResponse>,
    pub stats: WorkspaceStatsResponse,
}

#[derive(Debug, Serialize)]
pub struct WorkspaceSettingsResponse {
    pub default_circuit_permissions: HashMap<String, bool>,
    pub default_event_visibility: String,
    pub encryption_enabled: bool,
    pub retention_policy_days: u32,
    pub max_members: u32,
    pub allow_public_circuits: bool,
}

#[derive(Debug, Serialize)]
pub struct WorkspaceMemberResponse {
    pub user_id: String,
    pub role: String,
    pub joined_at: i64,
    pub last_active: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct WorkspaceStatsResponse {
    pub total_members: u32,
    pub total_circuits: u32,
    pub total_items: u32,
    pub total_events: u32,
    pub storage_used_mb: f64,
}

#[derive(Debug, Clone)]
pub struct Workspace {
    pub workspace_id: Uuid,
    pub name: String,
    pub description: String,
    pub owner_id: String,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
    pub settings: WorkspaceSettings,
    pub members: Vec<WorkspaceMember>,
}

#[derive(Debug, Clone)]
pub struct WorkspaceSettings {
    pub default_circuit_permissions: HashMap<String, bool>,
    pub default_event_visibility: String,
    pub encryption_enabled: bool,
    pub retention_policy_days: u32,
    pub max_members: u32,
    pub allow_public_circuits: bool,
}

#[derive(Debug, Clone)]
pub struct WorkspaceMember {
    pub user_id: String,
    pub role: String,
    pub joined_at: chrono::DateTime<Utc>,
    pub last_active: Option<chrono::DateTime<Utc>>,
}

impl Default for WorkspaceSettings {
    fn default() -> Self {
        let mut default_permissions = HashMap::new();
        default_permissions.insert("default_push".to_string(), true);
        default_permissions.insert("default_pull".to_string(), true);
        default_permissions.insert("require_approval_for_push".to_string(), false);
        default_permissions.insert("require_approval_for_pull".to_string(), false);

        Self {
            default_circuit_permissions: default_permissions,
            default_event_visibility: "Private".to_string(),
            encryption_enabled: true,
            retention_policy_days: 365,
            max_members: 100,
            allow_public_circuits: false,
        }
    }
}

pub struct WorkspaceState {
    pub workspaces: Arc<Mutex<HashMap<Uuid, Workspace>>>,
}

impl WorkspaceState {
    pub fn new() -> Self {
        Self {
            workspaces: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

pub fn workspace_routes() -> Router {
    let state = Arc::new(WorkspaceState::new());

    Router::new()
        .route("/", post(create_workspace))
        .route("/", get(list_workspaces))
        .route("/:workspace_id", get(get_workspace))
        .route("/:workspace_id", put(update_workspace))
        .route("/:workspace_id", delete(delete_workspace))
        .route("/:workspace_id/members", post(add_member))
        .route("/:workspace_id/members", get(list_members))
        .route("/:workspace_id/members/:user_id", put(update_member))
        .route("/:workspace_id/members/:user_id", delete(remove_member))
        .route("/:workspace_id/stats", get(get_workspace_stats))
        .route("/user/:user_id", get(get_workspaces_for_user))
        .with_state(state)
}

fn workspace_to_response(workspace: Workspace) -> WorkspaceResponse {
    WorkspaceResponse {
        workspace_id: workspace.workspace_id.to_string(),
        name: workspace.name,
        description: workspace.description,
        owner_id: workspace.owner_id,
        created_at: workspace.created_at.timestamp(),
        updated_at: workspace.updated_at.timestamp(),
        settings: WorkspaceSettingsResponse {
            default_circuit_permissions: workspace.settings.default_circuit_permissions,
            default_event_visibility: workspace.settings.default_event_visibility,
            encryption_enabled: workspace.settings.encryption_enabled,
            retention_policy_days: workspace.settings.retention_policy_days,
            max_members: workspace.settings.max_members,
            allow_public_circuits: workspace.settings.allow_public_circuits,
        },
        members: workspace.members.clone()
            .into_iter()
            .map(|member| WorkspaceMemberResponse {
                user_id: member.user_id,
                role: member.role,
                joined_at: member.joined_at.timestamp(),
                last_active: member.last_active.map(|t| t.timestamp()),
            })
            .collect(),
        stats: WorkspaceStatsResponse {
            total_members: workspace.members.len() as u32,
            total_circuits: 0, // Would be calculated from circuits engine
            total_items: 0,    // Would be calculated from items engine
            total_events: 0,   // Would be calculated from events engine
            storage_used_mb: 0.0, // Would be calculated from storage
        },
    }
}

async fn create_workspace(
    State(state): State<Arc<WorkspaceState>>,
    Json(payload): Json<CreateWorkspaceRequest>,
) -> Result<Json<WorkspaceResponse>, (StatusCode, Json<Value>)> {
    let workspace_id = Uuid::new_v4();
    let now = Utc::now();

    let settings = if let Some(settings_req) = payload.settings {
        let mut settings = WorkspaceSettings::default();

        if let Some(permissions) = settings_req.default_circuit_permissions {
            settings.default_circuit_permissions = permissions;
        }
        if let Some(visibility) = settings_req.default_event_visibility {
            settings.default_event_visibility = visibility;
        }
        if let Some(encryption) = settings_req.encryption_enabled {
            settings.encryption_enabled = encryption;
        }
        if let Some(retention) = settings_req.retention_policy_days {
            settings.retention_policy_days = retention;
        }
        if let Some(max_members) = settings_req.max_members {
            settings.max_members = max_members;
        }
        if let Some(allow_public) = settings_req.allow_public_circuits {
            settings.allow_public_circuits = allow_public;
        }

        settings
    } else {
        WorkspaceSettings::default()
    };

    let owner_member = WorkspaceMember {
        user_id: payload.owner_id.clone(),
        role: "Owner".to_string(),
        joined_at: now,
        last_active: Some(now),
    };

    let workspace = Workspace {
        workspace_id,
        name: payload.name,
        description: payload.description,
        owner_id: payload.owner_id,
        created_at: now,
        updated_at: now,
        settings,
        members: vec![owner_member],
    };

    let mut workspaces = state.workspaces.lock().unwrap();
    workspaces.insert(workspace_id, workspace.clone());

    Ok(Json(workspace_to_response(workspace)))
}

async fn get_workspace(
    State(state): State<Arc<WorkspaceState>>,
    Path(workspace_id): Path<String>,
) -> Result<Json<WorkspaceResponse>, (StatusCode, Json<Value>)> {
    let workspace_uuid = Uuid::parse_str(&workspace_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid workspace ID format"}))))?;

    let workspaces = state.workspaces.lock().unwrap();

    match workspaces.get(&workspace_uuid) {
        Some(workspace) => Ok(Json(workspace_to_response(workspace.clone()))),
        None => Err((StatusCode::NOT_FOUND, Json(json!({"error": "Workspace not found"})))),
    }
}

async fn update_workspace(
    State(state): State<Arc<WorkspaceState>>,
    Path(workspace_id): Path<String>,
    Json(payload): Json<UpdateWorkspaceRequest>,
) -> Result<Json<WorkspaceResponse>, (StatusCode, Json<Value>)> {
    let workspace_uuid = Uuid::parse_str(&workspace_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid workspace ID format"}))))?;

    let mut workspaces = state.workspaces.lock().unwrap();

    match workspaces.get_mut(&workspace_uuid) {
        Some(workspace) => {
            if let Some(name) = payload.name {
                workspace.name = name;
            }
            if let Some(description) = payload.description {
                workspace.description = description;
            }
            if let Some(settings_req) = payload.settings {
                if let Some(permissions) = settings_req.default_circuit_permissions {
                    workspace.settings.default_circuit_permissions = permissions;
                }
                if let Some(visibility) = settings_req.default_event_visibility {
                    workspace.settings.default_event_visibility = visibility;
                }
                if let Some(encryption) = settings_req.encryption_enabled {
                    workspace.settings.encryption_enabled = encryption;
                }
                if let Some(retention) = settings_req.retention_policy_days {
                    workspace.settings.retention_policy_days = retention;
                }
                if let Some(max_members) = settings_req.max_members {
                    workspace.settings.max_members = max_members;
                }
                if let Some(allow_public) = settings_req.allow_public_circuits {
                    workspace.settings.allow_public_circuits = allow_public;
                }
            }
            workspace.updated_at = Utc::now();

            Ok(Json(workspace_to_response(workspace.clone())))
        }
        None => Err((StatusCode::NOT_FOUND, Json(json!({"error": "Workspace not found"})))),
    }
}

async fn delete_workspace(
    State(state): State<Arc<WorkspaceState>>,
    Path(workspace_id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let workspace_uuid = Uuid::parse_str(&workspace_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid workspace ID format"}))))?;

    let mut workspaces = state.workspaces.lock().unwrap();

    match workspaces.remove(&workspace_uuid) {
        Some(_) => Ok(Json(json!({"message": "Workspace deleted successfully"}))),
        None => Err((StatusCode::NOT_FOUND, Json(json!({"error": "Workspace not found"})))),
    }
}

async fn list_workspaces(
    State(state): State<Arc<WorkspaceState>>,
) -> Result<Json<Vec<WorkspaceResponse>>, (StatusCode, Json<Value>)> {
    let workspaces = state.workspaces.lock().unwrap();

    let response: Vec<WorkspaceResponse> = workspaces
        .values()
        .map(|workspace| workspace_to_response(workspace.clone()))
        .collect();

    Ok(Json(response))
}

async fn add_member(
    State(state): State<Arc<WorkspaceState>>,
    Path(workspace_id): Path<String>,
    Json(payload): Json<AddMemberRequest>,
) -> Result<Json<WorkspaceResponse>, (StatusCode, Json<Value>)> {
    let workspace_uuid = Uuid::parse_str(&workspace_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid workspace ID format"}))))?;

    let mut workspaces = state.workspaces.lock().unwrap();

    match workspaces.get_mut(&workspace_uuid) {
        Some(workspace) => {
            // Check if user is already a member
            if workspace.members.iter().any(|m| m.user_id == payload.user_id) {
                return Err((StatusCode::CONFLICT, Json(json!({"error": "User is already a member"}))));
            }

            // Check max members limit
            if workspace.members.len() >= workspace.settings.max_members as usize {
                return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "Workspace has reached maximum member limit"}))));
            }

            // Validate role
            if !["Owner", "Admin", "Member", "Viewer"].contains(&payload.role.as_str()) {
                return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid role"}))));
            }

            let new_member = WorkspaceMember {
                user_id: payload.user_id,
                role: payload.role,
                joined_at: Utc::now(),
                last_active: None,
            };

            workspace.members.push(new_member);
            workspace.updated_at = Utc::now();

            Ok(Json(workspace_to_response(workspace.clone())))
        }
        None => Err((StatusCode::NOT_FOUND, Json(json!({"error": "Workspace not found"})))),
    }
}

async fn list_members(
    State(state): State<Arc<WorkspaceState>>,
    Path(workspace_id): Path<String>,
) -> Result<Json<Vec<WorkspaceMemberResponse>>, (StatusCode, Json<Value>)> {
    let workspace_uuid = Uuid::parse_str(&workspace_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid workspace ID format"}))))?;

    let workspaces = state.workspaces.lock().unwrap();

    match workspaces.get(&workspace_uuid) {
        Some(workspace) => {
            let response: Vec<WorkspaceMemberResponse> = workspace.members
                .iter()
                .map(|member| WorkspaceMemberResponse {
                    user_id: member.user_id.clone(),
                    role: member.role.clone(),
                    joined_at: member.joined_at.timestamp(),
                    last_active: member.last_active.map(|t| t.timestamp()),
                })
                .collect();

            Ok(Json(response))
        }
        None => Err((StatusCode::NOT_FOUND, Json(json!({"error": "Workspace not found"})))),
    }
}

async fn update_member(
    State(state): State<Arc<WorkspaceState>>,
    Path((workspace_id, user_id)): Path<(String, String)>,
    Json(payload): Json<UpdateMemberRequest>,
) -> Result<Json<WorkspaceMemberResponse>, (StatusCode, Json<Value>)> {
    let workspace_uuid = Uuid::parse_str(&workspace_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid workspace ID format"}))))?;

    let mut workspaces = state.workspaces.lock().unwrap();

    match workspaces.get_mut(&workspace_uuid) {
        Some(workspace) => {
            // Validate role
            if !["Owner", "Admin", "Member", "Viewer"].contains(&payload.role.as_str()) {
                return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid role"}))));
            }

            if let Some(member) = workspace.members.iter_mut().find(|m| m.user_id == user_id) {
                member.role = payload.role;
                workspace.updated_at = Utc::now();

                Ok(Json(WorkspaceMemberResponse {
                    user_id: member.user_id.clone(),
                    role: member.role.clone(),
                    joined_at: member.joined_at.timestamp(),
                    last_active: member.last_active.map(|t| t.timestamp()),
                }))
            } else {
                Err((StatusCode::NOT_FOUND, Json(json!({"error": "Member not found"}))))
            }
        }
        None => Err((StatusCode::NOT_FOUND, Json(json!({"error": "Workspace not found"})))),
    }
}

async fn remove_member(
    State(state): State<Arc<WorkspaceState>>,
    Path((workspace_id, user_id)): Path<(String, String)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let workspace_uuid = Uuid::parse_str(&workspace_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid workspace ID format"}))))?;

    let mut workspaces = state.workspaces.lock().unwrap();

    match workspaces.get_mut(&workspace_uuid) {
        Some(workspace) => {
            let initial_len = workspace.members.len();
            workspace.members.retain(|m| m.user_id != user_id);

            if workspace.members.len() < initial_len {
                workspace.updated_at = Utc::now();
                Ok(Json(json!({"message": "Member removed successfully"})))
            } else {
                Err((StatusCode::NOT_FOUND, Json(json!({"error": "Member not found"}))))
            }
        }
        None => Err((StatusCode::NOT_FOUND, Json(json!({"error": "Workspace not found"})))),
    }
}

async fn get_workspace_stats(
    State(state): State<Arc<WorkspaceState>>,
    Path(workspace_id): Path<String>,
) -> Result<Json<WorkspaceStatsResponse>, (StatusCode, Json<Value>)> {
    let workspace_uuid = Uuid::parse_str(&workspace_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid workspace ID format"}))))?;

    let workspaces = state.workspaces.lock().unwrap();

    match workspaces.get(&workspace_uuid) {
        Some(workspace) => {
            Ok(Json(WorkspaceStatsResponse {
                total_members: workspace.members.len() as u32,
                total_circuits: 0, // Would integrate with CircuitsEngine
                total_items: 0,    // Would integrate with ItemsEngine
                total_events: 0,   // Would integrate with EventsEngine
                storage_used_mb: 0.0, // Would calculate from storage
            }))
        }
        None => Err((StatusCode::NOT_FOUND, Json(json!({"error": "Workspace not found"})))),
    }
}

async fn get_workspaces_for_user(
    State(state): State<Arc<WorkspaceState>>,
    Path(user_id): Path<String>,
) -> Result<Json<Vec<WorkspaceResponse>>, (StatusCode, Json<Value>)> {
    let workspaces = state.workspaces.lock().unwrap();

    let response: Vec<WorkspaceResponse> = workspaces
        .values()
        .filter(|workspace| workspace.members.iter().any(|m| m.user_id == user_id))
        .map(|workspace| workspace_to_response(workspace.clone()))
        .collect();

    Ok(Json(response))
}
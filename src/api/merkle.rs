//! Merkle State Tree API Endpoints
//!
//! REST API for Merkle tree operations including:
//! - Root hash computation for items and circuits
//! - Proof generation and verification
//! - Sync comparison between users

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

use crate::api::auth::Claims;
use crate::api::shared_state::AppState;
use crate::api_key_middleware::ApiKeyContext;
use crate::merkle_engine::MerkleEngine;
use crate::merkle_tree::{ItemMerkleEntry, MerkleError, MerkleProof, MerkleTree};
use crate::postgres_storage_with_cache::PostgresStorageWithCache;
use std::sync::Mutex;

// Type alias matching SharedStorage from shared_state
type SharedStorage = Arc<Mutex<PostgresStorageWithCache>>;

// ============================================================================
// REQUEST/RESPONSE TYPES
// ============================================================================

#[derive(Debug, Serialize)]
pub struct MerkleRootResponse {
    pub success: bool,
    pub data: Value,
}

#[derive(Debug, Deserialize)]
pub struct SyncCheckRequest {
    /// Local Merkle root to compare
    pub local_root: String,
    /// Remote Merkle root to compare
    pub remote_root: String,
    /// Optional: Remote items for detailed diff
    pub remote_items: Option<Vec<ItemMerkleEntry>>,
}

#[derive(Debug, Serialize)]
pub struct SyncCheckResponse {
    pub in_sync: bool,
    pub local_root: String,
    pub remote_root: String,
    pub message: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub differing_items: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub local_only: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub remote_only: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub modified: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct VerifyProofRequest {
    /// The proof to verify
    pub proof: MerkleProof,
    /// Expected root hash
    pub expected_root: String,
}

#[derive(Debug, Serialize)]
pub struct VerifyProofResponse {
    pub valid: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ProofResponse {
    pub success: bool,
    pub proof: MerkleProof,
}

// ============================================================================
// ROUTER SETUP
// ============================================================================

/// Create Merkle routes (requires authentication)
pub fn merkle_routes() -> Router<Arc<AppState>> {
    Router::new()
        // Root hashes
        .route("/items/:dfid/merkle-root", get(get_item_merkle_root))
        .route(
            "/circuits/:circuit_id/merkle-root",
            get(get_circuit_merkle_root),
        )
        // Proofs
        .route("/items/:dfid/merkle-proof/:event_id", get(get_event_proof))
        .route(
            "/circuits/:circuit_id/merkle-proof/:dfid",
            get(get_item_proof),
        )
        // Verification
        .route("/verify-proof", post(verify_merkle_proof))
        // Sync comparison
        .route("/circuits/:circuit_id/sync-check", post(check_sync_status))
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Extract user_id from either JWT claims or API key context
fn extract_user_id(
    claims: &Option<Extension<Claims>>,
    api_key_ctx: &Option<Extension<ApiKeyContext>>,
) -> Result<String, (StatusCode, Json<Value>)> {
    if let Some(Extension(c)) = claims {
        return Ok(c.user_id.clone());
    }
    if let Some(Extension(ctx)) = api_key_ctx {
        return Ok(ctx.user_id.to_string());
    }
    Err((
        StatusCode::UNAUTHORIZED,
        Json(json!({"error": "Authentication required"})),
    ))
}

/// Create MerkleEngine from storage
fn create_merkle_engine(state: &AppState) -> MerkleEngine<SharedStorage> {
    MerkleEngine::new(state.shared_storage.clone())
}

// ============================================================================
// HANDLERS
// ============================================================================

/// GET /api/merkle/items/:dfid/merkle-root
/// Get the Merkle root hash for an item's events
async fn get_item_merkle_root(
    State(state): State<Arc<AppState>>,
    claims: Option<Extension<Claims>>,
    api_key_ctx: Option<Extension<ApiKeyContext>>,
    Path(dfid): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let _user_id = extract_user_id(&claims, &api_key_ctx)?;

    let engine = create_merkle_engine(&state);

    match engine.get_item_root(&dfid) {
        Ok(response) => Ok(Json(json!({
            "success": true,
            "data": {
                "dfid": response.dfid,
                "merkle_root": response.merkle_root,
                "event_count": response.event_count,
                "computed_at": response.computed_at,
            }
        }))),
        Err(MerkleError::EmptyTree) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "No events found for item",
                "dfid": dfid
            })),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": format!("Failed to compute Merkle root: {}", e)
            })),
        )),
    }
}

/// GET /api/merkle/circuits/:circuit_id/merkle-root
/// Get the Merkle root hash for a circuit's items
async fn get_circuit_merkle_root(
    State(state): State<Arc<AppState>>,
    claims: Option<Extension<Claims>>,
    api_key_ctx: Option<Extension<ApiKeyContext>>,
    Path(circuit_id): Path<Uuid>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let _user_id = extract_user_id(&claims, &api_key_ctx)?;

    let engine = create_merkle_engine(&state);

    match engine.get_circuit_root(&circuit_id) {
        Ok(response) => Ok(Json(json!({
            "success": true,
            "data": {
                "circuit_id": response.circuit_id,
                "merkle_root": response.merkle_root,
                "item_count": response.item_count,
                "items": response.items,
                "computed_at": response.computed_at,
            }
        }))),
        Err(MerkleError::EmptyTree) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "No items with events found in circuit",
                "circuit_id": circuit_id.to_string()
            })),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": format!("Failed to compute Merkle root: {}", e)
            })),
        )),
    }
}

/// GET /api/merkle/items/:dfid/merkle-proof/:event_id
/// Generate a proof that an event exists in an item
async fn get_event_proof(
    State(state): State<Arc<AppState>>,
    claims: Option<Extension<Claims>>,
    api_key_ctx: Option<Extension<ApiKeyContext>>,
    Path((dfid, event_id)): Path<(String, Uuid)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let _user_id = extract_user_id(&claims, &api_key_ctx)?;

    let engine = create_merkle_engine(&state);

    match engine.prove_event_in_item(&dfid, &event_id) {
        Ok(proof) => Ok(Json(json!({
            "success": true,
            "proof": proof,
            "item_dfid": dfid,
            "event_id": event_id.to_string(),
        }))),
        Err(MerkleError::EmptyTree) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "No events found for item",
                "dfid": dfid
            })),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": format!("Failed to generate proof: {}", e)
            })),
        )),
    }
}

/// GET /api/merkle/circuits/:circuit_id/merkle-proof/:dfid
/// Generate a proof that an item exists in a circuit
async fn get_item_proof(
    State(state): State<Arc<AppState>>,
    claims: Option<Extension<Claims>>,
    api_key_ctx: Option<Extension<ApiKeyContext>>,
    Path((circuit_id, dfid)): Path<(Uuid, String)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let _user_id = extract_user_id(&claims, &api_key_ctx)?;

    let engine = create_merkle_engine(&state);

    match engine.prove_item_in_circuit(&circuit_id, &dfid) {
        Ok(proof) => Ok(Json(json!({
            "success": true,
            "proof": proof,
            "circuit_id": circuit_id.to_string(),
            "item_dfid": dfid,
        }))),
        Err(MerkleError::EmptyTree) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "No items with events found in circuit",
                "circuit_id": circuit_id.to_string()
            })),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": format!("Failed to generate proof: {}", e)
            })),
        )),
    }
}

/// POST /api/merkle/verify-proof
/// Verify a Merkle proof against an expected root
async fn verify_merkle_proof(
    State(_state): State<Arc<AppState>>,
    claims: Option<Extension<Claims>>,
    api_key_ctx: Option<Extension<ApiKeyContext>>,
    Json(request): Json<VerifyProofRequest>,
) -> Result<Json<VerifyProofResponse>, (StatusCode, Json<Value>)> {
    let _user_id = extract_user_id(&claims, &api_key_ctx)?;

    let valid = MerkleTree::verify_proof(&request.proof, &request.expected_root);

    Ok(Json(VerifyProofResponse {
        valid,
        message: if valid {
            "Proof is valid - leaf exists in tree with expected root".to_string()
        } else {
            "Proof is invalid - verification failed".to_string()
        },
    }))
}

/// POST /api/merkle/circuits/:circuit_id/sync-check
/// Compare local and remote circuit states
async fn check_sync_status(
    State(state): State<Arc<AppState>>,
    claims: Option<Extension<Claims>>,
    api_key_ctx: Option<Extension<ApiKeyContext>>,
    Path(circuit_id): Path<Uuid>,
    Json(request): Json<SyncCheckRequest>,
) -> Result<Json<SyncCheckResponse>, (StatusCode, Json<Value>)> {
    let _user_id = extract_user_id(&claims, &api_key_ctx)?;

    let engine = create_merkle_engine(&state);

    match engine.compare_circuit_states(
        &circuit_id,
        &request.local_root,
        &request.remote_root,
        request.remote_items.as_deref(),
    ) {
        Ok(comparison) => Ok(Json(SyncCheckResponse {
            in_sync: comparison.in_sync,
            local_root: comparison.local_root,
            remote_root: comparison.remote_root,
            message: if comparison.in_sync {
                "States are identical".to_string()
            } else {
                format!(
                    "States differ - {} items affected",
                    comparison.differing_items.len()
                )
            },
            differing_items: comparison.differing_items,
            local_only: comparison.local_only,
            remote_only: comparison.remote_only,
            modified: comparison.modified,
        })),
        Err(MerkleError::EmptyTree) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "No items with events found in circuit",
                "circuit_id": circuit_id.to_string()
            })),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": format!("Failed to compare states: {}", e)
            })),
        )),
    }
}

// ============================================================================
// PUBLIC MERKLE ROUTES (No authentication required)
// ============================================================================

/// Create public Merkle routes (no authentication required)
/// Only works for items in public circuits
pub fn public_merkle_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/items/:dfid/merkle-root", get(public_get_item_merkle_root))
        .route(
            "/items/:dfid/merkle-proof/:event_id",
            get(public_get_event_proof),
        )
        .route(
            "/circuits/:circuit_id/merkle-root",
            get(public_get_circuit_merkle_root),
        )
        .route(
            "/circuits/:circuit_id/merkle-proof/:dfid",
            get(public_get_item_proof),
        )
        .route("/verify-proof", post(public_verify_merkle_proof))
}

/// Check if a DFID is in a public circuit (async version)
async fn is_item_in_public_circuit_async(state: &AppState, dfid: &str) -> bool {
    let engine_guard = state.circuits_engine.read().await;

    if let Ok(circuits) = engine_guard.list_circuits() {
        for circuit in circuits {
            // Check if circuit is publicly accessible
            if !circuit.is_publicly_accessible() {
                continue;
            }

            // Check if this DFID is in the circuit's published items
            if let Some(settings) = &circuit.public_settings {
                if settings.published_items.contains(&dfid.to_string()) {
                    return true;
                }
            }
        }
    }
    false
}

/// Check if a circuit is public (async version)
async fn is_circuit_public_async(state: &AppState, circuit_id: &Uuid) -> bool {
    let engine_guard = state.circuits_engine.read().await;

    if let Ok(Some(circuit)) = engine_guard.get_circuit(circuit_id) {
        return circuit.is_publicly_accessible();
    }
    false
}

/// GET /api/public/merkle/items/:dfid/merkle-root
/// Public endpoint - Get Merkle root for items in public circuits
async fn public_get_item_merkle_root(
    State(state): State<Arc<AppState>>,
    Path(dfid): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Check if item is in a public circuit
    if !is_item_in_public_circuit_async(&state, &dfid).await {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "Item not found in any public circuit",
                "dfid": dfid
            })),
        ));
    }

    let engine = create_merkle_engine(&state);

    match engine.get_item_root(&dfid) {
        Ok(response) => Ok(Json(json!({
            "success": true,
            "data": {
                "dfid": response.dfid,
                "merkle_root": response.merkle_root,
                "event_count": response.event_count,
                "computed_at": response.computed_at,
            }
        }))),
        Err(MerkleError::EmptyTree) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "No events found for item",
                "dfid": dfid
            })),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": format!("Failed to compute Merkle root: {}", e)
            })),
        )),
    }
}

/// GET /api/public/merkle/items/:dfid/merkle-proof/:event_id
/// Public endpoint - Generate event proof for items in public circuits
async fn public_get_event_proof(
    State(state): State<Arc<AppState>>,
    Path((dfid, event_id)): Path<(String, Uuid)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Check if item is in a public circuit
    if !is_item_in_public_circuit_async(&state, &dfid).await {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "Item not found in any public circuit",
                "dfid": dfid
            })),
        ));
    }

    let engine = create_merkle_engine(&state);

    match engine.prove_event_in_item(&dfid, &event_id) {
        Ok(proof) => Ok(Json(json!({
            "success": true,
            "proof": proof,
            "item_dfid": dfid,
            "event_id": event_id.to_string(),
        }))),
        Err(MerkleError::EmptyTree) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "No events found for item",
                "dfid": dfid
            })),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": format!("Failed to generate proof: {}", e)
            })),
        )),
    }
}

/// GET /api/public/merkle/circuits/:circuit_id/merkle-root
/// Public endpoint - Get circuit Merkle root for public circuits
async fn public_get_circuit_merkle_root(
    State(state): State<Arc<AppState>>,
    Path(circuit_id): Path<Uuid>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Check if circuit is public
    if !is_circuit_public_async(&state, &circuit_id).await {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "Circuit not found or not public",
                "circuit_id": circuit_id.to_string()
            })),
        ));
    }

    let engine = create_merkle_engine(&state);

    match engine.get_circuit_root(&circuit_id) {
        Ok(response) => Ok(Json(json!({
            "success": true,
            "data": {
                "circuit_id": response.circuit_id,
                "merkle_root": response.merkle_root,
                "item_count": response.item_count,
                "items": response.items,
                "computed_at": response.computed_at,
            }
        }))),
        Err(MerkleError::EmptyTree) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "No items with events found in circuit",
                "circuit_id": circuit_id.to_string()
            })),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": format!("Failed to compute Merkle root: {}", e)
            })),
        )),
    }
}

/// GET /api/public/merkle/circuits/:circuit_id/merkle-proof/:dfid
/// Public endpoint - Generate item proof for public circuits
async fn public_get_item_proof(
    State(state): State<Arc<AppState>>,
    Path((circuit_id, dfid)): Path<(Uuid, String)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Check if circuit is public
    if !is_circuit_public_async(&state, &circuit_id).await {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "Circuit not found or not public",
                "circuit_id": circuit_id.to_string()
            })),
        ));
    }

    let engine = create_merkle_engine(&state);

    match engine.prove_item_in_circuit(&circuit_id, &dfid) {
        Ok(proof) => Ok(Json(json!({
            "success": true,
            "proof": proof,
            "circuit_id": circuit_id.to_string(),
            "item_dfid": dfid,
        }))),
        Err(MerkleError::EmptyTree) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "No items with events found in circuit",
                "circuit_id": circuit_id.to_string()
            })),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": format!("Failed to generate proof: {}", e)
            })),
        )),
    }
}

/// POST /api/public/merkle/verify-proof
/// Public endpoint - Verify any Merkle proof (no auth needed)
async fn public_verify_merkle_proof(
    Json(request): Json<VerifyProofRequest>,
) -> Json<VerifyProofResponse> {
    let valid = MerkleTree::verify_proof(&request.proof, &request.expected_root);

    Json(VerifyProofResponse {
        valid,
        message: if valid {
            "Proof is valid - leaf exists in tree with expected root".to_string()
        } else {
            "Proof is invalid - verification failed".to_string()
        },
    })
}

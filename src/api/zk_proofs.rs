use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post},
    Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::api::shared_state::AppState;
use crate::zk_proof_engine::{CircuitType, ProofStatus, ZkProof, ZkProofEngine};

// API Request/Response types
#[derive(Debug, Deserialize)]
pub struct SubmitProofRequest {
    pub circuit_type: CircuitType,
    pub circuit_input: HashMap<String, serde_json::Value>,
    pub private_inputs: HashMap<String, serde_json::Value>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
pub struct ZkProofQuery {
    pub prover_id: Option<String>,
    pub circuit_types: Option<Vec<CircuitType>>,
    pub statuses: Option<Vec<ProofStatus>>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub offset: Option<u32>,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct ZkProofStatistics {
    #[serde(with = "crate::safe_json_numbers::u64_safe")]
    pub total_proofs: u64,
    #[serde(with = "crate::safe_json_numbers::u64_safe")]
    pub pending_proofs: u64,
    #[serde(with = "crate::safe_json_numbers::u64_safe")]
    pub verified_proofs: u64,
    #[serde(with = "crate::safe_json_numbers::u64_safe")]
    pub failed_proofs: u64,
    pub proof_types: HashMap<String, u64>,
}

#[derive(Debug, Deserialize)]
pub struct VerifyProofRequest {
    pub proof_id: Uuid,
    pub verification_key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ZkProofQueryParams {
    pub user_id: Option<String>,
    pub circuit_type: Option<String>,
    pub status: Option<String>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub offset: Option<u32>,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct ZkProofResponse {
    pub proof_id: Uuid,
    pub user_id: String,
    pub circuit_type: CircuitType,
    pub status: ProofStatus,
    pub proof_data: Option<String>,
    pub verification_result: Option<bool>,
    pub error_message: Option<String>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<ZkProof> for ZkProofResponse {
    fn from(proof: ZkProof) -> Self {
        Self {
            proof_id: proof.proof_id,
            user_id: proof.prover_id,
            circuit_type: proof.circuit_type,
            status: proof.status,
            proof_data: Some(String::from_utf8_lossy(&proof.proof_data).to_string()),
            verification_result: proof.verification_result.map(|vr| vr.is_valid),
            error_message: None, // ZkProof doesn't have error_message field
            metadata: None,      // ZkProof doesn't have metadata field
            created_at: proof.created_at,
            updated_at: proof.created_at, // Use created_at as updated_at since ZkProof doesn't have updated_at
        }
    }
}

// Handler functions
async fn submit_proof(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<SubmitProofRequest>,
) -> Result<Json<Value>, StatusCode> {
    // Create ZK proof engine using shared storage
    let zk_engine = ZkProofEngine::new(Arc::clone(&app_state.shared_storage));

    // TODO: In a real implementation, you would extract user_id from authentication
    let user_id = "anonymous_user".to_string();

    match zk_engine.submit_proof(
        request.circuit_type,
        user_id,
        request.circuit_input,
        request.private_inputs,
        None,
    ) {
        Ok(proof_id) => Ok(Json(json!({
            "success": true,
            "proof_id": proof_id,
            "message": "Proof submitted successfully"
        }))),
        Err(e) => {
            app_state.logging.lock().unwrap().error(
                "api_zk_proofs",
                "submit_proof_error",
                format!("Error submitting proof: {e:?}"),
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn verify_proof(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<VerifyProofRequest>,
) -> Result<Json<Value>, StatusCode> {
    let zk_engine = ZkProofEngine::new(Arc::clone(&app_state.shared_storage));

    let verifier_id = "anonymous_verifier".to_string();

    match zk_engine.verify_proof(request.proof_id, verifier_id) {
        Ok(verification_result) => Ok(Json(json!({
            "success": true,
            "verification_result": verification_result
        }))),
        Err(e) => {
            app_state.logging.lock().unwrap().error(
                "api_zk_proofs",
                "verify_proof_error",
                format!("Error verifying proof: {e:?}"),
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn get_proof(
    State(app_state): State<Arc<AppState>>,
    Path(proof_id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    let zk_engine = ZkProofEngine::new(Arc::clone(&app_state.shared_storage));

    match zk_engine.get_proof(&proof_id) {
        Ok(Some(proof)) => {
            let response = ZkProofResponse::from(proof);
            Ok(Json(json!({
                "success": true,
                "proof": response
            })))
        }
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            app_state.logging.lock().unwrap().error(
                "api_zk_proofs",
                "get_proof_error",
                format!("Error getting proof: {e:?}"),
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn list_proofs(
    State(app_state): State<Arc<AppState>>,
    Query(params): Query<ZkProofQueryParams>,
) -> Result<Json<Value>, StatusCode> {
    let zk_engine = ZkProofEngine::new(Arc::clone(&app_state.shared_storage));

    // Convert query params to ZkProofQuery
    let mut circuit_types = None;
    if let Some(circuit_type_str) = params.circuit_type {
        let circuit_type = match circuit_type_str.as_str() {
            "organic_certification" => CircuitType::OrganicCertification,
            "pesticide_threshold" => CircuitType::PesticideThreshold,
            "quality_grade" => CircuitType::QualityGrade,
            "ownership_proof" => CircuitType::OwnershipProof,
            "timestamp_freshness" => CircuitType::TimestampFreshness,
            custom => CircuitType::Custom(custom.to_string()),
        };
        circuit_types = Some(vec![circuit_type]);
    }

    let mut statuses = None;
    if let Some(status_str) = params.status {
        let status = match status_str.as_str() {
            "pending" => ProofStatus::Pending,
            "verified" => ProofStatus::Verified,
            "failed" => ProofStatus::Failed,
            "expired" => ProofStatus::Expired,
            _ => return Err(StatusCode::BAD_REQUEST),
        };
        statuses = Some(vec![status]);
    }

    let query = ZkProofQuery {
        prover_id: params.user_id,
        circuit_types,
        statuses,
        start_date: params.start_date,
        end_date: params.end_date,
        offset: params.offset,
        limit: params.limit,
    };

    match zk_engine.query_proofs(&query) {
        Ok(proofs) => {
            let responses: Vec<ZkProofResponse> =
                proofs.into_iter().map(ZkProofResponse::from).collect();
            Ok(Json(json!({
                "success": true,
                "proofs": responses,
                "count": responses.len()
            })))
        }
        Err(e) => {
            app_state.logging.lock().unwrap().error(
                "api_zk_proofs",
                "list_proofs_error",
                format!("Error querying proofs: {e:?}"),
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn get_proof_statistics(
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<Value>, StatusCode> {
    let zk_engine = ZkProofEngine::new(Arc::clone(&app_state.shared_storage));

    match zk_engine.get_statistics() {
        Ok(stats) => Ok(Json(json!({
            "success": true,
            "statistics": stats
        }))),
        Err(e) => {
            app_state.logging.lock().unwrap().error(
                "api_zk_proofs",
                "get_stats_error",
                format!("Error getting proof statistics: {e:?}"),
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn delete_proof(
    State(app_state): State<Arc<AppState>>,
    Path(proof_id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    let zk_engine = ZkProofEngine::new(Arc::clone(&app_state.shared_storage));

    match zk_engine.delete_proof(&proof_id) {
        Ok(()) => Ok(Json(json!({
            "success": true,
            "message": "Proof deleted successfully"
        }))),
        Err(e) => {
            app_state.logging.lock().unwrap().error(
                "api_zk_proofs",
                "delete_proof_error",
                format!("Error deleting proof: {e:?}"),
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn get_circuit_templates(
    State(_app_state): State<Arc<AppState>>,
) -> Result<Json<Value>, StatusCode> {
    // Return available circuit templates
    let templates = json!({
        "organic_certification": {
            "description": "Proves organic certification status without revealing certificate details",
            "required_inputs": ["certificate_id", "certification_body", "expiry_date"],
            "private_inputs": ["certificate_hash", "verification_code"]
        },
        "pesticide_threshold": {
            "description": "Proves pesticide levels are within acceptable thresholds",
            "required_inputs": ["test_results", "threshold_limits", "testing_date"],
            "private_inputs": ["raw_test_data", "lab_signature"]
        },
        "quality_grade": {
            "description": "Proves product quality grade without revealing assessment details",
            "required_inputs": ["grade_level", "assessment_date", "standards_version"],
            "private_inputs": ["assessment_data", "inspector_signature"]
        },
        "ownership_proof": {
            "description": "Proves ownership of agricultural assets",
            "required_inputs": ["asset_id", "ownership_date", "jurisdiction"],
            "private_inputs": ["title_deed_hash", "registration_number"]
        },
        "timestamp_freshness": {
            "description": "Proves harvest/production timestamp for freshness verification",
            "required_inputs": ["harvest_date", "location", "product_type"],
            "private_inputs": ["timestamp_signature", "gps_coordinates"]
        }
    });

    Ok(Json(json!({
        "success": true,
        "templates": templates
    })))
}

// Router function
pub fn zk_proof_routes(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/submit", post(submit_proof))
        .route("/verify", post(verify_proof))
        .route("/", get(list_proofs))
        .route("/statistics", get(get_proof_statistics))
        .route("/templates", get(get_circuit_templates))
        .route("/:proof_id", get(get_proof))
        .route("/:proof_id", delete(delete_proof))
        .with_state(app_state)
}

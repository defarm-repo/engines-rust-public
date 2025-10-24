// Simple test endpoint to verify REAL blockchain integration
use axum::{extract::State, http::StatusCode, response::Json, routing::post, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

use crate::adapters::{
    base::StorageLocation, IpfsIpfsAdapter, StellarMainnetIpfsAdapter, StellarTestnetIpfsAdapter,
    StorageAdapter,
};
use crate::api::shared_state::AppState;
use crate::storage::StorageBackend;
use crate::types::{Item, ItemStatus};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct TestPushRequest {
    pub adapter_type: String, // "ipfs", "stellar-testnet", or "stellar-mainnet"
    pub test_data: String,
}

#[derive(Debug, Serialize)]
pub struct TestPushResponse {
    pub success: bool,
    pub adapter_type: String,
    pub storage_location: String,
    pub hash: String,
    pub message: String,
}

async fn test_blockchain_push(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<TestPushRequest>,
) -> Result<Json<TestPushResponse>, (StatusCode, Json<Value>)> {
    // Create a test item
    let dfid = format!("TEST-DFID-{}", Uuid::new_v4());
    let mut enriched_data = HashMap::new();
    enriched_data.insert("test".to_string(), serde_json::json!(true));
    enriched_data.insert("data".to_string(), serde_json::json!(payload.test_data));
    enriched_data.insert(
        "timestamp".to_string(),
        serde_json::json!(chrono::Utc::now().to_rfc3339()),
    );

    let test_item = Item {
        dfid: dfid.clone(),
        local_id: Some(Uuid::new_v4()),
        legacy_mode: false,
        identifiers: vec![],
        aliases: vec![],
        fingerprint: None,
        enriched_data,
        creation_timestamp: chrono::Utc::now(),
        last_modified: chrono::Utc::now(),
        source_entries: vec![],
        confidence_score: 1.0,
        status: ItemStatus::Active,
    };

    // Store in database first
    {
        let mut storage = state.shared_storage.lock().unwrap();
        storage.store_item(&test_item).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Failed to store item: {}", e)})),
            )
        })?;
    }

    // Test the adapter based on type
    let result = match payload.adapter_type.as_str() {
        "ipfs" => {
            let adapter = IpfsIpfsAdapter::new().map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": format!("Failed to create IPFS adapter: {}", e)})),
                )
            })?;

            let upload_result = adapter.store_item(&test_item).await.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": format!("IPFS upload failed: {}", e)})),
                )
            })?;

            let cid = match &upload_result.metadata.item_location {
                StorageLocation::IPFS { cid, .. } => cid.clone(),
                _ => "unknown".to_string(),
            };

            TestPushResponse {
                success: true,
                adapter_type: "IPFS-IPFS".to_string(),
                storage_location: "IPFS CID".to_string(),
                hash: cid.clone(),
                message: format!("✅ SUCCESS! Item uploaded to REAL Pinata IPFS. CID: {cid}. Verify at: https://gateway.pinata.cloud/ipfs/{cid}"),
            }
        }
        "stellar-testnet" => {
            let adapter = StellarTestnetIpfsAdapter::new()
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to create Stellar Testnet adapter: {}", e)}))))?;

            let upload_result = adapter.store_item(&test_item).await.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": format!("Stellar Testnet upload failed: {}", e)})),
                )
            })?;

            let tx_hash = match &upload_result.metadata.item_location {
                StorageLocation::Stellar { transaction_id, .. } => transaction_id.clone(),
                _ => "unknown".to_string(),
            };

            TestPushResponse {
                success: true,
                adapter_type: "Stellar-Testnet-IPFS".to_string(),
                storage_location: "Stellar Testnet + IPFS".to_string(),
                hash: tx_hash.clone(),
                message: format!("✅ SUCCESS! Item uploaded to REAL Stellar Testnet + Pinata IPFS. Transaction: {tx_hash}"),
            }
        }
        "stellar-mainnet" => {
            let adapter = StellarMainnetIpfsAdapter::new()
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to create Stellar Mainnet adapter: {}", e)}))))?;

            let upload_result = adapter.store_item(&test_item).await.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": format!("Stellar Mainnet upload failed: {}", e)})),
                )
            })?;

            let tx_hash = match &upload_result.metadata.item_location {
                StorageLocation::Stellar { transaction_id, .. } => transaction_id.clone(),
                _ => "unknown".to_string(),
            };

            TestPushResponse {
                success: true,
                adapter_type: "Stellar-Mainnet-IPFS".to_string(),
                storage_location: "Stellar Mainnet + IPFS".to_string(),
                hash: tx_hash.clone(),
                message: format!("✅ SUCCESS! Item uploaded to REAL Stellar Mainnet + Pinata IPFS. Transaction: {tx_hash}. ⚠️ THIS USED REAL FUNDS!"),
            }
        }
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(
                    json!({"error": "Invalid adapter_type. Use: ipfs, stellar-testnet, or stellar-mainnet"}),
                ),
            ));
        }
    };

    Ok(Json(result))
}

pub fn test_blockchain_routes(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/test-push", post(test_blockchain_push))
        .with_state(app_state)
}

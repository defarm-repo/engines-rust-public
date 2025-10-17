/// Timeline API Routes
///
/// Provides endpoints for querying item CID timeline from blockchain events.
///
/// Endpoints:
/// - GET /api/items/:dfid/timeline - Get complete timeline for an item
/// - GET /api/items/:dfid/timeline/:sequence - Get specific timeline entry
/// - GET /api/timeline/indexing-progress/:network - Get blockchain indexing status
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::postgres_persistence::PostgresPersistence;
use crate::types::{IndexingProgress, TimelineEntry};

/// Timeline API state
#[derive(Clone)]
pub struct TimelineState {
    pub persistence: Arc<PostgresPersistence>,
}

/// Timeline response wrapper
#[derive(Debug, Serialize, Deserialize)]
pub struct TimelineResponse {
    pub dfid: String,
    pub total_entries: usize,
    pub timeline: Vec<TimelineEntryResponse>,
}

/// Timeline entry for API response
#[derive(Debug, Serialize, Deserialize)]
pub struct TimelineEntryResponse {
    pub sequence: i32,
    pub cid: String,
    pub transaction_hash: String,
    pub blockchain_timestamp: i64,
    pub network: String,
    pub created_at: String,
}

impl From<TimelineEntry> for TimelineEntryResponse {
    fn from(entry: TimelineEntry) -> Self {
        Self {
            sequence: entry.event_sequence,
            cid: entry.cid,
            transaction_hash: entry.ipcm_transaction_hash,
            blockchain_timestamp: entry.blockchain_timestamp,
            network: entry.network,
            created_at: entry.created_at.to_rfc3339(),
        }
    }
}

/// Indexing progress response
#[derive(Debug, Serialize, Deserialize)]
pub struct IndexingProgressResponse {
    pub network: String,
    pub last_indexed_ledger: i64,
    pub last_confirmed_ledger: i64,
    pub last_indexed_at: String,
    pub status: String,
    pub total_events_indexed: i64,
    pub error_message: Option<String>,
}

impl From<IndexingProgress> for IndexingProgressResponse {
    fn from(progress: IndexingProgress) -> Self {
        Self {
            network: progress.network,
            last_indexed_ledger: progress.last_indexed_ledger,
            last_confirmed_ledger: progress.last_confirmed_ledger,
            last_indexed_at: progress.last_indexed_at.to_rfc3339(),
            status: progress.status,
            total_events_indexed: progress.total_events_indexed,
            error_message: progress.error_message,
        }
    }
}

/// GET /api/items/:dfid/timeline
/// Get the complete CID timeline for an item
///
/// Returns chronological list of all CIDs for this DFID from blockchain events.
pub async fn get_item_timeline(
    State(state): State<TimelineState>,
    Path(dfid): Path<String>,
) -> impl IntoResponse {
    tracing::debug!("📋 Getting timeline for DFID: {}", dfid);

    match state.persistence.get_item_timeline(&dfid).await {
        Ok(entries) => {
            let response = TimelineResponse {
                dfid: dfid.clone(),
                total_entries: entries.len(),
                timeline: entries
                    .into_iter()
                    .map(TimelineEntryResponse::from)
                    .collect(),
            };

            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            tracing::error!("❌ Failed to get timeline for {}: {}", dfid, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to retrieve timeline",
                    "details": e
                })),
            )
                .into_response()
        }
    }
}

/// GET /api/items/:dfid/timeline/:sequence
/// Get a specific timeline entry by sequence number
pub async fn get_timeline_entry(
    State(state): State<TimelineState>,
    Path((dfid, sequence)): Path<(String, i32)>,
) -> impl IntoResponse {
    tracing::debug!("📋 Getting timeline entry {} for DFID: {}", sequence, dfid);

    match state
        .persistence
        .get_timeline_by_sequence(&dfid, sequence)
        .await
    {
        Ok(Some(entry)) => {
            let response = TimelineEntryResponse::from(entry);
            (StatusCode::OK, Json(response)).into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Timeline entry not found",
                "dfid": dfid,
                "sequence": sequence
            })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!(
                "❌ Failed to get timeline entry {} for {}: {}",
                sequence,
                dfid,
                e
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to retrieve timeline entry",
                    "details": e
                })),
            )
                .into_response()
        }
    }
}

/// GET /api/timeline/indexing-progress/:network
/// Get blockchain indexing progress for a network
///
/// Network should be "stellar-testnet" or "stellar-mainnet"
pub async fn get_indexing_progress(
    State(state): State<TimelineState>,
    Path(network): Path<String>,
) -> impl IntoResponse {
    tracing::debug!("📊 Getting indexing progress for network: {}", network);

    // Validate network
    if network != "stellar-testnet" && network != "stellar-mainnet" {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Invalid network",
                "valid_networks": ["stellar-testnet", "stellar-mainnet"]
            })),
        )
            .into_response();
    }

    match state.persistence.get_indexing_progress(&network).await {
        Ok(Some(progress)) => {
            let response = IndexingProgressResponse::from(progress);
            (StatusCode::OK, Json(response)).into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "No indexing progress found",
                "network": network,
                "hint": "Event listener may not be running"
            })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("❌ Failed to get indexing progress for {}: {}", network, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to retrieve indexing progress",
                    "details": e
                })),
            )
                .into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeline_entry_response_serialization() {
        let entry = TimelineEntry {
            id: uuid::Uuid::new_v4(),
            dfid: "DFID-20250101-000001-ABC123".to_string(),
            cid: "QmTest123".to_string(),
            event_sequence: 1,
            blockchain_timestamp: 1704067200,
            ipcm_transaction_hash: "abc123".to_string(),
            network: "stellar-testnet".to_string(),
            created_at: chrono::Utc::now(),
        };

        let response = TimelineEntryResponse::from(entry);
        assert_eq!(response.sequence, 1);
        assert_eq!(response.cid, "QmTest123");
    }
}

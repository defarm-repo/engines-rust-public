/// Blockchain Event Listener for IPCM Contract Events
///
/// This module provides functionality to:
/// - Poll Soroban RPC for IPCM contract update_ipcm events
/// - Parse events to extract DFID and CID information
/// - Store timeline entries in PostgreSQL
/// - Track indexing progress per network
///
/// Architecture:
/// 1. Event listener runs as background daemon
/// 2. Polls Stellar blockchain via Soroban RPC
/// 3. Detects IPCM contract events (update_ipcm calls)
/// 4. Extracts DFID, CID, transaction hash, timestamp
/// 5. Stores in item_cid_timeline table
/// 6. Updates indexing progress
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

use crate::postgres_persistence::PostgresPersistence;
use crate::stellar_client::StellarNetwork;

/// Configuration for the event listener
#[derive(Debug, Clone)]
pub struct EventListenerConfig {
    /// Network to listen on (testnet or mainnet)
    pub network: StellarNetwork,
    /// IPCM contract address to monitor
    pub ipcm_contract_address: String,
    /// Polling interval in seconds
    pub poll_interval_secs: u64,
    /// Number of ledgers to query per batch
    pub batch_size: u32,
    /// Soroban RPC endpoint URL
    pub soroban_rpc_url: String,
}

impl Default for EventListenerConfig {
    fn default() -> Self {
        Self {
            network: StellarNetwork::Testnet,
            ipcm_contract_address: crate::stellar_client::TESTNET_IPCM_CONTRACT.to_string(),
            poll_interval_secs: 10,
            batch_size: 100,
            soroban_rpc_url: "https://soroban-testnet.stellar.org".to_string(),
        }
    }
}

/// Represents a parsed IPCM event from the blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcmEvent {
    /// DFID (key in the IPCM contract)
    pub dfid: String,
    /// IPFS CID (value in the IPCM contract)
    pub cid: String,
    /// Stellar transaction hash
    pub transaction_hash: String,
    /// Ledger close timestamp (Unix timestamp)
    pub ledger_timestamp: i64,
    /// Ledger sequence number
    pub ledger_sequence: i64,
}

/// Event listener for blockchain IPCM events
pub struct BlockchainEventListener {
    config: EventListenerConfig,
    persistence: Arc<PostgresPersistence>,
    soroban_client: SorobanRpcClient,
}

impl BlockchainEventListener {
    /// Create a new event listener
    pub fn new(config: EventListenerConfig, persistence: Arc<PostgresPersistence>) -> Self {
        let soroban_client = SorobanRpcClient::new(config.soroban_rpc_url.clone());

        Self {
            config,
            persistence,
            soroban_client,
        }
    }

    /// Start listening for events (blocking)
    /// This should be run in a dedicated task/thread
    pub async fn start(&self) -> Result<(), String> {
        let network_name = match self.config.network {
            StellarNetwork::Testnet => "stellar-testnet",
            StellarNetwork::Mainnet => "stellar-mainnet",
        };

        tracing::info!("🎧 Starting blockchain event listener for {}", network_name);
        tracing::info!("   IPCM contract: {}", self.config.ipcm_contract_address);
        tracing::info!("   Poll interval: {}s", self.config.poll_interval_secs);

        loop {
            if let Err(e) = self.poll_and_process_events().await {
                tracing::error!("❌ Event listener error: {}", e);
                // Continue running despite errors
                sleep(Duration::from_secs(self.config.poll_interval_secs)).await;
                continue;
            }

            sleep(Duration::from_secs(self.config.poll_interval_secs)).await;
        }
    }

    /// Poll for new events and process them
    async fn poll_and_process_events(&self) -> Result<(), String> {
        let network_name = match self.config.network {
            StellarNetwork::Testnet => "stellar-testnet",
            StellarNetwork::Mainnet => "stellar-mainnet",
        };

        // Get last indexed ledger from database
        let progress = self
            .persistence
            .get_indexing_progress(network_name)
            .await?
            .unwrap_or_else(|| {
                // Start from recent ledger if no progress exists
                crate::types::IndexingProgress {
                    network: network_name.to_string(),
                    last_indexed_ledger: 0,
                    last_confirmed_ledger: 0,
                    last_indexed_at: Utc::now(),
                    status: "active".to_string(),
                    error_message: None,
                    total_events_indexed: 0,
                    last_error_at: None,
                }
            });

        let start_ledger = progress.last_indexed_ledger + 1;
        let end_ledger = start_ledger + self.config.batch_size as i64;

        tracing::debug!(
            "📊 Querying ledgers {} to {} on {}",
            start_ledger,
            end_ledger,
            network_name
        );

        // Query events from blockchain
        let events = self
            .soroban_client
            .get_ipcm_events(&self.config.ipcm_contract_address, start_ledger, end_ledger)
            .await?;

        if !events.is_empty() {
            tracing::info!("📦 Found {} IPCM events to process", events.len());
        }

        // Process each event
        for event in &events {
            if let Err(e) = self.process_event(event, network_name).await {
                tracing::warn!("⚠️  Failed to process event for DFID {}: {}", event.dfid, e);
                // Continue processing other events
            }
        }

        // Update indexing progress
        self.persistence
            .update_indexing_progress(network_name, end_ledger, end_ledger)
            .await?;

        if !events.is_empty() {
            self.persistence
                .increment_events_indexed(network_name, events.len() as i64)
                .await?;
        }

        Ok(())
    }

    /// Process a single IPCM event
    async fn process_event(&self, event: &IpcmEvent, network: &str) -> Result<(), String> {
        tracing::debug!(
            "Processing event: {} -> {} (TX: {})",
            event.dfid,
            event.cid,
            event.transaction_hash
        );

        // Add to timeline
        self.persistence
            .add_cid_to_timeline(
                &event.dfid,
                &event.cid,
                &event.transaction_hash,
                event.ledger_timestamp,
                network,
            )
            .await?;

        tracing::debug!("✅ Processed IPCM event: {} -> {}", event.dfid, event.cid);

        Ok(())
    }
}

/// Client for querying Soroban RPC
pub struct SorobanRpcClient {
    rpc_url: String,
    client: reqwest::Client,
}

impl SorobanRpcClient {
    /// Create a new Soroban RPC client
    pub fn new(rpc_url: String) -> Self {
        Self {
            rpc_url,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Get IPCM events from contract within ledger range
    /// This queries the Soroban RPC for contract events
    pub async fn get_ipcm_events(
        &self,
        contract_address: &str,
        start_ledger: i64,
        _end_ledger: i64,
    ) -> Result<Vec<IpcmEvent>, String> {
        // Query Soroban RPC for events
        // POST request to RPC endpoint with getEvents method
        // Using xdrFormat: "json" for easier parsing (can refactor to XDR decoding later)
        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getEvents",
            "params": {
                "startLedger": start_ledger,
                "filters": [{
                    "type": "contract",
                    "contractIds": [contract_address]
                }],
                "xdrFormat": "json"
            }
        });

        let response = self
            .client
            .post(&self.rpc_url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| format!("Failed to query Soroban RPC: {e}"))?;

        if !response.status().is_success() {
            return Err(format!("Soroban RPC error: HTTP {}", response.status()));
        }

        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse RPC response: {e}"))?;

        // Parse events from response
        let events = self.parse_events_response(response_json)?;

        Ok(events)
    }

    /// Parse Soroban RPC events response into IpcmEvent structs
    /// Expects JSON format response (xdrFormat: "json")
    fn parse_events_response(&self, response: serde_json::Value) -> Result<Vec<IpcmEvent>, String> {
        let result = response.get("result").ok_or("No result in RPC response")?;

        let events_array = result
            .get("events")
            .and_then(|v| v.as_array())
            .ok_or("No events array in result")?;

        let mut ipcm_events = Vec::new();

        for event in events_array {
            // Extract event data from JSON format
            // Event structure from IPCM contract:
            // Topic: (symbol_short!("update"), dfid)
            // Data: (cid, timestamp, updater_address)

            let topic = event
                .get("topic")
                .and_then(|t| t.as_array())
                .ok_or("No topic in event")?;

            let value = event.get("value").ok_or("No value in event")?;

            // Parse DFID from topic (second element)
            let dfid = self.extract_dfid(topic)?;

            // Parse CID from value (first element of tuple)
            let cid = self.extract_cid(value)?;

            let tx_hash = event
                .get("txHash")
                .and_then(|h| h.as_str())
                .ok_or("No txHash in event")?
                .to_string();

            let ledger_timestamp = event
                .get("ledgerClosedAt")
                .and_then(|t| t.as_str())
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.timestamp())
                .ok_or("Invalid ledger timestamp")?;

            let ledger_sequence = event
                .get("ledger")
                .and_then(|l| l.as_i64())
                .ok_or("No ledger sequence in event")?;

            ipcm_events.push(IpcmEvent {
                dfid,
                cid,
                transaction_hash: tx_hash,
                ledger_timestamp,
                ledger_sequence,
            });
        }

        Ok(ipcm_events)
    }

    /// Extract DFID from event topic (JSON format)
    /// Topic structure: [{"sym": "update"}, {"string": "DFID-..."}]
    /// Returns the DFID string from the second element
    fn extract_dfid(&self, topic: &[serde_json::Value]) -> Result<String, String> {
        if topic.len() < 2 {
            return Err(format!(
                "Topic too short: expected at least 2 elements, got {}",
                topic.len()
            ));
        }

        // Second element should be the DFID
        let dfid_value = &topic[1];

        // Try different possible JSON structures
        // Option 1: {"string": "DFID-..."}
        if let Some(s) = dfid_value.get("string").and_then(|v| v.as_str()) {
            return Ok(s.to_string());
        }

        // Option 2: {"String": "DFID-..."}
        if let Some(s) = dfid_value.get("String").and_then(|v| v.as_str()) {
            return Ok(s.to_string());
        }

        // Option 3: Direct string
        if let Some(s) = dfid_value.as_str() {
            return Ok(s.to_string());
        }

        Err(format!(
            "Failed to extract DFID from topic element: {}",
            dfid_value
        ))
    }

    /// Extract CID from event value (JSON format)
    /// Value structure: [{"string": "Qm..."}, {"u64": 123456}, {"address": "G..."}]
    /// Returns the CID string from the first element
    fn extract_cid(&self, value: &serde_json::Value) -> Result<String, String> {
        // Value should be an array (tuple in Soroban)
        let value_array = value.as_array().ok_or("Event value is not an array")?;

        if value_array.is_empty() {
            return Err("Event value array is empty".to_string());
        }

        // First element should be the CID
        let cid_value = &value_array[0];

        // Try different possible JSON structures
        // Option 1: {"string": "Qm..."}
        if let Some(s) = cid_value.get("string").and_then(|v| v.as_str()) {
            return Ok(s.to_string());
        }

        // Option 2: {"String": "Qm..."}
        if let Some(s) = cid_value.get("String").and_then(|v| v.as_str()) {
            return Ok(s.to_string());
        }

        // Option 3: Direct string
        if let Some(s) = cid_value.as_str() {
            return Ok(s.to_string());
        }

        Err(format!(
            "Failed to extract CID from value element: {}",
            cid_value
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_listener_config_default() {
        let config = EventListenerConfig::default();
        assert_eq!(config.poll_interval_secs, 10);
        assert_eq!(config.batch_size, 100);
    }

    #[test]
    fn test_ipcm_event_creation() {
        let event = IpcmEvent {
            dfid: "DFID-20250101-000001-ABC123".to_string(),
            cid: "QmTest123456789".to_string(),
            transaction_hash: "abc123def456".to_string(),
            ledger_timestamp: 1704067200,
            ledger_sequence: 12345,
        };

        assert_eq!(event.dfid, "DFID-20250101-000001-ABC123");
        assert_eq!(event.cid, "QmTest123456789");
    }
}

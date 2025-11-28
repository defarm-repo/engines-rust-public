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
    /// Soroban RPC endpoint URLs (first entry is treated as primary)
    pub soroban_rpc_urls: Vec<String>,
}

impl Default for EventListenerConfig {
    fn default() -> Self {
        let network = StellarNetwork::Testnet;
        Self {
            network: network.clone(),
            ipcm_contract_address: crate::stellar_client::TESTNET_IPCM_CONTRACT.to_string(),
            poll_interval_secs: 10,
            batch_size: 100,
            soroban_rpc_urls: Self::recommended_rpc_urls(&network),
        }
    }
}

// Based on Stellar's public RPC catalog:
// https://developers.stellar.org/docs/data/apis/rpc/providers (retrieved Nov 10, 2025)
const TESTNET_RPC_ENDPOINTS: &[&str] = &[
    "https://soroban-testnet.stellar.org",
    "https://soroban-rpc.testnet.stellar.gateway.fm",
    "https://stellar-soroban-testnet-public.nodies.app",
];

const MAINNET_RPC_ENDPOINTS: &[&str] = &[
    "https://soroban-mainnet.stellar.org",
    "https://soroban-rpc.mainnet.stellar.org",
    "https://soroban-rpc.mainnet.stellar.gateway.fm",
    "https://stellar-soroban-public.nodies.app",
    "https://stellar.api.onfinality.io/public",
    "https://rpc.lightsail.network/",
    "https://archive-rpc.lightsail.network/",
    "https://mainnet.sorobanrpc.com",
];

const DEFAULT_INITIAL_LEDGER_LOOKBACK: i64 = 5_000;

impl EventListenerConfig {
    /// Returns a curated list of RPC endpoints for a network, ordered by preference.
    pub fn recommended_rpc_urls(network: &StellarNetwork) -> Vec<String> {
        let defaults: &[&str] = match network {
            StellarNetwork::Testnet => TESTNET_RPC_ENDPOINTS,
            StellarNetwork::Mainnet => MAINNET_RPC_ENDPOINTS,
        };

        defaults.iter().map(|url| url.to_string()).collect()
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
        let soroban_client = SorobanRpcClient::new(config.soroban_rpc_urls.clone());

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

        let mut start_ledger = if progress.last_indexed_ledger <= 0 {
            let bootstrap_ledger = self
                .soroban_client
                .suggest_start_ledger(self.config.batch_size)
                .await?;
            tracing::info!(
                "🧭 No prior indexing progress for {}. Bootstrapping from ledger {}",
                network_name,
                bootstrap_ledger
            );
            bootstrap_ledger
        } else {
            progress.last_indexed_ledger + 1
        };
        let mut end_ledger = start_ledger + self.config.batch_size as i64;

        tracing::debug!(
            "📊 Querying ledgers {} to {} on {}",
            start_ledger,
            end_ledger,
            network_name
        );

        // Query events from blockchain
        let mut events_result = self
            .soroban_client
            .get_ipcm_events(&self.config.ipcm_contract_address, start_ledger, end_ledger)
            .await;

        if let Err(err) = &events_result {
            if Self::start_ledger_before_oldest(err) {
                tracing::warn!(
                    "📉 {} indexing window {:?}-{:?} is too old ({err}). Determining safe restart ledger...",
                    network_name,
                    start_ledger,
                    end_ledger
                );
                start_ledger = self
                    .soroban_client
                    .suggest_start_ledger(self.config.batch_size)
                    .await?;
                end_ledger = start_ledger + self.config.batch_size as i64;
                tracing::warn!(
                    "🔁 Restarting {} event sync from ledger {}",
                    network_name,
                    start_ledger
                );
                events_result = self
                    .soroban_client
                    .get_ipcm_events(&self.config.ipcm_contract_address, start_ledger, end_ledger)
                    .await;
            }
        }

        let events = events_result?;

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

    fn start_ledger_before_oldest(err: &str) -> bool {
        let lower = err.to_ascii_lowercase();
        (lower.contains("startledger") || lower.contains("start ledger"))
            && (lower.contains("oldest")
                || lower.contains("too low")
                || lower.contains("before")
                || lower.contains("range")
                || lower.contains("within"))
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
    rpc_urls: Vec<String>,
    client: reqwest::Client,
}

#[derive(Clone, Copy, Debug)]
struct LedgerWindow {
    latest_ledger: i64,
    oldest_ledger: Option<i64>,
}

impl LedgerWindow {
    fn safe_start(&self, lookback: i64) -> i64 {
        let mut start = self
            .latest_ledger
            .saturating_sub(std::cmp::max(lookback, 1));
        if let Some(oldest) = self.oldest_ledger {
            if start < oldest {
                start = oldest;
            }
        }

        // Soroban ledgers are 1-indexed
        if start < 1 {
            1
        } else {
            start
        }
    }
}

impl SorobanRpcClient {
    /// Create a new Soroban RPC client
    pub fn new(rpc_urls: Vec<String>) -> Self {
        let deduped = rpc_urls
            .into_iter()
            .map(|url| url.trim().to_string())
            .filter(|url| !url.is_empty())
            .fold(Vec::<String>::new(), |mut acc, url| {
                if !acc
                    .iter()
                    .any(|existing| existing.eq_ignore_ascii_case(&url))
                {
                    acc.push(url);
                }
                acc
            });

        assert!(
            !deduped.is_empty(),
            "At least one Soroban RPC endpoint must be provided"
        );

        Self {
            rpc_urls: deduped,
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
        let mut last_error = None;

        for rpc_url in &self.rpc_urls {
            match self
                .query_rpc_endpoint(rpc_url, contract_address, start_ledger)
                .await
            {
                Ok(events) => {
                    if tracing::enabled!(tracing::Level::DEBUG) {
                        tracing::debug!(
                            "📡 Soroban RPC {} returned {} events",
                            rpc_url,
                            events.len()
                        );
                    }
                    return Ok(events);
                }
                Err(err) => {
                    tracing::warn!(
                        "⚠️  Soroban RPC endpoint {} failed ({}). Trying next fallback...",
                        rpc_url,
                        err
                    );
                    last_error = Some(err);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            "All Soroban RPC endpoints failed for getEvents request".to_string()
        }))
    }

    async fn query_rpc_endpoint(
        &self,
        rpc_url: &str,
        contract_address: &str,
        start_ledger: i64,
    ) -> Result<Vec<IpcmEvent>, String> {
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
            .post(rpc_url)
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

        if let Some(err_obj) = response_json.get("error") {
            return Err(Self::format_rpc_error(err_obj));
        }

        // Parse events from response
        self.parse_events_response(response_json)
    }

    pub async fn suggest_start_ledger(&self, batch_size: u32) -> Result<i64, String> {
        let window = self.get_latest_ledger_window().await?;
        let lookback = std::cmp::max(DEFAULT_INITIAL_LEDGER_LOOKBACK, batch_size as i64);
        Ok(window.safe_start(lookback))
    }

    async fn get_latest_ledger_window(&self) -> Result<LedgerWindow, String> {
        let mut last_error = None;

        for rpc_url in &self.rpc_urls {
            match self.fetch_latest_ledger_from_endpoint(rpc_url).await {
                Ok(window) => return Ok(window),
                Err(err) => {
                    tracing::warn!(
                        "⚠️  Soroban RPC endpoint {} failed to provide latest ledger: {}",
                        rpc_url,
                        err
                    );
                    last_error = Some(err);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            "All Soroban RPC endpoints failed for getLatestLedger request".to_string()
        }))
    }

    async fn fetch_latest_ledger_from_endpoint(
        &self,
        rpc_url: &str,
    ) -> Result<LedgerWindow, String> {
        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getLatestLedger",
        });

        let response = self
            .client
            .post(rpc_url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| format!("Failed to query Soroban RPC: {e}"))?;

        if !response.status().is_success() {
            return Err(format!(
                "Soroban RPC error while fetching latest ledger: HTTP {}",
                response.status()
            ));
        }

        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse latest ledger response: {e}"))?;

        if let Some(err_obj) = response_json.get("error") {
            return Err(Self::format_rpc_error(err_obj));
        }

        let result = response_json
            .get("result")
            .ok_or("No result in getLatestLedger response")?;

        let sequence = result
            .get("sequence")
            .and_then(|v| v.as_i64())
            .ok_or("Missing sequence in getLatestLedger response")?;
        let oldest = result.get("oldestLedger").and_then(|v| v.as_i64());

        Ok(LedgerWindow {
            latest_ledger: sequence,
            oldest_ledger: oldest,
        })
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
            "Failed to extract DFID from topic element: {dfid_value}"
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
            "Failed to extract CID from value element: {cid_value}"
        ))
    }

    fn format_rpc_error(error_obj: &serde_json::Value) -> String {
        let code = error_obj.get("code").and_then(|c| c.as_i64());
        let message = error_obj
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown RPC error")
            .to_string();
        let data = error_obj.get("data");
        let mut formatted = if let Some(code) = code {
            format!("RPC error {code}: {message}")
        } else {
            format!("RPC error: {message}")
        };

        if let Some(data) = data {
            if !data.is_null() {
                formatted.push_str(&format!(" ({})", data));
            }
        }

        formatted
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
        assert!(!config.soroban_rpc_urls.is_empty());
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

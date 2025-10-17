/// IPCM Event Listener Binary
///
/// Background daemon that monitors Stellar blockchain for IPCM contract events
/// and populates the CID timeline database.
///
/// Usage:
///   cargo run --bin ipcm_event_listener
///
/// Environment variables:
///   DATABASE_URL               - PostgreSQL connection string (required)
///   STELLAR_NETWORK            - testnet or mainnet (default: testnet)
///   IPCM_CONTRACT_ADDRESS      - IPCM contract to monitor (optional, uses default)
///   SOROBAN_RPC_URL            - Soroban RPC endpoint (optional, uses default)
///   LISTENER_POLL_INTERVAL     - Poll interval in seconds (default: 10)
///   LISTENER_BATCH_SIZE        - Ledgers per batch (default: 100)
use std::env;
use std::sync::Arc;
use tokio;
use tracing::{error, info};

use defarm_engine::blockchain_event_listener::{BlockchainEventListener, EventListenerConfig};
use defarm_engine::postgres_persistence::PostgresPersistence;
use defarm_engine::stellar_client::{StellarNetwork, MAINNET_IPCM_CONTRACT, TESTNET_IPCM_CONTRACT};

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🚀 Starting IPCM Event Listener Daemon");

    // Load configuration from environment
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        error!("❌ DATABASE_URL environment variable is required");
        std::process::exit(1);
    });

    let network = match env::var("STELLAR_NETWORK")
        .unwrap_or_else(|_| "testnet".to_string())
        .as_str()
    {
        "mainnet" => StellarNetwork::Mainnet,
        "testnet" => StellarNetwork::Testnet,
        other => {
            error!("❌ Invalid STELLAR_NETWORK: {}", other);
            std::process::exit(1);
        }
    };

    let ipcm_contract_address =
        env::var("IPCM_CONTRACT_ADDRESS").unwrap_or_else(|_| match network {
            StellarNetwork::Testnet => TESTNET_IPCM_CONTRACT.to_string(),
            StellarNetwork::Mainnet => MAINNET_IPCM_CONTRACT.to_string(),
        });

    let soroban_rpc_url = env::var("SOROBAN_RPC_URL").unwrap_or_else(|_| match network {
        StellarNetwork::Testnet => "https://soroban-testnet.stellar.org".to_string(),
        StellarNetwork::Mainnet => "https://soroban-mainnet.stellar.org".to_string(),
    });

    let poll_interval_secs = env::var("LISTENER_POLL_INTERVAL")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);

    let batch_size = env::var("LISTENER_BATCH_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);

    info!("📋 Configuration:");
    info!("   Network: {:?}", network);
    info!("   IPCM Contract: {}", ipcm_contract_address);
    info!("   Soroban RPC: {}", soroban_rpc_url);
    info!("   Poll Interval: {}s", poll_interval_secs);
    info!("   Batch Size: {} ledgers", batch_size);

    // Initialize PostgreSQL persistence
    info!("🗄️  Connecting to PostgreSQL...");
    let mut persistence = PostgresPersistence::new(database_url);

    if let Err(e) = persistence.connect().await {
        error!("❌ Failed to connect to PostgreSQL: {}", e);
        std::process::exit(1);
    }

    info!("✅ PostgreSQL connected");

    // Create event listener configuration
    let config = EventListenerConfig {
        network,
        ipcm_contract_address,
        poll_interval_secs,
        batch_size,
        soroban_rpc_url,
    };

    // Create and start event listener
    let listener = BlockchainEventListener::new(config, Arc::new(persistence));

    info!("🎧 Event listener initialized, starting polling loop...");

    // Start listening (this will block forever)
    if let Err(e) = listener.start().await {
        error!("❌ Event listener failed: {}", e);
        std::process::exit(1);
    }
}

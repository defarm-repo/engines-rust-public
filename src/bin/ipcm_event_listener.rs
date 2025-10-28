/// IPCM Event Listener Binary - Dual Network Support
///
/// Background daemon that monitors Stellar blockchain for IPCM contract events
/// and populates the CID timeline database.
///
/// Supports monitoring both testnet and mainnet simultaneously.
///
/// Usage:
///   cargo run --bin ipcm_event_listener
///
/// Environment variables:
///   DATABASE_URL                      - PostgreSQL connection string (required)
///
///   ENABLE_TESTNET_LISTENER           - Enable testnet listener (default: true)
///   STELLAR_TESTNET_IPCM_CONTRACT     - Testnet IPCM contract (optional, uses default)
///   STELLAR_TESTNET_RPC_URL           - Testnet Soroban RPC (optional, uses default)
///   TESTNET_POLL_INTERVAL             - Testnet poll interval in seconds (default: 10)
///   TESTNET_BATCH_SIZE                - Testnet ledgers per batch (default: 100)
///
///   ENABLE_MAINNET_LISTENER           - Enable mainnet listener (default: false)
///   STELLAR_MAINNET_IPCM_CONTRACT     - Mainnet IPCM contract (optional, uses default)
///   STELLAR_MAINNET_RPC_URL           - Mainnet Soroban RPC (optional, uses default)
///   MAINNET_POLL_INTERVAL             - Mainnet poll interval in seconds (default: 10)
///   MAINNET_BATCH_SIZE                - Mainnet ledgers per batch (default: 100)
use std::env;
use std::sync::Arc;
use tracing::{error, info, warn};

use defarm_engine::blockchain_event_listener::{BlockchainEventListener, EventListenerConfig};
use defarm_engine::postgres_persistence::PostgresPersistence;
use defarm_engine::stellar_client::{StellarNetwork, MAINNET_IPCM_CONTRACT, TESTNET_IPCM_CONTRACT};

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🚀 Starting IPCM Event Listener Daemon (Dual Network Support)");

    // Load database URL
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        error!("❌ DATABASE_URL environment variable is required");
        std::process::exit(1);
    });

    // Initialize PostgreSQL persistence (shared across both networks)
    info!("🗄️  Connecting to PostgreSQL...");
    let mut persistence = PostgresPersistence::new(database_url);

    if let Err(e) = persistence.connect().await {
        error!("❌ Failed to connect to PostgreSQL: {}", e);
        std::process::exit(1);
    }

    info!("✅ PostgreSQL connected");
    let persistence = Arc::new(persistence);

    // Check which networks are enabled
    let enable_testnet = env::var("ENABLE_TESTNET_LISTENER")
        .ok()
        .and_then(|s| s.parse::<bool>().ok())
        .unwrap_or(true); // Default: enabled

    let enable_mainnet = env::var("ENABLE_MAINNET_LISTENER")
        .ok()
        .and_then(|s| s.parse::<bool>().ok())
        .unwrap_or(false); // Default: disabled (safety)

    if !enable_testnet && !enable_mainnet {
        error!("❌ At least one network must be enabled (ENABLE_TESTNET_LISTENER or ENABLE_MAINNET_LISTENER)");
        std::process::exit(1);
    }

    info!("📋 Network Configuration:");
    info!(
        "   Testnet Listener: {}",
        if enable_testnet {
            "✅ ENABLED"
        } else {
            "❌ DISABLED"
        }
    );
    info!(
        "   Mainnet Listener: {}",
        if enable_mainnet {
            "✅ ENABLED"
        } else {
            "❌ DISABLED"
        }
    );

    let mut tasks = vec![];

    // Start testnet listener if enabled
    if enable_testnet {
        let testnet_contract = env::var("STELLAR_TESTNET_IPCM_CONTRACT")
            .unwrap_or_else(|_| TESTNET_IPCM_CONTRACT.to_string());
        let testnet_rpc = env::var("STELLAR_TESTNET_RPC_URL")
            .unwrap_or_else(|_| "https://soroban-testnet.stellar.org".to_string());
        let testnet_poll = env::var("TESTNET_POLL_INTERVAL")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(10);
        let testnet_batch = env::var("TESTNET_BATCH_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(100);

        info!("🌐 Testnet Configuration:");
        info!("   IPCM Contract: {}", testnet_contract);
        info!("   Soroban RPC: {}", testnet_rpc);
        info!("   Poll Interval: {}s", testnet_poll);
        info!("   Batch Size: {} ledgers", testnet_batch);

        let testnet_config = EventListenerConfig {
            network: StellarNetwork::Testnet,
            ipcm_contract_address: testnet_contract,
            poll_interval_secs: testnet_poll,
            batch_size: testnet_batch,
            soroban_rpc_url: testnet_rpc,
        };

        let testnet_persistence = persistence.clone();
        let testnet_task = tokio::spawn(async move {
            let listener = BlockchainEventListener::new(testnet_config, testnet_persistence);
            info!("🎧 Starting testnet event listener...");
            if let Err(e) = listener.start().await {
                error!("❌ Testnet listener failed: {}", e);
            }
        });
        tasks.push(testnet_task);
    }

    // Start mainnet listener if enabled
    if enable_mainnet {
        let mainnet_contract = env::var("STELLAR_MAINNET_IPCM_CONTRACT")
            .unwrap_or_else(|_| MAINNET_IPCM_CONTRACT.to_string());
        let mainnet_rpc = env::var("STELLAR_MAINNET_RPC_URL")
            .unwrap_or_else(|_| "https://soroban-mainnet.stellar.org".to_string());
        let mainnet_poll = env::var("MAINNET_POLL_INTERVAL")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(10);
        let mainnet_batch = env::var("MAINNET_BATCH_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(100);

        info!("🌐 Mainnet Configuration:");
        info!("   IPCM Contract: {}", mainnet_contract);
        info!("   Soroban RPC: {}", mainnet_rpc);
        info!("   Poll Interval: {}s", mainnet_poll);
        info!("   Batch Size: {} ledgers", mainnet_batch);

        let mainnet_config = EventListenerConfig {
            network: StellarNetwork::Mainnet,
            ipcm_contract_address: mainnet_contract,
            poll_interval_secs: mainnet_poll,
            batch_size: mainnet_batch,
            soroban_rpc_url: mainnet_rpc,
        };

        let mainnet_persistence = persistence.clone();
        let mainnet_task = tokio::spawn(async move {
            let listener = BlockchainEventListener::new(mainnet_config, mainnet_persistence);
            info!("🎧 Starting mainnet event listener...");
            if let Err(e) = listener.start().await {
                error!("❌ Mainnet listener failed: {}", e);
            }
        });
        tasks.push(mainnet_task);
    }

    // Wait for all tasks (they run forever unless they error)
    for task in tasks {
        if let Err(e) = task.await {
            error!("❌ Listener task panicked: {}", e);
            std::process::exit(1);
        }
    }

    warn!("⚠️  All listener tasks completed (unexpected)");
    std::process::exit(1);
}

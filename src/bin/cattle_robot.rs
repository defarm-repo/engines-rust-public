use chrono::Utc;
use defarm_engine::cattle_robot::{
    api_client::RailwayApiClient,
    config::RobotConfig,
    data_generator::DataGenerator,
    operations::{mint_new_cattle, update_existing_cattle, OperationError},
    scheduler::{CattleScheduler, OperationType},
};
use sqlx::postgres::PgPoolOptions;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::signal;
use tokio::time::sleep;

/// Autonomous cattle minting robot
/// Continuously mints and updates cattle NFTs on Stellar testnet + IPFS

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("🤖 Cattle Robot Starting...");

    // Load configuration
    let config = RobotConfig::from_env()?;
    config.validate()?;

    log::info!("{}", config.summary());

    if config.is_dry_run() {
        log::warn!("⚠️  DRY RUN MODE - No actual operations will be performed");
    }

    // Setup shutdown signal handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    tokio::spawn(async move {
        shutdown_signal().await;
        log::warn!("Shutdown signal received, stopping robot...");
        r.store(false, Ordering::SeqCst);
    });

    // Connect to database
    log::info!("Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await?;

    log::info!("✓ Database connected");

    // Initialize API client
    let api_client =
        RailwayApiClient::new(config.railway_api_url.clone(), config.robot_api_key.clone())?;

    // Check API health
    log::info!("Checking API health...");
    match api_client.health_check().await {
        Ok(true) => log::info!("✓ API is healthy"),
        Ok(false) => log::warn!("⚠️  API health check failed"),
        Err(e) => log::error!("❌ API health check error: {e}"),
    }

    // Ensure circuit is configured
    let circuit_id = if let Some(id) = config.circuit_id() {
        log::info!("Using existing circuit: {id}");
        id.to_string()
    } else {
        log::info!("Creating new robot circuit...");
        let circuit = api_client
            .create_circuit(
                "Autonomous Cattle Registry",
                "Automated cattle data generation for testing and demonstration",
            )
            .await?;

        log::info!("✓ Circuit created: {}", circuit.id);

        // Configure adapter
        log::info!("Configuring Stellar Testnet + IPFS adapter...");
        api_client.configure_circuit_adapter(&circuit.id).await?;
        log::info!("✓ Adapter configured");

        circuit.id
    };

    // Initialize components
    let mut scheduler = CattleScheduler::new();
    let mut data_generator = DataGenerator::new();

    // Get requester ID (extract from API key or use default)
    let requester_id = "robot-system";

    // Statistics
    let mut total_mints = 0u64;
    let mut total_updates = 0u64;
    let mut total_errors = 0u64;
    let start_time = Utc::now();

    log::info!("🚀 Robot is now running");
    log::info!("Circuit ID: {circuit_id}");
    log::info!("Press Ctrl+C to stop");
    log::info!("----------------------------------------");

    // Main loop
    while running.load(Ordering::SeqCst) {
        // Get time info
        let time_info = scheduler.current_time_info();

        // Select operation type
        let operation_type = scheduler.select_operation_type();

        log::info!("⏰ {}", time_info.summary());
        log::info!("🎲 Selected operation: {operation_type:?}");

        // Execute operation
        if !config.is_dry_run() {
            match operation_type {
                OperationType::NewMint => {
                    match mint_new_cattle(
                        &api_client,
                        &mut data_generator,
                        &pool,
                        &circuit_id,
                        requester_id,
                    )
                    .await
                    {
                        Ok(result) => {
                            total_mints += 1;
                            log::info!(
                                "✅ MINT SUCCESS: SISBOV={}, DFID={}, CID={:?}",
                                result.sisbov,
                                result.dfid,
                                result.cid
                            );
                        }
                        Err(e) => {
                            total_errors += 1;
                            log::error!("❌ MINT FAILED: {e}");
                        }
                    }
                }
                OperationType::Update => {
                    match update_existing_cattle(
                        &api_client,
                        &mut data_generator,
                        &pool,
                        &circuit_id,
                        requester_id,
                    )
                    .await
                    {
                        Ok(result) => {
                            total_updates += 1;
                            log::info!(
                                "✅ UPDATE SUCCESS: Event={}, DFID={}",
                                result.event_type,
                                result.dfid
                            );
                        }
                        Err(e) => match e {
                            OperationError::NoCattleAvailable => {
                                log::warn!("⚠️  No cattle available for update, skipping...");
                            }
                            _ => {
                                total_errors += 1;
                                log::error!("❌ UPDATE FAILED: {e}");
                            }
                        },
                    }
                }
            }
        } else {
            log::info!("DRY RUN: Would execute {operation_type:?}");
        }

        // Print statistics
        let uptime = (Utc::now() - start_time).num_seconds();
        log::info!(
            "📊 Stats: Mints={total_mints}, Updates={total_updates}, Errors={total_errors}, Uptime={uptime}s"
        );

        // Calculate next delay
        let delay = scheduler.next_operation_delay();
        let delay_mins = delay.as_secs() / 60;
        let delay_secs = delay.as_secs() % 60;

        log::info!("⏳ Next operation in {delay_mins}m {delay_secs}s");
        log::info!("----------------------------------------");

        // Sleep until next operation
        sleep(delay).await;
    }

    // Graceful shutdown
    log::info!("🛑 Shutting down gracefully...");
    log::info!("Final stats: Mints={total_mints}, Updates={total_updates}, Errors={total_errors}");

    pool.close().await;

    log::info!("👋 Robot stopped");

    Ok(())
}

/// Handle shutdown signals (SIGTERM, SIGINT)
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

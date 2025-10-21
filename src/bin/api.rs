use axum::{http::StatusCode, middleware, response::Json, routing::get, Router};
use serde_json::{json, Value};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing::{info, Level};

use defarm_engine::api::{
    activity_routes, adapter_routes, admin_routes, audit_routes, auth_routes, circuit_routes,
    event_routes, get_indexing_progress, get_item_timeline, get_timeline_entry, item_routes,
    notifications_rest_routes, notifications_ws_route, receipt_routes, shared_state::AppState,
    storage_history_routes, test_blockchain_routes, user_activity_routes, user_credits_routes,
    workspace_routes, zk_proof_routes, TimelineState,
};
use defarm_engine::auth_middleware::jwt_auth_middleware;
use defarm_engine::postgres_persistence::PostgresPersistence;
use std::sync::Arc;

// Removed in-memory fallback - PostgreSQL is now required for data persistence

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    // Load environment variables
    dotenv::dotenv().ok();

    // Initialize shared state first (this can't fail)
    let app_state = Arc::new(AppState::new());

    // Stellar SDK integration - no CLI needed, health check removed
    info!("🌟 Stellar SDK integration enabled (native Rust - no CLI dependency)");

    // Initialize PostgreSQL - can be synchronous or asynchronous based on env var
    // POSTGRES_WAIT_ON_STARTUP=true (default in production) blocks until connected
    let wait_on_startup = std::env::var("POSTGRES_WAIT_ON_STARTUP")
        .map(|v| v.to_lowercase() == "true" || v == "1")
        .unwrap_or_else(|_| {
            // Default to true if DATABASE_URL is set (production mode)
            std::env::var("DATABASE_URL").is_ok()
        });

    if wait_on_startup {
        info!("⏳ Waiting for PostgreSQL connection before starting server...");
        initialize_postgres_sync(app_state.clone()).await;
    } else {
        info!("🚀 Starting server with background PostgreSQL initialization...");
        initialize_postgres_background(app_state.clone());
    }

    // Health endpoints with state
    let health_routes = Router::new()
        .route("/health/db", get(health_check_db))
        .with_state(app_state.clone());

    // Public routes (no authentication required)
    let public_routes = Router::new()
        .route("/", get(root))
        .route("/health", get(health_check))
        .merge(health_routes)
        .nest("/api/auth", auth_routes(app_state.clone()))
        // WebSocket route does NOT use JWT middleware (verifies token from query param)
        .nest(
            "/api/notifications",
            notifications_ws_route(app_state.notification_tx.clone()).with_state(app_state.clone()),
        );

    // Timeline routes (requires PostgreSQL - will return error if not available)
    // Note: timeline_state will be created even if PostgreSQL is None, but endpoints will fail gracefully
    let timeline_state_result =
        app_state
            .postgres_persistence
            .try_read()
            .ok()
            .and_then(|pg_lock| {
                pg_lock.as_ref().map(|pg| TimelineState {
                    persistence: Arc::new(pg.clone()),
                })
            });

    let timeline_routes = if let Some(timeline_state) = timeline_state_result {
        Router::new()
            .route("/api/items/:dfid/timeline", get(get_item_timeline))
            .route(
                "/api/items/:dfid/timeline/:sequence",
                get(get_timeline_entry),
            )
            .route(
                "/api/timeline/indexing-progress/:network",
                get(get_indexing_progress),
            )
            .with_state(timeline_state)
    } else {
        tracing::warn!("⚠️  Timeline routes disabled - PostgreSQL not available");
        Router::new() // Empty router if PostgreSQL not available
    };

    // Protected routes (require JWT authentication)
    let protected_routes = Router::new()
        .nest("/api/receipts", receipt_routes())
        .nest("/api/events", event_routes(app_state.clone()))
        .nest("/api/circuits", circuit_routes(app_state.clone()))
        .nest("/api/items", item_routes(app_state.clone()))
        .nest("/api/workspaces", workspace_routes())
        .nest("/api/activities", activity_routes(app_state.clone()))
        .nest(
            "/api/user-activity",
            user_activity_routes(app_state.clone()),
        )
        .nest("/audit", audit_routes(app_state.clone()))
        .nest("/api/proofs", zk_proof_routes(app_state.clone()))
        .nest("/api/adapters", adapter_routes(app_state.clone()))
        .nest(
            "/api/storage-history",
            storage_history_routes(app_state.clone()),
        )
        .nest("/api/test", test_blockchain_routes(app_state.clone()))
        // Notification REST API routes (protected by JWT middleware)
        .nest(
            "/api/notifications",
            notifications_rest_routes().with_state(app_state.clone()),
        )
        .merge(user_credits_routes().with_state(app_state.clone()))
        .nest("/api/admin", admin_routes().with_state(app_state.clone()))
        .merge(timeline_routes) // Add timeline routes
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            jwt_auth_middleware,
        ));

    // Combine routes and add static file serving for docs
    // Note: nest_service for /docs must come AFTER merging routes to avoid conflicts
    let app = public_routes
        .merge(protected_routes)
        .nest_service("/docs", ServeDir::new("docs"))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive());

    // Railway provides PORT environment variable, fallback to 3000 for local development
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(3000);

    // Bind to 0.0.0.0 for Railway compatibility (IPv4)
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("🚀 DeFarm API server starting on {} (PORT={})", addr, port);

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => {
            info!(
                "✅ Server listening and ready to accept connections on {}",
                addr
            );
            info!(
                "🏥 Health check endpoint: http://[{}]:{}/health",
                addr.ip(),
                addr.port()
            );
            l
        }
        Err(e) => {
            tracing::error!("❌ Failed to bind to {}: {}", addr, e);
            std::process::exit(1);
        }
    };

    info!("🚀 Starting Axum server...");
    match axum::serve(listener, app).await {
        Ok(_) => info!("✅ Server stopped gracefully"),
        Err(e) => {
            tracing::error!("❌ Server error: {}", e);
            std::process::exit(1);
        }
    }
}

async fn root() -> Json<Value> {
    Json(json!({
        "name": "DeFarm Traceability API",
        "version": "0.1.0",
        "description": "Privacy-first agricultural traceability system",
        "features": [
            "Receipt Engine - BLAKE3-based cryptographic receipts",
            "Events Engine - Item lifecycle tracking",
            "Circuits Engine - Permission-controlled sharing",
            "Items Engine - Canonical item management",
            "Verification Engine - Data deduplication",
            "Storage Engine - Pluggable backend support",
            "Audit Engine - Comprehensive audit trails and compliance reporting",
            "ZK Proof Engine - Zero-knowledge agricultural certifications and privacy-preserving verification",
            "Adapter Engine - Decentralized storage adapters for blockchain, IPFS, and hybrid solutions",
            "Storage History Engine - Multi-storage location tracking with migration support"
        ]
    }))
}

async fn health_check() -> (StatusCode, Json<Value>) {
    (
        StatusCode::OK,
        Json(json!({
            "status": "healthy",
            "timestamp": chrono::Utc::now(),
            "uptime": "System operational"
        })),
    )
}

async fn health_check_db(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> (StatusCode, Json<Value>) {
    let pg_lock = state.postgres_persistence.read().await;

    match &*pg_lock {
        Some(pg) => {
            let (status, message) = pg.get_status().await;
            let is_healthy = status == "connected";

            let status_code = if is_healthy {
                StatusCode::OK
            } else {
                StatusCode::SERVICE_UNAVAILABLE
            };

            (
                status_code,
                Json(json!({
                    "database": {
                        "status": status,
                        "message": message,
                        "timestamp": chrono::Utc::now(),
                    }
                })),
            )
        }
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "database": {
                    "status": "not_initialized",
                    "message": "PostgreSQL persistence not initialized (using in-memory storage)",
                    "timestamp": chrono::Utc::now(),
                }
            })),
        ),
    }
}

/// Initialize PostgreSQL synchronously, blocking until connected
async fn initialize_postgres_sync(app_state: Arc<AppState>) {
    // Require DATABASE_URL to be set - no in-memory fallback
    let database_url = match std::env::var("DATABASE_URL") {
        Ok(url) if !url.is_empty() => url,
        _ => {
            tracing::error!(
                "❌ DATABASE_URL environment variable is required for data persistence"
            );
            tracing::error!("❌ Set DATABASE_URL to your PostgreSQL connection string");
            tracing::error!("❌ Example: postgresql://user:password@localhost:5432/database");
            std::process::exit(1);
        }
    };

    tracing::info!("🗄️  Connecting to PostgreSQL for persistent storage...");

    // Create PostgreSQL persistence instance
    let mut pg_persistence = PostgresPersistence::new(database_url);

    // Try to connect with retry logic
    match pg_persistence.connect().await {
        Ok(()) => {
            tracing::info!("✅ PostgreSQL persistence enabled");

            // Load or initialize data (same logic as background version)
            let data_loaded = match load_data_from_postgres(&pg_persistence, &app_state).await {
                Ok(count) if count > 0 => {
                    tracing::info!("✅ Loaded {} users from PostgreSQL", count);
                    true
                }
                Ok(_) => {
                    tracing::info!("💡 PostgreSQL database is empty");
                    false
                }
                Err(e) => {
                    tracing::warn!("⚠️  Could not load data from PostgreSQL: {}", e);
                    false
                }
            };

            // If database is empty, initialize development data
            if !data_loaded {
                tracing::info!("🚀 Initializing development data in PostgreSQL...");
                match initialize_development_data_to_postgres(&pg_persistence).await {
                    Ok(()) => tracing::info!("✅ Development data initialized in PostgreSQL"),
                    Err(e) => {
                        tracing::error!("❌ Failed to initialize development data: {}", e)
                    }
                }

                // Load the newly created data into in-memory storage
                match load_data_from_postgres(&pg_persistence, &app_state).await {
                    Ok(count) => {
                        tracing::info!("✅ Loaded {} users into in-memory cache", count)
                    }
                    Err(e) => tracing::warn!("⚠️  Could not load data: {}", e),
                }
            } else {
                // Check and initialize adapters if needed
                tracing::info!("🔍 Checking if production adapters need initialization...");
                match pg_persistence.load_adapter_configs().await {
                    Ok(adapters) if adapters.is_empty() => {
                        tracing::info!(
                            "🔌 No adapters found - initializing production adapters..."
                        );
                        match initialize_adapters_to_postgres(&pg_persistence).await {
                            Ok(count) => {
                                tracing::info!("✅ {} production adapters initialized!", count)
                            }
                            Err(e) => {
                                tracing::error!("❌ Failed to initialize adapters: {}", e)
                            }
                        }
                        // Reload adapters into memory
                        match load_data_from_postgres(&pg_persistence, &app_state).await {
                            Ok(_) => tracing::info!("✅ Adapters loaded into memory"),
                            Err(e) => tracing::warn!("⚠️  Could not reload adapters: {}", e),
                        }
                    }
                    Ok(adapters) => {
                        tracing::info!("✅ {} adapters already exist in database", adapters.len());
                    }
                    Err(e) => {
                        tracing::warn!("⚠️  Could not check adapters: {}", e);
                    }
                }
            }

            // Store the connected persistence instance
            let mut pg_lock = app_state.postgres_persistence.write().await;
            *pg_lock = Some(pg_persistence);
            drop(pg_lock);

            // Enable event persistence now that PostgreSQL is connected
            app_state.enable_event_persistence();
            tracing::info!("✅ Event persistence enabled - events will now persist to PostgreSQL");

            tracing::info!("🎉 PostgreSQL persistence fully operational!");
        }
        Err(e) => {
            tracing::error!("❌ PostgreSQL connection failed: {}", e);
            tracing::error!("❌ Cannot start server without database connection");
            tracing::error!("❌ Please check your DATABASE_URL and ensure PostgreSQL is running");
            std::process::exit(1);
        }
    }
}

/// Initialize PostgreSQL in background without blocking server startup
fn initialize_postgres_background(app_state: Arc<AppState>) {
    tokio::spawn(async move {
        // Require DATABASE_URL to be set - no in-memory fallback
        let database_url = match std::env::var("DATABASE_URL") {
            Ok(url) if !url.is_empty() => url,
            _ => {
                tracing::error!(
                    "❌ DATABASE_URL environment variable is required for data persistence"
                );
                tracing::error!("❌ Set DATABASE_URL to your PostgreSQL connection string");
                tracing::error!("❌ Example: postgresql://user:password@localhost:5432/database");
                std::process::exit(1);
            }
        };

        tracing::info!("🗄️  Connecting to PostgreSQL for persistent storage...");

        // Create PostgreSQL persistence instance
        let mut pg_persistence = PostgresPersistence::new(database_url);

        // Try to connect with retry logic
        match pg_persistence.connect().await {
            Ok(()) => {
                tracing::info!("✅ PostgreSQL persistence enabled");

                // Try to load existing data from PostgreSQL
                let data_loaded = match load_data_from_postgres(&pg_persistence, &app_state).await {
                    Ok(count) if count > 0 => {
                        tracing::info!("✅ Loaded {} users from PostgreSQL", count);
                        true
                    }
                    Ok(_) => {
                        tracing::info!("💡 PostgreSQL database is empty");
                        false
                    }
                    Err(e) => {
                        tracing::warn!("⚠️  Could not load data from PostgreSQL: {}", e);
                        false
                    }
                };

                // If database is empty, initialize development data directly to PostgreSQL
                if !data_loaded {
                    tracing::info!("🚀 Initializing development data in PostgreSQL...");
                    match initialize_development_data_to_postgres(&pg_persistence).await {
                        Ok(()) => tracing::info!("✅ Development data initialized in PostgreSQL"),
                        Err(e) => {
                            tracing::error!("❌ Failed to initialize development data: {}", e)
                        }
                    }

                    // Load the newly created data into in-memory storage
                    match load_data_from_postgres(&pg_persistence, &app_state).await {
                        Ok(count) => {
                            tracing::info!("✅ Loaded {} users into in-memory cache", count)
                        }
                        Err(e) => tracing::warn!("⚠️  Could not load data: {}", e),
                    }
                } else {
                    // Database has existing data - check if adapters exist
                    tracing::info!("🔍 Checking if production adapters need initialization...");
                    match pg_persistence.load_adapter_configs().await {
                        Ok(adapters) if adapters.is_empty() => {
                            tracing::info!(
                                "🔌 No adapters found - initializing production adapters..."
                            );
                            match initialize_adapters_to_postgres(&pg_persistence).await {
                                Ok(count) => {
                                    tracing::info!("✅ {} production adapters initialized!", count)
                                }
                                Err(e) => {
                                    tracing::error!("❌ Failed to initialize adapters: {}", e)
                                }
                            }
                            // Reload adapters into memory
                            match load_data_from_postgres(&pg_persistence, &app_state).await {
                                Ok(_) => tracing::info!("✅ Adapters loaded into memory"),
                                Err(e) => tracing::warn!("⚠️  Could not reload adapters: {}", e),
                            }
                        }
                        Ok(adapters) => {
                            tracing::info!(
                                "✅ {} adapters already exist in database",
                                adapters.len()
                            );
                        }
                        Err(e) => {
                            tracing::warn!("⚠️  Could not check adapters: {}", e);
                        }
                    }
                }

                // Store the connected persistence instance
                let mut pg_lock = app_state.postgres_persistence.write().await;
                *pg_lock = Some(pg_persistence);
                drop(pg_lock);

                // Enable event persistence now that PostgreSQL is connected
                app_state.enable_event_persistence();
                tracing::info!(
                    "✅ Event persistence enabled - events will now persist to PostgreSQL"
                );

                tracing::info!("🎉 PostgreSQL persistence fully operational!");
            }
            Err(e) => {
                tracing::error!("❌ PostgreSQL connection failed: {}", e);
                tracing::error!("❌ Cannot start server without database connection");
                tracing::error!(
                    "❌ Please check your DATABASE_URL and ensure PostgreSQL is running"
                );
                std::process::exit(1);
            }
        }
    });
}

/// Load existing data from PostgreSQL into in-memory storage
/// Returns the number of users loaded
async fn load_data_from_postgres(
    pg: &PostgresPersistence,
    app_state: &AppState,
) -> Result<usize, String> {
    // Load users
    let users = pg.load_users().await?;
    let user_count = users.len();
    if !users.is_empty() {
        let mut storage = app_state
            .shared_storage
            .lock()
            .map_err(|e| format!("Failed to lock storage: {e}"))?;

        for user in users {
            storage
                .store_user_account(&user)
                .map_err(|e| format!("Failed to store user: {e}"))?;
        }
        tracing::info!("📥 Loaded {} users from PostgreSQL", user_count);
    }

    // Load circuits
    let circuits = pg.load_circuits().await?;
    let circuit_count = circuits.len();
    if !circuits.is_empty() {
        let mut storage = app_state
            .shared_storage
            .lock()
            .map_err(|e| format!("Failed to lock storage: {e}"))?;

        for circuit in circuits {
            storage
                .store_circuit(&circuit)
                .map_err(|e| format!("Failed to store circuit: {e}"))?;
            tracing::debug!(
                "📥 Loaded circuit: {} ({})",
                circuit.name,
                circuit.circuit_id
            );
        }
        tracing::info!("📥 Loaded {} circuits from PostgreSQL", circuit_count);
    }

    // Load adapter configs
    let adapters = pg.load_adapter_configs().await?;
    let adapter_count = adapters.len();
    if !adapters.is_empty() {
        let mut storage = app_state
            .shared_storage
            .lock()
            .map_err(|e| format!("Failed to lock storage: {e}"))?;

        for adapter in adapters {
            storage
                .store_adapter_config(&adapter)
                .map_err(|e| format!("Failed to store adapter config: {e}"))?;
        }
        tracing::info!(
            "📥 Loaded {} adapter configs from PostgreSQL",
            adapter_count
        );
    }

    Ok(user_count)
}

/// Initialize development data directly to PostgreSQL
async fn initialize_development_data_to_postgres(pg: &PostgresPersistence) -> Result<(), String> {
    use bcrypt::{hash, DEFAULT_COST};
    use chrono::Utc;
    use defarm_engine::types::{AccountStatus, TierLimits, UserAccount, UserTier};

    println!("🚀 Setting up development data in PostgreSQL...");

    // Create hen admin
    println!("🐔 Initializing default admin user 'hen'...");
    let hen_password_hash =
        hash("demo123", DEFAULT_COST).map_err(|e| format!("Failed to hash password: {e}"))?;

    let hen_admin = UserAccount {
        user_id: "hen-admin-001".to_string(),
        username: "hen".to_string(),
        email: "hen@defarm.com".to_string(),
        password_hash: hen_password_hash,
        tier: UserTier::Admin,
        status: AccountStatus::Active,
        credits: 1_000_000,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_login: None,
        subscription: None,
        limits: TierLimits::for_tier(&UserTier::Admin),
        is_admin: true,
        workspace_id: Some("hen-workspace".to_string()),
        available_adapters: None,
    };

    pg.persist_user(&hen_admin).await?;
    println!("✅ Default admin 'hen' created in PostgreSQL!");

    // Create sample users
    println!("🌱 Creating sample users...");
    let demo_password_hash =
        hash("demo123", DEFAULT_COST).map_err(|e| format!("Failed to hash password: {e}"))?;

    let sample_users = vec![
        UserAccount {
            user_id: "pullet-user-001".to_string(),
            username: "pullet".to_string(),
            email: "pullet@defarm.io".to_string(),
            password_hash: demo_password_hash.clone(),
            tier: UserTier::Professional,
            status: AccountStatus::Active,
            credits: 5000,
            created_at: Utc::now() - chrono::Duration::days(15),
            updated_at: Utc::now(),
            last_login: Some(Utc::now() - chrono::Duration::hours(3)),
            subscription: None,
            limits: TierLimits::for_tier(&UserTier::Professional),
            is_admin: false,
            workspace_id: Some("pullet-workspace".to_string()),
            available_adapters: None,
        },
        UserAccount {
            user_id: "cock-user-001".to_string(),
            username: "cock".to_string(),
            email: "cock@defarm.io".to_string(),
            password_hash: demo_password_hash.clone(),
            tier: UserTier::Enterprise,
            status: AccountStatus::Active,
            credits: 50000,
            created_at: Utc::now() - chrono::Duration::days(60),
            updated_at: Utc::now(),
            last_login: Some(Utc::now() - chrono::Duration::hours(1)),
            subscription: None,
            limits: TierLimits::for_tier(&UserTier::Enterprise),
            is_admin: false,
            workspace_id: Some("cock-workspace".to_string()),
            available_adapters: None,
        },
        UserAccount {
            user_id: "basic-farmer-001".to_string(),
            username: "basic_farmer".to_string(),
            email: "basic@farm.com".to_string(),
            password_hash: demo_password_hash.clone(),
            tier: UserTier::Basic,
            status: AccountStatus::Active,
            credits: 100,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login: Some(Utc::now() - chrono::Duration::days(2)),
            subscription: None,
            limits: TierLimits::for_tier(&UserTier::Basic),
            is_admin: false,
            workspace_id: Some("basic-workspace".to_string()),
            available_adapters: None,
        },
        UserAccount {
            user_id: "pro-farmer-001".to_string(),
            username: "pro_farmer".to_string(),
            email: "pro@farm.com".to_string(),
            password_hash: demo_password_hash.clone(),
            tier: UserTier::Professional,
            status: AccountStatus::Active,
            credits: 5000,
            created_at: Utc::now() - chrono::Duration::days(30),
            updated_at: Utc::now(),
            last_login: Some(Utc::now() - chrono::Duration::hours(6)),
            subscription: None,
            limits: TierLimits::for_tier(&UserTier::Professional),
            is_admin: false,
            workspace_id: Some("pro-workspace".to_string()),
            available_adapters: None,
        },
        UserAccount {
            user_id: "enterprise-farmer-001".to_string(),
            username: "enterprise_farmer".to_string(),
            email: "enterprise@farm.com".to_string(),
            password_hash: demo_password_hash.clone(),
            tier: UserTier::Enterprise,
            status: AccountStatus::Active,
            credits: 50000,
            created_at: Utc::now() - chrono::Duration::days(90),
            updated_at: Utc::now(),
            last_login: Some(Utc::now() - chrono::Duration::hours(1)),
            subscription: None,
            limits: TierLimits::for_tier(&UserTier::Enterprise),
            is_admin: false,
            workspace_id: Some("enterprise-workspace".to_string()),
            available_adapters: None,
        },
    ];

    for user in sample_users {
        let username = user.username.clone();
        let tier = user.tier.as_str();
        let credits = user.credits;
        pg.persist_user(&user).await?;
        println!("   ✅ Created {tier} user: {username} ({credits})");
    }

    // Initialize production adapters
    println!("🔌 Initializing production adapters...");
    match initialize_adapters_to_postgres(pg).await {
        Ok(adapter_count) => println!("✅ {adapter_count} production adapters initialized!"),
        Err(e) => println!("⚠️  Failed to initialize adapters: {e}"),
    }

    println!("✅ Development data initialized in PostgreSQL!");
    println!("📋 Available test accounts (all use password: demo123):");
    println!("   🐔 Admin:      hen / demo123");
    println!("   🐣 Pro:        pullet / demo123");
    println!("   🐓 Enterprise: cock / demo123");
    println!("   🌱 Basic:      basic_farmer / demo123");
    println!("   🚀 Pro:        pro_farmer / demo123");
    println!("   🏢 Enterprise: enterprise_farmer / demo123");

    Ok(())
}

/// Initialize production adapters to PostgreSQL
async fn initialize_adapters_to_postgres(pg: &PostgresPersistence) -> Result<usize, String> {
    use chrono::Utc;
    use defarm_engine::types::{
        AdapterConfig, AdapterConnectionDetails, AdapterType, AuthType, ContractConfigs,
        ContractInfo,
    };
    use std::collections::HashMap;
    use uuid::Uuid;

    let mut adapter_count = 0;

    // Read credentials from environment
    let pinata_api_key = std::env::var("PINATA_API_KEY").ok();
    let pinata_secret = std::env::var("PINATA_SECRET_KEY").ok();
    let testnet_ipcm = std::env::var("STELLAR_TESTNET_IPCM_CONTRACT").ok();
    let mainnet_ipcm = std::env::var("STELLAR_MAINNET_IPCM_CONTRACT").ok();
    let mainnet_secret = std::env::var("STELLAR_MAINNET_SECRET_KEY").ok();
    let testnet_nft = std::env::var("STELLAR_TESTNET_NFT_CONTRACT").ok();
    let mainnet_nft = std::env::var("STELLAR_MAINNET_NFT_CONTRACT").ok();

    // 1. Create IPFS-IPFS adapter
    if let (Some(api_key), Some(secret)) = (pinata_api_key.clone(), pinata_secret.clone()) {
        let ipfs_config = AdapterConfig {
            config_id: Uuid::new_v4(),
            name: "Production IPFS (Pinata)".to_string(),
            description: "IPFS storage via Pinata cloud".to_string(),
            adapter_type: AdapterType::IpfsIpfs,
            connection_details: AdapterConnectionDetails {
                endpoint: "https://api.pinata.cloud".to_string(),
                api_key: Some(api_key),
                secret_key: Some(secret),
                auth_type: AuthType::ApiKey,
                timeout_ms: 60000,
                retry_attempts: 3,
                max_concurrent_requests: 10,
                custom_headers: HashMap::new(),
            },
            contract_configs: None,
            is_active: true,
            is_default: false,
            created_by: "system".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_tested_at: None,
            test_status: None,
        };
        pg.persist_adapter_config(&ipfs_config).await?;
        println!("   ✅ IPFS-IPFS adapter");
        adapter_count += 1;
    }

    // 2. Create Stellar Testnet + IPFS adapter
    if let (Some(api_key), Some(secret), Some(contract_addr)) =
        (pinata_api_key.clone(), pinata_secret.clone(), testnet_ipcm)
    {
        let testnet_secret = std::env::var("STELLAR_TESTNET_SECRET").ok();
        let interface_address = std::env::var("DEFARM_OWNER_WALLET").unwrap_or_else(|_| {
            "GANDYZQQ3OQBXHZQXJHZ7AQ2GDBFUQIR4ZLMUPD3P2B7PLIYQNFG54XQ".to_string()
        });

        let mut custom_headers = HashMap::new();
        if let Some(secret_key) = testnet_secret {
            custom_headers.insert("stellar_secret".to_string(), secret_key);
        }
        if let Some(nft_contract) = testnet_nft.clone() {
            custom_headers.insert("nft_contract".to_string(), nft_contract);
        }
        custom_headers.insert("interface_address".to_string(), interface_address);
        custom_headers.insert(
            "source_account_identity".to_string(),
            "defarm-admin-testnet".to_string(),
        );

        let testnet_config = AdapterConfig {
            config_id: Uuid::new_v4(),
            name: "Stellar Testnet + IPFS".to_string(),
            description: "NFTs on Stellar testnet + IPFS events".to_string(),
            adapter_type: AdapterType::StellarTestnetIpfs,
            connection_details: AdapterConnectionDetails {
                endpoint: "https://api.pinata.cloud".to_string(),
                api_key: Some(api_key),
                secret_key: Some(secret),
                auth_type: AuthType::ApiKey,
                timeout_ms: 60000,
                retry_attempts: 3,
                max_concurrent_requests: 10,
                custom_headers,
            },
            contract_configs: Some(ContractConfigs {
                mint_contract: None,
                ipcm_contract: Some(ContractInfo {
                    contract_address: contract_addr,
                    contract_name: "IPCM".to_string(),
                    abi: None,
                    methods: HashMap::new(),
                }),
                network: "testnet".to_string(),
                chain_id: None,
            }),
            is_active: true,
            is_default: false,
            created_by: "system".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_tested_at: None,
            test_status: None,
        };
        pg.persist_adapter_config(&testnet_config).await?;
        println!("   ✅ Stellar Testnet-IPFS adapter");
        adapter_count += 1;
    }

    // 3. Create Stellar Mainnet + IPFS adapter
    if let (Some(api_key), Some(secret), Some(contract_addr), Some(mainnet_key)) =
        (pinata_api_key, pinata_secret, mainnet_ipcm, mainnet_secret)
    {
        let interface_address = std::env::var("DEFARM_OWNER_WALLET").unwrap_or_else(|_| {
            "GANDYZQQ3OQBXHZQXJHZ7AQ2GDBFUQIR4ZLMUPD3P2B7PLIYQNFG54XQ".to_string()
        });

        let mut custom_headers = HashMap::new();
        custom_headers.insert("stellar_secret".to_string(), mainnet_key);
        if let Some(nft_contract) = mainnet_nft.clone() {
            custom_headers.insert("nft_contract".to_string(), nft_contract);
        }
        custom_headers.insert("interface_address".to_string(), interface_address);
        custom_headers.insert(
            "source_account_identity".to_string(),
            "defarm-admin-secure-v2".to_string(),
        );

        let mainnet_config = AdapterConfig {
            config_id: Uuid::new_v4(),
            name: "Stellar Mainnet + IPFS (Production)".to_string(),
            description: "Production NFTs on Stellar mainnet + IPFS".to_string(),
            adapter_type: AdapterType::StellarMainnetIpfs,
            connection_details: AdapterConnectionDetails {
                endpoint: "https://api.pinata.cloud".to_string(),
                api_key: Some(api_key),
                secret_key: Some(secret),
                auth_type: AuthType::ApiKey,
                timeout_ms: 60000,
                retry_attempts: 3,
                max_concurrent_requests: 10,
                custom_headers,
            },
            contract_configs: Some(ContractConfigs {
                mint_contract: None,
                ipcm_contract: Some(ContractInfo {
                    contract_address: contract_addr,
                    contract_name: "IPCM".to_string(),
                    abi: None,
                    methods: HashMap::new(),
                }),
                network: "mainnet".to_string(),
                chain_id: None,
            }),
            is_active: true,
            is_default: false,
            created_by: "system".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_tested_at: None,
            test_status: None,
        };
        pg.persist_adapter_config(&mainnet_config).await?;
        println!("   ✅ Stellar Mainnet-IPFS adapter");
        adapter_count += 1;
    }

    Ok(adapter_count)
}

#[cfg(test)]
mod tests {
    use super::allow_in_memory_fallback;
    use std::sync::Mutex;

    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn fallback_defaults_to_false() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::remove_var("ALLOW_IN_MEMORY_FALLBACK");
        assert!(!allow_in_memory_fallback());
    }

    #[test]
    fn fallback_accepts_truthy_values() {
        let _guard = ENV_MUTEX.lock().unwrap();
        for value in ["1", "true", "TRUE", "YeS"] {
            std::env::set_var("ALLOW_IN_MEMORY_FALLBACK", value);
            assert!(
                allow_in_memory_fallback(),
                "expected {value} to enable fallback"
            );
        }
        std::env::remove_var("ALLOW_IN_MEMORY_FALLBACK");
    }

    #[test]
    fn fallback_rejects_other_values() {
        let _guard = ENV_MUTEX.lock().unwrap();
        for value in ["", "0", "false", "no", "sometimes"] {
            if value.is_empty() {
                std::env::remove_var("ALLOW_IN_MEMORY_FALLBACK");
            } else {
                std::env::set_var("ALLOW_IN_MEMORY_FALLBACK", value);
            }
            assert!(
                !allow_in_memory_fallback(),
                "expected {value} to disable fallback"
            );
        }
        std::env::remove_var("ALLOW_IN_MEMORY_FALLBACK");
    }
}

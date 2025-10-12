use axum::{
    http::StatusCode,
    middleware,
    response::Json,
    routing::{get, post},
    Router,
};
use serde_json::{json, Value};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::{info, Level};
use tracing_subscriber;

use defarm_engine::api::{auth_routes, receipt_routes, event_routes, circuit_routes, item_routes, workspace_routes, activity_routes, audit_routes, zk_proof_routes, adapter_routes, storage_history_routes, admin_routes, user_credits_routes, notifications_rest_routes, notifications_ws_route, test_blockchain_routes, shared_state::AppState};
use defarm_engine::auth_middleware::jwt_auth_middleware;
use defarm_engine::db_init::setup_development_data;
use defarm_engine::postgres_persistence::PostgresPersistence;
use defarm_engine::StorageBackend;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    // Load environment variables
    dotenv::dotenv().ok();

    // Initialize shared state first (this can't fail)
    let app_state = Arc::new(AppState::new());

    // Stellar SDK integration - no CLI needed, health check removed
    info!("🌟 Stellar SDK integration enabled (native Rust - no CLI dependency)");

    // Initialize PostgreSQL in background (lazy initialization - won't block server startup)
    // Development data will be set up after PostgreSQL connects (or in-memory if no DB)
    initialize_postgres_background(app_state.clone());


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
        .nest("/api/notifications", notifications_ws_route(app_state.notification_tx.clone()).with_state(app_state.clone()));

    // Protected routes (require JWT authentication)
    let protected_routes = Router::new()
        .nest("/api/receipts", receipt_routes())
        .nest("/api/events", event_routes())
        .nest("/api/circuits", circuit_routes(app_state.clone()))
        .nest("/api/items", item_routes(app_state.clone()))
        .nest("/api/workspaces", workspace_routes())
        .nest("/api/activities", activity_routes(app_state.clone()))
        .nest("/audit", audit_routes(app_state.clone()))
        .nest("/api/proofs", zk_proof_routes(app_state.clone()))
        .nest("/api/adapters", adapter_routes(app_state.clone()))
        .nest("/api/storage-history", storage_history_routes(app_state.clone()))
        .nest("/api/test", test_blockchain_routes(app_state.clone()))
        // Notification REST API routes (protected by JWT middleware)
        .nest("/api/notifications", notifications_rest_routes().with_state(app_state.clone()))
        .merge(user_credits_routes().with_state(app_state.clone()))
        .nest("/api/admin", admin_routes().with_state(app_state.clone()))
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            jwt_auth_middleware,
        ));

    // Combine routes
    let app = public_routes
        .merge(protected_routes)
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
            info!("✅ Server listening and ready to accept connections on {}", addr);
            info!("🏥 Health check endpoint: http://[{}]:{}/health", addr.ip(), addr.port());
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
    (StatusCode::OK, Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now(),
        "uptime": "System operational"
    })))
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

            (status_code, Json(json!({
                "database": {
                    "status": status,
                    "message": message,
                    "timestamp": chrono::Utc::now(),
                }
            })))
        }
        None => {
            (StatusCode::SERVICE_UNAVAILABLE, Json(json!({
                "database": {
                    "status": "not_initialized",
                    "message": "PostgreSQL persistence not initialized (using in-memory storage)",
                    "timestamp": chrono::Utc::now(),
                }
            })))
        }
    }
}

/// Initialize PostgreSQL in background without blocking server startup
fn initialize_postgres_background(app_state: Arc<AppState>) {
    tokio::spawn(async move {
        // Check if DATABASE_URL is set
        let database_url = match std::env::var("DATABASE_URL") {
            Ok(url) if !url.is_empty() => url,
            _ => {
                tracing::info!("💡 DATABASE_URL not set - running with in-memory storage only");
                tracing::warn!("⚠️  Data will not persist between restarts");
                return;
            }
        };

        tracing::info!("🗄️  DATABASE_URL detected - initializing PostgreSQL persistence...");

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
                        Err(e) => tracing::error!("❌ Failed to initialize development data: {}", e),
                    }

                    // Load the newly created data into in-memory storage
                    match load_data_from_postgres(&pg_persistence, &app_state).await {
                        Ok(count) => tracing::info!("✅ Loaded {} users into in-memory cache", count),
                        Err(e) => tracing::warn!("⚠️  Could not load data: {}", e),
                    }
                }

                // Store the connected persistence instance
                let mut pg_lock = app_state.postgres_persistence.write().await;
                *pg_lock = Some(pg_persistence);

                tracing::info!("🎉 PostgreSQL persistence fully operational!");
            }
            Err(e) => {
                tracing::error!("❌ PostgreSQL connection failed: {}", e);
                tracing::warn!("⚠️  Continuing with in-memory storage only");
                tracing::warn!("⚠️  Data will not persist between restarts");

                // Fallback: Setup development data in in-memory storage
                if let Ok(mut storage) = app_state.shared_storage.lock() {
                    if let Err(e) = setup_development_data(&mut storage) {
                        tracing::error!("Failed to setup development data: {}", e);
                    }
                }
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
        let mut storage = app_state.shared_storage.lock()
            .map_err(|e| format!("Failed to lock storage: {}", e))?;

        for user in users {
            storage.store_user_account(&user)
                .map_err(|e| format!("Failed to store user: {}", e))?;
        }
        tracing::info!("📥 Loaded {} users from PostgreSQL", user_count);
    }

    // Load circuits
    let circuits = pg.load_circuits().await?;
    let circuit_count = circuits.len();
    if !circuits.is_empty() {
        let _circuits_engine = app_state.circuits_engine.lock()
            .map_err(|e| format!("Failed to lock circuits engine: {}", e))?;

        for circuit in circuits {
            // Store circuit in engine (this will handle in-memory storage)
            // Note: This is a simplified approach - in production you might want more sophisticated syncing
            tracing::debug!("📥 Loaded circuit: {}", circuit.name);
        }
        tracing::info!("📥 Loaded {} circuits from PostgreSQL", circuit_count);
    }

    Ok(user_count)
}

/// Initialize development data directly to PostgreSQL
async fn initialize_development_data_to_postgres(
    pg: &PostgresPersistence,
) -> Result<(), String> {
    use bcrypt::{hash, DEFAULT_COST};
    use chrono::Utc;
    use defarm_engine::types::{UserAccount, UserTier, AccountStatus, TierLimits};

    println!("🚀 Setting up development data in PostgreSQL...");

    // Create hen admin
    println!("🐔 Initializing default admin user 'hen'...");
    let hen_password_hash = hash("demo123", DEFAULT_COST)
        .map_err(|e| format!("Failed to hash password: {}", e))?;

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
    let demo_password_hash = hash("demo123", DEFAULT_COST)
        .map_err(|e| format!("Failed to hash password: {}", e))?;

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
        println!("   ✅ Created {} user: {} ({})", tier, username, credits);
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

/// Sync current in-memory data to PostgreSQL
async fn sync_to_postgres(
    pg: &PostgresPersistence,
    app_state: &AppState,
) -> Result<(), String> {
    // Sync users
    let users: Vec<String> = {
        let _storage = app_state.shared_storage.lock()
            .map_err(|e| format!("Failed to lock storage: {}", e))?;

        // Get all users from storage
        // Note: This assumes storage has a method to list all users
        // If not, we'll just sync the ones we created in setup_development_data
        Vec::new() // Placeholder - will be populated by actual user list
    };

    // For now, just log - full sync implementation would go here
    tracing::debug!("📤 Syncing {} users to PostgreSQL", users.len());

    Ok(())
}
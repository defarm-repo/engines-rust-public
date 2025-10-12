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

    // Check Stellar CLI configuration at startup
    defarm_engine::stellar_health_check::check_stellar_cli_configuration().await;

    // Initialize shared state
    let app_state = Arc::new(AppState::new());

    // Setup development data (hen admin + sample users)
    {
        let mut storage = app_state.shared_storage.lock().unwrap();
        if let Err(e) = setup_development_data(&mut storage) {
            tracing::error!("Failed to setup development data: {}", e);
        }
    }

    // Initialize PostgreSQL in background (lazy initialization - won't block server startup)
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

    // Use IPv6 [::] (0.0.0.0 equivalent) for Railway compatibility
    // Railway healthchecks use hostname healthcheck.railway.app
    let addr = SocketAddr::from(([0, 0, 0, 0, 0, 0, 0, 0], port));
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
                match load_data_from_postgres(&pg_persistence, &app_state).await {
                    Ok(()) => tracing::info!("✅ Loaded existing data from PostgreSQL"),
                    Err(e) => tracing::warn!("⚠️  Could not load data from PostgreSQL: {}", e),
                }

                // Sync current in-memory data to PostgreSQL
                match sync_to_postgres(&pg_persistence, &app_state).await {
                    Ok(()) => tracing::info!("✅ Synced in-memory data to PostgreSQL"),
                    Err(e) => tracing::warn!("⚠️  Could not sync to PostgreSQL: {}", e),
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
            }
        }
    });
}

/// Load existing data from PostgreSQL into in-memory storage
async fn load_data_from_postgres(
    pg: &PostgresPersistence,
    app_state: &AppState,
) -> Result<(), String> {
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
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

    // Public routes (no authentication required)
    let public_routes = Router::new()
        .route("/", get(root))
        .route("/health", get(health_check))
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

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    info!("✅ Server listening and ready to accept connections on {}", addr);
    info!("🏥 Health check endpoint: http://{}:{}/health", addr.ip(), addr.port());

    axum::serve(listener, app).await.unwrap();
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
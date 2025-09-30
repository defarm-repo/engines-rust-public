use axum::{
    http::StatusCode,
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

use defarm_engine::api::{auth_routes, receipt_routes, event_routes, circuit_routes, item_routes, workspace_routes, activity_routes, audit_routes, zk_proof_routes, adapter_routes, shared_state::AppState};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    // Load environment variables
    dotenv::dotenv().ok();

    // Initialize shared state
    let app_state = Arc::new(AppState::new());

    // Build our application with routes
    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health_check))
        .nest("/api/auth", auth_routes())
        .nest("/api/receipts", receipt_routes())
        .nest("/api/events", event_routes())
        .nest("/api/circuits", circuit_routes(app_state.clone()))
        .nest("/api/items", item_routes(app_state.clone()))
        .nest("/api/workspaces", workspace_routes())
        .nest("/api/activities", activity_routes(app_state.clone()))
        .nest("/audit", audit_routes(app_state.clone()))
        .nest("/api/proofs", zk_proof_routes(app_state.clone()))
        .nest("/api/adapters", adapter_routes(app_state.clone()))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    info!("DeFarm API server starting on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
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
            "Adapter Engine - Decentralized storage adapters for blockchain, IPFS, and hybrid solutions"
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
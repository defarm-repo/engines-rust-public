mod api;
mod engine;

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use api::AppState;
use engine::DfidEngine;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "dfid_service=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    dotenv::dotenv().ok();

    let engine = if let Ok(redis_url) = std::env::var("REDIS_URL") {
        #[cfg(feature = "redis-persistence")]
        {
            tracing::info!("Initializing DFID engine with Redis persistence: {}", redis_url);
            match DfidEngine::new_with_redis(&redis_url).await {
                Ok(engine) => Arc::new(engine),
                Err(e) => {
                    tracing::error!("Failed to initialize Redis connection: {}. Falling back to in-memory.", e);
                    Arc::new(DfidEngine::new())
                }
            }
        }
        #[cfg(not(feature = "redis-persistence"))]
        {
            tracing::warn!("Redis URL provided but redis-persistence feature is disabled");
            Arc::new(DfidEngine::new())
        }
    } else {
        tracing::info!("Initializing DFID engine without Redis persistence");
        Arc::new(DfidEngine::new())
    };

    let state = Arc::new(AppState {
        engine: engine.clone(),
    });

    #[cfg(feature = "redis-persistence")]
    {
        let persist_engine = engine.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
            loop {
                interval.tick().await;
                if let Err(e) = persist_engine.persist_sequence().await {
                    tracing::error!("Failed to persist sequence to Redis: {}", e);
                }
            }
        });
    }

    let app = Router::new()
        .route("/dfid/generate", post(api::generate_dfid))
        .route("/dfid/batch", post(api::generate_batch))
        .route("/dfid/:id/validate", get(api::validate_dfid))
        .route("/health", get(api::health_check))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state);

    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3001);

    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("DFID Service listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind to address");

    axum::serve(listener, app)
        .await
        .expect("Server failed to start");
}

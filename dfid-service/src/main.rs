mod api;
mod engine;
mod metrics;

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use api::AppState;
use engine::DfidEngine;

#[derive(OpenApi)]
#[openapi(
    paths(
        api::generate_dfid,
        api::generate_batch,
        api::validate_dfid,
        api::health_check,
    ),
    components(
        schemas(
            api::GenerateRequest,
            api::GenerateResponse,
            api::ValidateResponse,
            api::HealthResponse,
        )
    ),
    tags(
        (name = "dfid", description = "DFID generation and validation endpoints"),
        (name = "health", description = "Service health endpoints")
    ),
    info(
        title = "DFID Service API",
        version = "1.0.0",
        description = "DeFarm ID (DFID) Generation and Validation Service\n\nProvides globally unique identifiers for agricultural items with:\n- Per-day sequence numbering\n- BLAKE3 24-bit checksums\n- Redis persistence\n- Automatic backups",
        contact(
            name = "DeFarm",
            url = "https://defarm.net"
        )
    )
)]
struct ApiDoc;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "dfid_service=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Initialize metrics
    metrics::init_metrics();

    // Load environment variables
    dotenv::dotenv().ok();

    // Initialize DFID engine
    let engine = if let Ok(redis_url) = std::env::var("REDIS_URL") {
        #[cfg(feature = "redis-persistence")]
        {
            tracing::info!(
                "Initializing DFID engine with Redis persistence: {}",
                redis_url
            );
            match DfidEngine::new_with_redis(&redis_url).await {
                Ok(engine) => Arc::new(engine),
                Err(e) => {
                    tracing::error!(
                        "Failed to initialize Redis connection: {}. Falling back to in-memory.",
                        e
                    );
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

    // Start periodic sequence persistence task
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

    // Configure rate limiting (2 req/s with burst of 10)
    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(2)
            .burst_size(10)
            .finish()
            .unwrap(),
    );

    let rate_limit_layer = GovernorLayer {
        config: governor_conf,
    };

    // Build router with rate limiting
    let app = Router::new()
        .route("/dfid/generate", post(api::generate_dfid))
        .route("/dfid/batch", post(api::generate_batch))
        .route("/dfid/:id/validate", get(api::validate_dfid))
        .route("/health", get(api::health_check))
        .route("/metrics", get(api::metrics))
        .layer(
            ServiceBuilder::new()
                .layer(rate_limit_layer)
                .layer(
                    CorsLayer::new()
                        .allow_origin(Any)
                        .allow_methods(Any)
                        .allow_headers(Any),
                ),
        )
        .merge(
            SwaggerUi::new("/swagger-ui")
                .url("/api-docs/openapi.json", ApiDoc::openapi()),
        )
        .with_state(state);

    // Get port from environment or use default
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3001);

    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("🚀 DFID Service listening on {}", addr);
    tracing::info!("📊 Metrics available at http://{}/metrics", addr);
    tracing::info!("📚 API docs available at http://{}/swagger-ui", addr);
    tracing::info!("🔒 Rate limit: 2 req/s with burst of 10");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind to address");

    axum::serve(listener, app)
        .await
        .expect("Server failed to start");
}

use crate::api::notifications::NotificationMessage;
use crate::api_key_engine::ApiKeyEngine;
use crate::api_key_storage::ApiKeyStorage;
use crate::logging::LoggingEngine;
use crate::postgres_persistence::PostgresPersistence;
use crate::rate_limiter::RateLimiter;
use crate::storage_history_reader::StorageHistoryReader;
use crate::{
    ActivityEngine, AuditEngine, CircuitsEngine, EventsEngine, InMemoryStorage, ItemsEngine,
    NotificationEngine,
};
use std::sync::{Arc, Mutex};
use tokio::sync::{broadcast, RwLock};

pub struct AppState<S: ApiKeyStorage = crate::api_key_storage::InMemoryApiKeyStorage> {
    pub circuits_engine: Arc<Mutex<CircuitsEngine<InMemoryStorage>>>,
    pub items_engine: Arc<Mutex<ItemsEngine<Arc<Mutex<InMemoryStorage>>>>>,
    pub events_engine: Arc<Mutex<EventsEngine<InMemoryStorage>>>,
    pub audit_engine: AuditEngine<InMemoryStorage>,
    pub activity_engine: Arc<Mutex<ActivityEngine<InMemoryStorage>>>,
    pub shared_storage: Arc<Mutex<InMemoryStorage>>,
    pub storage_history_reader: StorageHistoryReader<InMemoryStorage>,
    pub logging: Arc<Mutex<LoggingEngine>>,
    pub api_key_engine: Arc<ApiKeyEngine>,
    pub api_key_storage: Arc<S>,
    pub rate_limiter: Arc<RateLimiter>,
    pub notification_engine: Arc<Mutex<NotificationEngine<InMemoryStorage>>>,
    pub notification_tx: broadcast::Sender<NotificationMessage>,
    pub jwt_secret: String,
    /// Optional PostgreSQL persistence layer - lazy initialized
    pub postgres_persistence: Arc<RwLock<Option<PostgresPersistence>>>,
}

impl Default for AppState<crate::api_key_storage::InMemoryApiKeyStorage> {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState<crate::api_key_storage::InMemoryApiKeyStorage> {
    pub fn new() -> Self {
        // Create a single shared storage instance for all engines
        // This fixes the storage isolation issue - items created via Items API
        // will now be accessible to Circuits API and audit events will be shared
        let shared_storage = Arc::new(Mutex::new(InMemoryStorage::new()));

        // All engines now use the same shared storage backend
        // The engines will wrap the storage in Arc<Mutex<>> internally
        let storage_for_circuits = Arc::clone(&shared_storage);
        let storage_for_items = Arc::clone(&shared_storage);
        let storage_for_events = Arc::clone(&shared_storage);
        let storage_for_audit = Arc::clone(&shared_storage);
        let storage_for_activity = Arc::clone(&shared_storage);

        let circuits_engine = Arc::new(Mutex::new(CircuitsEngine::new(storage_for_circuits)));
        let items_engine = Arc::new(Mutex::new(ItemsEngine::new(storage_for_items)));
        let events_engine = Arc::new(Mutex::new(EventsEngine::new(storage_for_events)));
        let audit_engine = AuditEngine::new(storage_for_audit);
        let activity_engine = Arc::new(Mutex::new(ActivityEngine::new(storage_for_activity)));

        let storage_for_history = Arc::clone(&shared_storage);
        let storage_history_reader = StorageHistoryReader::new(storage_for_history);

        // Initialize notification engine with Arc<Mutex<>> wrapped storage
        let storage_for_notifications = Arc::clone(&shared_storage);
        let notification_engine = Arc::new(Mutex::new(NotificationEngine::new(
            storage_for_notifications.clone(),
        )));

        // Create broadcast channel for WebSocket notifications
        let (notification_tx, _notification_rx) = broadcast::channel(1000);

        // Initialize API key infrastructure
        let logging = Arc::new(Mutex::new(LoggingEngine::new()));
        let api_key_engine = Arc::new(ApiKeyEngine::new());
        let api_key_storage = Arc::new(crate::api_key_storage::InMemoryApiKeyStorage::new());
        let rate_limiter = Arc::new(RateLimiter::new());

        // Get JWT secret from environment - required for security
        let jwt_secret = std::env::var("JWT_SECRET")
            .expect("JWT_SECRET environment variable must be set. Please set a secure secret key for JWT authentication.");

        if jwt_secret.len() < 32 {
            panic!("JWT_SECRET must be at least 32 characters long for security");
        }

        Self {
            circuits_engine,
            items_engine,
            events_engine,
            audit_engine,
            activity_engine,
            shared_storage,
            storage_history_reader,
            logging,
            api_key_engine,
            api_key_storage,
            rate_limiter,
            notification_engine,
            notification_tx,
            jwt_secret,
            postgres_persistence: Arc::new(RwLock::new(None)),
        }
    }

    /// Call this after PostgreSQL connection is established to enable event persistence
    pub fn enable_event_persistence(&self) {
        if let Ok(mut engine) = self.events_engine.lock() {
            *engine = EventsEngine::new(Arc::clone(&self.shared_storage))
                .with_postgres(Arc::clone(&self.postgres_persistence));
        }
    }
}

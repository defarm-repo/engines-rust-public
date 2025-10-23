use crate::api::notifications::NotificationMessage;
use crate::api_key_engine::ApiKeyEngine;
use crate::api_key_storage::ApiKeyStorage;
use crate::logging::LoggingEngine;
use crate::postgres_persistence::PostgresPersistence;
use crate::postgres_storage_with_cache::PostgresStorageWithCache;
use crate::rate_limiter::RateLimiter;
use crate::redis_cache::RedisCache;
use crate::storage::{StorageBackend, StorageError};
use crate::storage_history_reader::StorageHistoryReader;
use crate::{
    ActivityEngine, AuditEngine, CircuitsEngine, EventsEngine, InMemoryStorage, ItemsEngine,
    NotificationEngine,
};
use std::sync::{Arc, Mutex};
use tokio::sync::{broadcast, RwLock};

/// Production AppState using PostgreSQL primary storage with Redis cache
pub type ProductionAppState =
    AppState<PostgresStorageWithCache, crate::api_key_storage::InMemoryApiKeyStorage>;

/// AppState with generic storage backend
///
/// Type Parameters:
/// - B: Raw storage backend type (PostgresStorageWithCache, InMemoryStorage, etc.)
///     Will be wrapped in Arc<Mutex<B>> and passed to engines
/// - K: API Key storage type (InMemoryApiKeyStorage by default)
pub struct AppState<
    B: StorageBackend + 'static = PostgresStorageWithCache,
    K: ApiKeyStorage = crate::api_key_storage::InMemoryApiKeyStorage,
> where
    Arc<Mutex<B>>: StorageBackend,
{
    pub circuits_engine: Arc<Mutex<CircuitsEngine<B>>>,
    pub items_engine: Arc<Mutex<ItemsEngine<Arc<Mutex<B>>>>>,
    pub events_engine: Arc<Mutex<EventsEngine<B>>>,
    pub audit_engine: AuditEngine<B>,
    pub activity_engine: Arc<Mutex<ActivityEngine<B>>>,
    pub shared_storage: Arc<Mutex<B>>,
    pub storage_history_reader: StorageHistoryReader<B>,
    pub logging: Arc<Mutex<LoggingEngine>>,
    pub api_key_engine: Arc<ApiKeyEngine>,
    pub api_key_storage: Arc<K>,
    pub rate_limiter: Arc<RateLimiter>,
    pub notification_engine: Arc<Mutex<NotificationEngine<B>>>,
    pub notification_tx: broadcast::Sender<NotificationMessage>,
    pub jwt_secret: String,
    /// Optional PostgreSQL persistence layer - lazy initialized
    pub postgres_persistence: Arc<RwLock<Option<PostgresPersistence>>>,
    /// Optional Redis cache layer for horizontal scaling
    pub redis_cache: Arc<RwLock<Option<RedisCache>>>,
}

impl AppState<PostgresStorageWithCache, crate::api_key_storage::InMemoryApiKeyStorage> {
    /// Create AppState with PostgreSQL primary storage (with optional Redis cache)
    pub fn new_with_postgres(storage: Arc<Mutex<PostgresStorageWithCache>>) -> Self {
        // All engines use the same shared storage backend (cloning Arc, not the data)
        let storage_for_circuits = storage.clone();
        let storage_for_items = storage.clone();
        let storage_for_events = storage.clone();
        let storage_for_audit = storage.clone();
        let storage_for_activity = storage.clone();
        let storage_for_notifications = storage.clone();
        let storage_for_history = storage.clone();

        let circuits_engine = Arc::new(Mutex::new(
            CircuitsEngine::<PostgresStorageWithCache>::new(storage_for_circuits),
        ));
        let items_engine = Arc::new(Mutex::new(ItemsEngine::<
            Arc<Mutex<PostgresStorageWithCache>>,
        >::new(storage_for_items)));
        let events_engine = Arc::new(Mutex::new(EventsEngine::<PostgresStorageWithCache>::new(
            storage_for_events,
        )));
        let audit_engine = AuditEngine::<PostgresStorageWithCache>::new(storage_for_audit);
        let activity_engine = Arc::new(Mutex::new(
            ActivityEngine::<PostgresStorageWithCache>::new(storage_for_activity),
        ));
        let notification_engine = Arc::new(Mutex::new(NotificationEngine::<
            PostgresStorageWithCache,
        >::new(storage_for_notifications)));
        let storage_history_reader =
            StorageHistoryReader::<PostgresStorageWithCache>::new(storage_for_history);

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
            shared_storage: storage,
            storage_history_reader,
            logging,
            api_key_engine,
            api_key_storage,
            rate_limiter,
            notification_engine,
            notification_tx,
            jwt_secret,
            postgres_persistence: Arc::new(RwLock::new(None)),
            redis_cache: Arc::new(RwLock::new(None)),
        }
    }

    /// Enable PostgreSQL persistence for events engine
    pub fn enable_event_persistence(&self) {
        if let Ok(mut engine) = self.events_engine.lock() {
            *engine = EventsEngine::new(Arc::clone(&self.shared_storage))
                .with_postgres(Arc::clone(&self.postgres_persistence));
        }
    }

    /// Enable PostgreSQL persistence for user activity tracking
    pub fn enable_activity_persistence(&self) {
        if let Ok(mut engine) = self.activity_engine.lock() {
            engine.set_postgres(Arc::clone(&self.postgres_persistence));
        }
    }

    /// Enable PostgreSQL persistence for circuit activity logs
    pub fn enable_circuit_activity_persistence(&self) {
        if let Ok(mut engine) = self.circuits_engine.lock() {
            engine.set_postgres(Arc::clone(&self.postgres_persistence));
        }
    }
}

impl Default for AppState<InMemoryStorage, crate::api_key_storage::InMemoryApiKeyStorage> {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState<InMemoryStorage, crate::api_key_storage::InMemoryApiKeyStorage> {
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

        let circuits_engine = Arc::new(Mutex::new(CircuitsEngine::<InMemoryStorage>::new(
            storage_for_circuits,
        )));
        let items_engine = Arc::new(Mutex::new(ItemsEngine::<Arc<Mutex<InMemoryStorage>>>::new(
            storage_for_items,
        )));
        let events_engine = Arc::new(Mutex::new(EventsEngine::<InMemoryStorage>::new(
            storage_for_events,
        )));
        let audit_engine = AuditEngine::<InMemoryStorage>::new(storage_for_audit);
        let activity_engine = Arc::new(Mutex::new(ActivityEngine::<InMemoryStorage>::new(
            storage_for_activity,
        )));

        let storage_for_history = Arc::clone(&shared_storage);
        let storage_history_reader =
            StorageHistoryReader::<InMemoryStorage>::new(storage_for_history);

        // Initialize notification engine with Arc<Mutex<>> wrapped storage
        let storage_for_notifications = Arc::clone(&shared_storage);
        let notification_engine = Arc::new(Mutex::new(NotificationEngine::<InMemoryStorage>::new(
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
            redis_cache: Arc::new(RwLock::new(None)),
        }
    }

    /// Call this after PostgreSQL connection is established to enable event persistence
    pub fn enable_event_persistence(&self) {
        if let Ok(mut engine) = self.events_engine.lock() {
            *engine = EventsEngine::new(Arc::clone(&self.shared_storage))
                .with_postgres(Arc::clone(&self.postgres_persistence));
        }
    }

    /// Call this after PostgreSQL connection is established to enable user activity persistence
    pub fn enable_activity_persistence(&self) {
        if let Ok(mut engine) = self.activity_engine.lock() {
            engine.set_postgres(Arc::clone(&self.postgres_persistence));
        }
    }

    /// Enable persistence for circuit activity logs generated by the circuits engine
    pub fn enable_circuit_activity_persistence(&self) {
        if let Ok(mut engine) = self.circuits_engine.lock() {
            engine.set_postgres(Arc::clone(&self.postgres_persistence));
        }
    }
}

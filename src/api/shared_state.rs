use crate::api::notifications::NotificationMessage;
use crate::api_key_engine::ApiKeyEngine;
use crate::dfid_client::DfidClient;
use crate::index_client::IndexClient;
use crate::logging::LoggingEngine;
use crate::postgres_persistence::PostgresPersistence;
use crate::postgres_storage_with_cache::PostgresStorageWithCache;
use crate::rate_limiter::RateLimiter;
use crate::redis_cache::RedisCache;
use crate::storage_helpers::{with_storage, StorageLockError};
use crate::storage_history_reader::StorageHistoryReader;
use crate::{
    ActivityEngine, AuditEngine, CircuitsEngine, EventsEngine, ItemsEngine, NotificationEngine,
    ReceiptEngine,
};
use std::sync::{Arc, Mutex};
use tokio::sync::{broadcast, RwLock as AsyncRwLock};

/// AppState with PostgreSQL primary storage and optional Redis cache
///
/// All production and test environments use PostgreSQL as the storage backend.
/// This ensures consistency between development, testing, and production.
///
/// Storage is accessed via spawn_blocking to safely bridge sync storage with async runtime.
type SharedStorage = Arc<Mutex<PostgresStorageWithCache>>;

pub struct AppState {
    pub circuits_engine: Arc<AsyncRwLock<CircuitsEngine<SharedStorage>>>,
    pub items_engine: Arc<AsyncRwLock<ItemsEngine<SharedStorage>>>,
    pub events_engine: Arc<AsyncRwLock<EventsEngine<SharedStorage>>>,
    pub audit_engine: AuditEngine<SharedStorage>,
    pub activity_engine: Arc<AsyncRwLock<ActivityEngine<SharedStorage>>>,
    pub receipt_engine: Arc<Mutex<ReceiptEngine<SharedStorage>>>,
    pub shared_storage: SharedStorage,
    pub storage_history_reader: StorageHistoryReader<SharedStorage>,
    pub logging: Arc<Mutex<LoggingEngine>>,
    pub api_key_engine: Arc<ApiKeyEngine>,
    pub api_key_storage: Arc<crate::api_key_storage::InMemoryApiKeyStorage>,
    pub rate_limiter: Arc<RateLimiter>,
    pub notification_engine: Arc<AsyncRwLock<NotificationEngine<SharedStorage>>>,
    pub notification_tx: broadcast::Sender<NotificationMessage>,
    pub jwt_secret: String,
    /// Optional PostgreSQL persistence layer - lazy initialized
    pub postgres_persistence: Arc<AsyncRwLock<Option<PostgresPersistence>>>,
    /// Optional Redis cache layer for horizontal scaling
    pub redis_cache: Arc<AsyncRwLock<Option<RedisCache>>>,
    /// Optional DFID client for remote DFID generation
    pub dfid_client: Option<DfidClient>,
    /// Optional Index client for DFID location discovery
    pub index_client: Option<IndexClient>,
}

impl AppState {
    /// Create AppState with PostgreSQL primary storage (with optional Redis cache)
    pub fn new(storage: SharedStorage) -> Self {
        Self::new_with_clients(storage, None, None)
    }

    /// Create AppState with optional DFID client for remote DFID generation
    pub fn new_with_dfid_client(storage: SharedStorage, dfid_client: Option<DfidClient>) -> Self {
        Self::new_with_clients(storage, dfid_client, None)
    }

    /// Create AppState with optional DFID and Index clients
    pub fn new_with_clients(
        storage: SharedStorage,
        dfid_client: Option<DfidClient>,
        index_client: Option<IndexClient>,
    ) -> Self {
        // All engines share the same storage Arc - access via spawn_blocking for thread safety
        let storage_for_circuits = Arc::clone(&storage);
        let storage_for_items = Arc::clone(&storage);
        let storage_for_events = Arc::clone(&storage);
        let storage_for_audit = Arc::clone(&storage);
        let storage_for_activity = Arc::clone(&storage);
        let storage_for_notifications = Arc::clone(&storage);
        let storage_for_receipts = Arc::clone(&storage);
        let storage_for_history = Arc::clone(&storage);

        // Configure circuits engine with DFID client and Index client if provided
        let circuits_engine = {
            let mut engine = CircuitsEngine::<SharedStorage>::new(storage_for_circuits);
            if let Some(ref client) = dfid_client {
                engine = engine.with_dfid_client(client.clone());
            }
            if let Some(ref client) = index_client {
                engine = engine.with_index_client(client.clone());
            }
            Arc::new(AsyncRwLock::new(engine))
        };

        // Configure items engine with DFID client if provided
        let items_engine = {
            let mut engine = ItemsEngine::<SharedStorage>::new(storage_for_items);
            if let Some(ref client) = dfid_client {
                engine = engine.with_dfid_client(client.clone());
            }
            Arc::new(AsyncRwLock::new(engine))
        };
        let events_engine = Arc::new(AsyncRwLock::new(EventsEngine::<SharedStorage>::new(
            storage_for_events,
        )));
        let audit_engine = AuditEngine::<SharedStorage>::new(storage_for_audit);
        let activity_engine = Arc::new(AsyncRwLock::new(ActivityEngine::<SharedStorage>::new(
            storage_for_activity,
        )));
        let notification_engine = Arc::new(AsyncRwLock::new(
            NotificationEngine::<SharedStorage>::new(storage_for_notifications),
        ));
        let receipt_engine = Arc::new(Mutex::new(ReceiptEngine::new(storage_for_receipts)));
        let storage_history_reader =
            StorageHistoryReader::<SharedStorage>::new(storage_for_history);

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
            receipt_engine,
            shared_storage: storage,
            storage_history_reader,
            logging,
            api_key_engine,
            api_key_storage,
            rate_limiter,
            notification_engine,
            notification_tx,
            jwt_secret,
            postgres_persistence: Arc::new(AsyncRwLock::new(None)),
            redis_cache: Arc::new(AsyncRwLock::new(None)),
            dfid_client,
            index_client,
        }
    }

    /// Enable PostgreSQL persistence for events engine
    pub async fn enable_event_persistence(&self) {
        let mut engine = self.events_engine.write().await;
        let new_engine = EventsEngine::new(self.shared_storage.clone())
            .with_postgres(Arc::clone(&self.postgres_persistence));
        *engine = new_engine;
    }

    /// Enable PostgreSQL persistence for user activity tracking
    pub async fn enable_activity_persistence(&self) {
        let mut engine = self.activity_engine.write().await;
        engine.set_postgres(Arc::clone(&self.postgres_persistence));
    }

    /// Enable PostgreSQL persistence for circuit activity logs
    pub async fn enable_circuit_activity_persistence(&self) {
        let mut engine = self.circuits_engine.write().await;
        engine.set_postgres(Arc::clone(&self.postgres_persistence));
    }

    /// Helper to safely access storage with read lock from async context
    /// Uses spawn_blocking to avoid holding sync lock across await points
    pub async fn with_storage_read<F, R>(&self, f: F) -> Result<R, StorageLockError>
    where
        F: FnOnce(&PostgresStorageWithCache) -> Result<R, Box<dyn std::error::Error>>
            + Send
            + 'static,
        R: Send + 'static,
    {
        let storage = self.shared_storage.clone();
        tokio::task::spawn_blocking(move || {
            with_storage(&storage, "shared_state.rs::with_storage_read::read", f)
        })
        .await
        .expect("Storage read task panicked")
    }

    /// Helper to safely access storage with write lock from async context
    /// Uses spawn_blocking to avoid holding sync lock across await points
    pub async fn with_storage_write<F, R>(&self, f: F) -> Result<R, StorageLockError>
    where
        F: FnOnce(&mut PostgresStorageWithCache) -> Result<R, Box<dyn std::error::Error>>
            + Send
            + 'static,
        R: Send + 'static,
    {
        let storage = self.shared_storage.clone();
        tokio::task::spawn_blocking(move || {
            crate::storage_helpers::with_lock_mut(
                &storage,
                "shared_state.rs::with_storage_write::write",
                f,
            )
        })
        .await
        .expect("Storage write task panicked")
    }
}

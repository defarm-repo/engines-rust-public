use std::sync::{Arc, Mutex};
use crate::{CircuitsEngine, ItemsEngine, AuditEngine, InMemoryStorage};
use crate::storage_history_manager::StorageHistoryManager;

pub struct AppState {
    pub circuits_engine: Arc<Mutex<CircuitsEngine<InMemoryStorage>>>,
    pub items_engine: Arc<Mutex<ItemsEngine<Arc<Mutex<InMemoryStorage>>>>>,
    pub audit_engine: AuditEngine<InMemoryStorage>,
    pub shared_storage: Arc<Mutex<InMemoryStorage>>,
    pub storage_history_manager: StorageHistoryManager<InMemoryStorage>,
}

impl AppState {
    pub fn new() -> Self {
        // Create a single shared storage instance for all engines
        // This fixes the storage isolation issue - items created via Items API
        // will now be accessible to Circuits API and audit events will be shared
        let shared_storage = Arc::new(Mutex::new(InMemoryStorage::new()));

        // All engines now use the same shared storage backend
        // The engines will wrap the storage in Arc<Mutex<>> internally
        let storage_for_circuits = Arc::clone(&shared_storage);
        let storage_for_items = Arc::clone(&shared_storage);
        let storage_for_audit = Arc::clone(&shared_storage);

        let circuits_engine = Arc::new(Mutex::new(CircuitsEngine::new(storage_for_circuits)));
        let items_engine = Arc::new(Mutex::new(ItemsEngine::new(storage_for_items)));
        let audit_engine = AuditEngine::new(storage_for_audit);

        let storage_for_history = Arc::clone(&shared_storage);
        let storage_history_manager = StorageHistoryManager::new(storage_for_history);

        Self {
            circuits_engine,
            items_engine,
            audit_engine,
            shared_storage,
            storage_history_manager,
        }
    }
}
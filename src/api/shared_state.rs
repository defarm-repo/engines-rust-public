use std::sync::{Arc, Mutex};
use crate::{CircuitsEngine, ItemsEngine, InMemoryStorage};

pub struct AppState {
    pub circuits_engine: Arc<Mutex<CircuitsEngine<InMemoryStorage>>>,
    pub items_engine: Arc<Mutex<ItemsEngine<Arc<Mutex<InMemoryStorage>>>>>,
    pub shared_storage: Arc<Mutex<InMemoryStorage>>,
}

impl AppState {
    pub fn new() -> Self {
        // Create a single shared storage instance for both engines
        // This fixes the storage isolation issue - items created via Items API
        // will now be accessible to Circuits API and vice versa
        let shared_storage = Arc::new(Mutex::new(InMemoryStorage::new()));

        // Both engines now use the same shared storage backend
        // The engines will wrap the storage in Arc<Mutex<>> internally
        let storage_for_circuits = Arc::clone(&shared_storage);
        let storage_for_items = Arc::clone(&shared_storage);

        let circuits_engine = Arc::new(Mutex::new(CircuitsEngine::new(storage_for_circuits)));
        let items_engine = Arc::new(Mutex::new(ItemsEngine::new(storage_for_items)));

        Self {
            circuits_engine,
            items_engine,
            shared_storage,
        }
    }
}
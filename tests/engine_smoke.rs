use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use defarm_engine::identifier_types::EnhancedIdentifier;
use defarm_engine::{
    CircuitsEngine, EventType, EventVisibility, EventsEngine, Identifier, InMemoryStorage,
    ItemsEngine, StorageBackend,
};
use serde_json::json;
use uuid::Uuid;

fn new_storage() -> Arc<Mutex<InMemoryStorage>> {
    Arc::new(Mutex::new(InMemoryStorage::new()))
}

#[tokio::test]
async fn circuits_items_and_events_smoke() {
    let storage = new_storage();
    let mut circuits_engine = CircuitsEngine::new(Arc::clone(&storage));
    let mut items_engine = ItemsEngine::new(Arc::clone(&storage));
    let mut events_engine = EventsEngine::new(Arc::clone(&storage));

    let circuit = circuits_engine
        .create_circuit(
            "Smoke Circuit".into(),
            "Basic circuit for smoke test".into(),
            "user123".into(),
            None,
            None,
        )
        .expect("circuit should be created");

    let identifiers = vec![Identifier::new("secondary_id", "123")];
    let item = items_engine
        .create_local_item(identifiers.clone(), vec![], None, Uuid::new_v4())
        .expect("local item should be created");
    assert!(item.dfid.starts_with("LID-"));

    let lid = Uuid::new_v4();
    let enhanced = vec![
        EnhancedIdentifier::canonical("bovino", "sisbov", "BR123"),
        EnhancedIdentifier::contextual("bovino", "peso", "450kg"),
    ];
    let push_result = circuits_engine
        .push_local_item_to_circuit(&lid, enhanced, None, &circuit.circuit_id, "user123")
        .await
        .expect("push should succeed");

    let mapped = {
        let guard = storage.lock().unwrap();
        guard
            .get_dfid_by_lid(&lid)
            .expect("mapping lookup should succeed")
            .expect("mapping should exist")
    };
    assert_eq!(mapped, push_result.dfid);

    let event = events_engine
        .create_event(
            mapped.clone(),
            EventType::Created,
            "smoke".into(),
            EventVisibility::Public,
        )
        .expect("event should be created");
    let events = events_engine
        .get_events_for_item(&mapped)
        .expect("events lookup should succeed");
    assert!(events.iter().any(|e| e.event_id == event.event_id));

    let mut enrichment = HashMap::new();
    enrichment.insert("peso".to_string(), json!(470));
    let enriched_item = items_engine
        .enrich_item(&mapped, enrichment.clone(), Uuid::new_v4())
        .expect("enrichment should succeed");
    assert_eq!(enriched_item.enriched_data.get("peso"), Some(&json!(470)));
}

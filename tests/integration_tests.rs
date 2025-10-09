use defarm_engine::*;
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn test_circuit_creation() {
    // Setup
    let storage = Arc::new(std::sync::Mutex::new(InMemoryStorage::new()));
    let mut circuits_engine = CircuitsEngine::new(storage.clone());

    // Create a circuit
    let circuit_result = circuits_engine.create_circuit(
        "Test Circuit".to_string(),
        "Integration test circuit".to_string(),
        "user123".to_string(),
    );

    assert!(circuit_result.is_ok());
    let circuit = circuit_result.unwrap();

    // Verify circuit was created
    assert_eq!(circuit.name, "Test Circuit");
    assert_eq!(circuit.owner_id, "user123");
    assert!(matches!(circuit.status, CircuitStatus::Active));

    // Get circuit by ID
    let retrieved = circuits_engine.get_circuit(&circuit.circuit_id).unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().circuit_id, circuit.circuit_id);
}

#[tokio::test]
async fn test_local_item_creation() {
    // Setup
    let storage = Arc::new(std::sync::Mutex::new(InMemoryStorage::new()));
    let mut items_engine = ItemsEngine::new(storage.clone());

    // Create local item
    let identifiers = vec![
        Identifier::new("test_key".to_string(), "test_value".to_string())
    ];

    let item_result = items_engine.create_local_item(
        identifiers.clone(),
        vec![], // No enhanced identifiers for this test
        None,
        Uuid::new_v4(),
    );

    assert!(item_result.is_ok());
    let item = item_result.unwrap();

    // Verify temporary DFID was generated (LID format)
    assert!(item.dfid.starts_with("LID-"));
    assert_eq!(item.identifiers.len(), 1);
}

#[tokio::test]
async fn test_legacy_item_creation() {
    // Setup
    let storage = Arc::new(std::sync::Mutex::new(InMemoryStorage::new()));
    let mut items_engine = ItemsEngine::new(storage.clone());

    // Create item with legacy method
    let dfid = "DFID-TEST-001".to_string();
    let identifiers = vec![Identifier::new("test_key".to_string(), "test_value".to_string())];
    let source = Uuid::new_v4();

    let result = items_engine.create_item(dfid.clone(), identifiers, source);
    assert!(result.is_ok());

    // Verify item can be retrieved
    let item = items_engine.get_item(&dfid).unwrap();
    assert!(item.is_some());
    assert_eq!(item.unwrap().dfid, dfid);
}

#[tokio::test]
async fn test_event_creation_and_visibility() {
    // Setup
    let storage = Arc::new(std::sync::Mutex::new(InMemoryStorage::new()));
    let mut events_engine = EventsEngine::new(storage.clone());

    // Create public event
    let public_event = events_engine.create_event(
        "DFID-EVENT-TEST".to_string(),
        EventType::Created,
        "test_source".to_string(),
        EventVisibility::Public,
    ).unwrap();

    assert_eq!(public_event.visibility, EventVisibility::Public);
    assert!(!public_event.is_encrypted);

    // Create private event
    let private_event = events_engine.create_event(
        "DFID-EVENT-TEST".to_string(),
        EventType::Enriched,
        "test_source".to_string(),
        EventVisibility::Private,
    ).unwrap();

    assert_eq!(private_event.visibility, EventVisibility::Private);
    assert!(private_event.is_encrypted);

    // Get all events for item
    let item_events = events_engine.get_events_for_item("DFID-EVENT-TEST").unwrap();
    assert_eq!(item_events.len(), 2);
}

#[tokio::test]
async fn test_item_merge_workflow() {
    // Setup
    let storage = Arc::new(std::sync::Mutex::new(InMemoryStorage::new()));
    let mut items_engine = ItemsEngine::new(storage.clone());

    // Create two items
    let dfid1 = "DFID-MERGE-001".to_string();
    let dfid2 = "DFID-MERGE-002".to_string();

    let identifiers1 = vec![Identifier::new("user_id".to_string(), "12345".to_string())];
    let identifiers2 = vec![Identifier::new("email".to_string(), "test@example.com".to_string())];

    items_engine.create_item(dfid1.clone(), identifiers1, Uuid::new_v4()).unwrap();
    items_engine.create_item(dfid2.clone(), identifiers2, Uuid::new_v4()).unwrap();

    // Merge items
    let merged = items_engine.merge_items(&dfid1, &dfid2).unwrap();

    // Verify merge
    assert_eq!(merged.dfid, dfid1);
    assert_eq!(merged.identifiers.len(), 2);
    assert!(matches!(merged.status, ItemStatus::Active));

    // Verify secondary item is marked as merged
    let secondary = items_engine.get_item(&dfid2).unwrap().unwrap();
    assert!(matches!(secondary.status, ItemStatus::Merged));
}

#[tokio::test]
async fn test_audit_logging() {
    // Setup
    let storage = Arc::new(std::sync::Mutex::new(InMemoryStorage::new()));
    let audit_engine = AuditEngine::new(storage.clone());

    // Log audit event
    let event_id = audit_engine.log_event(
        "user123".to_string(),
        AuditEventType::User,
        "login".to_string(),
        "authentication_system".to_string(),
        AuditOutcome::Success,
        AuditSeverity::Low,
        None,
        None,
        None,
    ).unwrap();

    assert!(!event_id.is_nil());

    // Retrieve user events
    let user_events = audit_engine.get_user_events("user123").unwrap();
    assert_eq!(user_events.len(), 1);
    assert_eq!(user_events[0].action, "login");
}

#[tokio::test]
async fn test_dfid_generation() {
    let dfid_engine = DfidEngine::new();

    // Generate DFID
    let dfid = dfid_engine.generate_dfid();

    // Verify format: DFID-{timestamp}-{sequence}-{checksum}
    assert!(dfid.starts_with("DFID-"));

    let parts: Vec<&str> = dfid.split('-').collect();
    assert_eq!(parts.len(), 4);
    assert_eq!(parts[0], "DFID");

    // Validate DFID
    assert!(dfid_engine.validate_dfid(&dfid));

    // Generate multiple DFIDs and ensure uniqueness
    let dfid2 = dfid_engine.generate_dfid();
    assert_ne!(dfid, dfid2);
}

#[tokio::test]
async fn test_storage_error_handling() {
    // Setup
    let storage = Arc::new(std::sync::Mutex::new(InMemoryStorage::new()));
    let items_engine = ItemsEngine::new(storage.clone());

    // Try to get non-existent item
    let result = items_engine.get_item("NON-EXISTENT-DFID");

    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[tokio::test]
async fn test_concurrent_circuit_operations() {
    use tokio::task;

    // Setup
    let storage = Arc::new(std::sync::Mutex::new(InMemoryStorage::new()));
    let circuits_engine = Arc::new(std::sync::Mutex::new(CircuitsEngine::new(storage.clone())));

    // Spawn multiple concurrent circuit creation tasks
    let mut handles = vec![];

    for i in 0..5 {
        let engine = circuits_engine.clone();
        let handle = task::spawn(async move {
            let mut eng = engine.lock().unwrap();
            eng.create_circuit(
                format!("Circuit {}", i),
                format!("Concurrent test {}", i),
                format!("user{}", i),
            )
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }

    // Verify circuits were created
    let all_circuits = {
        let eng = circuits_engine.lock().unwrap();
        eng.list_circuits().unwrap()
    };

    assert!(all_circuits.len() >= 5);
}

#[tokio::test]
async fn test_circuit_push_workflow() {
    // Setup
    let storage = Arc::new(std::sync::Mutex::new(InMemoryStorage::new()));
    let mut circuits_engine = CircuitsEngine::new(storage.clone());
    let mut items_engine = ItemsEngine::new(storage.clone());

    // Create circuit
    let circuit = circuits_engine.create_circuit(
        "Push Circuit".to_string(),
        "Testing push".to_string(),
        "owner123".to_string(),
    ).unwrap();

    // Create item with legacy identifiers
    let dfid = "DFID-PUSH-001".to_string();
    let identifiers = vec![Identifier::new("test_key".to_string(), "test_value".to_string())];
    let source = Uuid::new_v4();

    items_engine.create_item(dfid.clone(), identifiers, source).unwrap();

    // Push item to circuit (owner has permission by default)
    let push_result = circuits_engine.push_item_to_circuit(&dfid, &circuit.circuit_id, "owner123").await;

    assert!(push_result.is_ok());

    // Verify item is in circuit
    let circuit_items = circuits_engine.get_circuit_items(&circuit.circuit_id).unwrap();
    assert_eq!(circuit_items.len(), 1);
    assert_eq!(circuit_items[0].dfid, dfid);
}

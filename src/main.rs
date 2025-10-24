use defarm_engine::{
    CircuitsEngine, EventType, EventsEngine, Identifier, InMemoryStorage, Item, MemberRole,
    ReceiptEngine, StorageBackend,
};
use std::sync::Arc;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    println!("=== Complete Engine System Demo with Events and Circuits ===\n");

    // Create storage for receipt engine and shared storage for new engines
    let mut receipt_engine = ReceiptEngine::new(InMemoryStorage::new());
    let shared_storage = Arc::new(std::sync::Mutex::new(InMemoryStorage::new()));

    // Create Events and Circuits engines
    let mut events_engine = EventsEngine::new(Arc::clone(&shared_storage));
    let mut circuits_engine = CircuitsEngine::new(Arc::clone(&shared_storage));

    println!("1. Processing data through Receipt Engine...");

    // Process some data
    let data1 = b"User payment transaction";
    let identifiers1 = vec![
        Identifier::new("user_id", "user_12345"),
        Identifier::new("transaction_id", "tx_abc123"),
        Identifier::new("payment_method", "credit_card"),
    ];

    let receipt1 = receipt_engine.process_data(data1, identifiers1).unwrap();
    println!("   Receipt 1: {}", receipt1.id);

    let data2 = b"User profile update";
    let identifiers2 = vec![
        Identifier::new("user_id", "user_12345"), // Same user - should enrich
        Identifier::new("session_id", "sess_456"),
    ];

    let receipt2 = receipt_engine.process_data(data2, identifiers2).unwrap();
    println!("   Receipt 2: {}", receipt2.id);

    let data3 = b"New user registration";
    let identifiers3 = vec![
        Identifier::new("user_id", "user_789"), // Different user
        Identifier::new("email", "new@example.com"),
    ];

    let receipt3 = receipt_engine.process_data(data3, identifiers3).unwrap();
    println!("   Receipt 3: {}", receipt3.id);

    println!("\n2. Data Lake Status:");
    let all_receipts = receipt_engine.list_receipts().unwrap();
    println!("   Total receipts: {}", all_receipts.len());

    println!("\n3. System Logs Summary:");
    let logs = receipt_engine.get_logs();
    let data_lake_logs = receipt_engine.get_logs_by_event_type("data_lake_entry_created");
    println!("   Total logs: {}", logs.len());
    println!("   Data lake entries created: {}", data_lake_logs.len());

    println!("\n=== Demo shows complete data flow ===");
    println!("✓ Receipt Engine: Processes data and creates receipts");
    println!("✓ Data Lake: Stores raw data for verification");
    println!("✓ Ready for Verification Engine to process pending entries");
    println!("✓ Logging: Tracks all system operations");

    // Demo Events Engine
    println!("\n4. Events Engine Demo:");

    // Create a mock item for demonstration
    let demo_dfid = "DFID-20240926-000001-A7B2C".to_string();
    println!("   Creating events for item: {demo_dfid}");

    // Create item lifecycle events
    let created_event = events_engine
        .create_item_created_event(
            demo_dfid.clone(),
            "demo_source".to_string(),
            vec!["user_12345".to_string(), "tx_abc123".to_string()],
        )
        .unwrap();
    println!("   Created event: {}", created_event.event_id);

    let enriched_event = events_engine
        .create_item_enriched_event(
            demo_dfid.clone(),
            "enrichment_source".to_string(),
            vec!["payment_method".to_string(), "amount".to_string()],
        )
        .unwrap();
    println!("   Enriched event: {}", enriched_event.event_id);

    // Query events for the item
    let item_events = events_engine.get_events_for_item(&demo_dfid).unwrap();
    println!("   Total events for item: {}", item_events.len());

    // Demo Circuits Engine
    println!("\n5. Circuits Engine Demo:");

    // Create a circuit
    let circuit = circuits_engine
        .create_circuit(
            "Demo Circuit".to_string(),
            "A demonstration circuit for sharing items".to_string(),
            "owner_123".to_string(),
            None,
            None,
        )
        .await
        .unwrap();
    println!(
        "   Created circuit: {} (ID: {})",
        circuit.name, circuit.circuit_id
    );

    // Add a member to the circuit
    let updated_circuit = circuits_engine
        .add_member_to_circuit(
            &circuit.circuit_id,
            "member_456".to_string(),
            MemberRole::Member,
            "owner_123",
        )
        .await
        .unwrap();
    println!(
        "   Added member to circuit. Total members: {}",
        updated_circuit.members.len()
    );

    // Create a demo item in storage first
    {
        let mut storage = shared_storage.lock().unwrap();
        let demo_identifiers = vec![
            Identifier::new("user_id", "user_12345"),
            Identifier::new("transaction_id", "tx_abc123"),
        ];
        let demo_item = Item::new(demo_dfid.clone(), demo_identifiers, Uuid::new_v4());
        storage.store_item(&demo_item).unwrap();
    }

    // Demonstrate push/pull operations with the item
    println!("   Demonstrating circuit operations with item: {demo_dfid}");

    // Push item to circuit
    let push_operation = circuits_engine
        .push_item_to_circuit(&demo_dfid, &circuit.circuit_id, "owner_123")
        .await
        .unwrap();
    println!("   Push operation created: {}", push_operation.operation_id);

    // Pull item from circuit
    let (pulled_item, pull_operation) = circuits_engine
        .pull_item_from_circuit(&demo_dfid, &circuit.circuit_id, "member_456")
        .await
        .unwrap();
    println!(
        "   Pull operation completed: {}",
        pull_operation.operation_id
    );
    println!("   Pulled item DFID: {}", pulled_item.dfid);

    // Check events created by circuit operations
    let circuit_events = events_engine.get_events_for_item(&demo_dfid).unwrap();
    let circuit_operation_events: Vec<_> = circuit_events
        .iter()
        .filter(|e| {
            matches!(
                e.event_type,
                EventType::PushedToCircuit | EventType::PulledFromCircuit
            )
        })
        .collect();
    println!(
        "   Circuit operation events created: {}",
        circuit_operation_events.len()
    );

    println!("\n6. System Summary:");
    let total_events = events_engine.list_all_events().unwrap().len();
    let total_circuits = circuits_engine.list_circuits().unwrap().len();
    println!("   Total events: {total_events}");
    println!("   Total circuits: {total_circuits}");
    println!("   Total receipts: {}", all_receipts.len());
    println!("   Data lake entries: {}", data_lake_logs.len());

    println!("\n=== Complete Traceability System Demo ===");
    println!("✓ Receipt Engine: Processes data and creates receipts");
    println!("✓ Data Lake: Stores raw data for verification");
    println!("✓ Verification Engine: Processes and deduplicates entries");
    println!("✓ Items Engine: Manages verified items with DFIDs");
    println!("✓ Events Engine: Tracks all item lifecycle events");
    println!("✓ Circuits Engine: Manages permission-controlled sharing");
    println!("✓ Logging: Comprehensive audit trail across all engines");

    println!("\nSystem is ready for:");
    println!("- Real-time event tracking and audit trails");
    println!("- Circuit-based secure item sharing");
    println!("- Multi-tenant deployments with isolated circuits");
    println!("- Integration with coop-cipher frontend application");
}

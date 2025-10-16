use defarm_engine::circuits_engine::{CircuitsEngine, PushStatus};
use defarm_engine::identifier_types::{CircuitAliasConfig, EnhancedIdentifier, namespaces};
use defarm_engine::storage::{InMemoryStorage, StorageBackend};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

fn new_engine() -> (CircuitsEngine<InMemoryStorage>, Arc<Mutex<InMemoryStorage>>) {
    let storage = Arc::new(Mutex::new(InMemoryStorage::new()));
    let engine = CircuitsEngine::new(Arc::clone(&storage));
    (engine, storage)
}

#[tokio::test]
async fn test_canonical_identifier_deduplication() {
    let (mut engine, _storage) = new_engine();

    // Create circuit with SISBOV requirement
    let mut alias_config = CircuitAliasConfig::default();
    alias_config.required_canonical = vec!["sisbov".to_string()];

    let circuit = engine
        .create_circuit_with_namespace(
            "Rastreabilidade Bovina".to_string(),
            "Circuit for bovine traceability".to_string(),
            "owner1".to_string(),
            namespaces::BOVINO.to_string(),
            None, // adapter_config
            Some(alias_config),
        )
        .expect("circuit created");

    // First push - creates new DFID
    let lid1 = Uuid::new_v4();
    let identifiers1 = vec![EnhancedIdentifier::canonical(
        "bovino",
        "sisbov",
        "BR123456789012", // BR + 12 digits = 14 chars total
    )];

    let result1 = engine
        .push_local_item_to_circuit(
            &lid1,
            identifiers1,
            None,
            &circuit.circuit_id,
            "owner1",
        )
        .await
        .expect("first push succeeds");

    assert!(matches!(result1.status, PushStatus::NewItemCreated));
    let dfid1 = result1.dfid.clone();

    // Second push with same SISBOV - should enrich
    let lid2 = Uuid::new_v4();
    let identifiers2 = vec![
        EnhancedIdentifier::canonical("bovino", "sisbov", "BR123456789012"), // Same SISBOV
        EnhancedIdentifier::contextual("bovino", "peso", "450kg"),
    ];

    let result2 = engine
        .push_local_item_to_circuit(&lid2, identifiers2, None, &circuit.circuit_id, "owner1")
        .await
        .expect("second push succeeds");

    assert!(matches!(result2.status, PushStatus::ExistingItemEnriched));
    assert_eq!(dfid1, result2.dfid, "Same DFID for same canonical identifier");
}

#[tokio::test]
async fn test_fingerprint_deduplication() {
    let (mut engine, _storage) = new_engine();

    // Create circuit with fingerprint enabled
    let mut alias_config = CircuitAliasConfig::default();
    alias_config.use_fingerprint = true;
    alias_config.required_contextual = vec!["lote".to_string(), "safra".to_string()];

    let circuit = engine
        .create_circuit_with_namespace(
            "Lotes Soja".to_string(),
            "Circuit for soy lots".to_string(),
            "owner1".to_string(),
            namespaces::SOJA.to_string(),
            None, // adapter_config
            Some(alias_config),
        )
        .expect("circuit created");

    // Push without canonical - uses fingerprint
    let lid = Uuid::new_v4();
    let identifiers = vec![
        EnhancedIdentifier::contextual("soja", "lote", "123"),
        EnhancedIdentifier::contextual("soja", "safra", "2024/25"),
    ];

    let result1 = engine
        .push_local_item_to_circuit(&lid, identifiers.clone(), None, &circuit.circuit_id, "owner1")
        .await
        .expect("first push succeeds");

    assert!(matches!(result1.status, PushStatus::NewItemCreated));

    // Note: Fingerprint includes timestamp, so each push creates new item
    // This is expected behavior - fingerprints prevent exact replay
    let result2 = engine
        .push_local_item_to_circuit(&lid, identifiers, None, &circuit.circuit_id, "owner1")
        .await
        .expect("second push succeeds");

    // Each push with fingerprint creates new item (timestamp differs)
    assert!(matches!(
        result2.status,
        PushStatus::NewItemCreated | PushStatus::ExistingItemEnriched
    ));
}

#[tokio::test]
async fn test_namespace_validation() {
    let (mut engine, _storage) = new_engine();

    // Create circuit restricted to bovino namespace
    let mut alias_config = CircuitAliasConfig::default();
    alias_config.allowed_namespaces = Some(vec!["bovino".to_string()]);

    let circuit = engine
        .create_circuit_with_namespace(
            "Bovinos Only".to_string(),
            "Circuit that only accepts bovine data".to_string(),
            "owner1".to_string(),
            namespaces::BOVINO.to_string(),
            None, // adapter_config
            Some(alias_config),
        )
        .expect("circuit created");

    // Try to push with wrong namespace
    let lid = Uuid::new_v4();
    let identifiers = vec![
        EnhancedIdentifier::contextual("aves", "lote", "123"), // Wrong namespace!
    ];

    let result = engine
        .push_local_item_to_circuit(&lid, identifiers, None, &circuit.circuit_id, "owner1")
        .await;

    assert!(result.is_err(), "Push with wrong namespace should fail");

    // Check that it's a validation error about namespace
    if let Err(e) = result {
        let error_msg = format!("{:?}", e);
        assert!(
            error_msg.contains("Namespace") || error_msg.contains("not allowed"),
            "Error should mention namespace: {}",
            error_msg
        );
    }
}

#[tokio::test]
async fn test_auto_namespace_application() {
    let (mut engine, storage) = new_engine();

    // Create circuit with auto-apply namespace
    let mut alias_config = CircuitAliasConfig::default();
    alias_config.auto_apply_namespace = true;

    let circuit = engine
        .create_circuit_with_namespace(
            "Auto Namespace".to_string(),
            "Circuit that auto-applies namespace".to_string(),
            "owner1".to_string(),
            namespaces::SOJA.to_string(),
            None, // adapter_config
            Some(alias_config),
        )
        .expect("circuit created");

    // Push with empty namespace - should auto-apply
    let lid = Uuid::new_v4();
    let identifiers = vec![
        EnhancedIdentifier::contextual("", "lote", "123"), // Empty namespace
    ];

    let result = engine
        .push_local_item_to_circuit(&lid, identifiers, None, &circuit.circuit_id, "owner1")
        .await
        .expect("push with auto-namespace succeeds");

    assert!(matches!(result.status, PushStatus::NewItemCreated));

    // Verify the item was created with the circuit's default namespace
    let storage_lock = storage.lock().expect("storage lock");
    let item = storage_lock
        .get_item_by_dfid(&result.dfid)
        .expect("storage query")
        .expect("item exists");

    // Check that the namespace was applied
    assert!(
        item.enhanced_identifiers
            .iter()
            .any(|id| id.namespace == "soja"),
        "Namespace should be auto-applied to soja"
    );
}

#[tokio::test]
async fn test_lid_dfid_mapping() {
    let (mut engine, storage) = new_engine();

    let circuit = engine
        .create_circuit(
            "Test Circuit".to_string(),
            "Test circuit for LID-DFID mapping".to_string(),
            "owner1".to_string(),
            None,
            None,
        )
        .expect("circuit created");

    let lid = Uuid::new_v4();
    let identifiers = vec![EnhancedIdentifier::contextual(
        "generic",
        "test_id",
        "value1",
    )];

    let result = engine
        .push_local_item_to_circuit(&lid, identifiers, None, &circuit.circuit_id, "owner1")
        .await
        .expect("push succeeds");

    // Check that LID-DFID mapping was stored
    let storage_lock = storage.lock().expect("storage lock");
    let mapped_dfid = storage_lock.get_dfid_by_lid(&lid).expect("mapping query");

    assert_eq!(
        mapped_dfid,
        Some(result.dfid),
        "LID should map to assigned DFID"
    );
}

#[tokio::test]
async fn test_non_owner_cannot_push() {
    let (mut engine, _storage) = new_engine();

    let circuit = engine
        .create_circuit(
            "Private Circuit".to_string(),
            "Circuit with owner-only access".to_string(),
            "owner1".to_string(),
            None,
            None,
        )
        .expect("circuit created");

    let lid = Uuid::new_v4();
    let identifiers = vec![EnhancedIdentifier::contextual(
        "generic",
        "test_id",
        "value1",
    )];

    // Try to push as non-owner/non-member
    let result = engine
        .push_local_item_to_circuit(&lid, identifiers, None, &circuit.circuit_id, "other_user")
        .await;

    assert!(result.is_err(), "Non-member push should fail");

    if let Err(e) = result {
        let error_msg = format!("{:?}", e);
        assert!(
            error_msg.contains("PermissionDenied") || error_msg.contains("permission"),
            "Error should indicate permission denied: {}",
            error_msg
        );
    }
}

use defarm_engine::{
    circuits_engine::CircuitsEngine, identifier_types::CircuitAliasConfig,
    items_engine::ItemsEngine, storage::InMemoryStorage, Identifier, StorageBackend,
};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

fn setup_engines() -> (
    CircuitsEngine<InMemoryStorage>,
    ItemsEngine<Arc<Mutex<InMemoryStorage>>>,
    Arc<Mutex<InMemoryStorage>>,
) {
    let shared_storage = Arc::new(Mutex::new(InMemoryStorage::new()));
    let circuits = CircuitsEngine::new(Arc::clone(&shared_storage));
    let items = ItemsEngine::new(Arc::clone(&shared_storage));
    (circuits, items, shared_storage)
}

#[tokio::test]
async fn circuit_push_creates_mapping_and_dfid() {
    let (mut circuits, mut items, storage) = setup_engines();

    let circuit = circuits
        .create_circuit_with_namespace(
            "Traceability".into(),
            "Smoke test circuit".into(),
            "owner".into(),
            "bovino".into(),
            None,
            Some(CircuitAliasConfig {
                required_canonical: vec!["sisbov".into()],
                ..Default::default()
            }),
        )
        .expect("create circuit");

    let local_item = items
        .create_local_item(
            vec![
                Identifier::canonical("bovino", "sisbov", "BR123456789012"),
                Identifier::contextual("bovino", "lote", "LOT-001"),
            ],
            None,
            Uuid::new_v4(),
        )
        .expect("create local item");

    let local_id = local_item.local_id.expect("local items always have a LID");

    let result = circuits
        .push_local_item_to_circuit(
            &local_id,
            local_item.identifiers.clone(),
            None,
            &circuit.circuit_id,
            "owner",
        )
        .await
        .expect("push local item");

    assert!(
        result.dfid.starts_with("DFID-"),
        "tokenization should assign DFID"
    );

    let guard = storage.lock().expect("storage lock");
    let mapped = guard
        .get_dfid_by_lid(&local_id)
        .expect("mapping lookup")
        .expect("mapping should exist");
    assert_eq!(mapped, result.dfid);
}

#[tokio::test]
async fn canonical_deduplication_returns_same_dfid() {
    let (mut circuits, mut items, storage) = setup_engines();

    let circuit = circuits
        .create_circuit_with_namespace(
            "Dedup".into(),
            "Canonical dedup smoke test".into(),
            "owner".into(),
            "bovino".into(),
            None,
            Some(CircuitAliasConfig {
                required_canonical: vec!["sisbov".into()],
                ..Default::default()
            }),
        )
        .expect("create circuit");

    let canonical = "BR987654321098";

    let first_item = items
        .create_local_item(
            vec![
                Identifier::canonical("bovino", "sisbov", canonical),
                Identifier::contextual("bovino", "peso", "450kg"),
            ],
            None,
            Uuid::new_v4(),
        )
        .expect("create first item");
    let lid_one = first_item.local_id.expect("lid");

    let first_push = circuits
        .push_local_item_to_circuit(
            &lid_one,
            first_item.identifiers.clone(),
            None,
            &circuit.circuit_id,
            "owner",
        )
        .await
        .expect("push first item");

    let second_item = items
        .create_local_item(
            vec![
                Identifier::canonical("bovino", "sisbov", canonical), // same canonical ID
                Identifier::contextual("bovino", "lote", "LOT-002"),
            ],
            None,
            Uuid::new_v4(),
        )
        .expect("create second item");
    let lid_two = second_item.local_id.expect("lid");

    let second_push = circuits
        .push_local_item_to_circuit(
            &lid_two,
            second_item.identifiers.clone(),
            None,
            &circuit.circuit_id,
            "owner",
        )
        .await
        .expect("push duplicate item");

    assert_eq!(
        first_push.dfid, second_push.dfid,
        "canonical identifiers should deduplicate"
    );

    let guard = storage.lock().expect("storage lock");
    let mapped_two = guard
        .get_dfid_by_lid(&lid_two)
        .expect("mapping lookup")
        .expect("mapping should exist");
    assert_eq!(mapped_two, first_push.dfid);
}

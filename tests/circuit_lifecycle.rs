use defarm_engine::{
    circuits_engine::CircuitsEngine,
    identifier_types::CircuitAliasConfig,
    items_engine::ItemsEngine,
    storage::{InMemoryStorage, StorageBackend},
    Identifier,
};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

fn setup() -> (
    CircuitsEngine<Arc<Mutex<InMemoryStorage>>>,
    ItemsEngine<Arc<Mutex<InMemoryStorage>>>,
    Arc<Mutex<InMemoryStorage>>,
) {
    let storage = Arc::new(Mutex::new(InMemoryStorage::new()));
    let circuits = CircuitsEngine::new(Arc::clone(&storage));
    let items = ItemsEngine::new(Arc::clone(&storage));
    (circuits, items, storage)
}

#[tokio::test]
async fn lifecycle_smoke_test() {
    let (mut circuits, mut items, storage) = setup();

    let circuit = circuits
        .create_circuit(
            "Lifecycle".into(),
            "Lifecycle smoke test".into(),
            "tester".into(),
            None,
            Some(CircuitAliasConfig {
                required_canonical: vec!["tag".into()],
                ..Default::default()
            }),
        )
        .await
        .expect("create circuit");

    let local_item = items
        .create_local_item(
            vec![
                Identifier::canonical("generic", "tag", "LC-SMOKE-001"),
                Identifier::contextual("generic", "batch", "BATCH-123"),
            ],
            None,
            Uuid::new_v4(),
        )
        .expect("create local item");
    let local_id = local_item.local_id.expect("lid exists");

    let push = circuits
        .push_local_item_to_circuit(
            &local_id,
            local_item.identifiers.clone(),
            None,
            &circuit.circuit_id,
            "tester",
        )
        .await
        .expect("push local item");

    assert!(
        push.dfid.starts_with("DFID-"),
        "push assigns canonical DFID"
    );

    let guard = storage.lock().expect("lock shared storage");
    let mapped = guard
        .get_dfid_by_lid(&local_id)
        .expect("mapping query")
        .expect("mapping exists");
    assert_eq!(mapped, push.dfid);
}

#[tokio::test]
async fn timeline_entries_can_be_added() {
    let (mut circuits, mut items, storage) = setup();

    let circuit = circuits
        .create_circuit(
            "Timeline".into(),
            "Timeline smoke test".into(),
            "tester".into(),
            None,
            None,
        )
        .await
        .expect("create circuit");

    let item = items
        .create_local_item(
            vec![Identifier::canonical("generic", "tag", "TL-001")],
            None,
            Uuid::new_v4(),
        )
        .expect("create local item");
    let lid = item.local_id.expect("lid");

    let push = circuits
        .push_local_item_to_circuit(
            &lid,
            item.identifiers.clone(),
            None,
            &circuit.circuit_id,
            "tester",
        )
        .await
        .expect("push");

    let guard = storage.lock().expect("lock storage");
    guard
        .add_cid_to_timeline(
            &push.dfid,
            "QmTestCid",
            "stellar-tx-hash",
            1_717_171_717,
            "testnet",
        )
        .expect("add timeline entry");

    let timeline = guard.get_item_timeline(&push.dfid).expect("query timeline");
    assert_eq!(timeline.len(), 1);
    assert_eq!(timeline[0].cid, "QmTestCid");
}

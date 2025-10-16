use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use defarm_engine::storage::InMemoryStorage;
use defarm_engine::{Identifier, ItemStatus, ItemsEngine};
use uuid::Uuid;

fn new_engine() -> ItemsEngine<Arc<Mutex<InMemoryStorage>>> {
    let storage = Arc::new(Mutex::new(InMemoryStorage::new()));
    ItemsEngine::new(storage)
}

#[test]
fn create_item_persists_basic_fields() {
    let mut engine = new_engine();

    let dfid = "DFID-TEST-1".to_string();
    let identifiers = vec![Identifier::new("user_id", "12345")];
    let source_entry = Uuid::new_v4();

    let item = engine
        .create_item(dfid.clone(), identifiers.clone(), source_entry)
        .expect("item is created");

    assert_eq!(item.dfid, dfid);
    assert_eq!(item.identifiers, identifiers);
    assert_eq!(item.source_entries, vec![source_entry]);
    assert!(matches!(item.status, ItemStatus::Active));
}

#[test]
fn enrich_item_merges_data() {
    let mut engine = new_engine();

    let dfid = "DFID-TEST-2".to_string();
    let identifiers = vec![Identifier::new("user_id", "12345")];
    let source_entry = Uuid::new_v4();

    engine
        .create_item(dfid.clone(), identifiers, source_entry)
        .expect("seed item");

    let mut enrichment = HashMap::new();
    enrichment.insert(
        "name".to_string(),
        serde_json::Value::String("John Doe".into()),
    );
    enrichment.insert("age".to_string(), serde_json::Value::Number(30.into()));

    let new_source = Uuid::new_v4();
    let enriched_item = engine
        .enrich_item(&dfid, enrichment, new_source)
        .expect("enrichment succeeds");

    assert_eq!(enriched_item.enriched_data.len(), 2);
    assert!(enriched_item.source_entries.contains(&new_source));
}

#[test]
fn merge_items_combines_identifiers() {
    let mut engine = new_engine();

    let dfid1 = "DFID-TEST-3A".to_string();
    let dfid2 = "DFID-TEST-3B".to_string();

    engine
        .create_item(
            dfid1.clone(),
            vec![Identifier::new("user_id", "12345")],
            Uuid::new_v4(),
        )
        .expect("first item");
    engine
        .create_item(
            dfid2.clone(),
            vec![Identifier::new("email", "test@example.com")],
            Uuid::new_v4(),
        )
        .expect("second item");

    let merged_item = engine
        .merge_items(&dfid1, &dfid2)
        .expect("items merge cleanly");

    assert_eq!(merged_item.dfid, dfid1);
    assert_eq!(merged_item.identifiers.len(), 2);
    let secondary = engine
        .get_item(&dfid2)
        .expect("lookup")
        .expect("secondary item");
    assert!(matches!(secondary.status, ItemStatus::Merged));
}

#[test]
fn item_statistics_reflect_basic_counts() {
    let mut engine = new_engine();

    engine
        .create_item(
            "DFID-STAT-1".into(),
            vec![Identifier::new("id", "1")],
            Uuid::new_v4(),
        )
        .expect("first item");
    engine
        .create_item(
            "DFID-STAT-2".into(),
            vec![Identifier::new("id", "2")],
            Uuid::new_v4(),
        )
        .expect("second item");
    engine.deprecate_item("DFID-STAT-2").expect("deprecate");

    let stats = engine.get_item_statistics().expect("stats computed");

    assert_eq!(stats.total_items, 2);
    assert_eq!(stats.active_items, 1);
    assert_eq!(stats.deprecated_items, 1);
    assert_eq!(stats.total_identifiers, 2);
    assert_eq!(stats.average_confidence, 1.0);
}

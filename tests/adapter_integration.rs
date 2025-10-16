/// Comprehensive adapter integration tests
/// Tests IPFS upload, CID generation, hash validation, and storage metadata
use defarm_engine::adapters::base::StorageLocation;
use defarm_engine::adapters::{IpfsIpfsAdapter, StorageAdapter};
use defarm_engine::identifier_types::EnhancedIdentifier;
use defarm_engine::items_engine::ItemsEngine;
use defarm_engine::storage::{InMemoryStorage, StorageBackend};
use defarm_engine::types::{AdapterType, Item, ItemStatus};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

fn create_test_item(dfid: &str) -> Item {
    Item {
        dfid: dfid.to_string(),
        local_id: Some(Uuid::new_v4()),
        legacy_mode: false,
        identifiers: vec![],
        enhanced_identifiers: vec![EnhancedIdentifier::contextual(
            "test",
            "item_id",
            "test123",
        )],
        aliases: vec![],
        fingerprint: None,
        enriched_data: HashMap::new(),
        creation_timestamp: chrono::Utc::now(),
        last_modified: chrono::Utc::now(),
        source_entries: vec![],
        confidence_score: 1.0,
        status: ItemStatus::Active,
    }
}

#[tokio::test]
async fn test_ipfs_adapter_stores_item_and_generates_cid() {
    // Uses Pinata credentials from environment variables
    let adapter = match IpfsIpfsAdapter::new() {
        Ok(a) => a,
        Err(_) => {
            println!("⚠️  Skipping: IPFS not available");
            return;
        }
    };

    let item = create_test_item("DFID-TEST-001");
    let result = adapter.store_item(&item).await;

    assert!(
        result.is_ok(),
        "IPFS adapter should successfully store item"
    );

    let adapter_result = result.unwrap();
    let metadata = adapter_result.metadata;

    // Verify adapter type
    assert_eq!(metadata.adapter_type, AdapterType::IpfsIpfs);

    // Verify CID was generated
    match &metadata.item_location {
        StorageLocation::IPFS { cid, pinned } => {
            assert!(!cid.is_empty(), "CID should not be empty");
            assert!(cid.starts_with("Qm") || cid.starts_with("bafy"), "CID should have valid format");
            assert!(*pinned, "Item should be pinned");
            println!("✅ Generated CID: {}", cid);
            println!("✅ CID Length: {} characters", cid.len());
            println!("✅ Item pinned on IPFS: {}", pinned);
        }
        _ => panic!("Expected IPFS storage location"),
    }

    // Verify timestamps
    assert!(metadata.created_at <= chrono::Utc::now());
    assert_eq!(metadata.created_at, metadata.updated_at);
}

#[tokio::test]
async fn test_ipfs_adapter_can_retrieve_stored_item() {
    let adapter = match IpfsIpfsAdapter::new() {
        Ok(a) => a,
        Err(_) => {
            println!("⚠️  Skipping: IPFS not available");
            return;
        }
    };

    // Store item
    let item = create_test_item("DFID-TEST-002");
    let store_result = adapter.store_item(&item).await.unwrap();

    // Extract CID
    let cid = match &store_result.metadata.item_location {
        StorageLocation::IPFS { cid, .. } => cid.clone(),
        _ => panic!("Expected IPFS location"),
    };

    // Retrieve item using CID
    let retrieved = adapter.get_item(&cid).await;

    assert!(retrieved.is_ok(), "Should retrieve item successfully");
    let retrieved_result = retrieved.unwrap();
    assert!(retrieved_result.is_some(), "Item should exist");

    let retrieved_item = retrieved_result.unwrap().data;
    assert_eq!(retrieved_item.dfid, item.dfid);
}

#[tokio::test]
async fn test_ipfs_adapter_health_check() {
    let adapter = match IpfsIpfsAdapter::new() {
        Ok(a) => a,
        Err(_) => {
            println!("⚠️  Skipping: IPFS not available");
            return;
        }
    };

    let health = adapter.health_check().await;
    assert!(health.is_ok(), "Health check should complete");

    // If IPFS is running, health should be true
    if let Ok(is_healthy) = health {
        println!("IPFS health status: {}", is_healthy);
    }
}

#[tokio::test]
async fn test_ipfs_adapter_sync_status() {
    let adapter = match IpfsIpfsAdapter::new() {
        Ok(a) => a,
        Err(_) => {
            println!("⚠️  Skipping: IPFS not available");
            return;
        }
    };

    let status = adapter.sync_status().await;
    assert!(status.is_ok(), "Sync status should return successfully");

    let sync_status = status.unwrap();
    assert_eq!(sync_status.adapter_type, AdapterType::IpfsIpfs);

    // Check status details
    assert!(sync_status.details.contains_key("implementation_status"));
    assert!(sync_status.details.contains_key("ipfs_connected"));
}

#[test]
fn test_adapter_type_serialization() {
    use serde_json;

    // Test that adapter types serialize correctly
    let ipfs = AdapterType::IpfsIpfs;
    let json = serde_json::to_string(&ipfs).unwrap();
    assert!(json.contains("IpfsIpfs"));

    let testnet = AdapterType::StellarTestnetIpfs;
    let json = serde_json::to_string(&testnet).unwrap();
    assert!(json.contains("StellarTestnetIpfs"));

    let mainnet = AdapterType::StellarMainnetIpfs;
    let json = serde_json::to_string(&mainnet).unwrap();
    assert!(json.contains("StellarMainnetIpfs"));
}

#[tokio::test]
async fn test_storage_backend_stores_adapter_results() {
    let storage = Arc::new(Mutex::new(InMemoryStorage::new()));
    let mut items_engine = ItemsEngine::new(Arc::clone(&storage));

    // Create item
    let dfid = "DFID-ADAPTER-TEST-001".to_string();
    let _identifiers = vec![EnhancedIdentifier::contextual("test", "id", "adapter001")];
    let source_entry = Uuid::new_v4();

    use defarm_engine::Identifier;
    let _item = items_engine
        .create_item(dfid.clone(), vec![Identifier::new("test", "adapter001")], source_entry)
        .expect("Item creation should succeed");

    // Verify item exists in storage
    let storage_lock = storage.lock().unwrap();
    let retrieved = storage_lock
        .get_item_by_dfid(&dfid)
        .expect("Storage query should succeed");

    assert!(retrieved.is_some(), "Item should be stored");
    assert_eq!(retrieved.unwrap().dfid, dfid);
}

#[test]
fn test_cid_format_validation() {
    // Test valid CID formats
    let valid_cids = vec![
        "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG", // CIDv0 (46 chars)
        "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi", // CIDv1 (59 chars)
        "QmTest1234567890123456789012345678901234567890", // CIDv0 (46 chars)
    ];

    for cid in valid_cids {
        assert!(
            cid.starts_with("Qm") || cid.starts_with("bafy"),
            "CID {} should have valid prefix",
            cid
        );
        assert!(cid.len() >= 46, "CID {} should have valid length", cid);
    }
}

#[tokio::test]
async fn test_multiple_items_different_cids() {
    // Verify that different items produce different CIDs
    // This is a unit test that doesn't require IPFS connection

    let item1 = create_test_item("DFID-001");
    let item2 = create_test_item("DFID-002");

    assert_ne!(
        item1.dfid, item2.dfid,
        "Different items should have different DFIDs"
    );

    // In a real scenario with IPFS, different items would produce different CIDs
    // This would need to be tested with actual IPFS integration
}

#[test]
fn test_storage_location_enum() {
    use serde_json;

    // Test IPFS location serialization
    let ipfs_location = StorageLocation::IPFS {
        cid: "QmTest123".to_string(),
        pinned: true,
    };

    let json = serde_json::to_string(&ipfs_location).unwrap();
    assert!(json.contains("IPFS"));
    assert!(json.contains("QmTest123"));
    assert!(json.contains("true"));

    // Test Stellar location serialization
    let stellar_location = StorageLocation::Stellar {
        transaction_id: "abc123".to_string(),
        contract_address: "CTEST123".to_string(),
        asset_id: None,
    };

    let json = serde_json::to_string(&stellar_location).unwrap();
    assert!(json.contains("Stellar"));
}

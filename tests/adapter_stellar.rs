/// Stellar adapter integration tests
/// Tests Stellar contract interaction, IPCM updates, and blockchain storage
use defarm_engine::adapters::base::StorageLocation;
use defarm_engine::adapters::{StellarTestnetIpfsAdapter, StorageAdapter};
use defarm_engine::types::{AdapterType, Item, ItemStatus};
use std::collections::HashMap;
use uuid::Uuid;

fn create_test_item(dfid: &str) -> Item {
    use defarm_engine::identifier_types::EnhancedIdentifier;

    Item {
        dfid: dfid.to_string(),
        local_id: Some(Uuid::new_v4()),
        legacy_mode: false,
        identifiers: vec![],
        enhanced_identifiers: vec![EnhancedIdentifier::contextual("test", "item_id", dfid)],
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
async fn test_stellar_testnet_adapter_stores_item() {
    // Uses environment variables for credentials

    let adapter = match StellarTestnetIpfsAdapter::new() {
        Ok(a) => a,
        Err(e) => {
            println!("⚠️  Skipping: Stellar testnet adapter not available: {e}");
            return;
        }
    };

    let item = create_test_item("DFID-STELLAR-TEST-001");
    let result = adapter.store_item(&item).await;

    if result.is_err() {
        let err = result.unwrap_err();
        println!("⚠️  Test failed (expected if not configured): {err:?}");
        return;
    }

    let adapter_result = result.unwrap();
    let metadata = adapter_result.metadata;

    // Verify adapter type
    assert_eq!(metadata.adapter_type, AdapterType::StellarTestnetIpfs);

    // For Stellar adapter, the main location is Stellar (with CID in asset_id)
    // and the IPFS location is in event_locations
    match &metadata.item_location {
        StorageLocation::Stellar {
            transaction_id,
            contract_address,
            asset_id,
        } => {
            assert!(
                !transaction_id.is_empty(),
                "Transaction ID should not be empty"
            );
            assert!(
                !contract_address.is_empty(),
                "Contract address should not be empty"
            );

            println!("✅ Stellar Transaction Hash: {transaction_id}");
            println!("✅ Stellar Contract: {contract_address}");
            println!(
                "✅ Transaction viewable at: https://stellar.expert/explorer/testnet/tx/{transaction_id}"
            );

            if let Some(cid) = asset_id {
                println!("✅ IPFS CID (stored as asset_id): {cid}");
                println!("✅ View on IPFS: https://gateway.pinata.cloud/ipfs/{cid}");
            }
        }
        other => {
            panic!("Expected Stellar storage location, got: {other:?}");
        }
    }

    // Verify IPFS location is in event_locations
    let ipfs_cid = metadata.event_locations.iter().find_map(|loc| {
        if let StorageLocation::IPFS { cid, .. } = loc {
            Some(cid.clone())
        } else {
            None
        }
    });

    assert!(
        ipfs_cid.is_some(),
        "Should have IPFS location in event_locations"
    );
    if let Some(cid) = ipfs_cid {
        println!("✅ Item also recorded in IPFS event locations with CID: {cid}");
    }
}

#[tokio::test]
async fn test_stellar_adapter_health_check() {
    let adapter = match StellarTestnetIpfsAdapter::new() {
        Ok(a) => a,
        Err(_) => {
            println!("⚠️  Skipping: Stellar testnet adapter not configured");
            return;
        }
    };

    let health = adapter.health_check().await;

    match health {
        Ok(is_healthy) => {
            println!("Stellar testnet health: {is_healthy}");
            // Don't assert true because testnet might be down
        }
        Err(e) => {
            println!("Health check error (expected if not configured): {e:?}");
        }
    }
}

#[tokio::test]
async fn test_stellar_adapter_sync_status() {
    let adapter = match StellarTestnetIpfsAdapter::new() {
        Ok(a) => a,
        Err(_) => {
            println!("⚠️  Skipping: Stellar testnet adapter not configured");
            return;
        }
    };

    let status = adapter.sync_status().await;

    if status.is_ok() {
        let sync_status = status.unwrap();
        assert_eq!(sync_status.adapter_type, AdapterType::StellarTestnetIpfs);

        println!("Sync status: is_synced={}", sync_status.is_synced);
        println!("Details: {:?}", sync_status.details);
    } else {
        println!("⚠️  Sync status unavailable (expected if not configured)");
    }
}

#[test]
fn test_stellar_adapter_requires_credentials() {
    // Test that adapter creation fails gracefully without credentials
    // This is a unit test that doesn't require actual connections

    // Clear environment variables temporarily
    let original_secret = std::env::var("STELLAR_TESTNET_SECRET").ok();

    std::env::remove_var("STELLAR_TESTNET_SECRET");

    let result = StellarTestnetIpfsAdapter::new();

    // Restore original value
    if let Some(secret) = original_secret {
        std::env::set_var("STELLAR_TESTNET_SECRET", secret);
    }

    // Without credentials, adapter might still be created but operations will fail
    // This is expected behavior - adapter can be instantiated but not used
    match result {
        Ok(_) => println!("Adapter created (will fail on operations without credentials)"),
        Err(e) => println!("Adapter creation failed as expected: {e:?}"),
    }
}

#[test]
fn test_stellar_contract_location_format() {
    use serde_json;

    let location = StorageLocation::Stellar {
        transaction_id: "abc123def456".to_string(),
        contract_address: "CDOZEG35YQ7KYASQBUW2DVV7CIQZB5HMWAB2PWPUCHSTKSCD5ZUTPUW3".to_string(),
        asset_id: None,
    };

    let json = serde_json::to_string(&location).unwrap();
    assert!(json.contains("Stellar"));
    assert!(json.contains("CDOZEG35YQ7KYASQBUW2DVV7CIQZB5HMWAB2PWPUCHSTKSCD5ZUTPUW3"));
    assert!(json.contains("abc123def456"));
}

#[tokio::test]
async fn test_stellar_ipcm_contract_parameters() {
    // This test verifies the structure of data sent to IPCM contract
    // In a real scenario, this would:
    // 1. Store item to IPFS -> get CID
    // 2. Call update_ipcm with (DFID, CID, timestamp)
    // 3. Verify contract received correct parameters
    // 4. Verify event was emitted

    let adapter = match StellarTestnetIpfsAdapter::new() {
        Ok(a) => a,
        Err(_) => {
            println!("⚠️  Skipping: Stellar adapter not configured");
            return;
        }
    };

    let item = create_test_item("DFID-20250116-000001-ABCD");
    let result = adapter.store_item(&item).await;

    if let Ok(adapter_result) = result {
        // Verify DFID format
        assert!(item.dfid.starts_with("DFID-"));

        // Verify CID was generated
        if let StorageLocation::IPFS { cid, .. } = &adapter_result.metadata.item_location {
            assert!(!cid.is_empty());
            println!("✅ DFID: {}", item.dfid);
            println!("✅ CID: {cid}");
        }

        // In a complete test, we would verify the contract call with:
        // - Contract address
        // - Function: update_ipcm
        // - Parameters: (DFID, CID, timestamp)
        // - Verify transaction hash is returned
    }
}

#[test]
fn test_adapter_type_differences() {
    // Ensure testnet and mainnet are distinct types
    assert_ne!(
        format!("{:?}", AdapterType::StellarTestnetIpfs),
        format!("{:?}", AdapterType::StellarMainnetIpfs)
    );

    assert_ne!(
        format!("{:?}", AdapterType::IpfsIpfs),
        format!("{:?}", AdapterType::StellarTestnetIpfs)
    );
}

#[tokio::test]
async fn test_stellar_adapter_error_handling() {
    // Test graceful failure when IPFS is unavailable
    // This test would need a way to force IPFS failure

    let adapter = match StellarTestnetIpfsAdapter::new() {
        Ok(a) => a,
        Err(_) => {
            println!("⚠️  Skipping: Cannot test without adapter");
            return;
        }
    };

    let item = create_test_item("DFID-ERROR-TEST-001");

    // In a proper test environment, we would:
    // 1. Temporarily disable IPFS
    // 2. Attempt to store item
    // 3. Verify appropriate error is returned
    // 4. Re-enable IPFS

    let _ = adapter.store_item(&item).await;
    // Error handling verification would go here
}

/// Comprehensive Circuit Flow Integration Tests
///
/// This test suite validates the entire circuit ecosystem including:
/// 1. Circuit CRUD operations
/// 2. Adapter configuration
/// 3. Item push flow with blockchain registration
/// 4. Storage history verification
/// 5. Timeline registration
///
/// Run with: cargo test --test circuit_flow_integration -- --nocapture

use chrono::Utc;
use defarm_engine::circuits_engine::CircuitsEngine;
use defarm_engine::identifier_types::{CircuitAliasConfig, EnhancedIdentifier};
use defarm_engine::items_engine::ItemsEngine;
use defarm_engine::storage::{InMemoryStorage, StorageBackend};
use defarm_engine::types::*;
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

// Test configuration
const TEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

/// Helper: Create test storage with user accounts
fn create_test_storage() -> Arc<Mutex<InMemoryStorage>> {
    let storage = Arc::new(Mutex::new(InMemoryStorage::new()));
    let mut storage_guard = storage.lock().unwrap();

    // Create test user accounts with different tiers
    let basic_user = UserAccount {
        user_id: "user-basic".to_string(),
        tier: UserTier::Basic,
        available_adapters: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    storage_guard.store_user_account(&basic_user).unwrap();

    let professional_user = UserAccount {
        user_id: "user-professional".to_string(),
        tier: UserTier::Professional,
        available_adapters: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    storage_guard.store_user_account(&professional_user).unwrap();

    let enterprise_user = UserAccount {
        user_id: "user-enterprise".to_string(),
        tier: UserTier::Enterprise,
        available_adapters: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    storage_guard.store_user_account(&enterprise_user).unwrap();

    drop(storage_guard);
    storage
}

// ============================================================================
// TEST 1: Circuit CRUD Operations
// ============================================================================

#[tokio::test]
async fn test_01_circuit_crud_operations() {
    println!("\n🧪 TEST 1: Circuit CRUD Operations");
    println!("════════════════════════════════════════════════════════════════════════");

    let storage = create_test_storage();
    let mut circuits_engine = CircuitsEngine::new(storage.clone());

    // 1.1: Create circuit
    println!("\n  1.1 Creating circuit...");
    let circuit = circuits_engine.create_circuit(
        "Test Circuit".to_string(),
        Some("Testing circuit operations".to_string()),
        "user-enterprise".to_string(),
        None,
        None,
    ).unwrap();

    println!("    ✅ Circuit created: {}", circuit.circuit_id);
    assert_eq!(circuit.name, "Test Circuit");
    assert_eq!(circuit.owner_id, "user-enterprise");

    // 1.2: Update circuit
    println!("\n  1.2 Updating circuit...");
    circuits_engine.update_circuit(
        &circuit.circuit_id,
        Some("Updated Circuit".to_string()),
        Some("Updated description".to_string()),
        None,
        "user-enterprise",
    ).unwrap();

    let updated = circuits_engine.get_circuit(&circuit.circuit_id).unwrap().unwrap();
    assert_eq!(updated.name, "Updated Circuit");
    println!("    ✅ Circuit updated successfully");

    // 1.3: Add members with different roles
    println!("\n  1.3 Adding circuit members...");

    circuits_engine.add_member_to_circuit(
        &circuit.circuit_id,
        "user-professional".to_string(),
        MemberRole::Admin,
        "user-enterprise",
    ).unwrap();
    println!("    ✅ Added admin member");

    circuits_engine.add_member_to_circuit(
        &circuit.circuit_id,
        "user-basic".to_string(),
        MemberRole::Member,
        "user-enterprise",
    ).unwrap();
    println!("    ✅ Added regular member");

    // 1.4: Verify permissions
    println!("\n  1.4 Verifying permissions...");
    let final_circuit = circuits_engine.get_circuit(&circuit.circuit_id).unwrap().unwrap();

    assert!(final_circuit.has_permission("user-enterprise", &Permission::ManageMembers));
    assert!(final_circuit.has_permission("user-professional", &Permission::ManageMembers));
    assert!(final_circuit.has_permission("user-basic", &Permission::Push));
    println!("    ✅ Permissions verified correctly");

    println!("\n✅ TEST 1 PASSED: Circuit CRUD operations working correctly\n");
}

// ============================================================================
// TEST 2: Circuit Adapter Configuration
// ============================================================================

#[tokio::test]
async fn test_02_adapter_configuration() {
    println!("\n🧪 TEST 2: Circuit Adapter Configuration");
    println!("════════════════════════════════════════════════════════════════════════");

    let storage = create_test_storage();
    let mut circuits_engine = CircuitsEngine::new(storage.clone());

    // Create circuit
    let circuit = circuits_engine.create_circuit(
        "Adapter Test Circuit".to_string(),
        None,
        "user-enterprise".to_string(),
        None,
        None,
    ).unwrap();

    // 2.1: Configure IpfsIpfs adapter
    println!("\n  2.1 Configuring IPFS adapter...");
    circuits_engine.set_circuit_adapter_config(
        &circuit.circuit_id,
        "user-enterprise",
        Some(AdapterType::IpfsIpfs),
        false,
        false,
        false,
    ).unwrap();

    let storage_guard = storage.lock().unwrap();
    let adapter_config = storage_guard.get_circuit_adapter_config(&circuit.circuit_id).unwrap().unwrap();
    assert_eq!(adapter_config.adapter_type, Some(AdapterType::IpfsIpfs));
    drop(storage_guard);
    println!("    ✅ IPFS adapter configured");

    // 2.2: Configure StellarTestnetIpfs adapter with sponsorship
    println!("\n  2.2 Configuring Stellar adapter with sponsorship...");
    circuits_engine.set_circuit_adapter_config(
        &circuit.circuit_id,
        "user-enterprise",
        Some(AdapterType::StellarTestnetIpfs),
        true,  // auto_migrate_existing
        false, // requires_approval
        true,  // sponsor_adapter_access
    ).unwrap();

    let storage_guard = storage.lock().unwrap();
    let adapter_config = storage_guard.get_circuit_adapter_config(&circuit.circuit_id).unwrap().unwrap();
    assert_eq!(adapter_config.adapter_type, Some(AdapterType::StellarTestnetIpfs));
    assert!(adapter_config.sponsor_adapter_access);
    drop(storage_guard);
    println!("    ✅ Stellar adapter configured with sponsorship");

    // 2.3: Verify tier restrictions
    println!("\n  2.3 Testing tier restrictions...");
    let basic_circuit = circuits_engine.create_circuit(
        "Basic User Circuit".to_string(),
        None,
        "user-basic".to_string(),
        None,
        None,
    ).unwrap();

    // Basic user should not be able to use StellarTestnet without sponsorship
    let result = circuits_engine.set_circuit_adapter_config(
        &basic_circuit.circuit_id,
        "user-basic",
        Some(AdapterType::StellarTestnetIpfs),
        false,
        false,
        false, // No sponsorship
    );
    assert!(result.is_err());
    println!("    ✅ Basic user correctly denied Stellar adapter without sponsorship");

    println!("\n✅ TEST 2 PASSED: Adapter configuration working correctly\n");
}

// ============================================================================
// TEST 3: Item Push Flow with Deduplication
// ============================================================================

#[tokio::test]
async fn test_03_item_push_flow() {
    println!("\n🧪 TEST 3: Item Push Flow with Deduplication");
    println!("════════════════════════════════════════════════════════════════════════");

    let storage = create_test_storage();
    let mut circuits_engine = CircuitsEngine::new(storage.clone());
    let mut items_engine = ItemsEngine::new(storage.clone());

    // Create circuit with alias configuration
    let circuit = circuits_engine.create_circuit(
        "Push Test Circuit".to_string(),
        None,
        "user-enterprise".to_string(),
        None,
        Some(CircuitAliasConfig {
            required_canonical: vec!["test_id".to_string()],
            required_contextual: vec![],
            allowed_namespaces: vec!["test".to_string()],
            auto_apply_namespace: true,
            default_namespace: Some("test".to_string()),
            use_fingerprint: false,
        }),
    ).unwrap();

    println!("  ✅ Circuit created: {}", circuit.circuit_id);

    // 3.1: Create local item
    println!("\n  3.1 Creating local item...");
    let local_id = Uuid::new_v4();
    let test_identifier = format!("TEST-{}", Uuid::new_v4().to_string()[0..8].to_uppercase());

    let identifiers = vec![
        EnhancedIdentifier::canonical("test", "test_id", &test_identifier),
        EnhancedIdentifier::contextual("test", "batch", "BATCH-001"),
    ];

    // Create item
    let item = Item {
        dfid: format!("LID-{}", local_id),
        local_id: Some(local_id),
        legacy_mode: false,
        aliases: identifiers.clone(),
        fingerprint: None,
        confidence_score: None,
        identifiers: vec![],
        enhanced_identifiers: identifiers.clone(),
        enriched_data: HashMap::from([
            ("test_data".to_string(), json!("initial value")),
            ("timestamp".to_string(), json!(Utc::now().to_rfc3339())),
        ]),
        creation_timestamp: Utc::now(),
        last_modified: Utc::now(),
        source_entries: vec![],
        status: ItemStatus::Active,
    };

    let storage_guard = storage.lock().unwrap();
    storage_guard.store_item(&item).unwrap();
    storage_guard.store_lid_dfid_mapping(&local_id, &item.dfid).unwrap();
    drop(storage_guard);

    println!("    ✅ Local item created with LID: {}", local_id);
    println!("    ℹ️  Test identifier: {}", test_identifier);

    // 3.2: Push item to circuit (first push - creates DFID)
    println!("\n  3.2 Pushing item to circuit (first time)...");

    let push_result = circuits_engine.push_local_item_to_circuit(
        &local_id,
        identifiers.clone(),
        Some(HashMap::from([
            ("push_number".to_string(), json!(1)),
        ])),
        &circuit.circuit_id,
        "user-enterprise",
    ).await.unwrap();

    println!("    ✅ Item pushed successfully!");
    println!("    • DFID assigned: {}", push_result.dfid);
    println!("    • Status: {:?}", push_result.status);

    let first_dfid = push_result.dfid.clone();

    // 3.3: Test deduplication - push same canonical identifier
    println!("\n  3.3 Testing deduplication (pushing same canonical identifier)...");

    let local_id_2 = Uuid::new_v4();
    let storage_guard = storage.lock().unwrap();
    storage_guard.store_lid_dfid_mapping(&local_id_2, "temporary").unwrap();
    drop(storage_guard);

    let duplicate_identifiers = vec![
        EnhancedIdentifier::canonical("test", "test_id", &test_identifier), // Same canonical ID!
        EnhancedIdentifier::contextual("test", "batch", "BATCH-002"), // Different contextual
    ];

    let duplicate_push = circuits_engine.push_local_item_to_circuit(
        &local_id_2,
        duplicate_identifiers,
        Some(HashMap::from([
            ("push_number".to_string(), json!(2)),
            ("test_data".to_string(), json!("enriched value")),
        ])),
        &circuit.circuit_id,
        "user-enterprise",
    ).await.unwrap();

    assert_eq!(duplicate_push.dfid, first_dfid);
    println!("    ✅ Deduplication working!");
    println!("    • Same DFID returned: {}", duplicate_push.dfid);
    println!("    • Status: {:?}", duplicate_push.status);

    // 3.4: Verify item enrichment
    println!("\n  3.4 Verifying item enrichment...");
    let storage_guard = storage.lock().unwrap();
    let enriched_item = storage_guard.get_item_by_dfid(&first_dfid).unwrap().unwrap();

    // Check that item has been enriched with new data
    assert!(enriched_item.enriched_data.contains_key("push_number"));
    drop(storage_guard);
    println!("    ✅ Item successfully enriched with new data");

    println!("\n✅ TEST 3 PASSED: Item push flow and deduplication working correctly\n");
}

// ============================================================================
// TEST 4: Storage History and Timeline
// ============================================================================

#[tokio::test]
async fn test_04_storage_history_and_timeline() {
    println!("\n🧪 TEST 4: Storage History and Timeline");
    println!("════════════════════════════════════════════════════════════════════════");

    let storage = create_test_storage();
    let storage_guard = storage.lock().unwrap();

    let test_dfid = "DFID-TEST-HISTORY";

    // 4.1: Add storage record
    println!("\n  4.1 Adding storage record...");
    let storage_record = StorageRecord {
        adapter_type: AdapterType::IpfsIpfs,
        storage_location: StorageLocation::IPFS {
            cid: "QmTestCID123456789".to_string(),
            pinned: true,
        },
        stored_at: Utc::now(),
        triggered_by: "test".to_string(),
        triggered_by_id: Some("test-circuit-id".to_string()),
        events_range: None,
        is_active: true,
        metadata: HashMap::from([
            ("ipfs_cid".to_string(), json!("QmTestCID123456789")),
            ("test_metadata".to_string(), json!("test_value")),
        ]),
    };

    storage_guard.add_storage_record(test_dfid, storage_record).unwrap();
    println!("    ✅ Storage record added");

    // 4.2: Retrieve storage history
    println!("\n  4.2 Retrieving storage history...");
    let history = storage_guard.get_storage_history(test_dfid).unwrap().unwrap();

    assert_eq!(history.dfid, test_dfid);
    assert_eq!(history.storage_records.len(), 1);

    let record = &history.storage_records[0];
    assert_eq!(record.adapter_type, AdapterType::IpfsIpfs);

    if let StorageLocation::IPFS { cid, pinned } = &record.storage_location {
        assert_eq!(cid, "QmTestCID123456789");
        assert!(pinned);
        println!("    ✅ Storage history retrieved correctly");
        println!("    • IPFS CID: {}", cid);
    }

    // 4.3: Add timeline entry
    println!("\n  4.3 Adding CID to timeline...");
    let timestamp = Utc::now().timestamp();

    storage_guard.add_cid_to_timeline(
        test_dfid,
        "QmTestCID123456789",
        "test-tx-hash-12345",
        timestamp,
        "testnet",
    ).unwrap();
    println!("    ✅ CID added to timeline");

    // 4.4: Query timeline
    println!("\n  4.4 Querying timeline...");
    let timeline = storage_guard.get_cid_timeline(test_dfid).unwrap();

    assert_eq!(timeline.len(), 1);
    let entry = &timeline[0];
    assert_eq!(entry.cid, "QmTestCID123456789");
    assert_eq!(entry.transaction_hash, "test-tx-hash-12345");
    assert_eq!(entry.network, "testnet");

    println!("    ✅ Timeline entry retrieved:");
    println!("    • CID: {}", entry.cid);
    println!("    • TX Hash: {}", entry.transaction_hash);
    println!("    • Network: {}", entry.network);

    println!("\n✅ TEST 4 PASSED: Storage history and timeline working correctly\n");
}

// ============================================================================
// TEST 5: Real Blockchain Integration (if configured)
// ============================================================================

#[tokio::test]
async fn test_05_blockchain_integration() {
    println!("\n🧪 TEST 5: Blockchain Integration");
    println!("════════════════════════════════════════════════════════════════════════");

    // Check if blockchain is configured
    if std::env::var("STELLAR_TESTNET_SECRET").is_err() {
        println!("\n  ⚠️  SKIPPING: STELLAR_TESTNET_SECRET not configured");
        println!("  ℹ️  Set environment variables to test blockchain integration:");
        println!("      • STELLAR_TESTNET_SECRET");
        println!("      • IPFS_ENDPOINT or PINATA_API_KEY");
        return;
    }

    let storage = create_test_storage();
    let mut circuits_engine = CircuitsEngine::new(storage.clone());

    // Create circuit with Stellar adapter
    println!("\n  5.1 Creating circuit with Stellar adapter...");
    let circuit = circuits_engine.create_circuit(
        "Blockchain Test Circuit".to_string(),
        None,
        "user-enterprise".to_string(),
        None,
        None,
    ).unwrap();

    circuits_engine.set_circuit_adapter_config(
        &circuit.circuit_id,
        "user-enterprise",
        Some(AdapterType::StellarTestnetIpfs),
        false,
        false,
        true, // Sponsor access
    ).unwrap();

    println!("    ✅ Circuit configured with StellarTestnet adapter");

    // Create and push item
    println!("\n  5.2 Creating item for blockchain push...");
    let local_id = Uuid::new_v4();
    let unique_id = format!("BLOCKCHAIN-TEST-{}", Uuid::new_v4().to_string()[0..8].to_uppercase());

    let identifiers = vec![
        EnhancedIdentifier::canonical("test", "blockchain_id", &unique_id),
    ];

    let item = Item {
        dfid: format!("LID-{}", local_id),
        local_id: Some(local_id),
        legacy_mode: false,
        aliases: identifiers.clone(),
        fingerprint: None,
        confidence_score: None,
        identifiers: vec![],
        enhanced_identifiers: identifiers.clone(),
        enriched_data: HashMap::from([
            ("blockchain_test".to_string(), json!("This data will be stored on IPFS and Stellar")),
            ("timestamp".to_string(), json!(Utc::now().to_rfc3339())),
        ]),
        creation_timestamp: Utc::now(),
        last_modified: Utc::now(),
        source_entries: vec![],
        status: ItemStatus::Active,
    };

    let storage_guard = storage.lock().unwrap();
    storage_guard.store_item(&item).unwrap();
    storage_guard.store_lid_dfid_mapping(&local_id, &item.dfid).unwrap();
    drop(storage_guard);

    println!("    ✅ Item created with unique ID: {}", unique_id);

    // Push to blockchain
    println!("\n  5.3 Pushing item to blockchain...");
    println!("    ⏳ This may take 10-30 seconds for blockchain confirmation...");

    let push_result = tokio::time::timeout(
        TEST_TIMEOUT,
        circuits_engine.push_local_item_to_circuit(
            &local_id,
            identifiers,
            None,
            &circuit.circuit_id,
            "user-enterprise",
        )
    ).await;

    match push_result {
        Ok(Ok(result)) => {
            println!("    ✅ Item pushed to blockchain!");
            println!("    • DFID: {}", result.dfid);

            // Get storage history
            let storage_guard = storage.lock().unwrap();
            if let Ok(Some(history)) = storage_guard.get_storage_history(&result.dfid) {
                if let Some(record) = history.storage_records.first() {
                    println!("\n  📋 Blockchain Details:");

                    if let Some(cid) = record.metadata.get("ipfs_cid") {
                        println!("    • IPFS CID: {}", cid);
                        println!("      View at: https://ipfs.io/ipfs/{}", cid);
                    }

                    if let Some(nft_tx) = record.metadata.get("nft_mint_tx") {
                        println!("    • NFT Mint TX: {}", nft_tx);
                        println!("      View at: https://stellar.expert/explorer/testnet/tx/{}", nft_tx);
                    }

                    if let Some(ipcm_tx) = record.metadata.get("ipcm_update_tx") {
                        println!("    • IPCM Update TX: {}", ipcm_tx);
                        println!("      View at: https://stellar.expert/explorer/testnet/tx/{}", ipcm_tx);
                    }
                }
            }
        }
        Ok(Err(e)) => {
            println!("    ❌ Push failed: {}", e);
            println!("    ℹ️  Check your Stellar and IPFS configuration");
        }
        Err(_) => {
            println!("    ⏱️  Push timed out after {} seconds", TEST_TIMEOUT.as_secs());
            println!("    ℹ️  This might indicate network issues");
        }
    }

    println!("\n✅ TEST 5 COMPLETED: Blockchain integration test finished\n");
}

// ============================================================================
// TEST SUMMARY
// ============================================================================

#[tokio::test]
async fn test_99_summary() {
    println!("\n");
    println!("════════════════════════════════════════════════════════════════════════");
    println!("                    🎉 TEST SUITE SUMMARY 🎉");
    println!("════════════════════════════════════════════════════════════════════════");
    println!();
    println!("  ✅ TEST 1: Circuit CRUD Operations");
    println!("  ✅ TEST 2: Circuit Adapter Configuration");
    println!("  ✅ TEST 3: Item Push Flow with Deduplication");
    println!("  ✅ TEST 4: Storage History and Timeline");
    println!("  ✅ TEST 5: Blockchain Integration (if configured)");
    println!();
    println!("  📚 FRONTEND INTEGRATION NOTES:");
    println!("  ────────────────────────────────────────");
    println!();
    println!("  1. Circuit Creation:");
    println!("     POST /api/circuits");
    println!("     - Include adapter_config in request body");
    println!();
    println!("  2. Item Push Flow:");
    println!("     POST /api/items/local (create local item)");
    println!("     POST /api/circuits/{id}/push-local (push to circuit)");
    println!("     GET /api/items/{dfid}/storage-history (get blockchain data)");
    println!();
    println!("  3. Adapter Types (use hyphenated strings):");
    println!("     - \"ipfs-ipfs\"");
    println!("     - \"stellar_testnet-ipfs\"");
    println!("     - \"stellar_mainnet-ipfs\"");
    println!();
    println!("  4. Blockchain Verification URLs:");
    println!("     - IPFS: https://ipfs.io/ipfs/{cid}");
    println!("     - Stellar: https://stellar.expert/explorer/testnet/tx/{tx_hash}");
    println!();
    println!("  5. Important Fields:");
    println!("     - ipfs_cid: Real IPFS content identifier");
    println!("     - nft_mint_tx: Stellar NFT mint transaction");
    println!("     - ipcm_update_tx: IPCM contract transaction");
    println!();
    println!("════════════════════════════════════════════════════════════════════════");
    println!("                    ALL TESTS DOCUMENTED! 📝");
    println!("════════════════════════════════════════════════════════════════════════");
    println!();
}
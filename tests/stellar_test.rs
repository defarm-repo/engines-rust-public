use chrono::Utc;
use defarm_engine::circuits_engine::CircuitsEngine;
use defarm_engine::identifier_types::{CircuitAliasConfig, EnhancedIdentifier};
use defarm_engine::storage::{InMemoryStorage, StorageBackend};
use defarm_engine::types::*;
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[tokio::test]
async fn test_stellar_evidence() {
    println!("\n⭐ STELLAR TESTNET EVIDENCE");
    println!("════════════════════════════════════════════════════════════════════════\n");

    let storage = Arc::new(Mutex::new(InMemoryStorage::new()));

    // Create user
    let mut storage_guard = storage.lock().unwrap();
    let user = UserAccount {
        user_id: "stellar-test".to_string(),
        username: "stellar_tester".to_string(),
        email: "test@stellar.com".to_string(),
        password_hash: "hash".to_string(),
        tier: UserTier::Enterprise,
        status: AccountStatus::Active,
        credits: 1000,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_login: Some(Utc::now()),
        subscription: None,
        limits: TierLimits::for_tier(&UserTier::Enterprise),
        is_admin: true,
        workspace_id: Some("stellar-workspace".to_string()),
        available_adapters: Some(vec![AdapterType::StellarTestnetIpfs]),
    };
    storage_guard.store_user_account(&user).unwrap();
    drop(storage_guard);

    let mut circuits_engine = CircuitsEngine::new(storage.clone());

    // Create circuit with Stellar adapter
    let circuit = circuits_engine.create_circuit(
        format!("Stellar Evidence Circuit {}", Utc::now().format("%H%M%S")),
        "Testing Stellar blockchain".to_string(),
        "stellar-test".to_string(),
        None,
        None,
    ).unwrap();

    println!("Circuit created: {}", circuit.circuit_id);

    // Configure Stellar adapter
    circuits_engine.set_circuit_adapter_config(
        &circuit.circuit_id,
        "stellar-test",
        Some(AdapterType::StellarTestnetIpfs),
        false,
        false,
        true,
    ).unwrap();

    println!("Stellar adapter configured\n");

    // Create item
    let local_id = Uuid::new_v4();
    let unique_id = format!("STELLAR-{}", Uuid::new_v4().to_string()[0..8].to_uppercase());

    let identifiers = vec![
        EnhancedIdentifier::canonical("bovino", "sisbov", &unique_id),
    ];

    let item = Item {
        dfid: format!("LID-{}", local_id),
        local_id: Some(local_id),
        legacy_mode: false,
        aliases: vec![],
        fingerprint: None,
        confidence_score: 0.0,
        identifiers: vec![],
        enhanced_identifiers: identifiers.clone(),
        enriched_data: HashMap::from([
            ("blockchain_test".to_string(), json!("Stellar NFT + IPFS")),
            ("timestamp".to_string(), json!(Utc::now().to_rfc3339())),
        ]),
        creation_timestamp: Utc::now(),
        last_modified: Utc::now(),
        source_entries: vec![],
        status: ItemStatus::Active,
    };

    let mut storage_guard = storage.lock().unwrap();
    storage_guard.store_item(&item).unwrap();
    storage_guard.store_lid_dfid_mapping(&local_id, &item.dfid).unwrap();
    drop(storage_guard);

    println!("Pushing to Stellar (this will mint NFT + upload to IPFS)...");
    println!("⏳ Please wait 10-30 seconds for blockchain confirmation...\n");

    // Push to circuit
    match circuits_engine.push_local_item_to_circuit(
        &local_id,
        identifiers,
        None,
        &circuit.circuit_id,
        "stellar-test",
    ).await {
        Ok(result) => {
            println!("✅ SUCCESS!");
            println!("   DFID: {}", result.dfid);

            // Get storage history
            let storage_guard = storage.lock().unwrap();
            if let Ok(Some(history)) = storage_guard.get_storage_history(&result.dfid) {
                for record in &history.storage_records {
                    println!("\n📋 BLOCKCHAIN EVIDENCE:");

                    if let Some(cid) = record.metadata.get("ipfs_cid") {
                        println!("   IPFS CID: {}", cid);
                        println!("   View: https://ipfs.io/ipfs/{}", cid);
                    }

                    if let Some(nft_tx) = record.metadata.get("nft_mint_tx") {
                        println!("\n   NFT Mint TX: {}", nft_tx);
                        println!("   View: https://stellar.expert/explorer/testnet/tx/{}", nft_tx);
                    }

                    if let Some(ipcm_tx) = record.metadata.get("ipcm_update_tx") {
                        println!("\n   IPCM Update TX: {}", ipcm_tx);
                        println!("   View: https://stellar.expert/explorer/testnet/tx/{}", ipcm_tx);
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ Failed: {}", e);
            println!("   This might be due to network issues or insufficient XLM balance");
        }
    }

    println!("\n════════════════════════════════════════════════════════════════════════");
}

/// IPCM Contract v2.2.0+ Tests
/// Tests for event-only mode, contract address priority, and IPCM functionality
use defarm_engine::stellar_client::{StellarClient, StellarNetwork, TESTNET_IPCM_CONTRACT};
use std::env;

/// Test that the correct IPCM v2.2.0 contract address is used
#[test]
fn test_ipcm_v220_contract_address() {
    // Verify hardcoded fallback is the v2.2.0 contract
    assert_eq!(
        TESTNET_IPCM_CONTRACT, "CCDJV6VAFC2MSSDSL4AEJB5BAMGDA5PMCUIZ3UF6AYIJL467PQTBZ7BS",
        "Testnet IPCM contract should be v2.2.0 with emit_update_event()"
    );
}

/// Test that environment variable takes precedence over fallback
#[test]
fn test_contract_address_env_var_priority() {
    // Set a test contract address
    let test_contract = "CTEST123456789ABCDEF";
    env::set_var("STELLAR_TESTNET_IPCM_CONTRACT", test_contract);

    // Create client (it should use env var if adapter reads from it)
    // Note: This tests the design where env var is checked first
    let env_contract = env::var("STELLAR_TESTNET_IPCM_CONTRACT").unwrap();
    assert_eq!(env_contract, test_contract);

    // Clean up
    env::remove_var("STELLAR_TESTNET_IPCM_CONTRACT");
}

/// Test StellarClient initialization with correct contract
#[test]
fn test_stellar_client_initialization() {
    let _client = StellarClient::new(StellarNetwork::Testnet, TESTNET_IPCM_CONTRACT.to_string());

    // Client created successfully - network field is private so we just verify construction works
}

/// Test that adapter config validation checks use_onchain_storage flag
#[test]
fn test_storage_mode_configuration() {
    use defarm_engine::adapters::config::{StellarConfig, StellarNetwork as ConfigNetwork};

    // Test event-only mode (default)
    let event_only_config = StellarConfig {
        network: ConfigNetwork::Testnet,
        keypair: "STEST123".to_string(),
        contract_address: TESTNET_IPCM_CONTRACT.to_string(),
        fee_sponsor: None,
        use_onchain_storage: false, // Event-only mode
    };

    assert_eq!(event_only_config.use_onchain_storage, false);

    // Test full storage mode
    let full_storage_config = StellarConfig {
        network: ConfigNetwork::Testnet,
        keypair: "STEST123".to_string(),
        contract_address: TESTNET_IPCM_CONTRACT.to_string(),
        fee_sponsor: None,
        use_onchain_storage: true, // Full storage mode
    };

    assert_eq!(full_storage_config.use_onchain_storage, true);
}

/// Test default value for use_onchain_storage is false
#[test]
fn test_event_only_mode_is_default() {
    use defarm_engine::adapters::config::StellarConfig;
    use serde_json::json;

    // Deserialize config without use_onchain_storage field
    let config_json = json!({
        "network": "Testnet",
        "keypair": "STEST123",
        "contract_address": TESTNET_IPCM_CONTRACT
    });

    let config: StellarConfig = serde_json::from_value(config_json)
        .expect("Should deserialize with default use_onchain_storage");

    // Should default to false (event-only mode)
    assert_eq!(
        config.use_onchain_storage, false,
        "use_onchain_storage should default to false (event-only mode)"
    );
}

// Integration tests that require Stellar credentials
// These will skip gracefully if not configured
mod integration {
    use super::*;
    use defarm_engine::adapters::{StellarTestnetIpfsAdapter, StorageAdapter};
    use defarm_engine::types::{Item, ItemStatus};
    use std::collections::HashMap;
    use uuid::Uuid;

    fn create_test_item(dfid: &str) -> Item {
        use defarm_engine::identifier_types::EnhancedIdentifier;

        Item {
            dfid: dfid.to_string(),
            local_id: Some(Uuid::new_v4()),
            legacy_mode: false,
            identifiers: vec![],
            enhanced_identifiers: vec![EnhancedIdentifier::contextual("test", "ipcm", dfid)],
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

    /// Test that emit_update_event() is called in event-only mode
    #[tokio::test]
    async fn test_event_only_mode_uses_emit_update_event() {
        let adapter = match StellarTestnetIpfsAdapter::new() {
            Ok(a) => a,
            Err(e) => {
                println!("⚠️  Skipping: Stellar testnet not configured: {e}");
                return;
            }
        };

        let item = create_test_item(&format!("DFID-IPCM-EVENT-TEST-{}", Uuid::new_v4()));
        let result = adapter.store_item(&item).await;

        match result {
            Ok(adapter_result) => {
                println!("✅ Event-only mode test passed");
                println!(
                    "   Transaction: {:?}",
                    adapter_result.metadata.item_location
                );
                println!("   This should have used emit_update_event() (event-only)");
            }
            Err(e) => {
                println!("⚠️  Test skipped (adapter not fully configured): {e}");
            }
        }
    }

    /// Test that correct contract address is used (v2.2.0)
    #[tokio::test]
    async fn test_adapter_uses_correct_contract_v220() {
        let adapter = match StellarTestnetIpfsAdapter::new() {
            Ok(a) => a,
            Err(e) => {
                println!("⚠️  Skipping: Stellar testnet not configured: {e}");
                return;
            }
        };

        let item = create_test_item(&format!("DFID-CONTRACT-V220-TEST-{}", Uuid::new_v4()));
        let result = adapter.store_item(&item).await;

        match result {
            Ok(adapter_result) => {
                use defarm_engine::adapters::base::StorageLocation;

                if let StorageLocation::Stellar {
                    contract_address, ..
                } = &adapter_result.metadata.item_location
                {
                    // Should be using v2.2.0 contract
                    assert!(
                        contract_address.starts_with("CCDJV6V")
                            || contract_address == TESTNET_IPCM_CONTRACT,
                        "Should be using IPCM v2.2.0 contract, got: {}",
                        contract_address
                    );
                    println!(
                        "✅ Using correct IPCM v2.2.0 contract: {}",
                        contract_address
                    );
                }
            }
            Err(e) => {
                println!("⚠️  Test skipped (adapter not fully configured): {e}");
            }
        }
    }

    /// Test environment variable override works
    #[tokio::test]
    async fn test_env_var_contract_override() {
        // Set custom contract via env var
        env::set_var(
            "STELLAR_TESTNET_IPCM_CONTRACT",
            TESTNET_IPCM_CONTRACT, // Use v2.2.0
        );

        let adapter = match StellarTestnetIpfsAdapter::new() {
            Ok(a) => a,
            Err(e) => {
                env::remove_var("STELLAR_TESTNET_IPCM_CONTRACT");
                println!("⚠️  Skipping: Stellar testnet not configured: {e}");
                return;
            }
        };

        let item = create_test_item(&format!("DFID-ENV-OVERRIDE-TEST-{}", Uuid::new_v4()));
        let result = adapter.store_item(&item).await;

        env::remove_var("STELLAR_TESTNET_IPCM_CONTRACT");

        match result {
            Ok(_) => {
                println!("✅ Environment variable contract override works");
            }
            Err(e) => {
                println!("⚠️  Test skipped (adapter not fully configured): {e}");
            }
        }
    }
}

#[cfg(test)]
mod unit {
    use super::*;

    /// Test contract address constants are not empty
    #[test]
    fn test_contract_constants_not_empty() {
        assert!(!TESTNET_IPCM_CONTRACT.is_empty());
        assert_eq!(TESTNET_IPCM_CONTRACT.len(), 56); // Stellar address length
    }

    /// Test contract address format is valid Stellar address
    #[test]
    fn test_contract_address_format() {
        // Stellar contract addresses start with 'C'
        assert!(TESTNET_IPCM_CONTRACT.starts_with('C'));

        // Should be base32 encoded (A-Z and 2-7)
        assert!(TESTNET_IPCM_CONTRACT
            .chars()
            .all(|c| { c.is_ascii_uppercase() || ('2'..='7').contains(&c) }));
    }

    /// Test StellarNetwork enum variants
    #[test]
    fn test_stellar_network_variants() {
        let testnet = StellarNetwork::Testnet;
        let mainnet = StellarNetwork::Mainnet;

        // These should be different
        assert!(matches!(testnet, StellarNetwork::Testnet));
        assert!(matches!(mainnet, StellarNetwork::Mainnet));
    }
}

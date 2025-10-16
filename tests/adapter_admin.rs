/// Admin adapter management tests
/// Tests admin operations for creating, updating, testing, and managing adapter configurations
use defarm_engine::adapter_manager::AdapterManager;
use defarm_engine::logging::LoggingEngine;
use defarm_engine::storage::InMemoryStorage;
use defarm_engine::types::{
    AdapterConfig, AdapterConnectionDetails, AdapterType, AuthType, ContractConfigs, ContractInfo,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

fn create_test_adapter_config(adapter_type: AdapterType, name: &str) -> AdapterConfig {
    let mut custom_headers = HashMap::new();
    custom_headers.insert("stellar_secret".to_string(), "SECRET_KEY".to_string());

    let ipcm_info = ContractInfo {
        contract_address: "CTEST_CONTRACT_ADDRESS".to_string(),
        contract_name: "IPCM Contract".to_string(),
        abi: None,
        methods: HashMap::new(),
    };

    AdapterConfig {
        config_id: Uuid::new_v4(),
        name: name.to_string(),
        description: format!("{:?} adapter configuration", adapter_type),
        adapter_type,
        connection_details: AdapterConnectionDetails {
            endpoint: "https://api.pinata.cloud".to_string(),
            api_key: Some("test_api_key".to_string()),
            secret_key: Some("test_secret_key".to_string()),
            auth_type: AuthType::ApiKey,
            timeout_ms: 30000,
            retry_attempts: 3,
            max_concurrent_requests: 10,
            custom_headers,
        },
        contract_configs: Some(ContractConfigs {
            mint_contract: None,
            ipcm_contract: Some(ipcm_info),
            network: "testnet".to_string(),
            chain_id: None,
        }),
        is_active: true,
        is_default: false,
        created_by: "admin1".to_string(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        last_tested_at: None,
        test_status: None,
    }
}

#[test]
fn test_admin_creates_adapter_configuration() {
    let storage = Arc::new(Mutex::new(InMemoryStorage::new()));
    let logger = LoggingEngine::new();
    let mut manager = AdapterManager::new(Arc::clone(&storage), logger);

    let mut custom_headers = HashMap::new();
    custom_headers.insert("stellar_secret".to_string(), "SECRET_KEY".to_string());

    let connection_details = AdapterConnectionDetails {
        endpoint: "https://api.pinata.cloud".to_string(),
        api_key: Some("test_api_key".to_string()),
        secret_key: Some("test_secret_key".to_string()),
        auth_type: AuthType::ApiKey,
        timeout_ms: 30000,
        retry_attempts: 3,
        max_concurrent_requests: 10,
        custom_headers,
    };

    let result = manager.create_adapter_config(
        "Test IPFS Config".to_string(),
        "IPFS adapter for testing".to_string(),
        AdapterType::IpfsIpfs,
        connection_details,
        None,
        "admin1".to_string(),
    );

    assert!(
        result.is_ok(),
        "Admin should be able to create adapter configuration"
    );

    // Verify config was created
    let config = result.unwrap();
    assert_eq!(config.name, "Test IPFS Config");
    assert_eq!(config.adapter_type, AdapterType::IpfsIpfs);
    assert_eq!(config.created_by, "admin1");
}

#[test]
fn test_adapter_config_stores_contract_addresses() {
    let config = create_test_adapter_config(
        AdapterType::StellarTestnetIpfs,
        "Contract Test",
    );

    assert!(config.contract_configs.is_some());

    let contract_configs = config.contract_configs.unwrap();
    assert_eq!(contract_configs.network, "testnet");
    assert!(contract_configs.ipcm_contract.is_some());

    let ipcm = contract_configs.ipcm_contract.unwrap();
    assert_eq!(ipcm.contract_address, "CTEST_CONTRACT_ADDRESS");
    assert_eq!(ipcm.contract_name, "IPCM Contract");
}

#[test]
fn test_adapter_config_stores_api_credentials() {
    let config = create_test_adapter_config(AdapterType::IpfsIpfs, "Credentials Test");

    assert!(config.connection_details.api_key.is_some());
    assert!(config.connection_details.secret_key.is_some());
    assert_eq!(
        config.connection_details.api_key.unwrap(),
        "test_api_key"
    );
    assert_eq!(
        config.connection_details.secret_key.unwrap(),
        "test_secret_key"
    );
}

#[test]
fn test_adapter_config_serialization() {
    use serde_json;

    let config = create_test_adapter_config(AdapterType::StellarTestnetIpfs, "Serialization Test");

    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("StellarTestnetIpfs"));
    assert!(json.contains("Serialization Test"));

    // Deserialize back
    let deserialized: AdapterConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.name, config.name);
    assert_eq!(deserialized.adapter_type, config.adapter_type);
}

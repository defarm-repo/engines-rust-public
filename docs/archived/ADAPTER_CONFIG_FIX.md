# Adapter Configuration Architecture - Database-Driven

## Current Problem

Adapters are reading credentials from environment variables (hardcoded):
```rust
// WRONG - hardcoded env vars
let api_key = std::env::var("PINATA_API_KEY")?;
let secret = std::env::var("PINATA_SECRET_KEY")?;
```

## Correct Architecture

Adapters should read from `AdapterConfig` stored in database:
```rust
// CORRECT - from database config
pub struct StellarTestnetIpfsAdapter {
    config: AdapterConfig,  // Contains credentials from DB
    stellar_client: Arc<StellarClient>,
    ipfs_client: Arc<IpfsClient>,
}
```

## How It Should Work

### 1. Admin Creates Adapter Configs

```bash
POST /api/admin/adapters
{
  "name": "Production Pinata + Stellar Testnet",
  "description": "IPFS via Pinata + Stellar Testnet IPCM",
  "adapter_type": "stellar_testnet-ipfs",
  "connection_details": {
    "endpoint": "https://api.pinata.cloud",
    "api_key": "484ee5434683a9e07950",
    "secret_key": "7128ebb6d0415df4ea1d00099b98047798ee2be0d7d28b04a6cb61cde4115829",
    "auth_type": "ApiKey"
  },
  "contract_configs": {
    "contracts": [
      {
        "name": "IPCM",
        "address": "CCDJV6VAFC2MSSDSL4AEJB5BAMGDA5PMCUIZ3UF6AYIJL467PQTBZ7BS",
        "network": "testnet"
      }
    ]
  }
}
```

### 2. Circuit Owner Chooses Adapter

```bash
PUT /api/circuits/{circuit_id}/adapter
{
  "adapter_config_id": "uuid-of-adapter-config",
  "sponsor_adapter_access": false
}
```

### 3. Push Uses Adapter Config

```rust
// When push occurs:
// 1. Get circuit's adapter_config_id
// 2. Load AdapterConfig from database
// 3. Create adapter instance WITH config
// 4. Use config's credentials for IPFS/Stellar
```

## Database Schema

```
adapter_configs table:
- config_id (UUID)
- name (String) - e.g., "Production Pinata + Stellar Testnet"
- description
- adapter_type (enum) - stellar_testnet-ipfs, ipfs-ipfs, etc.
- connection_details (JSON):
  - endpoint: "https://api.pinata.cloud"
  - api_key: "484ee..."
  - secret_key: "7128ebb..."
  - auth_type: ApiKey
- contract_configs (JSON):
  - contracts: [{name, address, network}]
- is_active (bool)
- is_default (bool)
- created_by
- created_at
- updated_at
```

## Required Changes

### 1. Update Adapter Constructors

**BEFORE (Wrong)**:
```rust
impl StellarTestnetIpfsAdapter {
    pub fn new() -> Result<Self, StorageError> {
        let api_key = std::env::var("PINATA_API_KEY")?;  // WRONG!
        // ...
    }
}
```

**AFTER (Correct)**:
```rust
impl StellarTestnetIpfsAdapter {
    pub fn from_config(config: &AdapterConfig) -> Result<Self, StorageError> {
        // Read from config.connection_details
        let api_key = config.connection_details.api_key
            .as_ref()
            .ok_or(StorageError::ConfigurationError("Missing API key"))?;

        let secret = config.connection_details.secret_key
            .as_ref()
            .ok_or(StorageError::ConfigurationError("Missing secret"))?;

        let ipfs_client = IpfsClient::with_pinata(
            api_key.clone(),
            secret.clone()
        )?;

        // Get contract address from config.contract_configs
        let contract_address = config.contract_configs
            .as_ref()
            .and_then(|cc| cc.contracts.iter().find(|c| c.name == "IPCM"))
            .map(|c| c.address.clone())
            .ok_or(StorageError::ConfigurationError("Missing IPCM contract"))?;

        let stellar_client = StellarClient::new(
            StellarNetwork::Testnet,
            contract_address
        );

        Ok(Self {
            config: config.clone(),
            stellar_client: Arc::new(stellar_client),
            ipfs_client: Arc::new(ipfs_client),
        })
    }
}
```

### 2. Update Adapter Factory

**BEFORE**:
```rust
pub fn create_adapter_instance(adapter_type: &AdapterType)
    -> Result<AdapterInstance, StorageError>
{
    match adapter_type {
        AdapterType::StellarTestnetIpfs => {
            Ok(AdapterInstance::StellarTestnetIpfs(
                StellarTestnetIpfsAdapter::new()?  // WRONG!
            ))
        }
        // ...
    }
}
```

**AFTER**:
```rust
pub fn create_adapter_instance(
    adapter_config: &AdapterConfig,
    storage: &dyn StorageBackend
) -> Result<AdapterInstance, StorageError> {
    match adapter_config.adapter_type {
        AdapterType::StellarTestnetIpfs => {
            Ok(AdapterInstance::StellarTestnetIpfs(
                StellarTestnetIpfsAdapter::from_config(adapter_config)?
            ))
        }
        // ...
    }
}
```

### 3. Initialize Default Adapter Configs on Startup

```rust
// In db_init.rs
pub fn initialize_production_adapters(
    storage: &mut InMemoryStorage
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔌 Initializing production adapters...");

    // Read from .env
    let pinata_api_key = std::env::var("PINATA_API_KEY")?;
    let pinata_secret = std::env::var("PINATA_SECRET_KEY")?;
    let testnet_ipcm = std::env::var("STELLAR_TESTNET_IPCM_CONTRACT")?;
    let mainnet_ipcm = std::env::var("STELLAR_MAINNET_IPCM_CONTRACT")?;
    let mainnet_secret = std::env::var("STELLAR_MAINNET_SECRET_KEY")?;

    // Create IPFS-IPFS config
    let ipfs_config = AdapterConfig {
        config_id: Uuid::new_v4(),
        name: "Production IPFS (Pinata)".to_string(),
        description: "IPFS storage via Pinata cloud".to_string(),
        adapter_type: AdapterType::IpfsIpfs,
        connection_details: AdapterConnectionDetails {
            endpoint: "https://api.pinata.cloud".to_string(),
            api_key: Some(pinata_api_key.clone()),
            secret_key: Some(pinata_secret.clone()),
            auth_type: AuthType::ApiKey,
            ..Default::default()
        },
        contract_configs: None,
        is_active: true,
        is_default: false,
        created_by: "system".to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_tested_at: None,
        test_status: None,
    };
    storage.store_adapter_config(&ipfs_config)?;
    println!("   ✅ IPFS-IPFS adapter registered");

    // Create Stellar Testnet config
    let testnet_config = AdapterConfig {
        config_id: Uuid::new_v4(),
        name: "Stellar Testnet + IPFS".to_string(),
        description: "NFTs on Stellar testnet + IPFS events".to_string(),
        adapter_type: AdapterType::StellarTestnetIpfs,
        connection_details: AdapterConnectionDetails {
            endpoint: "https://api.pinata.cloud".to_string(),
            api_key: Some(pinata_api_key.clone()),
            secret_key: Some(pinata_secret.clone()),
            auth_type: AuthType::ApiKey,
            ..Default::default()
        },
        contract_configs: Some(ContractConfigs {
            contracts: vec![
                ContractInfo {
                    name: "IPCM".to_string(),
                    address: testnet_ipcm,
                    network: Some("testnet".to_string()),
                    abi_url: None,
                    method_configs: HashMap::new(),
                }
            ]
        }),
        is_active: true,
        is_default: false,
        created_by: "system".to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_tested_at: None,
        test_status: None,
    };
    storage.store_adapter_config(&testnet_config)?;
    println!("   ✅ Stellar Testnet-IPFS adapter registered");

    // Create Stellar Mainnet config
    let mainnet_config = AdapterConfig {
        config_id: Uuid::new_v4(),
        name: "Stellar Mainnet + IPFS (Production)".to_string(),
        description: "Production NFTs on Stellar mainnet + IPFS".to_string(),
        adapter_type: AdapterType::StellarMainnetIpfs,
        connection_details: AdapterConnectionDetails {
            endpoint: "https://api.pinata.cloud".to_string(),
            api_key: Some(pinata_api_key.clone()),
            secret_key: Some(pinata_secret.clone()),
            auth_type: AuthType::ApiKey,
            custom_headers: {
                let mut headers = HashMap::new();
                headers.insert(
                    "stellar_secret".to_string(),
                    mainnet_secret
                );
                headers
            },
            ..Default::default()
        },
        contract_configs: Some(ContractConfigs {
            contracts: vec![
                ContractInfo {
                    name: "IPCM".to_string(),
                    address: mainnet_ipcm,
                    network: Some("mainnet".to_string()),
                    abi_url: None,
                    method_configs: HashMap::new(),
                }
            ]
        }),
        is_active: true,
        is_default: false,
        created_by: "system".to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_tested_at: None,
        test_status: None,
    };
    storage.store_adapter_config(&mainnet_config)?;
    println!("   ✅ Stellar Mainnet-IPFS adapter registered (Admin-only)");

    println!("✅ Production adapters initialized successfully!");
    Ok(())
}
```

## Benefits

1. ✅ **Multi-tenant**: Different users can have different credentials
2. ✅ **Dynamic**: Admins can update credentials without code changes
3. ✅ **Secure**: Credentials in database, not hardcoded
4. ✅ **Flexible**: Easy to add new adapter configs
5. ✅ **Testable**: Can test with different configs
6. ✅ **Auditable**: Track who created/modified configs

## Next Steps

1. Add `initialize_production_adapters()` function to `db_init.rs`
2. Update all adapter constructors to use `from_config()`
3. Update `create_adapter_instance()` to load config from DB
4. Update circuit push flow to use config-based adapters
5. Add admin API endpoints for adapter management

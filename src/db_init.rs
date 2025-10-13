use crate::storage::{StorageBackend, InMemoryStorage};
use crate::types::{
    UserAccount, UserTier, AccountStatus, TierLimits, CreditTransaction, CreditTransactionType,
    AdapterConfig, AdapterConnectionDetails, AdapterType, AuthType, ContractConfigs, ContractInfo
};
use chrono::Utc;
use uuid::Uuid;
use bcrypt::{hash, DEFAULT_COST};
use std::collections::HashMap;

pub fn initialize_default_admin(storage: &mut InMemoryStorage) -> Result<(), Box<dyn std::error::Error>> {
    let admin_user_id = "hen-admin-001".to_string();

    // Check if admin already exists
    if storage.get_user_account(&admin_user_id).unwrap_or(None).is_some() {
        println!("Default admin 'hen' already exists, skipping initialization");
        return Ok(());
    }

    println!("🐔 Initializing default admin user 'hen'...");

    // Generate bcrypt hash for the admin password
    let hen_password_hash = hash("demo123", DEFAULT_COST)?;

    let hen_admin = UserAccount {
        user_id: admin_user_id.clone(),
        username: "hen".to_string(),
        email: "hen@defarm.com".to_string(),
        password_hash: hen_password_hash,
        tier: UserTier::Admin,
        status: AccountStatus::Active,
        credits: 1_000_000, // 1 million credits for admin
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_login: None,
        subscription: None,
        limits: TierLimits::for_tier(&UserTier::Admin),
        is_admin: true,
        workspace_id: Some("hen-workspace".to_string()),
        available_adapters: None, // Use tier defaults
    };

    // Store the admin user
    storage.store_user_account(&hen_admin)?;

    // Record initial credit grant
    let initial_credit_transaction = CreditTransaction {
        transaction_id: Uuid::new_v4().to_string(),
        user_id: admin_user_id.clone(),
        amount: 1_000_000,
        transaction_type: CreditTransactionType::Grant,
        description: "Initial admin credit allocation".to_string(),
        operation_type: Some("system_init".to_string()),
        operation_id: Some("default_setup".to_string()),
        timestamp: Utc::now(),
        balance_after: 1_000_000,
    };

    storage.record_credit_transaction(&initial_credit_transaction)?;

    println!("✅ Default admin 'hen' created successfully!");
    println!("   - User ID: {}", admin_user_id);
    println!("   - Username: hen");
    println!("   - Email: hen@defarm.com");
    println!("   - Tier: Admin");
    println!("   - Credits: 1,000,000");
    println!("   - Password: demo123");

    Ok(())
}

pub fn initialize_sample_users(storage: &mut InMemoryStorage) -> Result<(), Box<dyn std::error::Error>> {
    println!("🌱 Creating sample users for development...");

    // Generate bcrypt hashes for all passwords (using demo123 for all development users)
    let demo_password_hash = hash("demo123", DEFAULT_COST)?;

    let sample_users = vec![
        // Add pullet user (matches auth.rs)
        UserAccount {
            user_id: "pullet-user-001".to_string(),
            username: "pullet".to_string(),
            email: "pullet@defarm.io".to_string(),
            password_hash: demo_password_hash.clone(),
            tier: UserTier::Professional,
            status: AccountStatus::Active,
            credits: 5000,
            created_at: Utc::now() - chrono::Duration::days(15),
            updated_at: Utc::now(),
            last_login: Some(Utc::now() - chrono::Duration::hours(3)),
            subscription: None,
            limits: TierLimits::for_tier(&UserTier::Professional),
            is_admin: false,
            workspace_id: Some("pullet-workspace".to_string()),
            available_adapters: None, // Use tier defaults
        },
        // Add cock user (matches auth.rs)
        UserAccount {
            user_id: "cock-user-001".to_string(),
            username: "cock".to_string(),
            email: "cock@defarm.io".to_string(),
            password_hash: demo_password_hash.clone(),
            tier: UserTier::Enterprise,
            status: AccountStatus::Active,
            credits: 50000,
            created_at: Utc::now() - chrono::Duration::days(60),
            updated_at: Utc::now(),
            last_login: Some(Utc::now() - chrono::Duration::hours(1)),
            subscription: None,
            limits: TierLimits::for_tier(&UserTier::Enterprise),
            is_admin: false,
            workspace_id: Some("cock-workspace".to_string()),
            available_adapters: None, // Use tier defaults
        },
        UserAccount {
            user_id: "basic-farmer-001".to_string(),
            username: "basic_farmer".to_string(),
            email: "basic@farm.com".to_string(),
            password_hash: demo_password_hash.clone(),
            tier: UserTier::Basic,
            status: AccountStatus::Active,
            credits: 100,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login: Some(Utc::now() - chrono::Duration::days(2)),
            subscription: None,
            limits: TierLimits::for_tier(&UserTier::Basic),
            is_admin: false,
            workspace_id: Some("basic-workspace".to_string()),
            available_adapters: None, // Use tier defaults
        },
        UserAccount {
            user_id: "pro-farmer-001".to_string(),
            username: "pro_farmer".to_string(),
            email: "pro@farm.com".to_string(),
            password_hash: demo_password_hash.clone(),
            tier: UserTier::Professional,
            status: AccountStatus::Active,
            credits: 5000,
            created_at: Utc::now() - chrono::Duration::days(30),
            updated_at: Utc::now(),
            last_login: Some(Utc::now() - chrono::Duration::hours(6)),
            subscription: None,
            limits: TierLimits::for_tier(&UserTier::Professional),
            is_admin: false,
            workspace_id: Some("pro-workspace".to_string()),
            available_adapters: None, // Use tier defaults
        },
        UserAccount {
            user_id: "enterprise-farmer-001".to_string(),
            username: "enterprise_farmer".to_string(),
            email: "enterprise@farm.com".to_string(),
            password_hash: demo_password_hash.clone(),
            tier: UserTier::Enterprise,
            status: AccountStatus::Active,
            credits: 50000,
            created_at: Utc::now() - chrono::Duration::days(90),
            updated_at: Utc::now(),
            last_login: Some(Utc::now() - chrono::Duration::hours(1)),
            subscription: None,
            limits: TierLimits::for_tier(&UserTier::Enterprise),
            is_admin: false,
            workspace_id: Some("enterprise-workspace".to_string()),
            available_adapters: None, // Use tier defaults
        },
    ];

    for user in sample_users {
        // Check if user already exists
        if storage.get_user_account(&user.user_id).unwrap_or(None).is_some() {
            println!("   - User '{}' already exists, skipping", user.username);
            continue;
        }

        let initial_credits = user.credits;
        let user_id = user.user_id.clone();
        let username = user.username.clone();

        // Store the user
        storage.store_user_account(&user)?;

        // Record initial credit grant
        let initial_credit_transaction = CreditTransaction {
            transaction_id: Uuid::new_v4().to_string(),
            user_id: user_id.clone(),
            amount: initial_credits,
            transaction_type: CreditTransactionType::Grant,
            description: format!("Initial {} tier credit allocation", user.tier.as_str()),
            operation_type: Some("system_init".to_string()),
            operation_id: Some("sample_setup".to_string()),
            timestamp: Utc::now(),
            balance_after: initial_credits,
        };

        storage.record_credit_transaction(&initial_credit_transaction)?;

        println!("   ✅ Created {} user: {} ({})", user.tier.as_str(), username, initial_credits);
    }

    println!("✅ Sample users created successfully!");
    Ok(())
}

pub fn initialize_production_adapters(storage: &mut InMemoryStorage) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔌 Initializing production adapters...");

    // Read credentials from environment
    let pinata_api_key = std::env::var("PINATA_API_KEY").ok();
    let pinata_secret = std::env::var("PINATA_SECRET_KEY").ok();
    let testnet_ipcm = std::env::var("STELLAR_TESTNET_IPCM_CONTRACT").ok();
    let mainnet_ipcm = std::env::var("STELLAR_MAINNET_IPCM_CONTRACT").ok();
    let mainnet_secret = std::env::var("STELLAR_MAINNET_SECRET_KEY").ok();

    // Create IPFS-IPFS adapter config
    if let (Some(api_key), Some(secret)) = (pinata_api_key.clone(), pinata_secret.clone()) {
        let ipfs_config = AdapterConfig {
            config_id: Uuid::new_v4(),
            name: "Production IPFS (Pinata)".to_string(),
            description: "IPFS storage via Pinata cloud".to_string(),
            adapter_type: AdapterType::IpfsIpfs,
            connection_details: AdapterConnectionDetails {
                endpoint: "https://api.pinata.cloud".to_string(),
                api_key: Some(api_key),
                secret_key: Some(secret),
                auth_type: AuthType::ApiKey,
                timeout_ms: 60000,
                retry_attempts: 3,
                max_concurrent_requests: 10,
                custom_headers: HashMap::new(),
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
    }

    // Stellar adapters now use native RPC integration (no CLI dependency)
    // Create Stellar Testnet + IPFS adapter config
    if let (Some(api_key), Some(secret), Some(contract_addr)) = (pinata_api_key.clone(), pinata_secret.clone(), testnet_ipcm) {
        // Get testnet configuration from environment
        let testnet_secret = std::env::var("STELLAR_TESTNET_SECRET").ok();
        let interface_address = std::env::var("DEFARM_OWNER_WALLET")
            .unwrap_or_else(|_| "GANDYZQQ3OQBXHZQXJHZ7AQ2GDBFUQIR4ZLMUPD3P2B7PLIYQNFG54XQ".to_string());

        let mut custom_headers = HashMap::new();
        if let Some(secret_key) = testnet_secret {
            custom_headers.insert("stellar_secret".to_string(), secret_key);
        }
        custom_headers.insert("interface_address".to_string(), interface_address);
        custom_headers.insert("source_account_identity".to_string(), "defarm-admin-testnet".to_string());

        let testnet_config = AdapterConfig {
            config_id: Uuid::new_v4(),
            name: "Stellar Testnet + IPFS".to_string(),
            description: "NFTs on Stellar testnet + IPFS events".to_string(),
            adapter_type: AdapterType::StellarTestnetIpfs,
            connection_details: AdapterConnectionDetails {
                endpoint: "https://api.pinata.cloud".to_string(),
                api_key: Some(api_key),
                secret_key: Some(secret),
                auth_type: AuthType::ApiKey,
                timeout_ms: 60000,
                retry_attempts: 3,
                max_concurrent_requests: 10,
                custom_headers,
            },
            contract_configs: Some(ContractConfigs {
                mint_contract: None,
                ipcm_contract: Some(ContractInfo {
                    contract_address: contract_addr,
                    contract_name: "IPCM".to_string(),
                    abi: None,
                    methods: HashMap::new(),
                }),
                network: "testnet".to_string(),
                chain_id: None,
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
    }

    // Create Stellar Mainnet + IPFS adapter config
    if let (Some(api_key), Some(secret), Some(contract_addr), Some(mainnet_key)) = (pinata_api_key, pinata_secret, mainnet_ipcm, mainnet_secret) {
        let interface_address = std::env::var("DEFARM_OWNER_WALLET")
            .unwrap_or_else(|_| "GANDYZQQ3OQBXHZQXJHZ7AQ2GDBFUQIR4ZLMUPD3P2B7PLIYQNFG54XQ".to_string());

        let mut custom_headers = HashMap::new();
        custom_headers.insert("stellar_secret".to_string(), mainnet_key);
        custom_headers.insert("interface_address".to_string(), interface_address);
        custom_headers.insert("source_account_identity".to_string(), "defarm-admin-secure-v2".to_string());

        let mainnet_config = AdapterConfig {
            config_id: Uuid::new_v4(),
            name: "Stellar Mainnet + IPFS (Production)".to_string(),
            description: "Production NFTs on Stellar mainnet + IPFS".to_string(),
            adapter_type: AdapterType::StellarMainnetIpfs,
            connection_details: AdapterConnectionDetails {
                endpoint: "https://api.pinata.cloud".to_string(),
                api_key: Some(api_key),
                secret_key: Some(secret),
                auth_type: AuthType::ApiKey,
                timeout_ms: 60000,
                retry_attempts: 3,
                max_concurrent_requests: 10,
                custom_headers,
            },
            contract_configs: Some(ContractConfigs {
                mint_contract: None,
                ipcm_contract: Some(ContractInfo {
                    contract_address: contract_addr,
                    contract_name: "IPCM".to_string(),
                    abi: None,
                    methods: HashMap::new(),
                }),
                network: "mainnet".to_string(),
                chain_id: None,
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
    }

    println!("✅ Production adapters initialized successfully!");
    println!();
    println!("📋 Available Adapters:");
    println!("   🌐 IPFS-IPFS: Available to all tiers");
    println!("   ⭐ Stellar Testnet-IPFS: NFT tokenization on testnet");
    println!("   ⭐ Stellar Mainnet-IPFS: Production NFT tokenization (Admin-only)");
    println!();

    Ok(())
}

pub fn setup_development_data(storage: &mut InMemoryStorage) -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Setting up development data...");

    initialize_default_admin(storage)?;
    initialize_sample_users(storage)?;
    initialize_production_adapters(storage)?;

    println!("🎉 Development data setup complete!");
    println!();
    println!("📋 Available test accounts (all use password: demo123):");
    println!("   🐔 Admin:      hen / demo123");
    println!("   🐣 Pro:        pullet / demo123");
    println!("   🐓 Enterprise: cock / demo123");
    println!("   🌱 Basic:      basic_farmer / demo123");
    println!("   🚀 Pro:        pro_farmer / demo123");
    println!("   🏢 Enterprise: enterprise_farmer / demo123");
    println!();
    println!("🔗 Admin Panel: http://localhost:3000/api/admin/dashboard/stats");

    Ok(())
}
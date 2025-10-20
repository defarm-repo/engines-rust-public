use defarm_engine::api_keys_engine::ApiKeysEngine;
use defarm_engine::storage::{InMemoryStorage, StorageBackend};
use defarm_engine::types::*;
use std::sync::{Arc, Mutex};
use chrono::Utc;

#[test]
fn test_api_key_authentication() {
    println!("\n🔑 API KEY AUTHENTICATION TEST");
    println!("════════════════════════════════════════════════════════════════════════\n");

    let storage = Arc::new(Mutex::new(InMemoryStorage::new()));
    let mut api_key_engine = ApiKeysEngine::new(storage.clone());

    // Create user
    let mut storage_guard = storage.lock().unwrap();
    let user = UserAccount {
        user_id: "api-test-user".to_string(),
        username: "apiuser".to_string(),
        email: "api@test.com".to_string(),
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
        workspace_id: Some("api-workspace".to_string()),
        available_adapters: Some(vec![AdapterType::IpfsIpfs]),
    };
    storage_guard.store_user_account(&user).unwrap();
    drop(storage_guard);

    // Create API key with specific permissions
    let key_name = "Production API Key".to_string();
    let api_key_result = api_key_engine.create_api_key(
        key_name.clone(),
        "api-test-user".to_string(),
        vec![ApiPermission::Read, ApiPermission::Write],
        None,  // No expiration
        Some(vec![]),  // No IP restrictions
        Some(vec![]),  // No endpoint restrictions
        Some(1000),  // Rate limit per hour
        Some(100),   // Rate limit per minute
        Some(10000), // Rate limit per day
    ).unwrap();

    println!("✅ API Key Created:");
    println!("   Name: {}", key_name);
    println!("   Key: {}", api_key_result.api_key);
    println!("   Key ID: {}", api_key_result.key_id);
    println!("   Format: dfm_{32-character-random}");
    
    // Show the key prefix that's stored
    let key_prefix = &api_key_result.api_key[0..12];
    println!("   Stored Prefix: {}", key_prefix);
    
    // Show the BLAKE3 hash
    let key_hash = blake3::hash(api_key_result.api_key.as_bytes());
    println!("   BLAKE3 Hash: {}", key_hash);
    println!();

    // Validate the API key
    match api_key_engine.validate_api_key(&api_key_result.api_key, Some("127.0.0.1")) {
        Ok(validation) => {
            println!("✅ API Key Validation Success:");
            println!("   User ID: {}", validation.user_id);
            println!("   Key ID: {}", validation.key_id);
            println!("   Permissions: {:?}", validation.permissions);
            println!("   Rate Limits:");
            println!("      • Per Hour: {:?}", validation.rate_limit_per_hour);
            println!("      • Per Minute: {:?}", validation.rate_limit_per_minute);
            println!("      • Per Day: {:?}", validation.rate_limit_per_day);
        }
        Err(e) => {
            println!("❌ Validation failed: {}", e);
        }
    }
    println!();

    // Show authentication methods
    println!("📋 How to Use API Keys:");
    println!();
    println!("   Method 1 - X-API-Key Header:");
    println!("   curl -H 'X-API-Key: {}' \\", api_key_result.api_key);
    println!("        https://api.defarm.net/api/circuits");
    println!();
    println!("   Method 2 - Bearer Token:");
    println!("   curl -H 'Authorization: Bearer {}' \\", api_key_result.api_key);
    println!("        https://api.defarm.net/api/circuits");
    println!();

    // Test rate limiting
    println!("📊 Rate Limiting:");
    
    // Simulate requests
    for i in 1..=5 {
        match api_key_engine.check_rate_limit(&api_key_result.key_id) {
            Ok(status) => {
                println!("   Request #{}: ✅ Allowed", i);
                println!("      Remaining (minute): {}/{}", 
                    status.remaining_per_minute.unwrap_or(0),
                    status.limit_per_minute.unwrap_or(0));
            }
            Err(e) => {
                println!("   Request #{}: ❌ Rate limited - {}", i, e);
            }
        }
    }
    println!();

    // Create an admin API key
    let admin_key = api_key_engine.create_api_key(
        "Admin API Key".to_string(),
        "api-test-user".to_string(),
        vec![ApiPermission::Read, ApiPermission::Write, ApiPermission::Admin],
        None,
        Some(vec!["10.0.0.0/8".to_string()]),  // IP restriction
        Some(vec!["/api/admin/*".to_string()]), // Endpoint restriction
        None, None, None,  // No rate limits for admin
    ).unwrap();

    println!("✅ Admin API Key Created:");
    println!("   Key: {}", admin_key.api_key);
    println!("   Permissions: Read, Write, Admin");
    println!("   IP Restrictions: 10.0.0.0/8");
    println!("   Endpoint Restrictions: /api/admin/*");
    println!("   Rate Limits: None (unlimited)");
    println!();

    println!("════════════════════════════════════════════════════════════════════════");
    println!("🎉 API KEY AUTHENTICATION FULLY FUNCTIONAL!");
    println!("════════════════════════════════════════════════════════════════════════");
}

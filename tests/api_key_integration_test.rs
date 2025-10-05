use defarm_engine::*;
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn test_api_key_full_workflow() {
    // Setup
    let engine = Arc::new(ApiKeyEngine::new());
    let storage = Arc::new(InMemoryApiKeyStorage::new());
    let rate_limiter = Arc::new(RateLimiter::new());

    // Test 1: Generate and create API key
    let user_id = Uuid::new_v4();
    let (api_key_str, _, _) = engine.generate_key();

    let request = CreateApiKeyRequest {
        name: "Integration Test Key".to_string(),
        created_by: user_id,
        organization_type: OrganizationType::Producer,
        organization_id: None,
        permissions: Some(ApiKeyPermissions::read_write()),
        allowed_endpoints: Some(vec!["receipts".to_string(), "items".to_string()]),
        rate_limit_per_hour: Some(50),
        expires_in_days: Some(30),
        notes: Some("Test key for integration testing".to_string()),
        allowed_ips: None,
    };

    let mut api_key = engine.create_api_key(request);
    api_key.key_hash = engine.hash_key(&api_key_str);

    let stored_key = storage.create_api_key(api_key.clone()).await;
    assert!(stored_key.is_ok());
    let stored_key = stored_key.unwrap();

    println!("✓ API key created successfully");

    // Test 2: Retrieve API key by hash
    let retrieved = storage.get_api_key_by_hash(&stored_key.key_hash).await;
    assert!(retrieved.is_ok());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.id, stored_key.id);

    println!("✓ API key retrieved by hash");

    // Test 3: Validate API key
    let validation = engine.validate_key(&api_key_str, &retrieved);
    assert!(validation.is_ok());

    println!("✓ API key validation successful");

    // Test 4: Check rate limiting
    let config = RateLimitConfig::new(stored_key.rate_limit_per_hour);

    for i in 0..5 {
        let result = rate_limiter.check_rate_limit(stored_key.id, &config);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.allowed);
        rate_limiter.record_request(stored_key.id).unwrap();
        println!("✓ Request {} allowed (remaining: {})", i + 1, result.remaining);
    }

    // Test 5: Record usage
    storage.record_usage(stored_key.id).await.unwrap();
    let updated = storage.get_api_key(stored_key.id).await.unwrap();
    assert_eq!(updated.usage_count, 1);

    println!("✓ Usage recorded");

    // Test 6: Log usage
    let usage_log = ApiKeyUsageLog {
        id: Uuid::new_v4(),
        api_key_id: stored_key.id,
        endpoint: "/receipts".to_string(),
        method: "POST".to_string(),
        ip_address: Some("192.168.1.1".parse().unwrap()),
        user_agent: Some("Test Client/1.0".to_string()),
        request_size: Some(1024),
        response_status: 200,
        response_time_ms: Some(50),
        error_message: None,
        created_at: chrono::Utc::now(),
    };

    storage.log_usage(usage_log).await.unwrap();

    println!("✓ Usage logged");

    // Test 7: Get usage stats
    let stats = storage.get_usage_stats(stored_key.id, 7).await.unwrap();
    assert_eq!(stats.total_requests, 1);
    assert_eq!(stats.successful_requests, 1);

    println!("✓ Usage stats retrieved");

    // Test 8: Get user's API keys
    let user_keys = storage.get_user_api_keys(user_id).await.unwrap();
    assert_eq!(user_keys.len(), 1);
    assert_eq!(user_keys[0].id, stored_key.id);

    println!("✓ User API keys retrieved");

    // Test 9: Update API key
    let mut updated_key = stored_key.clone();
    updated_key.is_active = false;
    storage.update_api_key(updated_key).await.unwrap();

    let deactivated = storage.get_api_key(stored_key.id).await.unwrap();
    assert!(!deactivated.is_active);

    println!("✓ API key deactivated");

    // Test 10: Validate inactive key
    let validation = engine.validate_key(&api_key_str, &deactivated);
    assert!(validation.is_err());
    assert!(matches!(validation.unwrap_err(), ApiKeyError::Inactive));

    println!("✓ Inactive key validation failed correctly");

    // Test 11: Delete API key
    storage.delete_api_key(stored_key.id).await.unwrap();
    let deleted = storage.get_api_key(stored_key.id).await;
    assert!(deleted.is_err());

    println!("✓ API key deleted");

    println!("\n✅ All integration tests passed!");
}

#[tokio::test]
async fn test_rate_limiting_enforcement() {
    let rate_limiter = Arc::new(RateLimiter::new());

    let api_key_id = Uuid::new_v4();
    let config = RateLimitConfig::new(5); // Only 5 requests per hour

    // Make 5 requests - all should succeed
    for i in 0..5 {
        let result = rate_limiter.check_rate_limit(api_key_id, &config).unwrap();
        assert!(result.allowed, "Request {} should be allowed", i + 1);
        rate_limiter.record_request(api_key_id).unwrap();
    }

    // 6th request should be denied
    let result = rate_limiter.check_rate_limit(api_key_id, &config).unwrap();
    assert!(!result.allowed, "Request 6 should be denied");
    assert_eq!(result.remaining, 0);
    assert!(result.retry_after_seconds.is_some());

    println!("✅ Rate limiting enforcement test passed!");
}

#[tokio::test]
async fn test_multiple_users_isolation() {
    let storage = Arc::new(InMemoryApiKeyStorage::new());
    let engine = Arc::new(ApiKeyEngine::new());

    let user1 = Uuid::new_v4();
    let user2 = Uuid::new_v4();

    // Create keys for user1
    for i in 0..3 {
        let request = CreateApiKeyRequest {
            name: format!("User1 Key {}", i),
            created_by: user1,
            organization_type: OrganizationType::Producer,
            organization_id: None,
            permissions: None,
            allowed_endpoints: None,
            rate_limit_per_hour: None,
            expires_in_days: None,
            notes: None,
            allowed_ips: None,
        };
        let api_key = engine.create_api_key(request);
        storage.create_api_key(api_key).await.unwrap();
    }

    // Create keys for user2
    for i in 0..2 {
        let request = CreateApiKeyRequest {
            name: format!("User2 Key {}", i),
            created_by: user2,
            organization_type: OrganizationType::Enterprise,
            organization_id: None,
            permissions: None,
            allowed_endpoints: None,
            rate_limit_per_hour: None,
            expires_in_days: None,
            notes: None,
            allowed_ips: None,
        };
        let api_key = engine.create_api_key(request);
        storage.create_api_key(api_key).await.unwrap();
    }

    // Verify isolation
    let user1_keys = storage.get_user_api_keys(user1).await.unwrap();
    let user2_keys = storage.get_user_api_keys(user2).await.unwrap();

    assert_eq!(user1_keys.len(), 3);
    assert_eq!(user2_keys.len(), 2);

    // Verify no cross-contamination
    for key in &user1_keys {
        assert_eq!(key.created_by, user1);
    }
    for key in &user2_keys {
        assert_eq!(key.created_by, user2);
    }

    println!("✅ Multi-user isolation test passed!");
}

#[test]
fn test_error_recovery_suggestions() {
    use defarm_engine::{DeFarmError, RecoveryStrategy};

    // Test rate limit error
    let rate_error = DeFarmError::RateLimit(RateLimitError::Exceeded("test".to_string()));
    let suggestions = rate_error.get_recovery_suggestions();
    assert!(!suggestions.is_empty());
    assert!(suggestions.iter().any(|s| s.contains("backoff")));

    // Test API key error
    let api_error = DeFarmError::ApiKey(ApiKeyError::Expired);
    let suggestions = api_error.get_recovery_suggestions();
    assert!(!suggestions.is_empty());
    assert!(suggestions.iter().any(|s| s.contains("new API key")));

    // Test permission error
    let perm_error = DeFarmError::PermissionDenied("test".to_string());
    let suggestions = perm_error.get_recovery_suggestions();
    assert!(!suggestions.is_empty());

    println!("✅ Error recovery suggestions test passed!");
}

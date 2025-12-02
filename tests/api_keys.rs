use std::net::IpAddr;
use std::str::FromStr;
use std::sync::Arc;

use chrono::Utc;
use defarm_engine::ApiKeyStorage;
use defarm_engine::{
    ApiKeyEngine, ApiKeyPermissions, ApiKeyUsageLog, CreateApiKeyRequest, InMemoryApiKeyStorage,
    OrganizationType, RateLimitConfig, RateLimiter,
};
use uuid::Uuid;

#[tokio::test]
async fn api_key_end_to_end_smoke() {
    let engine = ApiKeyEngine::new();
    let storage = Arc::new(InMemoryApiKeyStorage::new());
    let rate_limiter = RateLimiter::new();

    // 1. Generate a key and create the record we expect to persist.
    let user_id = Uuid::new_v4();
    let (plain_key, hashed, prefix) = engine.generate_key();

    let request = CreateApiKeyRequest {
        name: "integration-key".into(),
        created_by: user_id,
        original_user_id: format!("user-{}", user_id),
        organization_type: OrganizationType::Producer,
        organization_id: None,
        permissions: Some(ApiKeyPermissions::read_write()),
        allowed_endpoints: Some(vec!["receipts".into(), "items".into()]),
        rate_limit_per_hour: Some(10),
        expires_in_days: Some(30),
        notes: Some("smoke test".into()),
        allowed_ips: Some(vec![IpAddr::from_str("203.0.113.10").unwrap()]),
    };

    let mut api_key = engine.create_api_key(request);
    api_key.key_hash = hashed.clone();
    api_key.key_prefix = prefix;

    storage.create_api_key(api_key.clone()).await.unwrap();

    // 2. Retrieve and validate the stored key.
    let stored = storage
        .get_api_key_by_hash(&hashed)
        .await
        .expect("storage lookup should succeed");

    assert_eq!(stored.created_by, user_id);
    engine.validate_key(&plain_key, &stored).unwrap();
    engine
        .check_ip_allowed(&stored, IpAddr::from_str("203.0.113.10").unwrap())
        .unwrap();

    // 3. Exercise rate limiting: allow 3 requests, deny the 4th.
    let config = RateLimitConfig::new(3);
    for _ in 0..3 {
        let check = rate_limiter.check_rate_limit(stored.id, &config).unwrap();
        assert!(check.allowed);
        rate_limiter.record_request(stored.id).unwrap();
    }
    let check = rate_limiter.check_rate_limit(stored.id, &config).unwrap();
    assert!(!check.allowed);

    // 4. Record usage and log metadata.
    storage.record_usage(stored.id).await.unwrap();
    let updated = storage.get_api_key(stored.id).await.unwrap();
    assert_eq!(updated.usage_count, 1);

    let usage_log = ApiKeyUsageLog {
        id: Uuid::new_v4(),
        api_key_id: stored.id,
        endpoint: "/receipts".into(),
        method: "POST".into(),
        ip_address: Some(IpAddr::from_str("203.0.113.10").unwrap()),
        user_agent: Some("integration-test".into()),
        request_size: Some(512),
        response_status: 200,
        response_time_ms: Some(20),
        error_message: None,
        created_at: Utc::now(),
    };
    storage.log_usage(usage_log).await.unwrap();

    // 5. Deactivate and ensure validation fails.
    let mut updated = updated;
    updated.is_active = false;
    storage.update_api_key(updated.clone()).await.unwrap();
    let inactive = storage.get_api_key(stored.id).await.unwrap();
    assert!(engine.validate_key(&plain_key, &inactive).is_err());

    // 6. Delete and ensure it disappears.
    storage.delete_api_key(stored.id).await.unwrap();
    assert!(storage.get_api_key(stored.id).await.is_err());
}

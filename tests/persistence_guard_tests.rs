use chrono::Utc;
use defarm_engine::postgres_persistence::PostgresPersistence;
use defarm_engine::types::{AccountStatus, TierLimits, UserAccount, UserTier};

#[tokio::test]
async fn persistence_retry_metrics_on_failure() {
    // Deliberately skip connect() so the pool remains uninitialized.
    let persistence = PostgresPersistence::new("postgres://invalid-url".to_string());

    let user = sample_user();

    let result = persistence.persist_user(&user).await;
    assert!(result.is_err());

    let metrics = persistence.metrics_snapshot();
    assert_eq!(metrics.total_attempts, 1);
    assert_eq!(metrics.total_failures, 1);
    assert!(
        metrics.total_retries >= 1,
        "expected at least one retry, got {}",
        metrics.total_retries
    );
}

fn sample_user() -> UserAccount {
    let tier = UserTier::Basic;
    UserAccount {
        user_id: "user-queue-test".to_string(),
        username: "queue_test".to_string(),
        email: "queue@example.com".to_string(),
        password_hash: "hash".to_string(),
        tier: tier.clone(),
        status: AccountStatus::Active,
        credits: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_login: None,
        subscription: None,
        limits: TierLimits::for_tier(&tier),
        is_admin: false,
        workspace_id: Some("workspace".to_string()),
        available_adapters: None,
    }
}

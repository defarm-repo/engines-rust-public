use crate::storage::{StorageBackend, StorageError};
use crate::types::{TierLimits, UserAccount, UserTier};
use chrono::{Datelike, Timelike};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct TierPermissionSystem<S: StorageBackend> {
    storage: Arc<std::sync::Mutex<S>>,
    tier_configs: HashMap<UserTier, TierConfiguration>,
}

#[derive(Debug, Clone)]
pub struct TierConfiguration {
    pub tier: UserTier,
    pub permissions: Vec<String>,
    pub tier_limits: TierLimits,
    pub features_enabled: Vec<String>,
    pub api_rate_limit_per_minute: u32,
    pub concurrent_operations_limit: u32,
    pub bulk_operations_per_month: i64,
    pub advanced_analytics: bool,
    pub admin_access: bool,
    pub custom_integrations: bool,
}

#[derive(Debug, Clone)]
pub struct PermissionCheck {
    pub user_id: String,
    pub operation: String,
    pub resource: Option<String>,
    pub context: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct PermissionResult {
    pub allowed: bool,
    pub reason: Option<String>,
    pub remaining_quota: Option<i64>,
    pub next_reset: Option<chrono::DateTime<chrono::Utc>>,
}

impl<S: StorageBackend> TierPermissionSystem<S> {
    pub fn new(storage: Arc<std::sync::Mutex<S>>) -> Self {
        let mut tier_configs = HashMap::new();

        // Basic tier configuration
        tier_configs.insert(
            UserTier::Basic,
            TierConfiguration {
                tier: UserTier::Basic,
                permissions: vec![
                    "read_items".to_string(),
                    "create_items".to_string(),
                    "read_events".to_string(),
                    "create_events".to_string(),
                ],
                tier_limits: TierLimits::for_tier(&UserTier::Basic),
                features_enabled: vec!["basic_storage".to_string(), "basic_api".to_string()],
                api_rate_limit_per_minute: 60,
                concurrent_operations_limit: 2,
                bulk_operations_per_month: 0,
                advanced_analytics: false,
                admin_access: false,
                custom_integrations: false,
            },
        );

        // Professional tier configuration
        tier_configs.insert(
            UserTier::Professional,
            TierConfiguration {
                tier: UserTier::Professional,
                permissions: vec![
                    "read_items".to_string(),
                    "create_items".to_string(),
                    "update_items".to_string(),
                    "delete_items".to_string(),
                    "read_events".to_string(),
                    "create_events".to_string(),
                    "update_events".to_string(),
                    "read_circuits".to_string(),
                    "create_circuits".to_string(),
                    "execute_circuits".to_string(),
                    "bulk_operations".to_string(),
                ],
                tier_limits: TierLimits::for_tier(&UserTier::Professional),
                features_enabled: vec![
                    "basic_storage".to_string(),
                    "basic_api".to_string(),
                    "advanced_storage".to_string(),
                    "circuits".to_string(),
                    "bulk_operations".to_string(),
                    "analytics".to_string(),
                ],
                api_rate_limit_per_minute: 300,
                concurrent_operations_limit: 5,
                bulk_operations_per_month: 10,
                advanced_analytics: true,
                admin_access: false,
                custom_integrations: false,
            },
        );

        // Enterprise tier configuration
        tier_configs.insert(
            UserTier::Enterprise,
            TierConfiguration {
                tier: UserTier::Enterprise,
                permissions: vec![
                    "read_items".to_string(),
                    "create_items".to_string(),
                    "update_items".to_string(),
                    "delete_items".to_string(),
                    "read_events".to_string(),
                    "create_events".to_string(),
                    "update_events".to_string(),
                    "delete_events".to_string(),
                    "read_circuits".to_string(),
                    "create_circuits".to_string(),
                    "update_circuits".to_string(),
                    "delete_circuits".to_string(),
                    "execute_circuits".to_string(),
                    "bulk_operations".to_string(),
                    "advanced_analytics".to_string(),
                    "custom_integrations".to_string(),
                    "priority_support".to_string(),
                    "audit_access".to_string(),
                ],
                tier_limits: TierLimits::for_tier(&UserTier::Enterprise),
                features_enabled: vec![
                    "basic_storage".to_string(),
                    "basic_api".to_string(),
                    "advanced_storage".to_string(),
                    "circuits".to_string(),
                    "bulk_operations".to_string(),
                    "analytics".to_string(),
                    "advanced_analytics".to_string(),
                    "custom_integrations".to_string(),
                    "priority_support".to_string(),
                    "audit_dashboard".to_string(),
                ],
                api_rate_limit_per_minute: 1000,
                concurrent_operations_limit: 20,
                bulk_operations_per_month: 100,
                advanced_analytics: true,
                admin_access: false,
                custom_integrations: true,
            },
        );

        // Admin tier configuration (unlimited)
        tier_configs.insert(
            UserTier::Admin,
            TierConfiguration {
                tier: UserTier::Admin,
                permissions: vec![
                    "read_items".to_string(),
                    "create_items".to_string(),
                    "update_items".to_string(),
                    "delete_items".to_string(),
                    "read_events".to_string(),
                    "create_events".to_string(),
                    "update_events".to_string(),
                    "delete_events".to_string(),
                    "read_circuits".to_string(),
                    "create_circuits".to_string(),
                    "update_circuits".to_string(),
                    "delete_circuits".to_string(),
                    "execute_circuits".to_string(),
                    "bulk_operations".to_string(),
                    "advanced_analytics".to_string(),
                    "custom_integrations".to_string(),
                    "priority_support".to_string(),
                    "audit_access".to_string(),
                    "admin_users".to_string(),
                    "admin_system".to_string(),
                    "admin_tiers".to_string(),
                    "admin_credits".to_string(),
                ],
                tier_limits: TierLimits::for_tier(&UserTier::Admin),
                features_enabled: vec!["all".to_string()],
                api_rate_limit_per_minute: u32::MAX,
                concurrent_operations_limit: u32::MAX,
                bulk_operations_per_month: i64::MAX,
                advanced_analytics: true,
                admin_access: true,
                custom_integrations: true,
            },
        );

        Self {
            storage,
            tier_configs,
        }
    }

    pub async fn check_permission(
        &self,
        check: &PermissionCheck,
    ) -> Result<PermissionResult, StorageError> {
        let storage = self.storage.lock().unwrap();
        let user = storage
            .get_user_account(&check.user_id)?
            .ok_or(StorageError::NotFound)?;

        let tier_config = self.tier_configs.get(&user.tier).ok_or_else(|| {
            StorageError::NotImplemented("Tier configuration not found".to_string())
        })?;

        // Check if operation is permitted for this tier
        if !tier_config.permissions.contains(&check.operation) {
            return Ok(PermissionResult {
                allowed: false,
                reason: Some(format!(
                    "Operation '{}' not permitted for {:?} tier",
                    check.operation, user.tier
                )),
                remaining_quota: None,
                next_reset: None,
            });
        }

        // Check feature availability
        if !self.is_feature_enabled(&user.tier, &check.operation) {
            return Ok(PermissionResult {
                allowed: false,
                reason: Some(format!(
                    "Feature '{}' not available for {:?} tier",
                    check.operation, user.tier
                )),
                remaining_quota: None,
                next_reset: None,
            });
        }

        // Check usage limits (this would require implementation of usage tracking)
        let usage_check = self.check_usage_limits(&user, &check.operation).await?;
        if !usage_check.allowed {
            return Ok(usage_check);
        }

        Ok(PermissionResult {
            allowed: true,
            reason: None,
            remaining_quota: usage_check.remaining_quota,
            next_reset: usage_check.next_reset,
        })
    }

    pub fn get_tier_limits(&self, tier: &UserTier) -> Option<&TierLimits> {
        self.tier_configs
            .get(tier)
            .map(|config| &config.tier_limits)
    }

    pub fn get_tier_permissions(&self, tier: &UserTier) -> Option<&Vec<String>> {
        self.tier_configs
            .get(tier)
            .map(|config| &config.permissions)
    }

    pub fn get_enabled_features(&self, tier: &UserTier) -> Option<&Vec<String>> {
        self.tier_configs
            .get(tier)
            .map(|config| &config.features_enabled)
    }

    pub fn can_perform_operation(&self, user: &UserAccount, operation: &str) -> bool {
        if let Some(tier_config) = self.tier_configs.get(&user.tier) {
            tier_config.permissions.contains(&operation.to_string())
        } else {
            false
        }
    }

    pub fn is_feature_enabled(&self, tier: &UserTier, feature: &str) -> bool {
        if let Some(tier_config) = self.tier_configs.get(tier) {
            tier_config.features_enabled.contains(&"all".to_string())
                || tier_config.features_enabled.contains(&feature.to_string())
        } else {
            false
        }
    }

    pub fn get_api_rate_limit(&self, tier: &UserTier) -> Option<u32> {
        self.tier_configs
            .get(tier)
            .map(|config| config.api_rate_limit_per_minute)
    }

    pub fn get_concurrent_operations_limit(&self, tier: &UserTier) -> Option<u32> {
        self.tier_configs
            .get(tier)
            .map(|config| config.concurrent_operations_limit)
    }

    async fn check_usage_limits(
        &self,
        user: &UserAccount,
        operation: &str,
    ) -> Result<PermissionResult, StorageError> {
        // This would integrate with usage tracking to check current month's usage
        // For now, returning allowed with placeholder values

        let tier_config = self.tier_configs.get(&user.tier).ok_or_else(|| {
            StorageError::NotImplemented("Tier configuration not found".to_string())
        })?;

        // Calculate remaining quota based on operation type
        let remaining_quota = match operation {
            "create_items" => tier_config.tier_limits.max_items_per_month,
            "create_circuits" => tier_config.tier_limits.max_circuits,
            "bulk_operations" => Some(tier_config.bulk_operations_per_month),
            _ => None,
        };

        // Calculate next reset time (beginning of next month)
        let now = chrono::Utc::now();
        let next_month = if now.month() == 12 {
            now.with_year(now.year() + 1)
                .unwrap()
                .with_month(1)
                .unwrap()
        } else {
            now.with_month(now.month() + 1).unwrap()
        };
        let next_reset = next_month
            .with_day(1)
            .unwrap()
            .with_hour(0)
            .unwrap()
            .with_minute(0)
            .unwrap()
            .with_second(0)
            .unwrap();

        Ok(PermissionResult {
            allowed: true,
            reason: None,
            remaining_quota,
            next_reset: Some(next_reset),
        })
    }

    pub fn upgrade_user_tier(&self, user_id: &str, new_tier: UserTier) -> Result<(), StorageError> {
        let mut storage = self.storage.lock().unwrap();
        let mut user = storage
            .get_user_account(user_id)?
            .ok_or(StorageError::NotFound)?;

        let _old_tier = user.tier.clone();
        user.tier = new_tier.clone();
        user.updated_at = chrono::Utc::now();

        // Update tier limits
        user.limits = TierLimits::for_tier(&new_tier);

        storage.update_user_account(&user)?;

        // Tier change is tracked in storage and can be audited via user account history
        Ok(())
    }

    pub fn get_all_tier_configurations(&self) -> &HashMap<UserTier, TierConfiguration> {
        &self.tier_configs
    }

    pub fn validate_tier_upgrade(&self, current_tier: &UserTier, target_tier: &UserTier) -> bool {
        match (current_tier, target_tier) {
            (UserTier::Basic, UserTier::Professional) => true,
            (UserTier::Basic, UserTier::Enterprise) => true,
            (UserTier::Professional, UserTier::Enterprise) => true,
            (_, UserTier::Admin) => false, // Admin tier can only be set by existing admins
            (UserTier::Admin, _) => false, // Admins cannot be downgraded through normal means
            _ => false,                    // No downgrades allowed
        }
    }
}

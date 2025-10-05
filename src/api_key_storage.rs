use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, RwLock};
use thiserror::Error;
use uuid::Uuid;

use crate::api_key_engine::{ApiKey, ApiKeyError, ApiKeyMetadata};

#[derive(Error, Debug)]
pub enum ApiKeyStorageError {
    #[error("API key not found: {0}")]
    NotFound(Uuid),

    #[error("API key with hash already exists")]
    AlreadyExists,

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Lock error: {0}")]
    LockError(String),
}

impl From<ApiKeyStorageError> for ApiKeyError {
    fn from(err: ApiKeyStorageError) -> Self {
        match err {
            ApiKeyStorageError::NotFound(_) => ApiKeyError::NotFound,
            _ => ApiKeyError::StorageError(err.to_string()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ApiKeyUsageLog {
    pub id: Uuid,
    pub api_key_id: Uuid,
    pub endpoint: String,
    pub method: String,
    pub ip_address: Option<IpAddr>,
    pub user_agent: Option<String>,
    pub request_size: Option<usize>,
    pub response_status: u16,
    pub response_time_ms: Option<u64>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ApiKeyUsageStats {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub avg_response_time_ms: f64,
    pub last_used_at: Option<DateTime<Utc>>,
    pub daily_usage: Vec<DailyUsage>,
}

#[derive(Debug, Clone)]
pub struct DailyUsage {
    pub date: String,
    pub requests: u64,
    pub errors: u64,
}

/// Trait for API key storage backends
#[async_trait]
pub trait ApiKeyStorage: Send + Sync {
    /// Create a new API key
    async fn create_api_key(&self, api_key: ApiKey) -> Result<ApiKey, ApiKeyStorageError>;

    /// Get API key by ID
    async fn get_api_key(&self, id: Uuid) -> Result<ApiKey, ApiKeyStorageError>;

    /// Get API key by hash
    async fn get_api_key_by_hash(&self, key_hash: &str) -> Result<ApiKey, ApiKeyStorageError>;

    /// Get all API keys for a user
    async fn get_user_api_keys(&self, user_id: Uuid) -> Result<Vec<ApiKey>, ApiKeyStorageError>;

    /// Update API key
    async fn update_api_key(&self, api_key: ApiKey) -> Result<ApiKey, ApiKeyStorageError>;

    /// Delete API key
    async fn delete_api_key(&self, id: Uuid) -> Result<(), ApiKeyStorageError>;

    /// Update last used timestamp and increment usage count
    async fn record_usage(&self, id: Uuid) -> Result<(), ApiKeyStorageError>;

    /// Log API key usage
    async fn log_usage(&self, log: ApiKeyUsageLog) -> Result<(), ApiKeyStorageError>;

    /// Get usage statistics for an API key
    async fn get_usage_stats(
        &self,
        api_key_id: Uuid,
        days: u32,
    ) -> Result<ApiKeyUsageStats, ApiKeyStorageError>;

    /// Get usage logs for an API key
    async fn get_usage_logs(
        &self,
        api_key_id: Uuid,
        limit: Option<usize>,
    ) -> Result<Vec<ApiKeyUsageLog>, ApiKeyStorageError>;
}

/// In-memory implementation of API key storage
pub struct InMemoryApiKeyStorage {
    api_keys: Arc<RwLock<HashMap<Uuid, ApiKey>>>,
    hash_index: Arc<RwLock<HashMap<String, Uuid>>>,
    user_index: Arc<RwLock<HashMap<Uuid, Vec<Uuid>>>>,
    usage_logs: Arc<RwLock<Vec<ApiKeyUsageLog>>>,
}

impl InMemoryApiKeyStorage {
    pub fn new() -> Self {
        Self {
            api_keys: Arc::new(RwLock::new(HashMap::new())),
            hash_index: Arc::new(RwLock::new(HashMap::new())),
            user_index: Arc::new(RwLock::new(HashMap::new())),
            usage_logs: Arc::new(RwLock::new(Vec::new())),
        }
    }

    fn calculate_usage_stats(
        &self,
        logs: &[ApiKeyUsageLog],
    ) -> ApiKeyUsageStats {
        let total_requests = logs.len() as u64;
        let successful_requests = logs.iter().filter(|l| l.response_status < 400).count() as u64;
        let failed_requests = total_requests - successful_requests;

        let avg_response_time_ms = if !logs.is_empty() {
            logs.iter()
                .filter_map(|l| l.response_time_ms)
                .sum::<u64>() as f64
                / logs.len() as f64
        } else {
            0.0
        };

        let last_used_at = logs.iter().map(|l| l.created_at).max();

        // Calculate daily usage
        let mut daily_map: HashMap<String, (u64, u64)> = HashMap::new();
        for log in logs {
            let date = log.created_at.format("%Y-%m-%d").to_string();
            let entry = daily_map.entry(date).or_insert((0, 0));
            entry.0 += 1;
            if log.response_status >= 400 {
                entry.1 += 1;
            }
        }

        let mut daily_usage: Vec<DailyUsage> = daily_map
            .into_iter()
            .map(|(date, (requests, errors))| DailyUsage {
                date,
                requests,
                errors,
            })
            .collect();
        daily_usage.sort_by(|a, b| a.date.cmp(&b.date));

        ApiKeyUsageStats {
            total_requests,
            successful_requests,
            failed_requests,
            avg_response_time_ms,
            last_used_at,
            daily_usage,
        }
    }
}

impl Default for InMemoryApiKeyStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ApiKeyStorage for InMemoryApiKeyStorage {
    async fn create_api_key(&self, api_key: ApiKey) -> Result<ApiKey, ApiKeyStorageError> {
        let mut keys = self.api_keys.write().map_err(|e| {
            ApiKeyStorageError::LockError(format!("Failed to acquire write lock: {}", e))
        })?;

        let mut hash_index = self.hash_index.write().map_err(|e| {
            ApiKeyStorageError::LockError(format!("Failed to acquire hash index lock: {}", e))
        })?;

        let mut user_index = self.user_index.write().map_err(|e| {
            ApiKeyStorageError::LockError(format!("Failed to acquire user index lock: {}", e))
        })?;

        // Check if key hash already exists
        if hash_index.contains_key(&api_key.key_hash) {
            return Err(ApiKeyStorageError::AlreadyExists);
        }

        // Store the key
        let key_id = api_key.id;
        let user_id = api_key.created_by;
        let key_hash = api_key.key_hash.clone();

        keys.insert(key_id, api_key.clone());
        hash_index.insert(key_hash, key_id);

        // Update user index
        user_index
            .entry(user_id)
            .or_insert_with(Vec::new)
            .push(key_id);

        Ok(api_key)
    }

    async fn get_api_key(&self, id: Uuid) -> Result<ApiKey, ApiKeyStorageError> {
        let keys = self.api_keys.read().map_err(|e| {
            ApiKeyStorageError::LockError(format!("Failed to acquire read lock: {}", e))
        })?;

        keys.get(&id)
            .cloned()
            .ok_or(ApiKeyStorageError::NotFound(id))
    }

    async fn get_api_key_by_hash(&self, key_hash: &str) -> Result<ApiKey, ApiKeyStorageError> {
        let key_id = {
            let hash_index = self.hash_index.read().map_err(|e| {
                ApiKeyStorageError::LockError(format!("Failed to acquire hash index lock: {}", e))
            })?;

            *hash_index
                .get(key_hash)
                .ok_or_else(|| ApiKeyStorageError::StorageError("Key hash not found".to_string()))?
        };

        self.get_api_key(key_id).await
    }

    async fn get_user_api_keys(&self, user_id: Uuid) -> Result<Vec<ApiKey>, ApiKeyStorageError> {
        let user_index = self.user_index.read().map_err(|e| {
            ApiKeyStorageError::LockError(format!("Failed to acquire user index lock: {}", e))
        })?;

        let key_ids = user_index.get(&user_id).cloned().unwrap_or_default();

        let keys = self.api_keys.read().map_err(|e| {
            ApiKeyStorageError::LockError(format!("Failed to acquire read lock: {}", e))
        })?;

        let user_keys = key_ids
            .iter()
            .filter_map(|id| keys.get(id).cloned())
            .collect();

        Ok(user_keys)
    }

    async fn update_api_key(&self, api_key: ApiKey) -> Result<ApiKey, ApiKeyStorageError> {
        let mut keys = self.api_keys.write().map_err(|e| {
            ApiKeyStorageError::LockError(format!("Failed to acquire write lock: {}", e))
        })?;

        if !keys.contains_key(&api_key.id) {
            return Err(ApiKeyStorageError::NotFound(api_key.id));
        }

        keys.insert(api_key.id, api_key.clone());
        Ok(api_key)
    }

    async fn delete_api_key(&self, id: Uuid) -> Result<(), ApiKeyStorageError> {
        let mut keys = self.api_keys.write().map_err(|e| {
            ApiKeyStorageError::LockError(format!("Failed to acquire write lock: {}", e))
        })?;

        let api_key = keys
            .remove(&id)
            .ok_or(ApiKeyStorageError::NotFound(id))?;

        // Clean up indexes
        let mut hash_index = self.hash_index.write().map_err(|e| {
            ApiKeyStorageError::LockError(format!("Failed to acquire hash index lock: {}", e))
        })?;

        let mut user_index = self.user_index.write().map_err(|e| {
            ApiKeyStorageError::LockError(format!("Failed to acquire user index lock: {}", e))
        })?;

        hash_index.remove(&api_key.key_hash);

        if let Some(user_keys) = user_index.get_mut(&api_key.created_by) {
            user_keys.retain(|&key_id| key_id != id);
        }

        Ok(())
    }

    async fn record_usage(&self, id: Uuid) -> Result<(), ApiKeyStorageError> {
        let mut keys = self.api_keys.write().map_err(|e| {
            ApiKeyStorageError::LockError(format!("Failed to acquire write lock: {}", e))
        })?;

        let api_key = keys
            .get_mut(&id)
            .ok_or(ApiKeyStorageError::NotFound(id))?;

        api_key.usage_count += 1;
        api_key.last_used_at = Some(Utc::now());

        Ok(())
    }

    async fn log_usage(&self, log: ApiKeyUsageLog) -> Result<(), ApiKeyStorageError> {
        let mut logs = self.usage_logs.write().map_err(|e| {
            ApiKeyStorageError::LockError(format!("Failed to acquire usage logs lock: {}", e))
        })?;

        logs.push(log);
        Ok(())
    }

    async fn get_usage_stats(
        &self,
        api_key_id: Uuid,
        days: u32,
    ) -> Result<ApiKeyUsageStats, ApiKeyStorageError> {
        let logs = self.usage_logs.read().map_err(|e| {
            ApiKeyStorageError::LockError(format!("Failed to acquire usage logs lock: {}", e))
        })?;

        let cutoff_date = Utc::now() - chrono::Duration::days(days as i64);

        let filtered_logs: Vec<ApiKeyUsageLog> = logs
            .iter()
            .filter(|log| log.api_key_id == api_key_id && log.created_at >= cutoff_date)
            .cloned()
            .collect();

        Ok(self.calculate_usage_stats(&filtered_logs))
    }

    async fn get_usage_logs(
        &self,
        api_key_id: Uuid,
        limit: Option<usize>,
    ) -> Result<Vec<ApiKeyUsageLog>, ApiKeyStorageError> {
        let logs = self.usage_logs.read().map_err(|e| {
            ApiKeyStorageError::LockError(format!("Failed to acquire usage logs lock: {}", e))
        })?;

        let mut filtered_logs: Vec<ApiKeyUsageLog> = logs
            .iter()
            .filter(|log| log.api_key_id == api_key_id)
            .cloned()
            .collect();

        // Sort by created_at descending (newest first)
        filtered_logs.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        if let Some(limit) = limit {
            filtered_logs.truncate(limit);
        }

        Ok(filtered_logs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api_key_engine::{CreateApiKeyRequest, OrganizationType, ApiKeyEngine, ApiKeyPermissions};
    use crate::logging::LoggingEngine;

    fn create_test_api_key(created_by: Uuid) -> ApiKey {
        let engine = ApiKeyEngine::new();

        let request = CreateApiKeyRequest {
            name: "Test Key".to_string(),
            created_by,
            organization_type: OrganizationType::Producer,
            organization_id: None,
            permissions: Some(ApiKeyPermissions::read_write()),
            allowed_endpoints: None,
            rate_limit_per_hour: Some(100),
            expires_in_days: None,
            notes: None,
            allowed_ips: None,
        };

        engine.create_api_key(request)
    }

    #[tokio::test]
    async fn test_create_and_get_api_key() {
        let storage = InMemoryApiKeyStorage::new();
        let user_id = Uuid::new_v4();
        let api_key = create_test_api_key(user_id);

        let result = storage.create_api_key(api_key.clone()).await;
        assert!(result.is_ok());

        let retrieved = storage.get_api_key(api_key.id).await;
        assert!(retrieved.is_ok());
        assert_eq!(retrieved.unwrap().id, api_key.id);
    }

    #[tokio::test]
    async fn test_get_api_key_by_hash() {
        let storage = InMemoryApiKeyStorage::new();
        let user_id = Uuid::new_v4();
        let api_key = create_test_api_key(user_id);
        let key_hash = api_key.key_hash.clone();

        storage.create_api_key(api_key.clone()).await.unwrap();

        let retrieved = storage.get_api_key_by_hash(&key_hash).await;
        assert!(retrieved.is_ok());
        assert_eq!(retrieved.unwrap().id, api_key.id);
    }

    #[tokio::test]
    async fn test_get_user_api_keys() {
        let storage = InMemoryApiKeyStorage::new();
        let user_id = Uuid::new_v4();

        let key1 = create_test_api_key(user_id);
        let key2 = create_test_api_key(user_id);

        storage.create_api_key(key1).await.unwrap();
        storage.create_api_key(key2).await.unwrap();

        let user_keys = storage.get_user_api_keys(user_id).await.unwrap();
        assert_eq!(user_keys.len(), 2);
    }

    #[tokio::test]
    async fn test_update_api_key() {
        let storage = InMemoryApiKeyStorage::new();
        let user_id = Uuid::new_v4();
        let mut api_key = create_test_api_key(user_id);

        storage.create_api_key(api_key.clone()).await.unwrap();

        api_key.is_active = false;
        let result = storage.update_api_key(api_key.clone()).await;
        assert!(result.is_ok());

        let updated = storage.get_api_key(api_key.id).await.unwrap();
        assert!(!updated.is_active);
    }

    #[tokio::test]
    async fn test_delete_api_key() {
        let storage = InMemoryApiKeyStorage::new();
        let user_id = Uuid::new_v4();
        let api_key = create_test_api_key(user_id);

        storage.create_api_key(api_key.clone()).await.unwrap();

        let result = storage.delete_api_key(api_key.id).await;
        assert!(result.is_ok());

        let retrieved = storage.get_api_key(api_key.id).await;
        assert!(retrieved.is_err());
    }

    #[tokio::test]
    async fn test_record_usage() {
        let storage = InMemoryApiKeyStorage::new();
        let user_id = Uuid::new_v4();
        let api_key = create_test_api_key(user_id);

        storage.create_api_key(api_key.clone()).await.unwrap();

        storage.record_usage(api_key.id).await.unwrap();

        let updated = storage.get_api_key(api_key.id).await.unwrap();
        assert_eq!(updated.usage_count, 1);
        assert!(updated.last_used_at.is_some());
    }

    #[tokio::test]
    async fn test_usage_logging_and_stats() {
        let storage = InMemoryApiKeyStorage::new();
        let user_id = Uuid::new_v4();
        let api_key = create_test_api_key(user_id);

        storage.create_api_key(api_key.clone()).await.unwrap();

        // Log some usage
        for i in 0..5 {
            let log = ApiKeyUsageLog {
                id: Uuid::new_v4(),
                api_key_id: api_key.id,
                endpoint: "/receipts".to_string(),
                method: "POST".to_string(),
                ip_address: None,
                user_agent: None,
                request_size: Some(1024),
                response_status: if i < 4 { 200 } else { 500 },
                response_time_ms: Some(100),
                error_message: None,
                created_at: Utc::now(),
            };
            storage.log_usage(log).await.unwrap();
        }

        let stats = storage.get_usage_stats(api_key.id, 7).await.unwrap();
        assert_eq!(stats.total_requests, 5);
        assert_eq!(stats.successful_requests, 4);
        assert_eq!(stats.failed_requests, 1);

        let logs = storage.get_usage_logs(api_key.id, Some(3)).await.unwrap();
        assert_eq!(logs.len(), 3);
    }
}

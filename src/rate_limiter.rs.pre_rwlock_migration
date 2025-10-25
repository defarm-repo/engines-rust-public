use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum RateLimitError {
    #[error("Rate limit exceeded: {0}")]
    Exceeded(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Lock error: {0}")]
    LockError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub requests_per_hour: u32,
    pub requests_per_minute: Option<u32>,
    pub requests_per_day: Option<u32>,
    pub burst_size: Option<u32>,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_hour: 100,
            requests_per_minute: None,
            requests_per_day: None,
            burst_size: None,
        }
    }
}

impl RateLimitConfig {
    pub fn new(requests_per_hour: u32) -> Self {
        Self {
            requests_per_hour,
            ..Default::default()
        }
    }

    pub fn with_minute_limit(mut self, requests_per_minute: u32) -> Self {
        self.requests_per_minute = Some(requests_per_minute);
        self
    }

    pub fn with_day_limit(mut self, requests_per_day: u32) -> Self {
        self.requests_per_day = Some(requests_per_day);
        self
    }

    pub fn with_burst(mut self, burst_size: u32) -> Self {
        self.burst_size = Some(burst_size);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitResult {
    pub allowed: bool,
    pub limit: u32,
    pub remaining: u32,
    pub reset_at: DateTime<Utc>,
    pub retry_after_seconds: Option<u64>,
}

#[derive(Debug, Clone)]
struct RequestRecord {
    timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct ApiKeyLimits {
    requests: VecDeque<RequestRecord>,
    config: RateLimitConfig,
}

impl ApiKeyLimits {
    fn new(config: RateLimitConfig) -> Self {
        Self {
            requests: VecDeque::new(),
            config,
        }
    }

    fn clean_old_requests(&mut self, window: Duration) {
        let cutoff = Utc::now() - window;
        while let Some(record) = self.requests.front() {
            if record.timestamp < cutoff {
                self.requests.pop_front();
            } else {
                break;
            }
        }
    }

    fn count_requests_in_window(&self, window: Duration) -> u32 {
        let cutoff = Utc::now() - window;
        self.requests
            .iter()
            .filter(|r| r.timestamp >= cutoff)
            .count() as u32
    }

    fn check_limit(&mut self) -> RateLimitResult {
        let now = Utc::now();

        // Clean old requests from all windows
        self.clean_old_requests(Duration::days(1));

        // Check minute limit
        if let Some(minute_limit) = self.config.requests_per_minute {
            let minute_count = self.count_requests_in_window(Duration::minutes(1));
            if minute_count >= minute_limit {
                let oldest_in_window = self
                    .requests
                    .iter()
                    .rev()
                    .nth((minute_count - 1) as usize)
                    .map(|r| r.timestamp)
                    .unwrap_or(now);

                let reset_at = oldest_in_window + Duration::minutes(1);
                let retry_after = (reset_at - now).num_seconds().max(0) as u64;

                return RateLimitResult {
                    allowed: false,
                    limit: minute_limit,
                    remaining: 0,
                    reset_at,
                    retry_after_seconds: Some(retry_after),
                };
            }
        }

        // Check hour limit
        let hour_count = self.count_requests_in_window(Duration::hours(1));
        if hour_count >= self.config.requests_per_hour {
            let oldest_in_window = self
                .requests
                .iter()
                .rev()
                .nth((hour_count - 1) as usize)
                .map(|r| r.timestamp)
                .unwrap_or(now);

            let reset_at = oldest_in_window + Duration::hours(1);
            let retry_after = (reset_at - now).num_seconds().max(0) as u64;

            return RateLimitResult {
                allowed: false,
                limit: self.config.requests_per_hour,
                remaining: 0,
                reset_at,
                retry_after_seconds: Some(retry_after),
            };
        }

        // Check day limit
        if let Some(day_limit) = self.config.requests_per_day {
            let day_count = self.count_requests_in_window(Duration::days(1));
            if day_count >= day_limit {
                let oldest_in_window = self
                    .requests
                    .iter()
                    .rev()
                    .nth((day_count - 1) as usize)
                    .map(|r| r.timestamp)
                    .unwrap_or(now);

                let reset_at = oldest_in_window + Duration::days(1);
                let retry_after = (reset_at - now).num_seconds().max(0) as u64;

                return RateLimitResult {
                    allowed: false,
                    limit: day_limit,
                    remaining: 0,
                    reset_at,
                    retry_after_seconds: Some(retry_after),
                };
            }
        }

        // Allowed - calculate remaining based on hour limit
        let remaining = self.config.requests_per_hour.saturating_sub(hour_count);
        let reset_at = if let Some(oldest) = self.requests.front() {
            oldest.timestamp + Duration::hours(1)
        } else {
            now + Duration::hours(1)
        };

        RateLimitResult {
            allowed: true,
            limit: self.config.requests_per_hour,
            remaining,
            reset_at,
            retry_after_seconds: None,
        }
    }

    fn record_request(&mut self) {
        self.requests.push_back(RequestRecord {
            timestamp: Utc::now(),
        });
    }
}

pub struct RateLimiter {
    limits: Arc<RwLock<HashMap<Uuid, ApiKeyLimits>>>,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            limits: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if request is allowed for given API key
    pub fn check_rate_limit(
        &self,
        api_key_id: Uuid,
        config: &RateLimitConfig,
    ) -> Result<RateLimitResult, RateLimitError> {
        let mut limits = self
            .limits
            .write()
            .map_err(|e| RateLimitError::LockError(format!("Failed to acquire write lock: {e}")))?;

        let key_limits = limits
            .entry(api_key_id)
            .or_insert_with(|| ApiKeyLimits::new(config.clone()));

        // Update config if changed
        key_limits.config = config.clone();

        let result = key_limits.check_limit();

        Ok(result)
    }

    /// Record a successful request
    pub fn record_request(&self, api_key_id: Uuid) -> Result<(), RateLimitError> {
        let mut limits = self
            .limits
            .write()
            .map_err(|e| RateLimitError::LockError(format!("Failed to acquire write lock: {e}")))?;

        if let Some(key_limits) = limits.get_mut(&api_key_id) {
            key_limits.record_request();
        }

        Ok(())
    }

    /// Get current rate limit status without recording a request
    pub fn get_rate_limit_status(
        &self,
        api_key_id: Uuid,
        config: &RateLimitConfig,
    ) -> Result<RateLimitResult, RateLimitError> {
        let mut limits = self
            .limits
            .write()
            .map_err(|e| RateLimitError::LockError(format!("Failed to acquire write lock: {e}")))?;

        let key_limits = limits
            .entry(api_key_id)
            .or_insert_with(|| ApiKeyLimits::new(config.clone()));

        Ok(key_limits.check_limit())
    }

    /// Reset rate limits for a specific API key
    pub fn reset_limits(&self, api_key_id: Uuid) -> Result<(), RateLimitError> {
        let mut limits = self
            .limits
            .write()
            .map_err(|e| RateLimitError::LockError(format!("Failed to acquire write lock: {e}")))?;

        limits.remove(&api_key_id);

        Ok(())
    }

    /// Clean up old data (should be called periodically)
    pub fn cleanup(&self) -> Result<(), RateLimitError> {
        let mut limits = self
            .limits
            .write()
            .map_err(|e| RateLimitError::LockError(format!("Failed to acquire write lock: {e}")))?;

        let cutoff = Utc::now() - Duration::days(1);
        limits.retain(|_, key_limits| {
            // Remove keys with no recent requests
            key_limits
                .requests
                .back()
                .is_some_and(|r| r.timestamp >= cutoff)
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_limiter() -> RateLimiter {
        RateLimiter::new()
    }

    #[test]
    fn test_rate_limit_basic() {
        let limiter = create_test_limiter();
        let api_key_id = Uuid::new_v4();
        let config = RateLimitConfig::new(5);

        // First 5 requests should be allowed
        for i in 0..5 {
            let result = limiter.check_rate_limit(api_key_id, &config).unwrap();
            assert!(result.allowed, "Request {i} should be allowed");
            limiter.record_request(api_key_id).unwrap();
        }

        // 6th request should be denied
        let result = limiter.check_rate_limit(api_key_id, &config).unwrap();
        assert!(!result.allowed);
        assert_eq!(result.remaining, 0);
        assert!(result.retry_after_seconds.is_some());
    }

    #[test]
    fn test_rate_limit_with_minute_limit() {
        let limiter = create_test_limiter();
        let api_key_id = Uuid::new_v4();
        let config = RateLimitConfig::new(100).with_minute_limit(3);

        // First 3 requests should be allowed
        for i in 0..3 {
            let result = limiter.check_rate_limit(api_key_id, &config).unwrap();
            assert!(result.allowed, "Request {i} should be allowed");
            limiter.record_request(api_key_id).unwrap();
        }

        // 4th request should be denied due to minute limit
        let result = limiter.check_rate_limit(api_key_id, &config).unwrap();
        assert!(!result.allowed);
    }

    #[test]
    fn test_rate_limit_remaining_count() {
        let limiter = create_test_limiter();
        let api_key_id = Uuid::new_v4();
        let config = RateLimitConfig::new(10);

        // Make 3 requests
        for _ in 0..3 {
            limiter.check_rate_limit(api_key_id, &config).unwrap();
            limiter.record_request(api_key_id).unwrap();
        }

        // Check remaining
        let result = limiter.check_rate_limit(api_key_id, &config).unwrap();
        assert_eq!(result.remaining, 7);
    }

    #[test]
    fn test_rate_limit_reset() {
        let limiter = create_test_limiter();
        let api_key_id = Uuid::new_v4();
        let config = RateLimitConfig::new(5);

        // Fill up the limit
        for _ in 0..5 {
            limiter.check_rate_limit(api_key_id, &config).unwrap();
            limiter.record_request(api_key_id).unwrap();
        }

        // Should be denied
        let result = limiter.check_rate_limit(api_key_id, &config).unwrap();
        assert!(!result.allowed);

        // Reset limits
        limiter.reset_limits(api_key_id).unwrap();

        // Should be allowed again
        let result = limiter.check_rate_limit(api_key_id, &config).unwrap();
        assert!(result.allowed);
    }

    #[test]
    fn test_multiple_api_keys() {
        let limiter = create_test_limiter();
        let key1 = Uuid::new_v4();
        let key2 = Uuid::new_v4();
        let config = RateLimitConfig::new(5);

        // Fill up key1
        for _ in 0..5 {
            limiter.check_rate_limit(key1, &config).unwrap();
            limiter.record_request(key1).unwrap();
        }

        // key1 should be denied
        let result = limiter.check_rate_limit(key1, &config).unwrap();
        assert!(!result.allowed);

        // key2 should still be allowed
        let result = limiter.check_rate_limit(key2, &config).unwrap();
        assert!(result.allowed);
    }

    #[test]
    fn test_get_rate_limit_status() {
        let limiter = create_test_limiter();
        let api_key_id = Uuid::new_v4();
        let config = RateLimitConfig::new(10);

        // Make some requests
        for _ in 0..3 {
            limiter.check_rate_limit(api_key_id, &config).unwrap();
            limiter.record_request(api_key_id).unwrap();
        }

        // Get status without recording
        let status = limiter.get_rate_limit_status(api_key_id, &config).unwrap();
        assert!(status.allowed);
        assert_eq!(status.remaining, 7);

        // Get status again - should be the same
        let status2 = limiter.get_rate_limit_status(api_key_id, &config).unwrap();
        assert_eq!(status2.remaining, 7);
    }

    #[test]
    fn test_cleanup() {
        let limiter = create_test_limiter();
        let api_key_id = Uuid::new_v4();
        let config = RateLimitConfig::new(10);

        limiter.check_rate_limit(api_key_id, &config).unwrap();
        limiter.record_request(api_key_id).unwrap();

        // Cleanup should not remove recent activity
        limiter.cleanup().unwrap();

        let result = limiter.check_rate_limit(api_key_id, &config).unwrap();
        assert_eq!(result.remaining, 9);
    }
}

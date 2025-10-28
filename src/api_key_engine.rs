use blake3;
use chrono::{DateTime, Duration, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum ApiKeyError {
    #[error("API key not found")]
    NotFound,

    #[error("API key is inactive")]
    Inactive,

    #[error("API key has expired")]
    Expired,

    #[error("Invalid API key format")]
    InvalidFormat,

    #[error("API key validation failed: {0}")]
    ValidationFailed(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("IP address not allowed: {0}")]
    IpNotAllowed(IpAddr),

    #[error("Permission denied: missing '{0}' permission")]
    PermissionDenied(String),

    #[error("Organization type mismatch: expected {expected}, got {actual}")]
    OrganizationTypeMismatch { expected: String, actual: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrganizationType {
    Admin,
    Producer,
    Association,
    Enterprise,
    Government,
    External,
}

impl std::fmt::Display for OrganizationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrganizationType::Admin => write!(f, "admin"),
            OrganizationType::Producer => write!(f, "producer"),
            OrganizationType::Association => write!(f, "association"),
            OrganizationType::Enterprise => write!(f, "enterprise"),
            OrganizationType::Government => write!(f, "government"),
            OrganizationType::External => write!(f, "external"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyPermissions {
    pub read: bool,
    pub write: bool,
    pub admin: bool,
    pub custom: HashMap<String, bool>,
}

impl Default for ApiKeyPermissions {
    fn default() -> Self {
        Self {
            read: true,
            write: false,
            admin: false,
            custom: HashMap::new(),
        }
    }
}

impl ApiKeyPermissions {
    pub fn read_only() -> Self {
        Self {
            read: true,
            write: false,
            admin: false,
            custom: HashMap::new(),
        }
    }

    pub fn read_write() -> Self {
        Self {
            read: true,
            write: true,
            admin: false,
            custom: HashMap::new(),
        }
    }

    pub fn admin() -> Self {
        Self {
            read: true,
            write: true,
            admin: true,
            custom: HashMap::new(),
        }
    }

    pub fn has_permission(&self, permission: &str) -> bool {
        match permission {
            "read" => self.read,
            "write" => self.write,
            "admin" => self.admin,
            custom => self.custom.get(custom).copied().unwrap_or(false),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: Uuid,
    pub name: String,
    pub key_hash: String,
    pub key_prefix: String,
    pub created_by: Uuid,
    pub organization_type: OrganizationType,
    pub organization_id: Option<Uuid>,
    pub permissions: ApiKeyPermissions,
    pub allowed_endpoints: Vec<String>,
    pub is_active: bool,
    pub last_used_at: Option<DateTime<Utc>>,
    pub usage_count: u64,
    pub rate_limit_per_hour: u32,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
    pub allowed_ips: Vec<IpAddr>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyMetadata {
    pub id: Uuid,
    pub name: String,
    pub key_prefix: String,
    pub organization_type: OrganizationType,
    pub permissions: ApiKeyPermissions,
    pub is_active: bool,
    pub last_used_at: Option<DateTime<Utc>>,
    pub usage_count: u64,
    pub rate_limit_per_hour: u32,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

impl From<ApiKey> for ApiKeyMetadata {
    fn from(key: ApiKey) -> Self {
        Self {
            id: key.id,
            name: key.name,
            key_prefix: key.key_prefix,
            organization_type: key.organization_type,
            permissions: key.permissions,
            is_active: key.is_active,
            last_used_at: key.last_used_at,
            usage_count: key.usage_count,
            rate_limit_per_hour: key.rate_limit_per_hour,
            created_at: key.created_at,
            expires_at: key.expires_at,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GeneratedApiKey {
    pub key: String,
    pub metadata: ApiKeyMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub created_by: Uuid,
    pub organization_type: OrganizationType,
    pub organization_id: Option<Uuid>,
    pub permissions: Option<ApiKeyPermissions>,
    pub allowed_endpoints: Option<Vec<String>>,
    pub rate_limit_per_hour: Option<u32>,
    pub expires_in_days: Option<i64>,
    pub notes: Option<String>,
    pub allowed_ips: Option<Vec<IpAddr>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyValidationResult {
    pub valid: bool,
    pub api_key_id: Option<Uuid>,
    pub created_by: Option<Uuid>,
    pub permissions: Option<ApiKeyPermissions>,
    pub organization_type: Option<OrganizationType>,
    pub rate_limit_per_hour: Option<u32>,
    pub error: Option<String>,
}

pub struct ApiKeyEngine {
    // Note: Logging is optional and can be added via wrapper if needed
}

impl Default for ApiKeyEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiKeyEngine {
    const API_KEY_PREFIX: &'static str = "dfm_";
    const KEY_LENGTH: usize = 32;

    pub fn new() -> Self {
        Self {}
    }

    /// Generate a new API key with cryptographic randomness
    pub fn generate_key(&self) -> (String, String, String) {
        let mut rng = rand::thread_rng();
        let chars: Vec<char> = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
            .chars()
            .collect();

        let random_part: String = (0..Self::KEY_LENGTH)
            .map(|_| chars[rng.gen_range(0..chars.len())])
            .collect();

        let full_key = format!("{}{}", Self::API_KEY_PREFIX, random_part);
        let key_hash = self.hash_key(&full_key);
        let key_prefix = full_key.chars().take(8).collect();

        (full_key, key_hash, key_prefix)
    }

    /// Hash an API key using BLAKE3
    pub fn hash_key(&self, key: &str) -> String {
        let hash = blake3::hash(key.as_bytes());
        hash.to_hex().to_string()
    }

    /// Create a new API key record
    pub fn create_api_key(&self, request: CreateApiKeyRequest) -> ApiKey {
        let (_, key_hash, key_prefix) = self.generate_key();
        let now = Utc::now();

        let expires_at = request
            .expires_in_days
            .map(|days| now + Duration::days(days));

        ApiKey {
            id: Uuid::new_v4(),
            name: request.name.clone(),
            key_hash,
            key_prefix,
            created_by: request.created_by,
            organization_type: request.organization_type.clone(),
            organization_id: request.organization_id,
            permissions: request.permissions.unwrap_or_default(),
            // Empty vector means no restrictions - allow all endpoints
            allowed_endpoints: request.allowed_endpoints.unwrap_or_default(),
            is_active: true,
            last_used_at: None,
            usage_count: 0,
            rate_limit_per_hour: request.rate_limit_per_hour.unwrap_or(100),
            created_at: now,
            expires_at,
            notes: request.notes,
            allowed_ips: request.allowed_ips.unwrap_or_default(),
        }
    }

    /// Validate an API key
    pub fn validate_key(&self, key: &str, stored_key: &ApiKey) -> Result<(), ApiKeyError> {
        // Validate format
        if !key.starts_with(Self::API_KEY_PREFIX) {
            return Err(ApiKeyError::InvalidFormat);
        }

        // Hash and compare
        let key_hash = self.hash_key(key);
        if key_hash != stored_key.key_hash {
            return Err(ApiKeyError::ValidationFailed(
                "Key hash mismatch".to_string(),
            ));
        }

        // Check if active
        if !stored_key.is_active {
            return Err(ApiKeyError::Inactive);
        }

        // Check expiration
        if let Some(expires_at) = stored_key.expires_at {
            if expires_at < Utc::now() {
                return Err(ApiKeyError::Expired);
            }
        }

        Ok(())
    }

    /// Check if IP is allowed for this API key
    pub fn check_ip_allowed(&self, api_key: &ApiKey, ip: IpAddr) -> Result<(), ApiKeyError> {
        if api_key.allowed_ips.is_empty() {
            return Ok(());
        }

        if api_key.allowed_ips.contains(&ip) {
            Ok(())
        } else {
            Err(ApiKeyError::IpNotAllowed(ip))
        }
    }

    /// Check if endpoint is allowed for this API key
    pub fn check_endpoint_allowed(&self, api_key: &ApiKey, endpoint: &str) -> bool {
        api_key.allowed_endpoints.is_empty()
            || api_key.allowed_endpoints.contains(&endpoint.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_engine() -> ApiKeyEngine {
        ApiKeyEngine::new()
    }

    #[test]
    fn test_generate_key() {
        let engine = create_test_engine();
        let (key1, hash1, prefix1) = engine.generate_key();
        let (key2, hash2, prefix2) = engine.generate_key();

        assert!(key1.starts_with("dfm_"));
        assert!(key2.starts_with("dfm_"));
        assert_ne!(key1, key2);
        assert_ne!(hash1, hash2);
        assert_eq!(prefix1.len(), 8);
        assert_eq!(prefix2.len(), 8);
    }

    #[test]
    fn test_hash_key() {
        let engine = create_test_engine();
        let key = "dfm_test123";
        let hash1 = engine.hash_key(key);
        let hash2 = engine.hash_key(key);

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // BLAKE3 produces 32 bytes = 64 hex chars
    }

    #[test]
    fn test_create_api_key() {
        let engine = create_test_engine();
        let request = CreateApiKeyRequest {
            name: "Test Key".to_string(),
            created_by: Uuid::new_v4(),
            organization_type: OrganizationType::Producer,
            organization_id: None,
            permissions: Some(ApiKeyPermissions::read_write()),
            allowed_endpoints: None,
            rate_limit_per_hour: Some(200),
            expires_in_days: Some(30),
            notes: Some("Test key".to_string()),
            allowed_ips: None,
        };

        let api_key = engine.create_api_key(request);

        assert_eq!(api_key.name, "Test Key");
        assert!(api_key.is_active);
        assert_eq!(api_key.usage_count, 0);
        assert_eq!(api_key.rate_limit_per_hour, 200);
        assert!(api_key.expires_at.is_some());
    }

    #[test]
    fn test_validate_key() {
        let engine = create_test_engine();
        let (key, _, _) = engine.generate_key();

        let request = CreateApiKeyRequest {
            name: "Test Key".to_string(),
            created_by: Uuid::new_v4(),
            organization_type: OrganizationType::Producer,
            organization_id: None,
            permissions: None,
            allowed_endpoints: None,
            rate_limit_per_hour: None,
            expires_in_days: None,
            notes: None,
            allowed_ips: None,
        };

        let mut api_key = engine.create_api_key(request);
        api_key.key_hash = engine.hash_key(&key);

        assert!(engine.validate_key(&key, &api_key).is_ok());
        assert!(engine.validate_key("invalid_key", &api_key).is_err());
    }

    #[test]
    fn test_permissions() {
        let read_only = ApiKeyPermissions::read_only();
        assert!(read_only.has_permission("read"));
        assert!(!read_only.has_permission("write"));
        assert!(!read_only.has_permission("admin"));

        let admin = ApiKeyPermissions::admin();
        assert!(admin.has_permission("read"));
        assert!(admin.has_permission("write"));
        assert!(admin.has_permission("admin"));
    }

    #[test]
    fn test_ip_restrictions() {
        let engine = create_test_engine();
        let allowed_ip: IpAddr = "192.168.1.100".parse().unwrap();
        let blocked_ip: IpAddr = "192.168.1.200".parse().unwrap();

        let request = CreateApiKeyRequest {
            name: "Test Key".to_string(),
            created_by: Uuid::new_v4(),
            organization_type: OrganizationType::Producer,
            organization_id: None,
            permissions: None,
            allowed_endpoints: None,
            rate_limit_per_hour: None,
            expires_in_days: None,
            notes: None,
            allowed_ips: Some(vec![allowed_ip]),
        };

        let api_key = engine.create_api_key(request);

        assert!(engine.check_ip_allowed(&api_key, allowed_ip).is_ok());
        assert!(engine.check_ip_allowed(&api_key, blocked_ip).is_err());
    }
}

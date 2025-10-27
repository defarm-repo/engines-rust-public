//! Structured error tracking and classification
//!
//! This module provides error classification for monitoring and analysis.

use serde::{Deserialize, Serialize};
use std::fmt;
use tracing::{error, warn};
use uuid::Uuid;

/// Error classification for monitoring
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    /// Storage lock timeout (with_storage helper timeout)
    StorageLockTimeout,
    /// Database connection pool exhausted or query timeout
    DatabaseTimeout,
    /// Database connection or query error
    DatabaseError,
    /// Invalid request validation (bad input)
    ValidationError,
    /// Authentication or authorization failure
    AuthError,
    /// External service timeout (HTTP, Redis, etc.)
    ExternalTimeout,
    /// External service error
    ExternalError,
    /// Internal server error (panic, unexpected state)
    InternalError,
    /// Resource not found
    NotFound,
    /// Rate limit exceeded
    RateLimitExceeded,
    /// Other/unclassified error
    Other,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::StorageLockTimeout => write!(f, "storage_lock_timeout"),
            ErrorKind::DatabaseTimeout => write!(f, "database_timeout"),
            ErrorKind::DatabaseError => write!(f, "database_error"),
            ErrorKind::ValidationError => write!(f, "validation_error"),
            ErrorKind::AuthError => write!(f, "auth_error"),
            ErrorKind::ExternalTimeout => write!(f, "external_timeout"),
            ErrorKind::ExternalError => write!(f, "external_error"),
            ErrorKind::InternalError => write!(f, "internal_error"),
            ErrorKind::NotFound => write!(f, "not_found"),
            ErrorKind::RateLimitExceeded => write!(f, "rate_limit_exceeded"),
            ErrorKind::Other => write!(f, "other"),
        }
    }
}

/// Structured error context for logging and monitoring
#[derive(Debug, Clone, Serialize)]
pub struct ErrorContext {
    pub trace_id: String,
    pub endpoint: String,
    pub method: String,
    pub status_code: u16,
    pub error_kind: ErrorKind,
    pub message: String,
    pub user_id: Option<String>,
    pub duration_ms: Option<u128>,
}

impl ErrorContext {
    /// Create a new error context with trace ID
    pub fn new(
        endpoint: String,
        method: String,
        status_code: u16,
        error_kind: ErrorKind,
        message: String,
    ) -> Self {
        Self {
            trace_id: Uuid::new_v4().to_string(),
            endpoint,
            method,
            status_code,
            error_kind,
            message,
            user_id: None,
            duration_ms: None,
        }
    }

    /// Set user ID context
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Set duration context
    pub fn with_duration_ms(mut self, duration_ms: u128) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }

    /// Log the error with appropriate level
    pub fn log(&self) {
        let is_client_error = self.status_code >= 400 && self.status_code < 500;
        let is_server_error = self.status_code >= 500;

        if is_server_error {
            error!(
                trace_id = %self.trace_id,
                endpoint = %self.endpoint,
                method = %self.method,
                status_code = self.status_code,
                error_kind = %self.error_kind,
                message = %self.message,
                user_id = ?self.user_id,
                duration_ms = ?self.duration_ms,
                "API error"
            );
        } else if is_client_error {
            warn!(
                trace_id = %self.trace_id,
                endpoint = %self.endpoint,
                method = %self.method,
                status_code = self.status_code,
                error_kind = %self.error_kind,
                message = %self.message,
                user_id = ?self.user_id,
                duration_ms = ?self.duration_ms,
                "API client error"
            );
        }
    }

    /// Convert to JSON for response headers
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "trace_id": self.trace_id,
            "error_kind": self.error_kind,
            "message": self.message,
        })
    }
}

/// Helper to classify error from message
pub fn classify_error(message: &str, status_code: u16) -> ErrorKind {
    let msg_lower = message.to_lowercase();

    // Storage lock timeout (from with_storage helper)
    if msg_lower.contains("storage lock timeout") || msg_lower.contains("storage temporarily busy")
    {
        return ErrorKind::StorageLockTimeout;
    }

    // Database errors
    if msg_lower.contains("pool timeout")
        || msg_lower.contains("connection pool")
        || msg_lower.contains("database timeout")
    {
        return ErrorKind::DatabaseTimeout;
    }

    if msg_lower.contains("database")
        || msg_lower.contains("postgres")
        || msg_lower.contains("sql")
        || msg_lower.contains("query")
    {
        return ErrorKind::DatabaseError;
    }

    // Validation errors
    if status_code == 400
        || msg_lower.contains("invalid")
        || msg_lower.contains("validation")
        || msg_lower.contains("malformed")
    {
        return ErrorKind::ValidationError;
    }

    // Auth errors
    if status_code == 401
        || status_code == 403
        || msg_lower.contains("unauthorized")
        || msg_lower.contains("forbidden")
        || msg_lower.contains("authentication")
        || msg_lower.contains("credentials")
    {
        return ErrorKind::AuthError;
    }

    // Not found
    if status_code == 404 {
        return ErrorKind::NotFound;
    }

    // Rate limit
    if status_code == 429 {
        return ErrorKind::RateLimitExceeded;
    }

    // External service errors
    if msg_lower.contains("redis")
        || msg_lower.contains("cache")
        || msg_lower.contains("http client")
        || msg_lower.contains("external")
    {
        if msg_lower.contains("timeout") {
            return ErrorKind::ExternalTimeout;
        }
        return ErrorKind::ExternalError;
    }

    // Service unavailable (often storage lock timeout)
    if status_code == 503 {
        return ErrorKind::StorageLockTimeout;
    }

    // Internal errors
    if status_code >= 500 {
        return ErrorKind::InternalError;
    }

    ErrorKind::Other
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_storage_lock_timeout() {
        assert_eq!(
            classify_error("Storage lock timeout", 503),
            ErrorKind::StorageLockTimeout
        );
        assert_eq!(
            classify_error("Service temporarily busy", 503),
            ErrorKind::StorageLockTimeout
        );
    }

    #[test]
    fn test_classify_database_errors() {
        assert_eq!(
            classify_error("Pool timeout waiting for connection", 500),
            ErrorKind::DatabaseTimeout
        );
        assert_eq!(
            classify_error("Database query failed", 500),
            ErrorKind::DatabaseError
        );
    }

    #[test]
    fn test_classify_auth_errors() {
        assert_eq!(
            classify_error("Invalid credentials", 401),
            ErrorKind::AuthError
        );
        assert_eq!(classify_error("Unauthorized", 401), ErrorKind::AuthError);
    }

    #[test]
    fn test_classify_validation_errors() {
        assert_eq!(
            classify_error("Invalid input format", 400),
            ErrorKind::ValidationError
        );
    }
}

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use thiserror::Error;

/// Comprehensive error types for the DeFarm system
#[derive(Error, Debug)]
pub enum DeFarmError {
    // API Key Errors
    #[error("API key error: {0}")]
    ApiKey(#[from] crate::api_key_engine::ApiKeyError),

    #[error("API key storage error: {0}")]
    ApiKeyStorage(#[from] crate::api_key_storage::ApiKeyStorageError),

    #[error("Rate limit error: {0}")]
    RateLimit(#[from] crate::rate_limiter::RateLimitError),

    // Storage Errors
    #[error("Storage error: {0}")]
    Storage(String),

    // Validation Errors
    #[error("Validation error: {0}")]
    Validation(String),

    // Not Found Errors
    #[error("{0} not found")]
    NotFound(String),

    // Permission Errors
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    // Credit/Tier Errors
    #[error("Insufficient credits: {0}")]
    InsufficientCredits(String),

    #[error("Tier limit exceeded: {0}")]
    TierLimitExceeded(String),

    // Circuit Errors
    #[error("Circuit error: {0}")]
    Circuit(String),

    // Item Errors
    #[error("Item error: {0}")]
    Item(String),

    // Conflict Errors
    #[error("Conflict detected: {0}")]
    Conflict(String),

    // Internal Errors
    #[error("Internal server error: {0}")]
    Internal(String),

    // External Errors
    #[error("External service error: {0}")]
    External(String),
}

/// Error response structure
#[derive(Debug, serde::Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_suggestions: Option<Vec<String>>,
}

impl ErrorResponse {
    pub fn new(error: &str, message: &str) -> Self {
        Self {
            error: error.to_string(),
            message: message.to_string(),
            details: None,
            recovery_suggestions: None,
        }
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn with_recovery(mut self, suggestions: Vec<String>) -> Self {
        self.recovery_suggestions = Some(suggestions);
        self
    }
}

/// Recovery strategies for different error types
pub trait RecoveryStrategy {
    fn get_recovery_suggestions(&self) -> Vec<String>;
}

impl RecoveryStrategy for DeFarmError {
    fn get_recovery_suggestions(&self) -> Vec<String> {
        match self {
            DeFarmError::ApiKey(err) => match err {
                crate::api_key_engine::ApiKeyError::NotFound => vec![
                    "Verify that the API key is correct".to_string(),
                    "Generate a new API key if the old one was deleted".to_string(),
                ],
                crate::api_key_engine::ApiKeyError::Inactive => vec![
                    "Activate the API key in your dashboard".to_string(),
                    "Generate a new API key if needed".to_string(),
                ],
                crate::api_key_engine::ApiKeyError::Expired => vec![
                    "Generate a new API key to replace the expired one".to_string(),
                    "Update your application with the new key".to_string(),
                ],
                crate::api_key_engine::ApiKeyError::InvalidFormat => vec![
                    "Ensure the API key starts with 'dfm_'".to_string(),
                    "Check for any extra spaces or characters".to_string(),
                ],
                crate::api_key_engine::ApiKeyError::IpNotAllowed(_) => vec![
                    "Add your current IP to the allowed list".to_string(),
                    "Contact your administrator to update IP restrictions".to_string(),
                ],
                crate::api_key_engine::ApiKeyError::PermissionDenied(_) => vec![
                    "Request additional permissions from your administrator".to_string(),
                    "Use an API key with the required permissions".to_string(),
                ],
                _ => vec!["Contact support for assistance".to_string()],
            },

            DeFarmError::RateLimit(_) => vec![
                "Wait for the rate limit window to reset".to_string(),
                "Upgrade to a higher tier for increased limits".to_string(),
                "Implement exponential backoff in your application".to_string(),
                "Reduce request frequency".to_string(),
            ],

            DeFarmError::InsufficientCredits(_) => vec![
                "Purchase additional credits".to_string(),
                "Upgrade to a higher tier".to_string(),
                "Wait for your credits to reset".to_string(),
            ],

            DeFarmError::TierLimitExceeded(_) => vec![
                "Upgrade to a higher tier".to_string(),
                "Remove unused resources to free up capacity".to_string(),
            ],

            DeFarmError::PermissionDenied(_) => vec![
                "Request access from the resource owner".to_string(),
                "Verify you're using the correct API key".to_string(),
            ],

            DeFarmError::NotFound(_) => vec![
                "Verify the resource ID is correct".to_string(),
                "Check if the resource was deleted".to_string(),
            ],

            DeFarmError::Validation(_) => vec![
                "Review the request payload for errors".to_string(),
                "Consult the API documentation for correct format".to_string(),
            ],

            DeFarmError::Conflict(_) => vec![
                "Review the conflict details".to_string(),
                "Resolve the conflict manually if auto-resolution failed".to_string(),
            ],

            DeFarmError::Internal(_) => vec![
                "Retry the request after a short delay".to_string(),
                "Contact support if the issue persists".to_string(),
            ],

            _ => vec!["Contact support for assistance".to_string()],
        }
    }
}

impl DeFarmError {
    pub fn to_status_code(&self) -> StatusCode {
        match self {
            DeFarmError::ApiKey(err) => match err {
                crate::api_key_engine::ApiKeyError::NotFound => StatusCode::UNAUTHORIZED,
                crate::api_key_engine::ApiKeyError::Inactive => StatusCode::UNAUTHORIZED,
                crate::api_key_engine::ApiKeyError::Expired => StatusCode::UNAUTHORIZED,
                crate::api_key_engine::ApiKeyError::InvalidFormat => StatusCode::BAD_REQUEST,
                crate::api_key_engine::ApiKeyError::ValidationFailed(_) => StatusCode::UNAUTHORIZED,
                crate::api_key_engine::ApiKeyError::IpNotAllowed(_) => StatusCode::FORBIDDEN,
                crate::api_key_engine::ApiKeyError::PermissionDenied(_) => StatusCode::FORBIDDEN,
                crate::api_key_engine::ApiKeyError::OrganizationTypeMismatch { .. } => {
                    StatusCode::FORBIDDEN
                }
                crate::api_key_engine::ApiKeyError::StorageError(_) => {
                    StatusCode::INTERNAL_SERVER_ERROR
                }
            },

            DeFarmError::ApiKeyStorage(_) => StatusCode::INTERNAL_SERVER_ERROR,
            DeFarmError::RateLimit(_) => StatusCode::TOO_MANY_REQUESTS,
            DeFarmError::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
            DeFarmError::Validation(_) => StatusCode::BAD_REQUEST,
            DeFarmError::NotFound(_) => StatusCode::NOT_FOUND,
            DeFarmError::PermissionDenied(_) => StatusCode::FORBIDDEN,
            DeFarmError::InsufficientCredits(_) => StatusCode::PAYMENT_REQUIRED,
            DeFarmError::TierLimitExceeded(_) => StatusCode::FORBIDDEN,
            DeFarmError::Circuit(_) => StatusCode::BAD_REQUEST,
            DeFarmError::Item(_) => StatusCode::BAD_REQUEST,
            DeFarmError::Conflict(_) => StatusCode::CONFLICT,
            DeFarmError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            DeFarmError::External(_) => StatusCode::BAD_GATEWAY,
        }
    }

    pub fn error_code(&self) -> &str {
        match self {
            DeFarmError::ApiKey(err) => match err {
                crate::api_key_engine::ApiKeyError::NotFound => "api_key_not_found",
                crate::api_key_engine::ApiKeyError::Inactive => "api_key_inactive",
                crate::api_key_engine::ApiKeyError::Expired => "api_key_expired",
                crate::api_key_engine::ApiKeyError::InvalidFormat => "api_key_invalid_format",
                crate::api_key_engine::ApiKeyError::ValidationFailed(_) => {
                    "api_key_validation_failed"
                }
                crate::api_key_engine::ApiKeyError::IpNotAllowed(_) => "ip_not_allowed",
                crate::api_key_engine::ApiKeyError::PermissionDenied(_) => "permission_denied",
                crate::api_key_engine::ApiKeyError::OrganizationTypeMismatch { .. } => {
                    "organization_type_mismatch"
                }
                crate::api_key_engine::ApiKeyError::StorageError(_) => "storage_error",
            },
            DeFarmError::ApiKeyStorage(_) => "api_key_storage_error",
            DeFarmError::RateLimit(_) => "rate_limit_exceeded",
            DeFarmError::Storage(_) => "storage_error",
            DeFarmError::Validation(_) => "validation_error",
            DeFarmError::NotFound(_) => "not_found",
            DeFarmError::PermissionDenied(_) => "permission_denied",
            DeFarmError::InsufficientCredits(_) => "insufficient_credits",
            DeFarmError::TierLimitExceeded(_) => "tier_limit_exceeded",
            DeFarmError::Circuit(_) => "circuit_error",
            DeFarmError::Item(_) => "item_error",
            DeFarmError::Conflict(_) => "conflict_detected",
            DeFarmError::Internal(_) => "internal_error",
            DeFarmError::External(_) => "external_error",
        }
    }
}

impl IntoResponse for DeFarmError {
    fn into_response(self) -> Response {
        let status = self.to_status_code();
        let error_code = self.error_code();
        let message = self.to_string();
        let recovery_suggestions = self.get_recovery_suggestions();

        let error_response =
            ErrorResponse::new(error_code, &message).with_recovery(recovery_suggestions);

        (status, Json(error_response)).into_response()
    }
}

/// Result type alias for DeFarm operations
pub type DeFarmResult<T> = Result<T, DeFarmError>;

/// Helper function to create validation errors
pub fn validation_error(message: impl Into<String>) -> DeFarmError {
    DeFarmError::Validation(message.into())
}

/// Helper function to create not found errors
pub fn not_found(resource: impl Into<String>) -> DeFarmError {
    DeFarmError::NotFound(resource.into())
}

/// Helper function to create permission denied errors
pub fn permission_denied(message: impl Into<String>) -> DeFarmError {
    DeFarmError::PermissionDenied(message.into())
}

/// Helper function to create internal errors
pub fn internal_error(message: impl Into<String>) -> DeFarmError {
    DeFarmError::Internal(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_to_status_code() {
        assert_eq!(
            DeFarmError::NotFound("test".to_string()).to_status_code(),
            StatusCode::NOT_FOUND
        );

        assert_eq!(
            DeFarmError::Validation("test".to_string()).to_status_code(),
            StatusCode::BAD_REQUEST
        );

        assert_eq!(
            DeFarmError::PermissionDenied("test".to_string()).to_status_code(),
            StatusCode::FORBIDDEN
        );
    }

    #[test]
    fn test_error_code() {
        assert_eq!(
            DeFarmError::NotFound("test".to_string()).error_code(),
            "not_found"
        );

        assert_eq!(
            DeFarmError::RateLimit(crate::rate_limiter::RateLimitError::Exceeded(
                "test".to_string()
            ))
            .error_code(),
            "rate_limit_exceeded"
        );
    }

    #[test]
    fn test_recovery_suggestions() {
        let err = DeFarmError::RateLimit(crate::rate_limiter::RateLimitError::Exceeded(
            "test".to_string(),
        ));
        let suggestions = err.get_recovery_suggestions();

        assert!(!suggestions.is_empty());
        assert!(suggestions
            .iter()
            .any(|s| s.contains("rate limit") || s.contains("backoff")));
    }

    #[test]
    fn test_helper_functions() {
        let err = validation_error("Invalid input");
        assert!(matches!(err, DeFarmError::Validation(_)));

        let err = not_found("Item");
        assert!(matches!(err, DeFarmError::NotFound(_)));

        let err = permission_denied("Access denied");
        assert!(matches!(err, DeFarmError::PermissionDenied(_)));

        let err = internal_error("Database connection failed");
        assert!(matches!(err, DeFarmError::Internal(_)));
    }
}

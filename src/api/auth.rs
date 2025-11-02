use crate::api::shared_state::AppState;
use crate::auth_middleware::jwt_auth_middleware;
use crate::http_utils::svc_unavailable_retry;
use crate::storage::StorageBackend;
use crate::storage_helpers::{with_storage, with_storage_traced, StorageLockError};
use crate::types::{
    AccountStatus, CreditTransaction, CreditTransactionType, PasswordResetToken, TierLimits,
    UserAccount, UserTier,
};
use axum::{
    extract::{Extension, State},
    http::StatusCode,
    middleware,
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use bcrypt::{hash, verify, DEFAULT_COST};
use blake3;
use chrono::{Duration, Utc};
use hex;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

/// Validates password complexity requirements
/// Requirements:
/// - Minimum 8 characters
/// - At least one uppercase letter
/// - At least one lowercase letter
/// - At least one digit
/// - At least one special character
pub fn validate_password_complexity(password: &str) -> Result<(), String> {
    if password.len() < 8 {
        return Err("Password must be at least 8 characters long".to_string());
    }

    if !password.chars().any(|c| c.is_uppercase()) {
        return Err("Password must contain at least one uppercase letter".to_string());
    }

    if !password.chars().any(|c| c.is_lowercase()) {
        return Err("Password must contain at least one lowercase letter".to_string());
    }

    if !password.chars().any(|c| c.is_ascii_digit()) {
        return Err("Password must contain at least one digit".to_string());
    }

    if !password.chars().any(|c| !c.is_alphanumeric()) {
        return Err("Password must contain at least one special character".to_string());
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub user_id: String,
    pub workspace_id: Option<String>,
    pub exp: usize,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    pub workspace_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub email: String,
    pub workspace_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user_id: String,
    pub workspace_id: Option<String>,
    pub expires_at: i64,
}

#[derive(Debug, Serialize)]
pub struct UserProfile {
    pub user_id: String,
    pub username: String,
    pub email: String,
    pub created_at: i64,
    pub workspace_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ForgotPasswordRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ForgotPasswordResponse {
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub token: String,
    pub new_password: String,
}

#[derive(Debug, Serialize)]
pub struct ResetPasswordResponse {
    pub message: String,
}

// Auth state contains JWT secret
#[derive(Debug)]
pub struct AuthState {
    pub jwt_secret: String,
}

impl Default for AuthState {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthState {
    pub fn new() -> Self {
        let jwt_secret = std::env::var("JWT_SECRET")
            .expect("JWT_SECRET environment variable must be set. Please set a secure secret key for JWT authentication.");

        if jwt_secret.len() < 32 {
            panic!("JWT_SECRET must be at least 32 characters long for security");
        }

        Self { jwt_secret }
    }

    pub fn generate_token(
        &self,
        user_id: &str,
        workspace_id: Option<String>,
    ) -> Result<String, jsonwebtoken::errors::Error> {
        let expiration = Utc::now()
            .checked_add_signed(Duration::hours(24))
            .expect("valid timestamp")
            .timestamp();

        let claims = Claims {
            user_id: user_id.to_string(),
            workspace_id,
            exp: expiration as usize,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_ref()),
        )
    }

    pub fn verify_token(&self, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_ref()),
            &Validation::default(),
        )
        .map(|data| data.claims)
    }
}

fn password_reset_rate_limit_per_hour() -> usize {
    std::env::var("PASSWORD_RESET_RATE_LIMIT_PER_HOUR")
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(3)
}

fn should_log_reset_token() -> bool {
    std::env::var("PASSWORD_RESET_DEBUG_LOG")
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "" | "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

fn sanitize_optional_identifier(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn map_storage_lock_error(e: StorageLockError) -> (StatusCode, Json<Value>) {
    match e {
        StorageLockError::Timeout => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "error": "Service temporarily unavailable, please retry"
            })),
        ),
        StorageLockError::Other(msg) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": msg
            })),
        ),
    }
}

pub fn auth_routes(app_state: Arc<AppState>) -> Router {
    // Create AuthState using the shared JWT secret from AppState
    let auth_state = Arc::new(AuthState {
        jwt_secret: app_state.jwt_secret.clone(),
    });

    // Unauthenticated routes
    let public_routes = Router::new()
        .route("/login", post(login))
        .route("/register", post(register)) // Active but hidden from public docs
        .route("/forgot-password", post(forgot_password))
        .route("/reset-password", post(reset_password));

    // Protected routes requiring JWT authentication
    let protected_routes = Router::new()
        .route("/profile", get(get_profile))
        .route("/refresh", post(refresh_token))
        .route_layer(middleware::from_fn_with_state(
            app_state.clone(),
            jwt_auth_middleware,
        ));

    // Merge public and protected routes
    public_routes
        .merge(protected_routes)
        .with_state((auth_state, app_state))
}

#[instrument(skip(app_state, payload))]
async fn forgot_password(
    State((_auth, app_state)): State<(Arc<AuthState>, Arc<AppState>)>,
    Json(payload): Json<ForgotPasswordRequest>,
) -> Result<Json<ForgotPasswordResponse>, (StatusCode, Json<Value>)> {
    let ForgotPasswordRequest { email, username } = payload;
    let email = sanitize_optional_identifier(email);
    let username = sanitize_optional_identifier(username);

    if email.is_none() && username.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "Provide email or username to reset your password"
            })),
        ));
    }

    let lookup_email = email.clone();
    let lookup_username = username.clone();

    let user = with_storage_traced(
        &app_state.shared_storage,
        "auth_forgot_password_lookup",
        "/api/auth/forgot-password",
        "POST",
        move |storage| {
            if let Some(ref email) = lookup_email {
                storage
                    .get_user_by_email(email)
                    .map_err(Box::<dyn std::error::Error>::from)
            } else if let Some(ref username) = lookup_username {
                storage
                    .get_user_by_username(username)
                    .map_err(Box::<dyn std::error::Error>::from)
            } else {
                Ok(None)
            }
        },
    )
    .map_err(map_storage_lock_error)?;

    if let Some(user) = user {
        let rate_window_start = Utc::now() - Duration::hours(1);
        let user_id = user.user_id.clone();

        let requests_in_window = with_storage_traced(
            &app_state.shared_storage,
            "auth_forgot_password_rate_limit",
            "/api/auth/forgot-password",
            "POST",
            move |storage| {
                storage
                    .count_recent_reset_requests(&user_id, rate_window_start)
                    .map_err(Box::<dyn std::error::Error>::from)
            },
        )
        .map_err(map_storage_lock_error)?;

        if requests_in_window >= password_reset_rate_limit_per_hour() {
            return Err((
                StatusCode::TOO_MANY_REQUESTS,
                Json(json!({
                    "error": "Too many password reset requests. Please try again later."
                })),
            ));
        }

        let (token_record, plaintext_token) =
            PasswordResetToken::new(user.user_id.clone(), None, None);

        let token_clone = token_record.clone();
        with_storage_traced(
            &app_state.shared_storage,
            "auth_forgot_password_store_token",
            "/api/auth/forgot-password",
            "POST",
            move |storage| {
                storage
                    .store_password_reset_token(&token_clone)
                    .map_err(Box::<dyn std::error::Error>::from)
            },
        )
        .map_err(map_storage_lock_error)?;

        // Send email if SendGrid is configured, otherwise log token for development
        if crate::email_service::EmailConfig::is_enabled() {
            // Send email via SendGrid
            match crate::email_service::send_password_reset_email(
                &user.email,
                &user.username,
                &plaintext_token,
            )
            .await
            {
                Ok(()) => {
                    info!(
                        "Password reset email sent successfully to {} (user: {})",
                        user.email, user.username
                    );
                }
                Err(e) => {
                    // Log error but don't reveal it to user (prevent enumeration)
                    error!(
                        "Failed to send password reset email to {}: {}",
                        user.email, e
                    );
                    // Fall back to logging token in development mode
                    if should_log_reset_token() {
                        warn!(
                            "Email failed - Password reset token for user {} ({}): {}",
                            user.username, user.user_id, plaintext_token
                        );
                    }
                }
            }
        } else if should_log_reset_token() {
            // Development mode: log token to console
            info!(
                "Password reset token generated for user {} ({}): {}",
                user.username, user.user_id, plaintext_token
            );
        }
    }

    Ok(Json(ForgotPasswordResponse {
        message: "If the account exists, password reset instructions have been sent.".to_string(),
    }))
}

#[instrument(skip(app_state, payload))]
async fn reset_password(
    State((_auth, app_state)): State<(Arc<AuthState>, Arc<AppState>)>,
    Json(payload): Json<ResetPasswordRequest>,
) -> Result<Json<ResetPasswordResponse>, (StatusCode, Json<Value>)> {
    let ResetPasswordRequest {
        token,
        new_password,
    } = payload;

    let trimmed_token = token.trim();
    if trimmed_token.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Password reset token is required"})),
        ));
    }

    let sanitized_password = new_password.trim().to_string();
    if let Err(msg) = validate_password_complexity(&sanitized_password) {
        return Err((StatusCode::BAD_REQUEST, Json(json!({ "error": msg }))));
    }

    let token_bytes = hex::decode(trimmed_token).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid password reset token"})),
        )
    })?;
    let token_hash = blake3::hash(&token_bytes).to_hex().to_string();
    let token_hash_clone = token_hash.clone();

    let token_record = with_storage_traced(
        &app_state.shared_storage,
        "auth_reset_password_get_token",
        "/api/auth/reset-password",
        "POST",
        move |storage| {
            storage
                .get_password_reset_token_by_hash(&token_hash_clone)
                .map_err(Box::<dyn std::error::Error>::from)
        },
    )
    .map_err(map_storage_lock_error)?
    .ok_or((
        StatusCode::BAD_REQUEST,
        Json(json!({"error": "Invalid or expired password reset token"})),
    ))?;

    if token_record.is_expired() || token_record.is_used() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid or expired password reset token"})),
        ));
    }

    let user_id = token_record.user_id.clone();
    let user = with_storage_traced(
        &app_state.shared_storage,
        "auth_reset_password_get_user",
        "/api/auth/reset-password",
        "POST",
        move |storage| {
            storage
                .get_user_account(&user_id)
                .map_err(Box::<dyn std::error::Error>::from)
        },
    )
    .map_err(map_storage_lock_error)?
    .ok_or((
        StatusCode::BAD_REQUEST,
        Json(json!({"error": "Account associated with this token was not found"})),
    ))?;

    let hashed_password = hash(&sanitized_password, DEFAULT_COST).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to process password"})),
        )
    })?;

    let mut updated_user = user.clone();
    updated_user.password_hash = hashed_password;
    updated_user.updated_at = Utc::now();

    let token_id = token_record.token_id.clone();
    let user_clone = updated_user.clone();

    with_storage_traced(
        &app_state.shared_storage,
        "auth_reset_password_apply",
        "/api/auth/reset-password",
        "POST",
        move |storage| {
            storage
                .update_user_account(&user_clone)
                .map_err(Box::<dyn std::error::Error>::from)?;
            storage
                .mark_token_as_used(&token_id)
                .map_err(Box::<dyn std::error::Error>::from)?;
            Ok(())
        },
    )
    .map_err(map_storage_lock_error)?;

    info!("Password reset completed for user {}", updated_user.user_id);

    Ok(Json(ResetPasswordResponse {
        message: "Password reset successful. You can now log in with your new password."
            .to_string(),
    }))
}

#[instrument(skip(auth, app_state, payload), fields(username = %payload.username))]
async fn login(
    State((auth, app_state)): State<(Arc<AuthState>, Arc<AppState>)>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, Response> {
    let request_start = std::time::Instant::now();

    // Get user by username using traced storage helper for structured error logging
    let user = with_storage_traced(
        &app_state.shared_storage,
        "auth_login_get_user",
        "/api/auth/login",
        "POST",
        |storage| Ok(storage.get_user_by_username(&payload.username)?),
    )
    .map_err(|e| match e {
        StorageLockError::Timeout => {
            svc_unavailable_retry("Service temporarily busy, please retry")
        }
        StorageLockError::Other(msg) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": msg})),
        )
            .into_response(),
    })?;

    if let Some(user) = user {
        // Verify password (bcrypt is CPU intensive - must be done WITHOUT holding the lock!)
        let bcrypt_start = std::time::Instant::now();
        let password_valid = verify(&payload.password, &user.password_hash).unwrap_or(false);
        let bcrypt_duration = bcrypt_start.elapsed();
        info!("Bcrypt verification took: {:?}", bcrypt_duration);

        if password_valid {
            // Check account status
            match user.status {
                AccountStatus::Suspended => {
                    return Err((
                        StatusCode::FORBIDDEN,
                        Json(
                            json!({"error": "Your account has been suspended. Please contact an administrator."}),
                        ),
                    ).into_response());
                }
                AccountStatus::Banned => {
                    return Err((
                        StatusCode::FORBIDDEN,
                        Json(
                            json!({"error": "Your account has been banned. Please contact an administrator."}),
                        ),
                    ).into_response());
                }
                AccountStatus::PendingVerification => {
                    return Err((
                        StatusCode::FORBIDDEN,
                        Json(
                            json!({"error": "Your account is pending verification. Please check your email."}),
                        ),
                    ).into_response());
                }
                AccountStatus::TrialExpired => {
                    return Err((
                        StatusCode::FORBIDDEN,
                        Json(
                            json!({"error": "Your trial has expired. Please upgrade your account."}),
                        ),
                    ).into_response());
                }
                AccountStatus::Active => {
                    // Account is active, proceed with login
                }
            }

            // Generate token with actual user_id
            let token = auth
                .generate_token(&user.user_id, user.workspace_id.clone())
                .map_err(|_| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": "Failed to generate token"})),
                    )
                        .into_response()
                })?;

            let expires_at = Utc::now()
                .checked_add_signed(Duration::hours(24))
                .expect("valid timestamp")
                .timestamp();

            let total_duration = request_start.elapsed();
            info!(
                endpoint = "/api/auth/login",
                method = "POST",
                user_id = %user.user_id,
                duration_ms = total_duration.as_millis(),
                "Login successful"
            );

            return Ok(Json(AuthResponse {
                token,
                user_id: user.user_id,
                workspace_id: user.workspace_id,
                expires_at,
            }));
        } else {
            warn!(
                "Login failed: invalid password for user {}",
                payload.username
            );
        }
    } else {
        warn!("Login failed: user not found: {}", payload.username);
    }

    Err((
        StatusCode::UNAUTHORIZED,
        Json(json!({"error": "Invalid credentials"})),
    )
        .into_response())
}

#[instrument(skip(auth, app_state, payload), fields(username = %payload.username, email = %payload.email))]
async fn register(
    State((auth, app_state)): State<(Arc<AuthState>, Arc<AppState>)>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, Response> {
    // Validate password complexity
    if let Err(e) = validate_password_complexity(&payload.password) {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": e}))).into_response());
    }

    // Check for existing username/email using traced storage helper for structured error logging
    with_storage_traced(
        &app_state.shared_storage,
        "auth_register_check_existing",
        "/api/auth/register",
        "POST",
        |storage| {
            // Check if username already exists
            if storage.get_user_by_username(&payload.username)?.is_some() {
                return Err("Username already exists".into());
            }

            // Check if email already exists
            if storage.get_user_by_email(&payload.email)?.is_some() {
                return Err("Email already exists".into());
            }

            Ok(())
        },
    )
    .map_err(|e| match e {
        StorageLockError::Timeout => {
            svc_unavailable_retry("Service temporarily busy, please retry")
        }
        StorageLockError::Other(msg) => {
            // Check if it's a conflict error
            if msg.contains("already exists") {
                (StatusCode::CONFLICT, Json(json!({"error": msg}))).into_response()
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": msg})),
                )
                    .into_response()
            }
        }
    })?;

    // Hash password (CPU intensive - must be done WITHOUT holding the lock!)
    let bcrypt_start = std::time::Instant::now();
    let password_hash = hash(&payload.password, DEFAULT_COST).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to hash password"})),
        )
            .into_response()
    })?;
    let bcrypt_duration = bcrypt_start.elapsed();
    info!("Bcrypt hash generation took: {:?}", bcrypt_duration);

    // Create new user account
    let user_id = format!("user-{}", Uuid::new_v4());
    let workspace_id = payload
        .workspace_name
        .as_ref()
        .map(|name| format!("{name}-workspace"))
        .or_else(|| Some(format!("{}-workspace", payload.username)));

    let new_user = UserAccount {
        user_id: user_id.clone(),
        username: payload.username.clone(),
        email: payload.email.clone(),
        password_hash,
        tier: UserTier::Basic,
        status: AccountStatus::Active,
        credits: 100, // Starting credits for Basic tier
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_login: None,
        subscription: None,
        limits: TierLimits::for_tier(&UserTier::Basic),
        is_admin: false,
        workspace_id: workspace_id.clone(),
        available_adapters: None, // Use tier defaults
    };

    // Store user account and initial credit using non-blocking storage helper
    let initial_credit_transaction = CreditTransaction {
        transaction_id: Uuid::new_v4().to_string(),
        user_id: user_id.clone(),
        amount: 100,
        transaction_type: CreditTransactionType::Grant,
        description: "New user registration bonus".to_string(),
        operation_type: Some("registration".to_string()),
        operation_id: Some(user_id.clone()),
        timestamp: Utc::now(),
        balance_after: 100,
    };

    let new_user_clone = new_user.clone();
    let credit_tx_clone = initial_credit_transaction.clone();
    with_storage_traced(
        &app_state.shared_storage,
        "auth_register_store_user",
        "/api/auth/register",
        "POST",
        move |storage| {
            storage.store_user_account(&new_user_clone)?;
            storage.record_credit_transaction(&credit_tx_clone)?;
            Ok(())
        },
    )
    .map_err(|e| match e {
        StorageLockError::Timeout => {
            svc_unavailable_retry("Service temporarily busy, please retry")
        }
        StorageLockError::Other(msg) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": msg})),
        )
            .into_response(),
    })?;

    // PostgreSQL persistence happens asynchronously in background
    // to avoid Send/Sync issues with the handler
    let pg = app_state.postgres_persistence.clone();
    let user_for_pg = new_user.clone();
    tokio::spawn(async move {
        let pg_lock = pg.read().await;
        if let Some(pg_instance) = &*pg_lock {
            if let Err(e) = pg_instance.persist_user(&user_for_pg).await {
                tracing::warn!("Failed to persist user to PostgreSQL: {}", e);
            } else {
                tracing::info!("User {} persisted to PostgreSQL", user_for_pg.username);
            }
        }
    });

    let token = auth
        .generate_token(&user_id, workspace_id.clone())
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to generate token"})),
            )
                .into_response()
        })?;

    let expires_at = Utc::now()
        .checked_add_signed(Duration::hours(24))
        .expect("valid timestamp")
        .timestamp();

    info!(
        "Registration successful for user: {} ({})",
        user_id, payload.username
    );
    Ok(Json(AuthResponse {
        token,
        user_id,
        workspace_id,
        expires_at,
    }))
}

async fn get_profile(
    State((_auth, app_state)): State<(Arc<AuthState>, Arc<AppState>)>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<UserProfile>, (StatusCode, Json<Value>)> {
    // Extract user_id from JWT Claims injected by jwt_auth_middleware
    let user_id = claims.user_id.clone();

    let user_opt = with_storage(&app_state.shared_storage, "auth_get_profile", |storage| {
        Ok(storage.get_user_account(&user_id)?)
    })
    .map_err(|e| match e {
        StorageLockError::Timeout => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "Service temporarily busy, please retry"})),
        ),
        StorageLockError::Other(msg) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": msg})),
        ),
    })?;

    if let Some(user) = user_opt {
        return Ok(Json(UserProfile {
            user_id: user.user_id,
            username: user.username,
            email: user.email,
            created_at: user.created_at.timestamp(),
            workspace_id: user.workspace_id,
        }));
    }

    Err((
        StatusCode::NOT_FOUND,
        Json(json!({"error": "User not found"})),
    ))
}

async fn refresh_token(
    State((auth, app_state)): State<(Arc<AuthState>, Arc<AppState>)>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<Value>)> {
    // Extract user_id from JWT Claims injected by jwt_auth_middleware
    let user_id = claims.user_id.clone();

    let user_opt = with_storage(&app_state.shared_storage, "auth_refresh_token", |storage| {
        Ok(storage.get_user_account(&user_id)?)
    })
    .map_err(|e| match e {
        StorageLockError::Timeout => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "Service temporarily busy, please retry"})),
        ),
        StorageLockError::Other(msg) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": msg})),
        ),
    })?;

    if let Some(user) = user_opt {
        let token = auth
            .generate_token(&user.user_id, user.workspace_id.clone())
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": "Failed to generate token"})),
                )
            })?;

        let expires_at = Utc::now()
            .checked_add_signed(Duration::hours(24))
            .expect("valid timestamp")
            .timestamp();

        return Ok(Json(AuthResponse {
            token,
            user_id: user.user_id,
            workspace_id: user.workspace_id,
            expires_at,
        }));
    }

    Err((
        StatusCode::NOT_FOUND,
        Json(json!({"error": "User not found"})),
    ))
}

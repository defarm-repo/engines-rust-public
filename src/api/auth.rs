use crate::api::shared_state::AppState;
use crate::auth_middleware::jwt_auth_middleware;
use crate::storage::StorageBackend;
use crate::types::{
    AccountStatus, CreditTransaction, CreditTransactionType, TierLimits, UserAccount, UserTier,
};
use axum::{
    extract::{Extension, State},
    http::StatusCode,
    middleware,
    response::Json,
    routing::{get, post},
    Router,
};
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
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

// Auth state contains JWT secret
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

pub fn auth_routes(app_state: Arc<AppState>) -> Router {
    // Create AuthState using the shared JWT secret from AppState
    let auth_state = Arc::new(AuthState {
        jwt_secret: app_state.jwt_secret.clone(),
    });

    // Unauthenticated routes
    let public_routes = Router::new()
        .route("/login", post(login))
        .route("/register", post(register)); // Active but hidden from public docs

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

async fn login(
    State((auth, app_state)): State<(Arc<AuthState>, Arc<AppState>)>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<Value>)> {
    // Get user by username from shared storage
    let storage = app_state.shared_storage.lock().unwrap();

    if let Some(user) = storage
        .get_user_by_username(&payload.username)
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        })?
    {
        // Verify password
        if verify(&payload.password, &user.password_hash).unwrap_or(false) {
            // Check account status
            match user.status {
                AccountStatus::Suspended => {
                    return Err((
                        StatusCode::FORBIDDEN,
                        Json(
                            json!({"error": "Your account has been suspended. Please contact an administrator."}),
                        ),
                    ));
                }
                AccountStatus::Banned => {
                    return Err((
                        StatusCode::FORBIDDEN,
                        Json(
                            json!({"error": "Your account has been banned. Please contact an administrator."}),
                        ),
                    ));
                }
                AccountStatus::PendingVerification => {
                    return Err((
                        StatusCode::FORBIDDEN,
                        Json(
                            json!({"error": "Your account is pending verification. Please check your email."}),
                        ),
                    ));
                }
                AccountStatus::TrialExpired => {
                    return Err((
                        StatusCode::FORBIDDEN,
                        Json(
                            json!({"error": "Your trial has expired. Please upgrade your account."}),
                        ),
                    ));
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
    }

    Err((
        StatusCode::UNAUTHORIZED,
        Json(json!({"error": "Invalid credentials"})),
    ))
}

async fn register(
    State((auth, app_state)): State<(Arc<AuthState>, Arc<AppState>)>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<Value>)> {
    // Validate password complexity
    if let Err(e) = validate_password_complexity(&payload.password) {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": e}))));
    }

    let mut storage = app_state.shared_storage.lock().unwrap();

    // Check if username already exists
    if storage
        .get_user_by_username(&payload.username)
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        })?
        .is_some()
    {
        return Err((
            StatusCode::CONFLICT,
            Json(json!({"error": "Username already exists"})),
        ));
    }

    // Check if email already exists
    if storage
        .get_user_by_email(&payload.email)
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        })?
        .is_some()
    {
        return Err((
            StatusCode::CONFLICT,
            Json(json!({"error": "Email already exists"})),
        ));
    }

    let password_hash = hash(&payload.password, DEFAULT_COST).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to hash password"})),
        )
    })?;

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

    // Store user account
    storage.store_user_account(&new_user).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
    })?;

    // Record initial credit grant
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

    storage
        .record_credit_transaction(&initial_credit_transaction)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
        })?;

    drop(storage); // Release the lock

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
        })?;

    let expires_at = Utc::now()
        .checked_add_signed(Duration::hours(24))
        .expect("valid timestamp")
        .timestamp();

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
    let user_id = &claims.user_id;

    let storage = app_state.shared_storage.lock().unwrap();

    if let Some(user) = storage.get_user_account(user_id).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"})),
        )
    })? {
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
    let user_id = &claims.user_id;

    let storage = app_state.shared_storage.lock().unwrap();

    if let Some(user) = storage.get_user_account(user_id).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"})),
        )
    })? {
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

use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{post, get},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use jsonwebtoken::{encode, decode, Header, EncodingKey, DecodingKey, Validation};
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{Utc, Duration};

#[derive(Debug, Serialize, Deserialize)]
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

// In a real application, this would be stored in a database
// For demo purposes, we'll use a simple in-memory store
pub struct AuthState {
    pub jwt_secret: String,
    // In production, use a proper database
    pub users: std::sync::Mutex<std::collections::HashMap<String, (String, String, String, i64)>>, // user_id -> (username, password_hash, email, created_at)
}

impl AuthState {
    pub fn new() -> Self {
        let jwt_secret = std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| "your-secret-key-change-in-production".to_string());

        let mut users = std::collections::HashMap::new();

        // Add demo users
        let demo_password = hash("demo123", DEFAULT_COST).unwrap();
        users.insert("hen".to_string(), ("hen".to_string(), demo_password.clone(), "hen@defarm.io".to_string(), Utc::now().timestamp()));
        users.insert("pullet".to_string(), ("pullet".to_string(), demo_password.clone(), "pullet@defarm.io".to_string(), Utc::now().timestamp()));
        users.insert("cock".to_string(), ("cock".to_string(), demo_password, "cock@defarm.io".to_string(), Utc::now().timestamp()));

        Self {
            jwt_secret,
            users: std::sync::Mutex::new(users),
        }
    }

    pub fn generate_token(&self, user_id: &str, workspace_id: Option<String>) -> Result<String, jsonwebtoken::errors::Error> {
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
        ).map(|data| data.claims)
    }
}

pub fn auth_routes() -> Router {
    let auth_state = Arc::new(AuthState::new());

    Router::new()
        .route("/login", post(login))
        .route("/register", post(register))
        .route("/profile", get(get_profile))
        .route("/refresh", post(refresh_token))
        .with_state(auth_state)
}

async fn login(
    State(auth): State<Arc<AuthState>>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<Value>)> {
    let users = auth.users.lock().unwrap();

    if let Some((username, password_hash, _email, _created_at)) = users.get(&payload.username) {
        if verify(&payload.password, password_hash).unwrap_or(false) {
            let token = auth.generate_token(&payload.username, payload.workspace_id.clone())
                .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to generate token"}))))?;

            let expires_at = Utc::now()
                .checked_add_signed(Duration::hours(24))
                .expect("valid timestamp")
                .timestamp();

            return Ok(Json(AuthResponse {
                token,
                user_id: payload.username.clone(),
                workspace_id: payload.workspace_id,
                expires_at,
            }));
        }
    }

    Err((StatusCode::UNAUTHORIZED, Json(json!({"error": "Invalid credentials"}))))
}

async fn register(
    State(auth): State<Arc<AuthState>>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<Value>)> {
    let mut users = auth.users.lock().unwrap();

    if users.contains_key(&payload.username) {
        return Err((StatusCode::CONFLICT, Json(json!({"error": "Username already exists"}))));
    }

    let password_hash = hash(&payload.password, DEFAULT_COST)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to hash password"}))))?;

    let created_at = Utc::now().timestamp();
    users.insert(
        payload.username.clone(),
        (payload.username.clone(), password_hash, payload.email.clone(), created_at)
    );

    drop(users); // Release the lock

    let token = auth.generate_token(&payload.username, None)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to generate token"}))))?;

    let expires_at = Utc::now()
        .checked_add_signed(Duration::hours(24))
        .expect("valid timestamp")
        .timestamp();

    Ok(Json(AuthResponse {
        token,
        user_id: payload.username.clone(),
        workspace_id: None,
        expires_at,
    }))
}

async fn get_profile(
    State(auth): State<Arc<AuthState>>,
    // In a real implementation, extract JWT token from headers
) -> Result<Json<UserProfile>, (StatusCode, Json<Value>)> {
    // For demo purposes, return a sample profile
    // In production, extract user from JWT token
    Ok(Json(UserProfile {
        user_id: "demo_user".to_string(),
        username: "demo_user".to_string(),
        email: "demo@defarm.io".to_string(),
        created_at: Utc::now().timestamp(),
        workspace_id: None,
    }))
}

async fn refresh_token(
    State(auth): State<Arc<AuthState>>,
    // In a real implementation, extract JWT token from headers
) -> Result<Json<AuthResponse>, (StatusCode, Json<Value>)> {
    // For demo purposes, generate a new token
    // In production, verify existing token and generate new one
    let token = auth.generate_token("demo_user", None)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to generate token"}))))?;

    let expires_at = Utc::now()
        .checked_add_signed(Duration::hours(24))
        .expect("valid timestamp")
        .timestamp();

    Ok(Json(AuthResponse {
        token,
        user_id: "demo_user".to_string(),
        workspace_id: None,
        expires_at,
    }))
}
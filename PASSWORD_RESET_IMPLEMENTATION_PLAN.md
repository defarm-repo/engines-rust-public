# Password Reset Implementation Plan

## Status: Database Schema Created ✅

### Completed
1. ✅ Created migration `V5__password_reset_tokens.sql` with:
   - `password_reset_tokens` table
   - Indexes for performance and cleanup
   - Foreign key to users table
   - Fields: token_id, user_id, token_hash, created_at, expires_at, used_at, ip_address, user_agent

### Remaining Implementation Steps

#### 1. Add Password Reset Types (src/types.rs)
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordResetToken {
    pub token_id: String,
    pub user_id: String,
    pub token_hash: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}
```

#### 2. Add Storage Backend Methods (src/storage.rs)
```rust
pub trait StorageBackend {
    // ... existing methods ...

    fn store_password_reset_token(&self, token: &PasswordResetToken) -> Result<(), String>;
    fn get_password_reset_token_by_hash(&self, token_hash: &str) -> Result<Option<PasswordResetToken>, String>;
    fn mark_token_as_used(&self, token_id: &str) -> Result<(), String>;
    fn count_recent_reset_requests(&self, user_id: &str, since: DateTime<Utc>) -> Result<usize, String>;
    fn cleanup_expired_tokens(&self) -> Result<usize, String>;
}
```

#### 3. Add PostgreSQL Persistence Methods (src/postgres_persistence.rs)
Implement the storage methods with proper SQL queries.

#### 4. Add API Endpoints (src/api/auth.rs)

**POST /api/auth/forgot-password**
- Request: `{"email": "user@example.com"}` or `{"username": "username"}`
- Generate secure random token (32 bytes)
- Hash with BLAKE3
- Store in database with 30-minute expiration
- **Development**: Log token to console
- **Production**: Send email (future implementation)
- Rate limit: 3 requests per hour per user
- Response: `{"message": "If account exists, reset instructions sent"}`

**POST /api/auth/reset-password**
- Request: `{"token": "...", "new_password": "..."}`
- Validate token exists and not expired
- Validate token not already used
- Validate password complexity
- Hash new password with bcrypt
- Update user password
- Mark token as used
- Response: `{"message": "Password reset successful"}`

#### 5. Rate Limiting Implementation
- Check last 3 requests in past hour before creating new token
- Return 429 Too Many Requests if limit exceeded
- Include Retry-After header

#### 6. Security Considerations
- ✅ Tokens hashed with BLAKE3 before storage
- ✅ 30-minute expiration window
- ✅ One-time use only
- ✅ Rate limiting (3/hour per user)
- Constant-time comparison for token validation
- Clear error messages don't reveal if email/username exists
- IP address logging for audit trail

#### 7. Cleanup Job
Add background task to periodically clean up expired tokens:
```rust
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(3600)); // Every hour
    loop {
        interval.tick().await;
        if let Some(pg) = &*postgres.read().await {
            let _ = pg.cleanup_expired_password_reset_tokens().await;
        }
    }
});
```

### Testing Plan
1. Test forgot password with valid email/username
2. Test forgot password with non-existent user (same response)
3. Test reset with valid token
4. Test reset with expired token
5. Test reset with already-used token
6. Test reset with invalid token
7. Test rate limiting (4th request in hour should fail)
8. Test password complexity validation

### Files to Modify
- ✅ `config/migrations/V5__password_reset_tokens.sql` - Created
- `src/types.rs` - Add PasswordResetToken struct
- `src/storage.rs` - Add trait methods
- `src/in_memory_storage.rs` - Implement for in-memory backend
- `src/postgres_persistence.rs` - Implement for PostgreSQL
- `src/api/auth.rs` - Add endpoints and route them
- `src/main.rs` - Add cleanup background task

### Environment Variables (Optional)
```
PASSWORD_RESET_TOKEN_EXPIRY_MINUTES=30  # Default: 30
PASSWORD_RESET_RATE_LIMIT_PER_HOUR=3    # Default: 3
SMTP_HOST=smtp.example.com              # For future email implementation
SMTP_PORT=587
SMTP_USERNAME=noreply@defarm.net
SMTP_PASSWORD=...
```

### Demo Data Issue (Separate Task)
The demo circuits are created but items fail to push due to identifier format mismatch:
- Circuit expects: `{"key": "sisbov", "value": "..."}`
- JSON provides: `{"key": "suino:sisbov", "value": "..."}`

This needs the demo JSON to be updated to match circuit alias configuration format.

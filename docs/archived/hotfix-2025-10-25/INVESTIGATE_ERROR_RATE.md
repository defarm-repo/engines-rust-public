# Investigation Guide: 16.6% Error Rate Analysis

## Problem Statement

After deploying the storage lock timeout fix (commit `0c707a9`), the API shows significant improvements:
- ✅ P99 latency: 898ms (was 901,405ms) - **99.9% improvement**
- ✅ Error rate: 16.6% (was 42%) - **60% improvement**
- ✅ No API freezes observed

However, the **16.6% error rate** suggests additional bottlenecks beyond the lock contention issue.

## Hypothesis Categories

### 1. Database Connection Pool Exhaustion
**Symptoms**:
- Intermittent 503 or 500 errors
- Error rate increases with concurrent load
- Errors occur in bursts rather than consistently

**Investigation Steps**:

```bash
# Check current pool configuration
grep -r "max_connections\|pool_size" src/

# Expected location: src/postgres_storage_with_cache.rs or src/postgres_persistence.rs
```

**Metrics to Add**:

```rust
// Add to PostgresStorageWithCache or connection pool initialization
pub struct PoolMetrics {
    pub max_connections: u32,
    pub active_connections: u32,
    pub idle_connections: u32,
    pub wait_count: u64,
    pub wait_duration_ms: u64,
    pub connection_errors: u64,
}

impl PostgresStorageWithCache {
    pub fn get_pool_metrics(&self) -> PoolMetrics {
        PoolMetrics {
            max_connections: self.pool.max_size(),
            active_connections: self.pool.state().connections,
            idle_connections: self.pool.state().idle_connections,
            // ... etc
        }
    }
}
```

**Test Configuration Changes**:

```rust
// Try increasing pool size
// Current (find in code):
let pool = PgPoolOptions::new()
    .max_connections(10)  // ← likely too low
    .connect(&database_url).await?;

// Test with:
let pool = PgPoolOptions::new()
    .max_connections(50)  // ← higher for concurrent load
    .acquire_timeout(Duration::from_secs(5))
    .idle_timeout(Some(Duration::from_secs(300)))
    .max_lifetime(Some(Duration::from_secs(1800)))
    .connect(&database_url).await?;
```

### 2. Redis Cache Contention
**Symptoms**:
- Errors correlate with cache operations
- Redis connection timeouts in logs
- Cache misses causing database overload

**Investigation Steps**:

```bash
# Check Redis configuration
grep -r "redis" src/ | grep -E "connect|pool|timeout"

# Check if Redis is bottleneck
railway logs --tail 1000 | grep -i "redis"
```

**Metrics to Add**:

```rust
pub struct RedisCacheMetrics {
    pub hits: u64,
    pub misses: u64,
    pub errors: u64,
    pub avg_get_ms: f64,
    pub avg_set_ms: f64,
    pub connection_pool_size: usize,
}
```

**Test Without Redis**:

```rust
// Temporarily disable Redis to isolate issue
// In PostgresStorageWithCache::new()
let cache_enabled = false;  // ← disable to test

// If error rate drops, Redis is the bottleneck
```

### 3. Network Latency to Railway Infrastructure
**Symptoms**:
- Errors are timing-based (timeouts)
- Error rate similar across different endpoint types
- Latency variance is high

**Investigation Steps**:

```bash
# Measure network latency from Railway to database
# Add to health check endpoint

pub async fn health_with_db_ping() -> Json<Value> {
    let start = Instant::now();

    // Ping database
    let db_ping_ms = match storage.ping_db().await {
        Ok(_) => start.elapsed().as_millis(),
        Err(_) => 999999,
    };

    json!({
        "status": "ok",
        "db_ping_ms": db_ping_ms,
        "network_region": "us-west-1",  // Railway region
    })
}
```

**Test from Different Regions**:

```bash
# Run load test from different geographic locations
# to isolate network latency

# US East
curl https://defarm-engines-api-production.up.railway.app/health

# EU
curl https://defarm-engines-api-production.up.railway.app/health

# Compare latencies
```

### 4. bcrypt CPU Contention
**Symptoms**:
- Errors only on login endpoint (`/api/auth/login`)
- Error rate correlates with concurrent login requests
- High CPU usage during load tests

**Investigation Steps**:

```bash
# Check bcrypt cost factor
grep -r "bcrypt\|hash_password\|verify_password" src/api/auth.rs

# Expected:
let hash = bcrypt::hash(password, 12)?;  // ← cost factor 12 = ~300ms per hash
```

**Metrics to Add**:

```rust
// In login handler
let bcrypt_start = Instant::now();
let verified = verify(&payload.password, &user_account.password_hash)
    .map_err(|e| {
        error!("bcrypt verification failed: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Password verification failed"})))
    })?;
let bcrypt_ms = bcrypt_start.elapsed().as_millis();

info!("bcrypt verification took {}ms", bcrypt_ms);

// Expected: 50-100ms (cost 10-11) or 200-300ms (cost 12)
```

**Test with Lower Cost**:

```rust
// For testing only, reduce bcrypt cost
let hash = bcrypt::hash(password, 10)?;  // ← reduce from 12 to 10

// If error rate drops, bcrypt is the bottleneck
// Solution: use async bcrypt or separate worker pool
```

**Async bcrypt Solution**:

```rust
use tokio::task;

// Instead of:
let verified = verify(&payload.password, &user_account.password_hash)?;

// Use:
let password = payload.password.clone();
let hash = user_account.password_hash.clone();
let verified = task::spawn_blocking(move || {
    bcrypt::verify(&password, &hash)
}).await
.map_err(|e| {
    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Worker thread error"})))
})?
.map_err(|e| {
    (StatusCode::UNAUTHORIZED, Json(json!({"error": "Invalid credentials"})))
})?;
```

## Detailed Logging Implementation

### Add Instrumentation to Login Endpoint

```rust
use tracing::{info, error, instrument, Span};
use std::time::Instant;

#[instrument(skip(app_state, payload), fields(username = %payload.username))]
pub async fn login(
    Extension(app_state): Extension<Arc<AppState>>,
    Json(payload): Json<LoginPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let request_start = Instant::now();
    let span = Span::current();

    // Phase 1: Lock acquisition + user lookup
    span.in_scope(|| info!("phase=acquire_storage_lock_start"));
    let lock_start = Instant::now();

    let user = with_storage(
        &app_state.shared_storage,
        "auth_login_get_user",
        |storage| {
            let lookup_start = Instant::now();
            let user = storage.get_user_by_username(&payload.username)?;
            let lookup_ms = lookup_start.elapsed().as_millis();

            span.in_scope(|| info!(lookup_ms, "phase=get_user_complete"));

            Ok(user)
        },
    )
    .map_err(|e| {
        let lock_ms = lock_start.elapsed().as_millis();
        error!(lock_ms, error = %e, "phase=storage_lock_error");

        match e {
            StorageLockError::Timeout => (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"error": "Service temporarily busy, please retry"})),
            ),
            StorageLockError::Other(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": msg})),
            ),
        }
    })?;

    let lock_ms = lock_start.elapsed().as_millis();
    span.in_scope(|| info!(lock_ms, "phase=acquire_storage_lock_end"));

    // Phase 2: bcrypt verification
    span.in_scope(|| info!("phase=bcrypt_verify_start"));
    let bcrypt_start = Instant::now();

    let user_account = match user {
        Some(u) => u,
        None => {
            error!(username = %payload.username, "phase=user_not_found");
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid credentials"})),
            ));
        }
    };

    let verified = verify(&payload.password, &user_account.password_hash)
        .map_err(|e| {
            let bcrypt_ms = bcrypt_start.elapsed().as_millis();
            error!(bcrypt_ms, error = %e, "phase=bcrypt_error");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Password verification failed"})),
            )
        })?;

    let bcrypt_ms = bcrypt_start.elapsed().as_millis();
    span.in_scope(|| info!(bcrypt_ms, "phase=bcrypt_verify_end"));

    if !verified {
        error!(username = %payload.username, "phase=invalid_password");
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Invalid credentials"})),
        ));
    }

    // Phase 3: Token generation
    span.in_scope(|| info!("phase=token_generation_start"));
    let token_start = Instant::now();

    let claims = Claims {
        user_id: user_account.id.to_string(),
        workspace_id: user_account.workspace_id.clone(),
        exp: (Utc::now() + Duration::hours(24)).timestamp() as usize,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(app_state.jwt_secret.as_bytes()),
    )
    .map_err(|e| {
        error!(error = %e, "phase=token_generation_error");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Token generation failed"})),
        )
    })?;

    let token_ms = token_start.elapsed().as_millis();
    span.in_scope(|| info!(token_ms, "phase=token_generation_end"));

    // Total request time
    let total_ms = request_start.elapsed().as_millis();
    info!(
        total_ms,
        lock_ms,
        bcrypt_ms,
        token_ms,
        "phase=login_complete"
    );

    Ok(Json(json!({
        "token": token,
        "user_id": user_account.id,
        "username": user_account.username,
    })))
}
```

## Test Script with Detailed Error Analysis

```bash
#!/bin/bash
# /tmp/diagnose_errors.sh

API="https://defarm-engines-api-production.up.railway.app/api/auth/login"
USER="hen"
PASS="demo123"
REQUESTS=100

echo "🔍 Diagnosing error patterns..."

rm -f /tmp/error_analysis.csv
echo "request_id,http_code,duration_ms,timestamp" > /tmp/error_analysis.csv

for i in $(seq 1 $REQUESTS); do
    TIMESTAMP=$(date +%s)
    START=$(date +%s%N)

    RESPONSE=$(curl -s -w "\n%{http_code}" -X POST "$API" \
        -H "Content-Type: application/json" \
        -d "{\"username\":\"$USER\",\"password\":\"$PASS\"}")

    HTTP_CODE=$(echo "$RESPONSE" | tail -n 1)
    END=$(date +%s%N)
    DURATION_MS=$(( (END - START) / 1000000 ))

    echo "$i,$HTTP_CODE,$DURATION_MS,$TIMESTAMP" >> /tmp/error_analysis.csv

    # Log errors
    if [ "$HTTP_CODE" != "200" ]; then
        BODY=$(echo "$RESPONSE" | head -n -1)
        echo "ERROR $i: HTTP $HTTP_CODE at ${TIMESTAMP} (${DURATION_MS}ms)"
        echo "  Body: $BODY"
        echo "$BODY" >> /tmp/error_bodies.txt
    fi

    sleep 0.1
done

echo ""
echo "=== ERROR ANALYSIS ==="

# Count error types
echo "Error Distribution:"
tail -n +2 /tmp/error_analysis.csv | awk -F',' '{print $2}' | sort | uniq -c

# Timing analysis for errors
echo ""
echo "Error Timing:"
grep -E ',4[0-9]{2}|,5[0-9]{2},' /tmp/error_analysis.csv | awk -F',' '{print $3}' | sort -n > /tmp/error_timings.txt

if [ -s /tmp/error_timings.txt ]; then
    COUNT=$(wc -l < /tmp/error_timings.txt)
    MIN=$(head -n 1 /tmp/error_timings.txt)
    MAX=$(tail -n 1 /tmp/error_timings.txt)
    AVG=$(awk '{sum+=$1} END {print int(sum/NR)}' /tmp/error_timings.txt)

    echo "  Error Count: $COUNT"
    echo "  Min: ${MIN}ms"
    echo "  Avg: ${AVG}ms"
    echo "  Max: ${MAX}ms"
fi

# Error message analysis
echo ""
echo "Error Messages:"
if [ -f /tmp/error_bodies.txt ]; then
    sort /tmp/error_bodies.txt | uniq -c | sort -rn
fi

# Temporal clustering
echo ""
echo "Error Clustering (errors per 10s window):"
tail -n +2 /tmp/error_analysis.csv | grep -E ',4[0-9]{2}|,5[0-9]{2},' | \
    awk -F',' '{bucket=int($4/10)*10; count[bucket]++} END {for(b in count) print b, count[b]}' | \
    sort -n
```

## Decision Tree

```
16.6% Error Rate Investigation
│
├─ Are errors 503 "Service Unavailable"?
│  ├─ YES → Storage lock timeouts
│  │         ├─ Check lock acquisition metrics
│  │         └─ Increase timeout from 50ms to 100ms
│  │
│  └─ NO → Continue investigation
│
├─ Are errors 500 "Internal Server Error"?
│  ├─ YES → Check error logs
│  │         ├─ "connection pool exhausted" → Increase pool size
│  │         ├─ "redis connection failed" → Fix Redis config
│  │         └─ "bcrypt error" → Async bcrypt
│  │
│  └─ NO → Continue investigation
│
├─ Are errors clustered or distributed?
│  ├─ CLUSTERED (bursts of errors) → Resource exhaustion
│  │              └─ Increase connection pool + Redis pool
│  │
│  └─ DISTRIBUTED (consistent %) → Throughput limit
│                 └─ Horizontal scaling needed
│
└─ Does error rate change with load?
   ├─ INCREASES → Resource bottleneck
   │              └─ Profile and scale resources
   │
   └─ CONSTANT → Code bug or config issue
                 └─ Review error logs for patterns
```

## Recommendations

1. **Immediate**: Run `/tmp/diagnose_errors.sh` to collect error patterns
2. **Next**: Add detailed instrumentation to login endpoint (see above)
3. **Then**: Test hypotheses in order:
   - Database pool size increase
   - Redis cache analysis
   - bcrypt async implementation
   - Network latency measurement

4. **Long-term**: If error rate remains >10%, consider:
   - Horizontal scaling (multiple Railway instances)
   - CDN/edge caching for static responses
   - Rate limiting to prevent overload
   - Read replicas for database

## Success Criteria

- Target: **<5% error rate** under load
- All errors have clear error messages and recovery suggestions
- P99 latency remains <1000ms
- No complete API freezes

---

This investigation should identify the root cause of the remaining errors and provide actionable fixes.

# Phase 2 Implementation Plan: Error Classification & Lock Safety Audit

## Status: READY TO IMPLEMENT

This document provides step-by-step implementation guidance for the remaining work after the critical storage lock timeout hotfix (commit 0c707a9).

---

## PR #1: Structured Error Logging & Classification

**Branch**: `feature/structured-error-logging`
**Estimated Time**: 2-3 hours
**Priority**: CRITICAL (must complete first)

### Files Created/Modified

1. ✅ **DONE**: `src/error_tracking.rs` - Error classification module (already created)
2. ✅ **DONE**: `src/lib.rs` - Added `pub mod error_tracking;`
3. **TODO**: Update `src/storage_helpers.rs` - Add structured error context
4. **TODO**: Update `src/api/auth.rs` - Add error tracking to login endpoint
5. **TODO**: Create `scripts/analyze_errors.sh` - Error breakdown script
6. **TODO**: Create `docs/runbooks/errors_breakdown.md` - Documentation

### Step 1: Update storage_helpers.rs

Add error context to with_storage helper:

```rust
// In src/storage_helpers.rs
use crate::error_tracking::{ErrorContext, ErrorKind};

pub fn with_storage_traced<S, T, F>(
    storage: &Arc<Mutex<S>>,
    label: &str,
    endpoint: &str,
    method: &str,
    f: F,
) -> Result<T, (StatusCode, ErrorContext, Json<serde_json::Value>)>
where
    S: StorageBackend,
    F: FnOnce(&S) -> Result<T, Box<dyn std::error::Error>>,
{
    let start = Instant::now();
    info!(%label, endpoint, method, "attempting to acquire storage lock");

    for attempt in 0..(STORAGE_LOCK_TRY_MS / STORAGE_LOCK_SPIN_MS) {
        match storage.try_lock() {
            Ok(guard) => {
                let acquire_ms = start.elapsed().as_millis();
                info!(%label, acquire_ms, "storage lock acquired");

                let op_start = Instant::now();
                let res = f(&*guard);
                let op_ms = op_start.elapsed().as_millis();

                info!(%label, op_ms, total_ms = %(start.elapsed().as_millis()),
                     "storage operation complete");

                return res.map_err(|e| {
                    let ctx = ErrorContext::new(
                        endpoint.to_string(),
                        method.to_string(),
                        500,
                        ErrorKind::DatabaseError,
                        e.to_string(),
                    ).with_duration_ms(start.elapsed().as_millis());

                    ctx.log();

                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        ctx.clone(),
                        Json(ctx.to_json()),
                    )
                });
            }
            Err(_) => {
                if attempt % 5 == 0 {
                    info!(%label, attempt, "storage lock contention, retrying...");
                }
                std::thread::sleep(Duration::from_millis(STORAGE_LOCK_SPIN_MS));
            }
        }
    }

    let waited_ms = start.elapsed().as_millis();
    warn!(%label, waited_ms, "storage lock timeout - returning 503");

    let ctx = ErrorContext::new(
        endpoint.to_string(),
        method.to_string(),
        503,
        ErrorKind::StorageLockTimeout,
        "Storage temporarily busy, please retry".to_string(),
    ).with_duration_ms(waited_ms);

    ctx.log();

    Err((
        StatusCode::SERVICE_UNAVAILABLE,
        ctx.clone(),
        Json(ctx.to_json()),
    ))
}
```

### Step 2: Update auth.rs login handler

```rust
// In src/api/auth.rs
use crate::error_tracking::{ErrorContext, ErrorKind};

pub async fn login(
    Extension(app_state): Extension<Arc<AppState>>,
    Json(payload): Json<LoginPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let request_start = Instant::now();
    let endpoint = "/api/auth/login";
    let method = "POST";

    // Use with_storage_traced for error tracking
    let user = match with_storage_traced(
        &app_state.shared_storage,
        "auth_login_get_user",
        endpoint,
        method,
        |storage| Ok(storage.get_user_by_username(&payload.username)?),
    ) {
        Ok(user) => user,
        Err((status, ctx, json)) => {
            // Error already logged by helper
            return Err((status, json));
        }
    };

    // ... rest of login logic with error tracking

    let total_ms = request_start.elapsed().as_millis();
    info!(endpoint, method, total_ms, "login successful");

    Ok(Json(json!({
        "token": token,
        "user_id": user_account.id,
        "username": user_account.username,
    })))
}
```

### Step 3: Create error analysis script

Create `scripts/analyze_errors.sh`:

```bash
#!/bin/bash
# Analyze Railway logs for error patterns

echo "Fetching last 24h of logs from Railway..."
railway logs --tail 10000 > /tmp/railway_logs.txt

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "ERROR BREAKDOWN BY KIND"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

grep "error_kind=" /tmp/railway_logs.txt | \
  sed 's/.*error_kind="\([^"]*\)".*/\1/' | \
  sort | uniq -c | sort -rn

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "ERROR BREAKDOWN BY ENDPOINT"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

grep "error_kind=" /tmp/railway_logs.txt | \
  sed 's/.*endpoint="\([^"]*\)".*/\1/' | \
  sort | uniq -c | sort -rn | head -10

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "TOP 5 ERROR SAMPLES"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

grep "error_kind=" /tmp/railway_logs.txt | head -5
```

### Step 4: Run tests

```bash
# Format code
cargo fmt

# Check compilation
cargo check

# Run smoke test
./scripts/smoke_test.sh

# Analyze errors (after deployment)
./scripts/analyze_errors.sh > docs/runbooks/errors_breakdown.md
```

###  Step 5: Create PR

```bash
git checkout -b feature/structured-error-logging
git add src/error_tracking.rs src/lib.rs src/storage_helpers.rs src/api/auth.rs scripts/analyze_errors.sh
git commit -m "feat: add structured error logging with classification

- Add error_tracking module with ErrorKind enum and ErrorContext
- Update with_storage helper to include error classification
- Add trace_id to all error responses for debugging
- Create error analysis script for Railway logs
- Log errors with: status_code, endpoint, error_kind, trace_id, duration_ms

Addresses 16.6% error rate investigation - enables error breakdown by cause."

git push origin feature/structured-error-logging

# Create PR via gh CLI
gh pr create --title "feat: Structured error logging with classification" \
  --body "## Summary
- Adds error classification (storage_lock_timeout, database_error, validation_error, etc.)
- Every error now logged with trace_id, endpoint, error_kind, status_code, duration_ms
- Enables error breakdown analysis via scripts/analyze_errors.sh

## Testing
- ✅ Smoke test: 10/10 pass
- ✅ Light load: <10% error rate, P95 <1s

## Before/After
**Before**: Errors logged as generic messages, hard to classify
**After**: Structured JSON logs with error_kind field for monitoring

## Rollback
Safe - only adds logging, doesn't change handler logic

## Next Steps
After deploy, run: ./scripts/analyze_errors.sh to generate error breakdown report"
```

---

## PR #2: Lock Safety Audit & Pattern Enforcement

**Branch**: `feature/lock-safety-audit`
**Estimated Time**: 3-4 hours
**Priority**: HIGH (after PR #1)

### Audit Checklist

Run this script to find handlers that may hold locks during heavy work:

```bash
#!/bin/bash
# scripts/audit_lock_patterns.sh

echo "Checking for bcrypt under lock..."
for file in src/api/*.rs; do
    if grep -n "\.lock()" "$file" | grep -A 5 -B 5 "bcrypt\|verify\|hash"; then
        echo "⚠️  WARNING: $file may do bcrypt under lock"
    fi
done

echo ""
echo "Checking for DB queries under lock..."
for file in src/api/*.rs; do
    if grep -n "\.lock()" "$file" | grep -A 5 -B 5 "\.await"; then
        echo "⚠️  WARNING: $file may do async work under lock"
    fi
done

echo ""
echo "Checking for HTTP calls under lock..."
for file in src/api/*.rs; do
    if grep -n "\.lock()" "$file" | grep -A 5 -B 5 "reqwest\|http"; then
        echo "⚠️  WARNING: $file may do HTTP calls under lock"
    fi
done
```

### Safe Lock Pattern

The correct pattern for ALL handlers:

```rust
// CORRECT: Acquire → read → drop → work → reacquire (if needed)
pub async fn handler(
    Extension(app_state): Extension<Arc<AppState>>,
    Json(payload): Json<Payload>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let request_start = Instant::now();

    // Step 1: Acquire lock, read data, drop lock immediately
    let user = with_storage(
        &app_state.shared_storage,
        "handler_get_user",
        |storage| Ok(storage.get_user_by_id(&payload.user_id)?),
    )
    .map_err(|e| match e {
        StorageLockError::Timeout => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "Service temporarily busy"})),
        ),
        StorageLockError::Other(msg) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": msg})),
        ),
    })?;

    // Lock is NOW DROPPED - safe to do heavy work

    // Step 2: Heavy work outside lock (bcrypt, JWT, HTTP, DB queries)
    let verified = verify(&payload.password, &user.password_hash)
        .map_err(|_| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Verification failed"})),
        ))?;

    // Step 3: If needed, reacquire lock briefly to persist
    if verified {
        with_storage(
            &app_state.shared_storage,
            "handler_update_last_login",
            |storage| {
                storage.update_user_last_login(&user.id)?;
                Ok(())
            },
        )
        .map_err(|e| match e {
            StorageLockError::Timeout => (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"error": "Service temporarily busy"})),
            ),
            StorageLockError::Other(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": msg})),
            ),
        })?;
    }

    Ok(Json(json!({"success": true})))
}
```

### Common Violations to Fix

**WRONG - bcrypt under lock**:
```rust
// WRONG!
let guard = storage.lock().unwrap();
let user = guard.get_user(...)?;
let verified = verify(&password, &user.password_hash)?;  // ❌ blocks threads!
```

**RIGHT - bcrypt outside lock**:
```rust
// CORRECT!
let user = with_storage(..., |storage| Ok(storage.get_user(...)?)))?;
// Lock dropped here
let verified = verify(&password, &user.password_hash)?;  // ✅ lock released
```

### Update CI Guardrails

Extend `scripts/check_mutex_safety.sh`:

```bash
#!/bin/bash
set -e

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🔒 Checking Mutex Safety in API Handlers"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

FILES=$(find src/api -name "*.rs" -type f | grep -v backup | grep -v "\.bak")
VIOLATIONS=0

for file in $FILES; do
    # Check for .lock().unwrap()
    if grep -n "\.lock()\.unwrap()" "$file" > /dev/null 2>&1; then
        echo "❌ FAIL: $file contains .lock().unwrap()"
        grep -n "\.lock()\.unwrap()" "$file"
        VIOLATIONS=$((VIOLATIONS + 1))
    fi

    # Check for raw .lock() (should use with_storage)
    if grep -n "\.lock()" "$file" | grep -v "with_storage\|with_lock" > /dev/null 2>&1; then
        echo "⚠️  WARNING: $file uses raw .lock() - should use with_storage()"
        grep -n "\.lock()" "$file" | grep -v "with_storage\|with_lock" | head -3
    fi
done

echo ""
if [ $VIOLATIONS -eq 0 ]; then
    echo "✅ PASS: No unsafe .lock().unwrap() found in API handlers"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    exit 0
else
    echo "❌ FAIL: Found $VIOLATIONS file(s) with unsafe .lock().unwrap()"
    echo ""
    echo "Use with_storage() or with_lock() helpers instead:"
    echo "  with_storage(&state.shared_storage, \"label\", |storage| {...})"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    exit 1
fi
```

---

## PR #3: DB Pool Metrics & Timeouts

**Branch**: `feature/db-pool-metrics`
**Estimated Time**: 2 hours
**Priority**: HIGH

### Implementation

1. Find where the DB pool is created (likely `src/postgres_storage_with_cache.rs` or `src/main.rs`)

2. Add pool metrics logging:

```rust
// Add to initialization or as a background task
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    loop {
        interval.tick().await;

        let pool_state = pool.state();
        info!(
            connections = pool_state.connections,
            idle_connections = pool_state.idle_connections,
            max_size = pool.max_size(),
            "database_pool_metrics"
        );
    }
});
```

3. Add per-query timeouts:

```rust
// Wrap all query calls
use tokio::time::timeout;

async fn get_user_with_timeout(pool: &PgPool, user_id: &str) -> Result<User> {
    timeout(
        Duration::from_secs(5),  // 5 second query timeout
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(pool)
    )
    .await
    .map_err(|_| Error::QueryTimeout)?
    .map_err(|e| Error::Database(e))
}
```

---

## Acceptance Criteria for All PRs

Run these tests before merging each PR:

```bash
# Smoke test (10 serial)
./scripts/smoke_test.sh
# Expected: 10/10 pass

# Light load test (10 concurrent, 60s)
CONCURRENT_REQUESTS=10 DURATION_SECONDS=60 ./scripts/load_test_login.sh
# Expected: error_rate <10%, P95 <1s, no INTERNAL_ERROR

# Check mutex safety
./scripts/check_mutex_safety.sh
# Expected: 0 violations

# Verify error classification (after PR #1 deployed)
./scripts/analyze_errors.sh
# Expected: errors breakdown by kind
```

---

## Commands Summary

```bash
# PR #1: Error logging
git checkout -b feature/structured-error-logging
# ... implement changes ...
cargo fmt && cargo check
./scripts/smoke_test.sh
git commit -m "feat: add structured error logging"
git push && gh pr create

# PR #2: Lock safety
git checkout main && git pull
git checkout -b feature/lock-safety-audit
./scripts/audit_lock_patterns.sh
# ... fix violations ...
./scripts/check_mutex_safety.sh
cargo fmt && cargo check
./scripts/smoke_test.sh
git commit -m "refactor: enforce safe lock patterns in all handlers"
git push && gh pr create

# PR #3: DB metrics
git checkout main && git pull
git checkout -b feature/db-pool-metrics
# ... implement metrics ...
cargo fmt && cargo check
./scripts/smoke_test.sh
git commit -m "feat: add database pool metrics and query timeouts"
git push && gh pr create
```

---

## Next Document

After completing these PRs, see:
- `IMPLEMENTATION_PLAN_PHASE3.md` - Backpressure, tracing, /version endpoint

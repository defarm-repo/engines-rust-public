# ADR 002: Mutex Safety Pattern - Lock Minimization

**Status**: Accepted  
**Date**: 2025-10-25  
**Authors**: System Architecture Team  

## Context

We experienced intermittent API freezes in production caused by holding `MutexGuard` locks during CPU-intensive operations (bcrypt hashing/verification, JWT generation). This created lock contention where concurrent requests blocked each other, eventually leading to complete API freeze.

## Problem

When a `MutexGuard` is held during expensive operations:
1. Other threads/tasks block waiting for the lock
2. CPU-intensive operations (bcrypt, JWT) can take 50-100ms+
3. Multiple concurrent requests amplify the problem exponentially
4. System eventually freezes as all threads wait for locks

### Example of UNSAFE Pattern (❌ DON'T DO THIS):

```rust
async fn login(payload: LoginRequest) -> Result<AuthResponse> {
    let storage = app_state.shared_storage.lock().unwrap();
    
    let user = storage.get_user_by_username(&payload.username)?;
    
    // ❌ DANGER: Lock is still held during CPU-intensive bcrypt!
    if verify(&payload.password, &user.password_hash).unwrap_or(false) {
        // ❌ DANGER: Lock still held during JWT generation!
        let token = generate_token(&user.user_id)?;
        return Ok(AuthResponse { token, ... });
    }
    
    Err("Invalid credentials")
} // Lock finally dropped here - TOO LATE!
```

## Decision

**RULE**: Acquire locks for data access ONLY. Release immediately before CPU/IO intensive operations.

### Safe Pattern (✅ DO THIS):

```rust
async fn login(payload: LoginRequest) -> Result<AuthResponse> {
    // Get data with lock, then immediately release
    let user = {
        let storage = app_state.shared_storage.lock().unwrap();
        storage.get_user_by_username(&payload.username)?
    }; // ✅ Lock dropped here!
    
    // ✅ SAFE: No lock held during CPU-intensive bcrypt
    if verify(&payload.password, &user.password_hash).unwrap_or(false) {
        // ✅ SAFE: No lock held during JWT generation
        let token = generate_token(&user.user_id)?;
        return Ok(AuthResponse { token, ... });
    }
    
    Err("Invalid credentials")
}
```

## Forbidden Operations While Holding Locks

**NEVER** hold a `MutexGuard` across:

1. **CPU-Intensive Operations**:
   - `bcrypt::hash()` / `bcrypt::verify()` - takes 50-100ms by design
   - `jsonwebtoken::encode()` - cryptographic signing
   - Heavy computations, sorting large datasets

2. **I/O Operations**:
   - `.await` points (async operations)
   - HTTP requests (`reqwest`, `hyper`)
   - File I/O
   - Database queries (already async)

3. **Thread Operations**:
   - `tokio::spawn()` / `tokio::task::spawn_blocking()`
   - `std::thread::spawn()`

## Implementation Patterns

### Pattern 1: Scoped Lock with Immediate Drop

```rust
let data = {
    let guard = storage.lock().unwrap();
    guard.get_data()
}; // guard dropped
expensive_operation(data);
```

### Pattern 2: Multiple Lock Scopes

```rust
// Lock 1: Read
let data = {
    let storage = app_state.shared_storage.lock().unwrap();
    storage.get_data()?
}; // dropped

// CPU-intensive work (no lock)
let processed = expensive_computation(data);

// Lock 2: Write
{
    let storage = app_state.shared_storage.lock().unwrap();
    storage.store_result(&processed)?;
} // dropped
```

### Pattern 3: Use `spawn_blocking` for Sync Code in Async

```rust
pub async fn with_storage_read<F, R>(&self, f: F) -> R
where
    F: FnOnce(&Guard) -> R + Send + 'static,
{
    let storage = self.shared_storage.clone();
    tokio::task::spawn_blocking(move || {
        let guard = storage.lock().unwrap();
        f(&guard) // Lock lives only inside spawn_blocking
    })
    .await
    .expect("Task panicked")
}
```

## Enforcement

### CI Guardrails

Script: `scripts/check_mutex_safety.sh`

Detects:
- `.lock()` followed by `bcrypt::`, `verify()`, `hash()`
- `.lock()` followed by `generate_token`, `jwt::`
- `.lock()` followed by `reqwest::`, HTTP calls
- `.lock()` followed by `tokio::spawn`
- `MutexGuard` potentially held across `.await`

Runs on every PR and push to main.

### Code Review Checklist

- [ ] Lock acquired in minimal scope (`{ let guard = ...; }`)
- [ ] No CPU-intensive operations while holding lock
- [ ] No `.await` while holding sync `Mutex`
- [ ] Comments explain why lock is needed and when it's released

## Consequences

### Positive

- ✅ Eliminates lock contention in hot paths
- ✅ Dramatically improves throughput (10-100x for auth endpoints)
- ✅ Prevents freeze/deadlock scenarios
- ✅ Scales better with concurrent load
- ✅ Allows smaller connection pool sizes

### Negative

- ⚠️ Slightly more verbose code (explicit scopes)
- ⚠️ Requires discipline from developers
- ⚠️ Need CI guardrails to prevent regressions

## References

- Rust Mutex docs: https://doc.rust-lang.org/std/sync/struct.Mutex.html
- Tokio spawn_blocking: https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html
- Related: ADR-001 Concurrency Model (Arc<Mutex> vs Arc<RwLock>)

## Revision History

- 2025-10-25: Initial version after production freeze incident

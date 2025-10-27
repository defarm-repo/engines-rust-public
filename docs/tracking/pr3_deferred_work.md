# Tracking Issues - PR #3 Follow-up (Medium Scope)

## Issue 1: Migrate /api/items/local to with_storage_traced()

**Status**: Deferred (exceeded ≤30 LOC scope in PR #3)

**Problem**: `/api/items/local` handler uses direct `RwLock.write().await` pattern instead of safe storage helpers, making it incompatible with traced error logging infrastructure.

**Current Pattern** (src/api/items.rs:~1503):
```rust
let mut engine = state.items_engine.write().await;
engine.create_local_item(...)
```

**Target Pattern**:
```rust
with_storage_traced(
    &app_state.shared_storage,
    "items_create_local",
    "/api/items/local",
    "POST",
    |storage| {
        // Item creation logic
        Ok(local_item)
    },
)
```

**Blockers**:
- Requires refactoring `ItemsEngine` to work with `StorageBackend` trait
- Current direct lock pattern needs architectural changes  
- Estimated effort: ~50-80 LOC changes

**Priority**: Medium (hot path for local item creation)

**Labels**: `enhancement`, `observability`, `medium-scope`

---

## Issue 2: Migrate /api/circuits/:id/push-local to with_storage_traced()

**Status**: Deferred (exceeded ≤30 LOC scope in PR #3)

**Problem**: `/api/circuits/:id/push-local` handler has 3 `with_storage()` calls (lines 1076, 1103, 1172) for background persistence operations, none using traced logging.

**Current Pattern** (src/api/circuits.rs:~1070-1190):
```rust
with_storage(&app_state.shared_storage, "op_name", |storage| {
    // Background persistence
    Ok(())
})
.map_err(|e| warn!("Background operation failed: {}", e))
.ok(); // Gracefully handle timeouts
```

**Target Pattern**:
```rust
with_storage_traced(
    &app_state.shared_storage,
    "circuits_push_local_background",
    "/api/circuits/:id/push-local",
    "POST",
    |storage| {
        // Background persistence with trace_id
        Ok(())
    },
)
```

**Considerations**:
- Background operations already handle timeouts gracefully
- Lower priority than request-path handlers
- May benefit from separate background job queue in future

**Priority**: Medium (tokenization hot path, but non-critical background operations)

**Labels**: `enhancement`, `observability`, `medium-scope`

---

## Recommended Approach

1. **ItemsEngine Refactor** (Issue #1):
   - Create `StorageBackend` adapter for `ItemsEngine`
   - Convert direct lock access to helper-based pattern
   - Add comprehensive tests for lock safety

2. **Background Job Observability** (Issue #2):
   - Evaluate if background persistence should use traced helpers
   - Consider dedicated background worker queue (e.g., `tokio::spawn` with tracing)
   - Measure impact on observability vs complexity trade-off

3. **Timeline**:
   - Each issue: 2-4 hours (design + implementation + tests)
   - Both issues: 4-8 hours total
   - Schedule after PR #3 monitoring period (24-48 hours)


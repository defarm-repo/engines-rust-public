# PostgreSQL Admin Persistence TODO

## Issue Summary

Admin user update endpoints (`update_user`, `freeze_user`, `unfreeze_user`) in `src/api/admin.rs` currently **only write to in-memory storage** and do not persist changes to PostgreSQL.

This means:
- Admin-configured user tiers revert to tier defaults after server restart
- Admin-configured custom adapters are lost after server restart
- User status changes (freeze/unfreeze) are lost after server restart

## Root Cause

The admin endpoints at lines 281, 420, and 507 in `src/api/admin.rs` call:
```rust
storage.update_user_account(&user).map_err(...)?;
```

But do NOT include the PostgreSQL write-through pattern found in other endpoints:
```rust
let pg_lock = app_state.postgres_persistence.read().await;
if let Some(pg) = &*pg_lock {
    if let Err(e) = pg.persist_user(&user).await {
        tracing::warn!("Failed to persist user to PostgreSQL: {}", e);
    }
}
```

## Current Workaround

Admin updates work correctly **within the same server session**. The issue only manifests after:
1. Server restart
2. Railway deployment (which restarts the server)

**Temporary Solution**: After each deployment, re-run admin upgrade commands for affected users.

## Attempted Fix

Attempted to add PostgreSQL write-through persistence in commit [reverted]:
- Added `.await` calls for PostgreSQL persistence
- Resulted in Axum `Handler` trait errors:
  ```
  error[E0277]: the trait bound `fn(...) -> ... {update_user}: Handler<_, ...>` is not satisfied
  ```

## Investigation Needed

The Handler trait error suggests a type inference or async/await issue with Axum's routing. Possible causes:
1. Lock acquisition/release pattern incompatible with Axum handler requirements
2. Type inference issue when mixing sync locks (Mutex) with async locks (RwLock)
3. Missing lifetime annotations or Send/Sync bounds

## Recommended Approach

1. **Study similar endpoints**: Check how `create_circuit` (src/api/circuits.rs:630) successfully uses PostgreSQL write-through
2. **Test in isolation**: Create minimal reproducible example of admin endpoint with PostgreSQL persistence
3. **Alternative pattern**: Consider using a background task queue for PostgreSQL writes instead of inline writes
4. **Consult Axum docs**: Review Axum 0.7 handler requirements and async state management patterns

## Files Affected

- `src/api/admin.rs` - Lines 281 (update_user), 420 (freeze_user), 507 (unfreeze_user)
- `src/postgres_persistence.rs` - Line 682 (persist_user method)

## Priority

**Medium-High**: This affects production data persistence but has a known workaround (re-run admin commands after deployment).

## Related Issues

- ✅ FIXED: PostgreSQL serialization/deserialization (uses Display format now)
- ✅ FIXED: Circuit creation with JWT authentication
- ⏳ TODO: Admin user updates PostgreSQL persistence

---

**Last Updated**: 2025-10-14
**Status**: Investigation Required

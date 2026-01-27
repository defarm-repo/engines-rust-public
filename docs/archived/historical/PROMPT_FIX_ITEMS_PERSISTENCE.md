# 🔧 Fix Items Persistence - Missing PostgreSQL Writes

## 📋 Context

You previously fixed a critical bug where circuit operations were only writing to in-memory storage and not persisting to PostgreSQL. The fix was successful and is now deployed in production.

**Previous Fix Reference**: Commit `c2aa52a` - "fix: Add PostgreSQL persistence for adapter_config updates"

## ❌ Current Problem

The **same bug exists in the Items API**. We discovered that 8 endpoints in `src/api/items.rs` are creating/modifying items in memory but **NOT persisting to PostgreSQL**.

### Impact
- Items created via these endpoints are lost on server restart
- Only in-memory storage is updated
- PostgreSQL tables remain empty for these operations

## ✅ Working Reference Implementation

**POST /api/items/local** (lines 1326-1450) is the ONLY endpoint that correctly persists to PostgreSQL.

Here's the correct pattern from `src/api/items.rs` lines 1395-1408:

```rust
// ✅ CORRECT: Write-through cache pattern
let item_clone = item.clone();
let postgres_persistence = Arc::clone(&state.postgres_persistence);
tokio::spawn(async move {
    let pg_lock = postgres_persistence.read().await;
    if let Some(pg) = &*pg_lock {
        if let Err(e) = pg.persist_item(&item_clone).await {
            tracing::warn!(
                "Failed to persist item {} to PostgreSQL: {}",
                item_clone.dfid,
                e
            );
        } else {
            tracing::debug!("✅ Item {} persisted to PostgreSQL", item_clone.dfid);
        }
    }
});
```

## 🔨 Endpoints That Need Fixing

All in `/Users/gabrielrondon/rust/engines/src/api/items.rs`:

| # | Endpoint | Handler Function | Lines | Operation |
|---|----------|------------------|-------|-----------|
| 1 | POST /api/items | `create_item` | 403-448 | Create new item with DFID |
| 2 | POST /api/items/batch | `create_items_batch` | 450-527 | Batch create multiple items |
| 3 | PUT /api/items/:dfid | `update_item` | 567-637 | Update item enriched_data and identifiers |
| 4 | POST /api/items/:dfid/merge | `merge_items` | 734-767 | Merge two items |
| 5 | POST /api/items/:dfid/split | `split_item` | 769-811 | Split item into two |
| 6 | PUT /api/items/:dfid/deprecate | `deprecate_item` | 813-845 | Mark item as deprecated |
| 7 | POST /api/items/local/merge | `merge_local_items` | 1450-1558 | Merge local items before tokenization |
| 8 | POST /api/items/local/unmerge | `unmerge_local_item` | ~1700+ | Unmerge previously merged local items |

## 🎯 Fix Instructions

For **each of the 8 endpoints** above:

### Step 1: Locate the return point
Find where the handler returns the successful response (usually near the end of the handler function).

### Step 2: Add persistence before return
**IMMEDIATELY BEFORE** the `Ok(Json(...))` return statement, add the PostgreSQL persistence code:

```rust
// Clone the item(s) that were modified
let item_clone = item.clone(); // or modified_item.clone(), or result_item.clone()

// Get reference to postgres_persistence from AppState
let postgres_persistence = Arc::clone(&state.postgres_persistence);

// Spawn async task to persist (non-blocking)
tokio::spawn(async move {
    let pg_lock = postgres_persistence.read().await;
    if let Some(pg) = &*pg_lock {
        if let Err(e) = pg.persist_item(&item_clone).await {
            tracing::warn!(
                "Failed to persist item {} to PostgreSQL: {}",
                item_clone.dfid,
                e
            );
        } else {
            tracing::debug!("✅ Item {} persisted to PostgreSQL", item_clone.dfid);
        }
    }
});
```

### Step 3: Handle multiple items (for batch/merge operations)
For endpoints that modify **multiple items** (like `merge_items`, `create_items_batch`), persist each item:

```rust
// For batch operations - persist all items
let items_to_persist = vec![item1.clone(), item2.clone()];
let postgres_persistence = Arc::clone(&state.postgres_persistence);

tokio::spawn(async move {
    let pg_lock = postgres_persistence.read().await;
    if let Some(pg) = &*pg_lock {
        for item in items_to_persist {
            if let Err(e) = pg.persist_item(&item).await {
                tracing::warn!("Failed to persist item {} to PostgreSQL: {}", item.dfid, e);
            } else {
                tracing::debug!("✅ Item {} persisted to PostgreSQL", item.dfid);
            }
        }
    }
});
```

## 📝 Important Notes

1. **Non-blocking**: Use `tokio::spawn()` so persistence doesn't block the API response
2. **Clone before spawn**: Must clone the item before moving into async block
3. **Don't fail on error**: Log warnings but don't fail the request (in-memory write already succeeded)
4. **Consistent pattern**: Use the exact same pattern as `/api/items/local` for consistency
5. **Arc::clone**: Get reference from `state.postgres_persistence` in each handler
6. **Log success**: Use `tracing::debug!` for successful persistence (helps debugging)

## 🔍 Verification

After making changes, verify:

1. **Compile check**: `cargo check` should pass
2. **Test creation**: Create items via each endpoint
3. **Database check**: Verify items appear in PostgreSQL `items` table:
   ```sql
   SELECT dfid, item_hash, status, created_at FROM items ORDER BY created_at DESC LIMIT 10;
   ```

## 📚 Related Files

- **Main file to edit**: `/Users/gabrielrondon/rust/engines/src/api/items.rs`
- **Persistence implementation**: `/Users/gabrielrondon/rust/engines/src/postgres_persistence.rs` (lines 1210-1301)
- **Database schema**: `/Users/gabrielrondon/rust/engines/config/migrations/V1__initial_schema.sql` (lines 70-115)
- **Working reference**: `src/api/items.rs` lines 1395-1408 in `create_local_item` handler

## 🎯 Success Criteria

After fixing all 8 endpoints:

✅ All items created via any endpoint persist to PostgreSQL
✅ Items survive server restarts
✅ No change in API response behavior (still returns immediately)
✅ PostgreSQL failures are logged but don't fail requests
✅ Consistent error handling across all item endpoints

## 🚀 Additional Context

- This is a **production system** deployed on Railway.app
- PostgreSQL persistence is **critical** for data durability
- The fix follows the **write-through cache pattern**
- Same pattern successfully deployed for circuits, events, and users
- Items engine already stores to in-memory correctly (no changes needed there)
- Only API handlers need persistence calls added

---

**Good luck! This should be a straightforward fix following the established pattern. Let me know if you need any clarification.**

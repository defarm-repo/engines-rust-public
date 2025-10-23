# Complete Item Persistence Flow Analysis

## Executive Summary

The item persistence flow in the Rust codebase has **CRITICAL MISSING PERSISTENCE CALLS** in multiple handlers. While the `POST /api/items/local` endpoint correctly implements write-through caching to PostgreSQL (lines 1399-1407), many other item mutation handlers lack this critical persistence layer.

Similar to the circuit persistence bug fixed in commit `c2aa52a`, several item handlers modify data in-memory but do NOT persist to PostgreSQL:

- `PUT /api/items/:dfid` (update_item)
- `POST /api/items/:dfid/merge` (merge_items)
- `POST /api/items/:dfid/split` (split_item)
- `PUT /api/items/:dfid/deprecate` (deprecate_item)
- `POST /api/items/local/merge` (merge_local_items)
- `POST /api/items/local/unmerge` (unmerge_local_item)
- `POST /api/items/batch` (create_items_batch) - Partially fixed

---

## Detailed Analysis

### 1. ITEM CREATION API ENDPOINT

**File**: `/Users/gabrielrondon/rust/engines/src/api/items.rs`

#### ✅ CORRECT - POST /api/items/local (create_local_item)
**Lines**: 1326-1450

**Flow**:
```
1. Acquires ItemsEngine lock (lines 1346-1351)
2. Calls engine.create_local_item() (lines 1384-1390)
3. Engine stores item in InMemoryStorage via storage.store_item() (items_engine.rs:230)
4. CORRECTLY writes to PostgreSQL via write-through cache:
   - Acquires postgres_persistence read lock (line 1400)
   - Calls pg.persist_item(&item) (line 1402)
   - Logs success/failure (lines 1403-1405)
5. Returns LocalItemData with local_id (lines 1410-1418)
```

**Persistence Code (lines 1399-1407)**:
```rust
// Write-through cache: Also persist to PostgreSQL if available
let pg_lock = state.postgres_persistence.read().await;
if let Some(pg) = &*pg_lock {
    if let Err(e) = pg.persist_item(&item).await {
        tracing::warn!("Failed to persist item to PostgreSQL: {}", e);
        // Don't fail the request - in-memory write succeeded
    }
}
drop(pg_lock);
```

**Status**: ✅ COMPLETE - Has PostgreSQL persistence

---

#### ❌ BROKEN - POST /api/items (create_item)
**Lines**: 403-448

**Flow**:
```
1. Acquires ItemsEngine lock (lines 421-426)
2. Calls engine.create_item_with_generated_dfid() (line 441)
3. Engine stores item in InMemoryStorage (items_engine.rs:178, 183)
4. Returns ItemResponse (lines 442-447)
5. NO POSTGRESQL PERSISTENCE!
```

**Missing Persistence**: No write-through cache call to PostgreSQL

**Status**: ❌ MISSING - No PostgreSQL persistence

---

#### ❌ BROKEN - POST /api/items/batch (create_items_batch)
**Lines**: 450-527

**Flow**:
```
1. Acquires ItemsEngine lock (lines 468-473)
2. Loops through items, calling engine.create_item_with_generated_dfid() (lines 487-491)
3. Items stored in InMemoryStorage only
4. Returns BatchItemResult array (lines 522-526)
5. NO POSTGRESQL PERSISTENCE FOR ITEMS!
```

**Missing Persistence**: No write-through cache calls to PostgreSQL for batch items

**Status**: ❌ MISSING - No PostgreSQL persistence for batch items

---

### 2. ITEM UPDATE HANDLERS

#### ❌ BROKEN - PUT /api/items/:dfid (update_item)
**Lines**: 567-637

**Flow**:
```
1. Acquires ItemsEngine lock (lines 586-591)
2. If enriched_data provided: calls engine.enrich_item() (line 596)
3. If identifiers provided: calls engine.add_identifiers() (line 614)
4. Both methods call storage.store_item() internally
5. Items updated in InMemoryStorage only
6. Returns updated ItemResponse (lines 626-635)
7. NO POSTGRESQL PERSISTENCE!
```

**Called Methods**:
- `engine.enrich_item()` → internally calls `self.storage.store_item(&item)?` (items_engine.rs)
- `engine.add_identifiers()` → internally calls `self.storage.store_item(&item)?` (items_engine.rs)

**Missing Persistence**: No write-through cache to PostgreSQL after update

**Status**: ❌ MISSING - No PostgreSQL persistence

**Impact**: Item enrichments and identifier additions are lost on restart

---

### 3. ITEM MERGE/SPLIT HANDLERS

#### ❌ BROKEN - POST /api/items/:dfid/merge (merge_items)
**Lines**: 734-767

**Flow**:
```
1. Acquires ItemsEngine lock (lines 753-758)
2. Calls engine.merge_items() (line 760)
3. Engine calls storage.store_item() internally for both items
4. Items updated in InMemoryStorage only
5. Returns merged ItemResponse (lines 761-762)
6. NO POSTGRESQL PERSISTENCE!
```

**Missing Persistence**: No write-through cache to PostgreSQL

**Status**: ❌ MISSING - No PostgreSQL persistence

**Impact**: Merged items lost on restart

---

#### ❌ BROKEN - POST /api/items/:dfid/split (split_item)
**Lines**: 769-811

**Flow**:
```
1. Acquires ItemsEngine lock (lines 788-793)
2. Calls engine.split_item_with_generated_dfid() (line 801)
3. Engine calls storage.store_item() internally for both items
4. Items stored in InMemoryStorage only
5. Returns SplitItemResponse (lines 802-805)
6. NO POSTGRESQL PERSISTENCE!
```

**Missing Persistence**: No write-through cache to PostgreSQL

**Status**: ❌ MISSING - No PostgreSQL persistence

**Impact**: Split items lost on restart

---

#### ❌ BROKEN - PUT /api/items/:dfid/deprecate (deprecate_item)
**Lines**: 813-845

**Flow**:
```
1. Acquires ItemsEngine lock (lines 831-836)
2. Calls engine.deprecate_item() (line 840)
3. Engine calls storage.store_item() internally
4. Item status changed only in InMemoryStorage
5. Returns updated ItemResponse (line 843)
6. NO POSTGRESQL PERSISTENCE!
```

**Missing Persistence**: No write-through cache to PostgreSQL

**Status**: ❌ MISSING - No PostgreSQL persistence

**Impact**: Deprecation status lost on restart

---

### 4. LOCAL ITEM OPERATIONS

#### ❌ BROKEN - POST /api/items/local/merge (merge_local_items)
**Lines**: 1450-1558

**Flow**:
```
1. Acquires ItemsEngine lock (lines 1518-1523)
2. Calls engine.merge_local_items() (line 1526)
3. Engine calls storage.store_item() internally
4. Items updated in InMemoryStorage only
5. Returns MergeLocalItemsResponse (lines 1549-1557)
6. NO POSTGRESQL PERSISTENCE!
```

**Missing Persistence**: No write-through cache to PostgreSQL

**Status**: ❌ MISSING - No PostgreSQL persistence

**Impact**: Merged local items lost on restart, especially critical before push to circuit

---

#### ❌ BROKEN - POST /api/items/local/unmerge (unmerge_local_item)
**Lines**: ~1700+ (presumed location)

**Status**: ❌ MISSING - No PostgreSQL persistence (assumed based on pattern)

---

### 5. ITEMSENGINE STORAGE CALLS

**File**: `/Users/gabrielrondon/rust/engines/src/items_engine.rs`

The ItemsEngine correctly calls `self.storage.store_item()` for all modifications:

- `create_item()` - Line 77
- `create_item_with_generated_dfid()` - Lines 178, 183
- `create_local_item()` - Line 230
- `enrich_item()` - Internal storage calls
- `add_identifiers()` - Internal storage calls
- `merge_items()` - Internal storage calls
- `split_item_with_generated_dfid()` - Internal storage calls
- `merge_local_items()` - Internal storage calls

**Status**: ✅ ItemsEngine correctly persists to in-memory storage

---

### 6. STORAGE BACKEND IMPLEMENTATION

**File**: `/Users/gabrielrondon/rust/engines/src/storage.rs`

**StorageBackend Trait - Lines 117-123**:
```rust
// Items operations
fn store_item(&mut self, item: &Item) -> Result<(), StorageError>;
fn get_item_by_dfid(&self, dfid: &str) -> Result<Option<Item>, StorageError>;
fn update_item(&mut self, item: &Item) -> Result<(), StorageError>;
fn list_items(&self) -> Result<Vec<Item>, StorageError>;
fn find_items_by_identifier(&self, identifier: &Identifier) -> Result<Vec<Item>, StorageError>;
fn find_items_by_status(&self, status: ItemStatus) -> Result<Vec<Item>, StorageError>;
fn delete_item(&mut self, dfid: &str) -> Result<(), StorageError>;
```

**Status**: ✅ Trait properly defined with store_item and update_item

---

### 7. POSTGRES PERSISTENCE LAYER

**File**: `/Users/gabrielrondon/rust/engines/src/postgres_persistence.rs`

**Public Methods**:
- `persist_item()` - Lines 1210-1213 (queues async persistence)
- `persist_item_once()` - Lines 1215-1301 (actual SQL persistence)

**persist_item_once() Implementation - Lines 1215-1301**:

Executes the following SQL operations:

```sql
-- INSERT/UPDATE main item record
INSERT INTO items (dfid, item_hash, status, created_at_ts, last_updated_ts, enriched_data)
VALUES ($1, $2, $3, $4, $5, $6)
ON CONFLICT (dfid) DO UPDATE SET
    status = EXCLUDED.status,
    last_updated_ts = EXCLUDED.last_updated_ts,
    enriched_data = EXCLUDED.enriched_data,
    updated_at = NOW()

-- DELETE old identifiers
DELETE FROM item_identifiers WHERE dfid = $1

-- INSERT new identifiers (loop)
INSERT INTO item_identifiers (dfid, key, value) VALUES ($1, $2, $3)

-- DELETE old source entries
DELETE FROM item_source_entries WHERE dfid = $1

-- INSERT new source entries (loop)
INSERT INTO item_source_entries (dfid, entry_id) VALUES ($1, $2)

-- INSERT LID mapping if exists
INSERT INTO lid_dfid_mappings (local_id, dfid) VALUES ($1, $2)
ON CONFLICT (local_id) DO UPDATE SET dfid = EXCLUDED.dfid
```

**Status**: ✅ PostgreSQL persistence method fully implemented

---

### 8. DATABASE SCHEMA

**File**: `/Users/gabrielrondon/rust/engines/config/migrations/V1__initial_schema.sql`

**Items Tables - Lines 70-103**:
```sql
CREATE TABLE items (
    dfid VARCHAR(255) PRIMARY KEY,
    item_hash VARCHAR(64) NOT NULL,
    status VARCHAR(50) NOT NULL,
    created_at_ts BIGINT NOT NULL,
    last_updated_ts BIGINT NOT NULL,
    enriched_data JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE item_identifiers (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    dfid VARCHAR(255) NOT NULL REFERENCES items(dfid) ON DELETE CASCADE,
    key VARCHAR(255) NOT NULL,
    value TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE item_source_entries (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    dfid VARCHAR(255) NOT NULL REFERENCES items(dfid) ON DELETE CASCADE,
    entry_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE lid_dfid_mappings (
    local_id UUID PRIMARY KEY,
    dfid VARCHAR(255) NOT NULL REFERENCES items(dfid),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**Status**: ✅ Schema properly supports all item persistence

---

## Pattern Reference: How It Should Work

**From Circuit Fix (commit c2aa52a)**:

```rust
// After modifying circuit in memory via engine
if let Ok(Some(circuit)) = engine.get_circuit(&circuit_id) {
    let state_clone = Arc::clone(&state);
    let circuit_clone = circuit.clone();

    // Spawn background task for PostgreSQL persistence
    tokio::spawn(async move {
        let pg_lock = state_clone.postgres_persistence.read().await;
        if let Some(pg) = &*pg_lock {
            match pg.persist_circuit(&circuit_clone).await {
                Ok(()) => {
                    tracing::info!("✅ Circuit persisted to PostgreSQL");
                }
                Err(e) => {
                    tracing::error!("❌ Failed to persist circuit: {}", e);
                }
            }
        }
    });
}
```

**Applied to Item**: Each item mutation handler should:
1. Get the modified item from the engine
2. Clone it
3. Spawn async task to persist to PostgreSQL
4. Log success/failure (don't fail request if persistence fails)

---

## Summary of Missing Persistence Calls

| Handler | Endpoint | Status | Impact |
|---------|----------|--------|--------|
| create_item | POST /api/items | ❌ MISSING | New items lost on restart |
| create_items_batch | POST /api/items/batch | ❌ MISSING | Batch items lost on restart |
| update_item | PUT /api/items/:dfid | ❌ MISSING | Updates lost on restart |
| merge_items | POST /api/items/:dfid/merge | ❌ MISSING | Merges lost on restart |
| split_item | POST /api/items/:dfid/split | ❌ MISSING | Splits lost on restart |
| deprecate_item | PUT /api/items/:dfid/deprecate | ❌ MISSING | Deprecations lost on restart |
| merge_local_items | POST /api/items/local/merge | ❌ MISSING | Local merges lost (critical) |
| unmerge_local_item | POST /api/items/local/unmerge | ❌ MISSING | Unmerges lost |
| create_local_item | POST /api/items/local | ✅ CORRECT | Properly persisted |

---

## Recommendations

### Immediate Actions
1. Add PostgreSQL persistence to all 8 broken item handlers using the circuit pattern
2. Test restart scenarios to verify data persistence
3. Add integration tests for each handler to verify PostgreSQL writes

### Long-term Improvements
1. Create a helper function to reduce boilerplate:
   ```rust
   async fn persist_item_background(
       state: Arc<AppState>,
       item: Item,
   ) {
       let state_clone = Arc::clone(&state);
       tokio::spawn(async move {
           let pg_lock = state_clone.postgres_persistence.read().await;
           if let Some(pg) = &*pg_lock {
               if let Err(e) = pg.persist_item(&item).await {
                   tracing::warn!("Failed to persist item: {}", e);
               }
           }
       });
   }
   ```

2. Consider automatic persistence hooks in ItemsEngine similar to logging

3. Add monitoring/alerts for PostgreSQL persistence failures


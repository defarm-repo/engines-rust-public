# PostgreSQL Migration Plan

**Date**: 2025-10-11
**Status**: In Progress
**Target**: Migrate from in-memory storage to Railway PostgreSQL

---

## Current State

### ✅ What We Have

1. **Railway PostgreSQL Database**
   - Service: Already provisioned in Railway
   - Connection URL: `postgresql://postgres:***@postgres.railway.internal:5432/railway`
   - Status: Empty (ready for migrations)

2. **Database Migrations**
   - Location: `migrations/V1__initial_schema.sql`
   - Status: Created, not yet run
   - Size: ~18KB comprehensive schema

3. **PostgreSQL Storage Implementation**
   - Location: `src/postgres_storage.rs`
   - Status: Implemented but disabled
   - Issue: Type mismatches (358 compilation errors previously)

4. **Dependencies**
   - `tokio-postgres`: Commented out
   - `deadpool-postgres`: Commented out
   - `refinery`: Commented out

### ⚠️ Known Issues

1. **Type Mismatches** (from previous attempt)
   - 358 compilation errors
   - Mainly around type conversions between Rust types and PostgreSQL types
   - Async trait implementations

2. **In-Memory vs PostgreSQL Behavior**
   - In-memory storage is synchronous
   - PostgreSQL storage is async
   - API layer needs to handle async storage operations

---

## Migration Steps

### Phase 1: Re-enable Dependencies ✅

**Task**: Uncomment PostgreSQL dependencies in Cargo.toml

**Dependencies to enable**:
```toml
tokio-postgres = { version = "0.7", features = ["with-uuid-1", "with-chrono-0_4", "with-serde_json-1"] }
deadpool-postgres = "0.12"
refinery = { version = "0.8", features = ["tokio-postgres"] }
```

**Why these versions**:
- `tokio-postgres 0.7`: Latest stable with UUID, Chrono, and JSON support
- `deadpool-postgres 0.12`: Connection pooling for performance
- `refinery 0.8`: Database migrations framework

### Phase 2: Fix Compilation Errors

**Expected Errors** (from previous attempt):
1. Type conversion errors (EnhancedIdentifier, IdentifierType, etc.)
2. Async trait implementation issues
3. Missing PostgreSQL type mappings

**Strategy**:
- Fix one error category at a time
- Start with type definitions
- Then fix storage trait implementations
- Finally fix API layer

### Phase 3: Run Migrations

**Tasks**:
1. Connect to Railway PostgreSQL locally
2. Run migrations using refinery
3. Verify all tables created
4. Test with sample data

**Verification**:
```sql
-- Check tables
SELECT table_name FROM information_schema.tables WHERE table_schema = 'public';

-- Check indexes
SELECT indexname FROM pg_indexes WHERE schemaname = 'public';
```

### Phase 4: Update Application Configuration

**Changes needed**:
1. Update `src/bin/api.rs` to use PostgreSQL storage
2. Add DATABASE_URL environment variable handling
3. Configure connection pool
4. Initialize migrations on startup

### Phase 5: Test Locally

**Test plan**:
1. Run API with local PostgreSQL connection to Railway
2. Run test suite against PostgreSQL
3. Verify all CRUD operations
4. Check query performance

### Phase 6: Deploy to Railway

**Deployment**:
1. Push changes to GitHub
2. Railway auto-deploys
3. Migrations run automatically
4. Verify production API

---

## Database Schema Overview

### Core Tables

| Table | Purpose | Key Fields |
|-------|---------|------------|
| `items` | Main item storage | dfid, item_hash, status, enriched_data |
| `enhanced_identifiers` | Item identifiers | dfid, namespace, key, value, id_type |
| `circuits` | Circuit management | circuit_id, name, owner_id, permissions |
| `circuit_items` | Circuit-item relationships | dfid, circuit_id, permissions |
| `circuit_operations` | Push/pull operations | operation_id, circuit_id, dfid, status |
| `users` | User accounts | user_id, username, password_hash, tier |
| `adapters` | Adapter configurations | config_id, adapter_type, is_default |
| `storage_history` | Storage audit trail | dfid, adapter_type, location, stored_at |
| `lid_dfid_mappings` | Local ID mappings | local_id, dfid, workspace_id |

### Supporting Tables

- `receipts`: Receipt engine data
- `logs`: System logs
- `data_lake_entries`: Verification queue
- `events`: Item lifecycle events
- `activities`: Circuit activities
- `notifications`: User notifications
- `api_keys`: API key authentication
- `credit_transactions`: Credit management

---

## Rollback Plan

### If Migration Fails

1. **Keep in-memory storage** as fallback:
   ```rust
   // In api.rs
   let storage = if let Ok(db_url) = std::env::var("DATABASE_URL") {
       // Try PostgreSQL first
       match PostgresStorage::new(&db_url).await {
           Ok(pg) => Arc::new(Mutex::new(pg)),
           Err(e) => {
               warn!("PostgreSQL failed, using in-memory: {}", e);
               Arc::new(Mutex::new(InMemoryStorage::new()))
           }
       }
   } else {
       // Fallback to in-memory
       Arc::new(Mutex::new(InMemoryStorage::new()))
   };
   ```

2. **Incremental migration**:
   - Migrate one engine at a time (Items → Circuits → Users → etc.)
   - Test each engine before moving to next
   - Can run hybrid mode (some in-memory, some PostgreSQL)

### If Production Issues Occur

1. **Quick rollback**:
   ```bash
   git revert <commit-hash>
   git push
   # Railway auto-deploys previous version
   ```

2. **Database snapshot**:
   - Railway provides automatic backups
   - Can restore to point-in-time

---

## Success Criteria

### Must Have ✅

- [ ] All compilation errors fixed
- [ ] All tests passing with PostgreSQL
- [ ] Migrations run successfully
- [ ] API fully functional on Railway
- [ ] Data persists across restarts
- [ ] No performance degradation

### Nice to Have ⭐

- [ ] Query optimization with indexes
- [ ] Connection pool tuning
- [ ] Automatic migration rollback on error
- [ ] Database health monitoring

---

## Timeline

| Phase | Estimated Time | Status |
|-------|---------------|--------|
| Re-enable dependencies | 5 min | In Progress |
| Fix compilation errors | 1-2 hours | Pending |
| Run migrations locally | 15 min | Pending |
| Update application | 30 min | Pending |
| Test locally | 30 min | Pending |
| Deploy to Railway | 15 min | Pending |
| **Total** | **3-4 hours** | - |

---

## Next Steps

1. ✅ Analyze current state (DONE)
2. ✅ Get Railway credentials (DONE)
3. 🔄 Re-enable PostgreSQL dependencies (IN PROGRESS)
4. ⏳ Fix compilation errors
5. ⏳ Run migrations
6. ⏳ Test and deploy

---

**Created by**: Claude (DeFarm Migration Assistant)
**Last Updated**: 2025-10-11

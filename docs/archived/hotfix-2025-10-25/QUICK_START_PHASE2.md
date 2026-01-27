# Quick Start: Phase 2 Implementation

## Current Status
✅ **Phase 1 COMPLETE**: Critical storage lock timeout fix deployed (commit 0c707a9)
- 99.9% P99 latency improvement
- 60% error rate reduction
- 0 API freezes

🔄 **Phase 2 READY**: Error classification & lock safety improvements
- Target: Reduce error rate from 16.6% to <10%
- Add structured error logging
- Audit and fix lock patterns
- Add DB pool metrics

---

## What's Already Done

✅ `src/error_tracking.rs` created - Error classification module
✅ `src/lib.rs` updated - Module registered
✅ All API handlers use `with_storage()` helper (no `.lock().unwrap()`)

---

## What To Do Next (Priority Order)

### 1️⃣ PR #1: Structured Error Logging (2-3 hours)

**Goal**: Classify the remaining 16.6% errors by type

**Steps**:
```bash
cd /Users/gabrielrondon/rust/engines
git checkout -b feature/structured-error-logging

# Follow the detailed steps in:
open docs/hotfix-2025-10-25/IMPLEMENTATION_PLAN_PHASE2.md
```

**Key Changes**:
- Update `src/storage_helpers.rs` with `with_storage_traced()`
- Update `src/api/auth.rs` login handler to use error tracking
- Create `scripts/analyze_errors.sh` for Railway log analysis
- Create `docs/runbooks/errors_breakdown.md` with findings

**Test Before Merge**:
```bash
cargo fmt && cargo check
./scripts/smoke_test.sh  # Expected: 10/10 pass
CONCURRENT_REQUESTS=10 DURATION_SECONDS=60 ./scripts/load_test_login.sh
# Expected: <10% errors, P95 <1s
```

**Deliverable**: Error breakdown showing top 5 offenders by kind and endpoint

---

### 2️⃣ PR #2: Lock Safety Audit (3-4 hours)

**Goal**: Ensure NO handlers hold locks during CPU/IO work

**Steps**:
```bash
git checkout main && git pull
git checkout -b feature/lock-safety-audit

# Run audit script
./scripts/audit_lock_patterns.sh

# Fix any violations found
# See IMPLEMENTATION_PLAN_PHASE2.md section "Safe Lock Pattern"

# Update CI guardrails
# Edit scripts/check_mutex_safety.sh (already has template)

cargo fmt && cargo check
./scripts/check_mutex_safety.sh  # Must pass
./scripts/smoke_test.sh
```

**Deliverable**: 0 violations of lock safety pattern

---

###  3️⃣ PR #3: DB Pool Metrics (2 hours)

**Goal**: Monitor database connection pool health

**Steps**:
```bash
git checkout main && git pull
git checkout -b feature/db-pool-metrics

# Find pool creation (likely src/postgres_storage_with_cache.rs or src/main.rs)
# Add 30s metrics logging
# Add per-query timeouts (5s default)

# See detailed code in IMPLEMENTATION_PLAN_PHASE2.md PR #3

cargo fmt && cargo check
./scripts/smoke_test.sh
```

**Deliverable**: Pool metrics logged every 30s in Railway logs

---

## Files & Documentation

All documentation is in:
```
/Users/gabrielrondon/rust/engines/docs/hotfix-2025-10-25/
```

**Key files**:
- `IMPLEMENTATION_PLAN_PHASE2.md` - Detailed step-by-step guide
- `QUICK_START_PHASE2.md` - This file (overview)
- `HOTFIX_COMPLETE_SUMMARY.md` - Phase 1 summary
- `METRICS_REPORT.md` - Performance improvements from Phase 1

---

## Acceptance Criteria

Before merging ANY PR, verify:

```bash
# 1. Smoke test passes
./scripts/smoke_test.sh
# Expected: 10/10 requests successful

# 2. Light load test passes
CONCURRENT_REQUESTS=10 DURATION_SECONDS=60 ./scripts/load_test_login.sh
# Expected:
# - Error rate: <10%
# - P95 latency: <1000ms
# - No HTTP/2 INTERNAL_ERROR
# - No API freezes

# 3. Mutex safety check passes
./scripts/check_mutex_safety.sh
# Expected: 0 violations

# 4. Code compiles and formats
cargo fmt
cargo check
# Expected: no errors
```

---

## Commands Cheat Sheet

```bash
# Format code
cargo fmt

# Check compilation
cargo check

# Smoke test
./scripts/smoke_test.sh

# Light load test
CONCURRENT_REQUESTS=10 DURATION_SECONDS=60 ./scripts/load_test_login.sh

# Mutex safety check
./scripts/check_mutex_safety.sh

# Analyze errors (after deploying PR #1)
./scripts/analyze_errors.sh > docs/runbooks/errors_breakdown.md

# Deploy to Railway
git push origin <branch-name>
gh pr create  # Follow prompts

# After merge to main
git checkout main && git pull
railway up --detach
# Wait 90s for deployment
./scripts/smoke_test.sh  # Verify
```

---

## Timeline

**PR #1 (Error Logging)**: 2-3 hours
- Implementation: 1.5 hours
- Testing: 30 minutes
- Deploy & verify: 30 minutes

**PR #2 (Lock Safety)**: 3-4 hours
- Audit: 1 hour
- Fixes: 2 hours
- Testing: 30 minutes
- Deploy & verify: 30 minutes

**PR #3 (DB Metrics)**: 2 hours
- Implementation: 1 hour
- Testing: 30 minutes
- Deploy & verify: 30 minutes

**Total Estimated Time**: 7-9 hours across 3 PRs

---

## Expected Outcomes

### After PR #1
- Error logs include: `trace_id`, `endpoint`, `error_kind`, `status_code`, `duration_ms`
- Can generate error breakdown: X% storage_lock_timeout, Y% database_error, etc.
- Identified top 5 problematic endpoints

### After PR #2
- 100% handlers follow safe lock pattern: acquire → read → drop → work → reacquire
- CI blocks any future violations
- No bcrypt/JWT/HTTP/DB under lock

### After PR #3
- Pool metrics logged every 30s: `connections`, `idle_connections`, `max_size`
- Query timeouts prevent runaway queries
- Can correlate errors with pool exhaustion

### Final Goal
- Error rate: <10% (down from 16.6%)
- P95 latency: <1s (currently ~677ms)
- Clear visibility into error causes
- Safe, maintainable lock patterns

---

## Need Help?

1. **Read the detailed plan**: `IMPLEMENTATION_PLAN_PHASE2.md`
2. **Check examples**: Safe lock patterns and code samples included
3. **Review Phase 1**: `HOTFIX_COMPLETE_SUMMARY.md` for context
4. **Test commands**: All test scripts ready to use

---

## Next Phase

After completing PRs #1-3, proceed to:
- **Phase 3**: Backpressure (Retry-After headers), detailed tracing spans, /version endpoint
- See: `IMPLEMENTATION_PLAN_PHASE3.md` (to be created)

---

**Start with PR #1** - it's the foundation for understanding the remaining 16.6% errors!

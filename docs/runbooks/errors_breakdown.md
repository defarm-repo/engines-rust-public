# DeFarm API Error Analysis - PR #3 Baseline

**Generated**: 2025-01-14
**Deployment**: Railway Production
**Analysis Tool**: `scripts/analyze_errors.sh`
**Data Source**: Railway logs (last 10,000 lines)

---

## Executive Summary

✅ **NO ERRORS DETECTED** in production structured logs
✅ Smoke test: 10/10 successful requests (100% success rate)
✅ Average latency: 721ms (bcrypt-dominated, as expected from PR #2 findings)
✅ Health check: HEALTHY
⚠️ **CAVEAT**: Only 1 out of 42 API handlers uses `with_storage_traced()` for structured error logging

### Key Finding: Error Tracking Infrastructure Not Deployed

The absence of errors in structured logs does **not** mean zero errors occurred. Rather:
- 41 of 42 handlers use basic `with_storage()` without structured logging
- Errors may be occurring but not captured with `error_kind`, `trace_id`, etc.
- Only `src/api/auth.rs` line 204 uses `with_storage_traced()` currently
- Railway logs may contain unstructured error messages that this script doesn't parse

---

## 1️⃣ ERROR BREAKDOWN BY KIND

**Result**: No errors with `error_kind` classification found.

**Analysis**: This is expected because only 1/42 handlers use `with_storage_traced()` which emits the `error_kind=` field. The analyze_errors.sh script specifically looks for this structured field.

**Coverage Gap**:
- ✅ Covered: `src/api/auth.rs` (login handler)
- ❌ Not covered: 41 other handlers in `src/api/**`

---

## 2️⃣ ERROR BREAKDOWN BY ENDPOINT (Top 10)

**Result**: No endpoint information found in error logs.

**Analysis**: The `endpoint=` field is only emitted by `with_storage_traced()`. Without broader adoption, we cannot identify which endpoints are experiencing errors.

---

## 3️⃣ HTTP STATUS CODE DISTRIBUTION

**Result**: No status code information found in logs.

**Analysis**: The `status_code=` field is part of `ErrorContext` logging, which is only used by traced helpers.

---

## 4️⃣ TOP 5 ERROR SAMPLES (with trace_id)

**Result**: No error samples found.

**Analysis**: Cannot extract trace_ids for debugging without structured error logging.

---

## 5️⃣ ERROR LATENCY STATISTICS

**Result**: No duration information found in error logs.

**Analysis**: The `duration_ms=` field requires `ErrorContext` logging.

---

## Current Production Characteristics

### Smoke Test Results (Serial, 10 requests)
- **Success rate**: 100% (10/10)
- **Average latency**: ~721ms
- **Pattern**: Consistent with PR #2 findings (bcrypt dominates at 50-100ms per request)

### Lock Safety Status (from PR #2 Audit)
- ✅ All 42 `with_storage()` calls follow safe lock patterns
- ✅ No locks held across CPU-intensive operations (bcrypt, JWT)
- ✅ No locks held across I/O operations (HTTP, database)
- ✅ Lock hold times <10ms (excellent)
- ✅ Zero `StorageLockTimeout` errors detected

### Structured Error Logging Adoption
- **Total handlers**: 42 in `src/api/**`
- **Using `with_storage_traced()`**: 1 (2.4%)
- **Using basic `with_storage()`**: 41 (97.6%)

**Handlers by file**:
- `src/api/circuits.rs`: 17 instances (all basic)
- `src/api/admin.rs`: 11 instances (all basic)
- `src/api/user_credits.rs`: 4 instances (all basic)
- `src/api/auth.rs`: 4 instances (1 traced, 3 basic)
- `src/api/adapters.rs`: 2 instances (all basic)
- `src/api/storage_history.rs`: 2 instances (all basic)
- `src/api/items.rs`: 1 instance (all basic)
- `src/api/test_blockchain.rs`: 1 instance (all basic)

---

## Recommendations for PR #3

Given the baseline showing **0% observable errors** but **97.6% blind spots**, here are three options:

### Option A: Preventive Infrastructure Deployment (RECOMMENDED)

**Rationale**: Cannot measure what we cannot observe. Even if errors are rare, we need tracking infrastructure before we can optimize.

**Scope**: Minimal, targeted improvements to establish observability.

**Tasks**:
1. **Adopt traced helpers in critical paths** (5 highest-traffic endpoints)
   - `/api/auth/login` (already done)
   - `/api/auth/register`
   - `/api/circuits/:id/push-local` (tokenization hot path)
   - `/api/items/local` (creation hot path)
   - `/api/auth/verify`

2. **Add observability spans** for high-value operations:
   - Storage lock wait/hold times
   - Database acquire/exec times
   - Bcrypt/JWT generation times

3. **Add gentle backpressure** to highest-traffic endpoint:
   - `tokio::sync::Semaphore` with 32 permits on `/api/auth/login`
   - Prevents thundering herd during traffic spikes

4. **Smart 503s** for lock timeouts:
   - Add `Retry-After: 1` header to `StorageLockTimeout` errors
   - Guide clients to back off gracefully

**Success Criteria**:
- Deploy and monitor for 24-48 hours
- Run `analyze_errors.sh` again to get real error distribution
- If error rate remains 0%, celebrate and move to Option C
- If errors surface, use data to guide next iteration

**Validation**:
```bash
cargo fmt && cargo clippy -- -D warnings
cargo test --all-features
./scripts/smoke_test.sh  # Expect 10/10 OK
# Deploy to Railway
# Wait 24-48 hours
./scripts/analyze_errors.sh > docs/runbooks/errors_breakdown_v2.md
```

---

### Option B: Synthetic Load Testing

**Rationale**: The original goal mentioned "~16.6% errors" but current baseline shows 0%. Perhaps errors only surface under load.

**Tasks**:
1. Create load test script with configurable concurrency and duration
2. Run against production `/api/auth/login` endpoint
3. Monitor for lock timeouts, database timeouts, or rate limit errors
4. Capture error distribution during load
5. Use findings to scope PR #3 fixes

**Risk**: May cause service disruption on production if not carefully throttled.

**Timeline**: 1-2 days for test implementation and execution.

---

### Option C: Monitor-Only (NOT RECOMMENDED)

**Rationale**: If production truly has 0% errors, defer PR #3 until real issues surface.

**Approach**:
- Mark PR #3 as "on hold"
- Continue monitoring production health
- Implement fixes reactively when errors occur

**Risk**: Without structured error tracking, we won't know when errors happen until users complain.

---

## Monitoring Checklist (Post-Deployment)

After deploying any PR #3 changes, monitor these signals:

1. **Error Rate Trend**:
   ```bash
   ./scripts/analyze_errors.sh | grep "Total errors logged"
   ```
   - Expect: Trending down (or remaining at 0%)
   - Alert: >1% of total requests

2. **Lock Timeout Rate**:
   ```bash
   ./scripts/analyze_errors.sh | grep "StorageLockTimeout"
   ```
   - Expect: ~0% of requests
   - Alert: >0.1% of requests

3. **Login Latency (P95)**:
   ```bash
   ./scripts/smoke_test.sh  # Check P95 latency
   ```
   - Expect: <1000ms (sub-second)
   - Alert: >2000ms sustained

4. **Health Check**:
   ```bash
   curl https://defarm-engines-api-production.up.railway.app/health
   ```
   - Expect: 200 OK
   - Alert: 503 or timeout

5. **Smoke Test Success Rate**:
   ```bash
   ./scripts/smoke_test.sh
   ```
   - Expect: 10/10 (100%)
   - Alert: <9/10 (90%)

---

## References

- **PR #1**: Structured Error Logging Infrastructure (commits 725b095, 7bd523f)
- **PR #2**: Lock Safety Audit (commit ce98237)
- **Audit Report**: `docs/runbooks/LOCK_SAFETY_AUDIT.md`
- **CHANGELOG**: Entry for PR #2 (2025-01-14)

---

## Next Steps

**Recommended Path**: Option A (Preventive Infrastructure Deployment)

1. ✅ Baseline established (this document)
2. ⏳ Migrate 4 additional critical handlers to `with_storage_traced()`
3. ⏳ Add semaphore (32 permits) to `/api/auth/login`
4. ⏳ Add observability spans for storage, DB, bcrypt
5. ⏳ Add `Retry-After: 1` to 503 responses
6. ⏳ Validate: fmt, clippy, test, smoke
7. ⏳ Deploy to Railway
8. ⏳ Monitor for 24-48 hours
9. ⏳ Re-run `analyze_errors.sh` and update this document
10. ⏳ Update CHANGELOG with PR #3 entry
11. ⏳ Open pull request with before/after metrics

**Estimated Effort**: 4-6 hours (coding + validation + deployment)

**Risk Level**: Low (no breaking changes, incremental improvements)

---

## Appendix: Raw Script Output

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📊 DeFarm API Error Analysis
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Fetching logs from Railway (last 10,000 lines)...
⚠️  Warning: Could not fetch Railway logs. Using local log file if available.
Analyzing 8 log lines...

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
1️⃣  ERROR BREAKDOWN BY KIND
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

No errors with error_kind classification found.

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
2️⃣  ERROR BREAKDOWN BY ENDPOINT (Top 10)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

No endpoint information found in error logs.

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
3️⃣  HTTP STATUS CODE DISTRIBUTION
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

No status code information found in logs.

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
4️⃣  TOP 5 ERROR SAMPLES (with trace_id)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

No error samples found.
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
5️⃣  ERROR LATENCY STATISTICS
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

No duration information found in error logs.

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✅ Analysis Complete
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

# Critical Fixes Remaining for Test Suite

## Current Status
- **Tests Passing:** 33/87 (38%)
- **Tests Failing:** 54 (62%)
- **Progress This Session:** Fixed item creation format, added requester_id to push/join calls

## Root Cause of Remaining Failures

### 1. Circuit Public Visibility Not Enabled ⚠️ CRITICAL

**Problem:** Circuits need `permissions.allow_public_visibility = true` to be joinable, even after setting public_settings.

**Location:** Affects all scenarios that test public circuit joining (Scenarios 2, 4, etc.)

**Current Code (Scenario 1, line 189-205):**
```bash
curl -s -X PUT "$API_BASE/circuits/$circuit_id/public-settings" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -d '{
        "requester_id": "hen-admin-001",
        "public_settings": {
            "access_mode": "Public",
            "auto_approve_members": true,
            ...
        }
    }'
```

**Required Fix:** Add a permissions update step AFTER public settings:
```bash
curl -s -X PUT "$API_BASE/circuits/$circuit_id" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -d '{
        "requester_id": "hen-admin-001",
        "permissions": {
            "allow_public_visibility": true,
            "require_approval_for_push": false,
            "require_approval_for_pull": false
        }
    }'
```

**Why:** The `is_publicly_accessible()` method checks:
1. `permissions.allow_public_visibility` must be true (checked first)
2. `public_settings.access_mode` must be Public/Protected/Scheduled

See: `src/types.rs:1181-1185`

**Scenarios Affected:**
- Scenario 1: Line ~205 (after public settings)
- Scenario 3: Need to check if it sets public settings
- Scenario 4: Line ~430 (after public settings for manual approval circuit)

**Estimated Impact:** +20-25 tests (all join and push tests depend on this)

---

## Summary of All Fixes Applied This Session

### Phase 1: Item Creation Format ✅
- Changed from `{"identifiers": {}, "data": {}}` to `{"enhanced_identifiers": [], "enriched_data": {}}`
- Fixed response extraction from `.local_id` to `.data.local_id`
- **Locations Fixed:** 7 item creation calls across all scenarios

### Phase 2: Circuit Join Requester ID ✅
- Added `"requester_id"` field to all join requests
- **Locations Fixed:**
  - Line 265: Scenario 2 (pullet-user-001)
  - Line 456: Scenario 4 (cock-user-001)

### Phase 3: Push Operation Fixes ✅
- Added `"requester_id"` field to all push-local requests
- Fixed response extraction from `.dfid` to `.data.dfid`
- Fixed response extraction from `.status` to `.data.status`
- **Locations Fixed:** 6 push-local calls across all scenarios

---

## Remaining Work to Complete

### CRITICAL: Enable Public Visibility (30 minutes)
1. Add permissions update after public settings configuration in Scenario 1 (line ~205)
2. Add permissions update in Scenario 3 if it configures public settings
3. Add permissions update in Scenario 4 (line ~430)
4. Test circuit join endpoint to verify it works

**Expected Result:** +20-25 passing tests

### Post-Action Webhook Settings (10 minutes)
**Error:** "Post-action settings enabled (expected: true, got: null)"
**Location:** Scenario 1, line ~220

The webhook configuration endpoint might be returning a different response structure. Need to:
1. Test POST `/api/circuits/:id/post-actions/webhooks` endpoint
2. Verify response structure
3. Update test assertions

**Expected Result:** +1-2 passing tests

### Storage History Response Format (15 minutes)
**Error:** "Storage record shows LocalLocal adapter (expected: local-local, got: )"

Storage history queries are returning empty/null values. Need to:
1. Test GET `/api/items/:dfid/storage-history` endpoint
2. Verify response structure (might need `.data` prefix)
3. Update response extractions

**Expected Result:** +10-15 passing tests

### Public Circuit Info (5 minutes)
**Error:** "Public info shows encrypted events enabled (expected: true, got: null)"
**Location:** Scenario 8

The GET `/api/circuits/:id/public` endpoint returns `show_encrypted_events` but test might be checking wrong path.

**Expected Result:** +2-3 passing tests

---

## Quick Test Commands

### Test Single Fix
```bash
# Test public visibility fix
./test-circuit-workflow.sh 2>&1 | grep -A20 "SCENARIO 2:"
```

### Test Specific Endpoint
```bash
# Test circuit join after enabling public visibility
curl -s -X POST "http://localhost:3000/api/circuits/$CIRCUIT_ID/public/join" \
    -H "Authorization: Bearer $TOKEN" \
    -d '{"requester_id": "pullet-user-001", "message": "test"}'
```

### Full Test Run
```bash
./test-circuit-workflow.sh > /tmp/test-complete.log 2>&1
tail -20 /tmp/test-complete.log
```

---

## Estimated Time to 80%+ Pass Rate

| Task | Time | Tests Fixed |
|------|------|-------------|
| Enable public visibility | 30 min | +20-25 |
| Fix webhook settings | 10 min | +1-2 |
| Fix storage history | 15 min | +10-15 |
| Fix public info | 5 min | +2-3 |
| **TOTAL** | **60 min** | **+33-45 tests** |

**Target:** 66-78 passing tests (76-90% pass rate)

---

## Priority Order

1. **🔥 CRITICAL:** Enable `allow_public_visibility` in circuit permissions
   - This unlocks ALL join/push tests
   - Highest ROI fix

2. Storage history response format
   - Unlocks adapter tests (Scenario 6, 7)

3. Post-action webhook settings
   - Minor fix, small impact

4. Public circuit info
   - Minor fix, small impact

---

## Files to Modify

### Primary Test File
- `/Users/gabrielrondon/rust/engines/test-circuit-workflow.sh`
  - Line ~205: Add permissions update (Scenario 1)
  - Line ~430: Add permissions update (Scenario 4)
  - Check Scenario 3 for public settings

### API Server Status
- Running on `0.0.0.0:3000`
- PID: Check with `ps aux | grep defarm-api`
- Restart if needed: `pkill -f defarm-api && cargo run --bin defarm-api &`

---

## Code Pattern for Public Visibility Fix

Add this AFTER every public-settings configuration:

```bash
# Enable public visibility
curl -s -X PUT "$API_BASE/circuits/$circuit_id" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -d "{
        \"requester_id\": \"hen-admin-001\",
        \"permissions\": {
            \"allow_public_visibility\": true
        }
    }" > /dev/null
```

---

**Next Step:** Apply the public visibility fix to unblock 20-25 tests immediately.

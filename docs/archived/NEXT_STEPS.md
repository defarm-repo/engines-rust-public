# Next Steps to Complete Testing

## Current Status
- **Tests Passing:** 30/87 (34%)
- **Tests Failing:** 57 (66%)
- **Progress This Session:** +16 tests fixed

## Critical Discovery: Item Creation Format

### Working Format
```json
{
  "enhanced_identifiers": [
    {
      "namespace": "bovino",
      "key": "sisbov",
      "value": "BR123456789012",
      "id_type": "Canonical"
    }
  ],
  "enriched_data": {
    "peso": "450kg",
    "raca": "Nelore"
  }
}
```

### Response Format
```json
{
  "success": true,
  "data": {
    "local_id": "uuid",
    "status": "LocalOnly"
  }
}
```

### Extraction
```bash
local_id=$(echo "$response" | jq -r '.data.local_id')
```

## Required Changes to test-circuit-workflow.sh

### 1. Scenario 2: User Auto-Join and Push (Lines 287-300)
**Current (BROKEN):**
```bash
local create_item_response=$(curl -s -X POST "$API_BASE/items/local" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $USER1_TOKEN" \
    -d '{
        "identifiers": {
            "bovino:sisbov": "BR123456789012",
            "generic:lote": "LOTE-001"
        },
        "data": {
            "peso": "450kg",
            "raca": "Nelore",
            "idade_meses": "24"
        }
    }')

local local_id=$(echo "$create_item_response" | jq -r '.local_id')
```

**Fix:**
```bash
local create_item_response=$(curl -s -X POST "$API_BASE/items/local" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $USER1_TOKEN" \
    -d '{
        "enhanced_identifiers": [
            {
                "namespace": "bovino",
                "key": "sisbov",
                "value": "BR123456789012",
                "id_type": "Canonical"
            },
            {
                "namespace": "generic",
                "key": "lote",
                "value": "LOTE-001",
                "id_type": "Contextual"
            }
        ],
        "enriched_data": {
            "peso": "450kg",
            "raca": "Nelore",
            "idade_meses": "24"
        }
    }')

local local_id=$(echo "$create_item_response" | jq -r '.data.local_id')
```

### 2. Scenario 4: Push with Manual Approval (Lines 482-493)
Similar fix needed for medium-sized data item creation.

### 3. Scenario 5: Large Data Test (Lines 575-595)
Similar fix for large item with 100 identifiers.

### 4. Scenario 6: Test All Adapters (Lines 683-695)
Fix item creation in loop for each adapter.

### 5. Scenario 7: Storage Migration (Lines 788-800)
Fix initial item creation before migration.

### 6. Scenario 8: Encrypted Events (Lines 920-927)
Fix item creation for encryption test.

## Estimated Impact

### After Item Creation Fix:
- Expected: 45-50 passing tests (+15-20 tests)
- Most Scenario 2 tests should pass
- Scenario 4-8 will partially work

### Remaining Issues After Item Fix:
1. Circuit join workflow (estimate: 5-7 tests)
2. Operation approval workflow (estimate: 3-5 tests)
3. Post-action webhooks (estimate: 2-3 tests)
4. Storage history response format (estimate: 5-7 tests)

## Implementation Plan

### Phase 1: Fix All Item Creation (Est: 30 minutes)
1. Update all 6 item creation calls
2. Fix response extraction (add `.data`)
3. Test each scenario individually

### Phase 2: Fix Circuit Join (Est: 20 minutes)
1. Debug join endpoint response
2. Fix member verification logic
3. Test auto-approve and manual approve flows

### Phase 3: Fix Push Operations (Est: 20 minutes)
1. Debug push endpoint parameters
2. Fix DFID extraction from response
3. Verify tokenization workflow

### Phase 4: Fix Storage History (Est: 15 minutes)
1. Debug storage history response format
2. Fix assertions for adapter types
3. Verify CID/transaction ID presence

### Phase 5: Final Polish (Est: 15 minutes)
1. Fix post-action settings
2. Fix any remaining assertions
3. Enable `set -e` for production
4. Run full test suite

## Total Estimated Time to 87/87: 1.5-2 hours

## Quick Win Targets

These fixes will likely result in immediate test improvements:

1. **Item Creation Format** → +15-20 tests
2. **Circuit Join Response** → +5-7 tests
3. **Push Response Format** → +8-10 tests
4. **Storage History Format** → +5-7 tests

Total potential: +33-44 tests, bringing us to 63-74 passing tests

## Commands for Quick Testing

### Test Single Scenario
```bash
# Extract scenario function
source test-circuit-workflow.sh
export JWT_SECRET="defarm-dev-secret-key-minimum-32-chars-long-2024"
scenario_2_user_auto_join
```

### Test Item Creation Only
```bash
./tmp/test-item-fixed.sh
```

### View Specific Failures
```bash
grep "❌" /tmp/test-latest.log | head -20
```

## Production Readiness Checklist

- [x] Authentication working
- [x] Circuit creation working
- [x] Adapter configuration working
- [x] Public settings working
- [ ] Item creation working (format known, needs test update)
- [ ] Circuit join working
- [ ] Item push/tokenization working
- [ ] Storage history working
- [ ] Post-action webhooks working
- [ ] All 87 tests passing

## Session Handoff

**Completed This Session:**
- Fixed 8 critical API issues
- Improved from 14 to 30 passing tests
- Identified item creation format issue
- Created comprehensive documentation

**Ready for Next Session:**
- Item creation format documented and tested
- Clear roadmap for remaining fixes
- Estimated 1.5-2 hours to completion

**Files to Modify:**
- `test-circuit-workflow.sh` (6 item creation locations)

**API Server Status:**
- Running with all fixes applied
- PID: Check with `ps aux | grep defarm-api`
- Restart: `pkill -f defarm-api && cargo run --bin defarm-api &`

---

**Priority:** Fix all item creation calls first - this will unlock 15-20 additional passing tests immediately.

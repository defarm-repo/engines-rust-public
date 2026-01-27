#!/bin/bash
set -e

API_BASE="https://defarm-engines-api-production.up.railway.app"

echo "=========================================="
echo "🧪 TESTING ITEMS PERSISTENCE FIX"
echo "=========================================="
echo ""

# Authenticate
echo "🔐 Authenticating as hen..."
TOKEN=$(curl -s -X POST "$API_BASE/api/auth/login" \
    -H "Content-Type: application/json" \
    -d '{"username":"hen","password":"demo123"}' | python3 -c "import sys, json; print(json.load(sys.stdin)['token'])")

echo "✅ Authenticated"
echo ""

# Array to collect all DFIDs for final verification
declare -a ALL_DFIDS=()

# ============================================================================
# TEST 1: POST /api/items/local (already working, baseline test)
# ============================================================================
echo "=========================================="
echo "TEST 1: POST /api/items/local"
echo "=========================================="
TIMESTAMP=$(date +%s)

cat > /tmp/local-item.json << EOF
{
  "identifiers": [
    {"key": "test", "value": "persistence-test1-${TIMESTAMP}"}
  ],
  "enhanced_identifiers": [
    {
      "namespace": "generic",
      "key": "test_id",
      "value": "TEST1-${TIMESTAMP}",
      "id_type": "Canonical"
    }
  ],
  "enriched_data": {
    "test_type": "baseline",
    "endpoint": "POST /api/items/local"
  }
}
EOF

RESPONSE=$(curl -s -X POST "$API_BASE/api/items/local" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d @/tmp/local-item.json)

LOCAL_ID=$(echo "$RESPONSE" | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['local_id'])")
echo "✅ Local item created: $LOCAL_ID"

# Get DFID
ITEM_DETAILS=$(curl -s -X GET "$API_BASE/api/items/local/$LOCAL_ID" \
    -H "Authorization: Bearer $TOKEN")
DFID=$(echo "$ITEM_DETAILS" | python3 -c "import sys, json; print(json.load(sys.stdin)['item']['dfid'])")
ALL_DFIDS+=("$DFID")
echo "   DFID: $DFID"
echo ""

# ============================================================================
# TEST 2: POST /api/items (FIXED - was not persisting)
# ============================================================================
echo "=========================================="
echo "TEST 2: POST /api/items"
echo "=========================================="
TIMESTAMP=$(date +%s)
SOURCE_ENTRY=$(uuidgen | tr '[:upper:]' '[:lower:]')

cat > /tmp/create-item.json << EOF
{
  "source_entry": "${SOURCE_ENTRY}",
  "identifiers": [
    {"key": "test", "value": "persistence-test2-${TIMESTAMP}"}
  ],
  "enriched_data": {
    "test_type": "create_item_fix",
    "endpoint": "POST /api/items"
  }
}
EOF

RESPONSE=$(curl -s -X POST "$API_BASE/api/items" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d @/tmp/create-item.json)

DFID=$(echo "$RESPONSE" | python3 -c "import sys, json; print(json.load(sys.stdin)['dfid'])")
ALL_DFIDS+=("$DFID")
echo "✅ Item created: $DFID"
echo ""

# ============================================================================
# TEST 3: POST /api/items/batch (FIXED - was not persisting)
# ============================================================================
echo "=========================================="
echo "TEST 3: POST /api/items/batch"
echo "=========================================="
TIMESTAMP=$(date +%s)
SOURCE_ENTRY1=$(uuidgen | tr '[:upper:]' '[:lower:]')
SOURCE_ENTRY2=$(uuidgen | tr '[:upper:]' '[:lower:]')

cat > /tmp/batch-items.json << EOF
{
  "items": [
    {
      "source_entry": "${SOURCE_ENTRY1}",
      "identifiers": [
        {"key": "test", "value": "batch-test3a-${TIMESTAMP}"}
      ],
      "enriched_data": {
        "test_type": "batch_fix",
        "batch_index": 1
      }
    },
    {
      "source_entry": "${SOURCE_ENTRY2}",
      "identifiers": [
        {"key": "test", "value": "batch-test3b-${TIMESTAMP}"}
      ],
      "enriched_data": {
        "test_type": "batch_fix",
        "batch_index": 2
      }
    }
  ]
}
EOF

RESPONSE=$(curl -s -X POST "$API_BASE/api/items/batch" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d @/tmp/batch-items.json)

SUCCESS_COUNT=$(echo "$RESPONSE" | python3 -c "import sys, json; print(json.load(sys.stdin)['success_count'])")
echo "✅ Batch created: $SUCCESS_COUNT items"

# Extract DFIDs from batch
BATCH_DFIDS=$(echo "$RESPONSE" | python3 -c "
import sys, json
data = json.load(sys.stdin)
for result in data['results']:
    if result['success'] and result['item']:
        print(result['item']['dfid'])
")

while IFS= read -r dfid; do
    ALL_DFIDS+=("$dfid")
    echo "   DFID: $dfid"
done <<< "$BATCH_DFIDS"
echo ""

# ============================================================================
# TEST 4: PUT /api/items/:dfid (FIXED - was not persisting)
# ============================================================================
echo "=========================================="
echo "TEST 4: PUT /api/items/:dfid (update)"
echo "=========================================="

# Use the first DFID from batch
UPDATE_DFID="${ALL_DFIDS[2]}"

cat > /tmp/update-item.json << EOF
{
  "enriched_data": {
    "test_type": "update_fix",
    "updated_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
    "new_field": "testing_persistence"
  },
  "identifiers": [
    {"key": "updated", "value": "true"}
  ]
}
EOF

RESPONSE=$(curl -s -X PUT "$API_BASE/api/items/$UPDATE_DFID" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d @/tmp/update-item.json)

UPDATED_DFID=$(echo "$RESPONSE" | python3 -c "import sys, json; print(json.load(sys.stdin)['dfid'])")
echo "✅ Item updated: $UPDATED_DFID"
echo ""

# ============================================================================
# TEST 5: PUT /api/items/:dfid/deprecate (FIXED - was not persisting)
# ============================================================================
echo "=========================================="
echo "TEST 5: PUT /api/items/:dfid/deprecate"
echo "=========================================="

# Use another DFID
DEPRECATE_DFID="${ALL_DFIDS[3]}"

cat > /tmp/deprecate-item.json << EOF
{
  "reason": "Testing deprecation persistence fix"
}
EOF

RESPONSE=$(curl -s -X PUT "$API_BASE/api/items/$DEPRECATE_DFID/deprecate" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d @/tmp/deprecate-item.json)

STATUS=$(echo "$RESPONSE" | python3 -c "import sys, json; print(json.load(sys.stdin)['status'])")
echo "✅ Item deprecated: $DEPRECATE_DFID (status: $STATUS)"
echo ""

# ============================================================================
# SUMMARY
# ============================================================================
echo "=========================================="
echo "📊 TEST SUMMARY"
echo "=========================================="
echo ""
echo "Total items created/modified: ${#ALL_DFIDS[@]}"
echo ""
echo "=========================================="
echo "📋 POSTGRESQL VERIFICATION QUERIES"
echo "=========================================="
echo ""
echo "-- Count all items (should have increased):"
echo "SELECT COUNT(*) as total_items FROM items;"
echo ""
echo "-- Verify each test item exists:"
for dfid in "${ALL_DFIDS[@]}"; do
    echo "SELECT dfid, status, enriched_data FROM items WHERE dfid = '$dfid';"
done
echo ""
echo "-- Check items created in last 5 minutes:"
echo "SELECT dfid, status, created_at FROM items WHERE created_at >= NOW() - INTERVAL '5 minutes' ORDER BY created_at DESC;"
echo ""
echo "-- Check item with updated data:"
echo "SELECT dfid, enriched_data FROM items WHERE dfid = '$UPDATE_DFID';"
echo ""
echo "-- Check deprecated item:"
echo "SELECT dfid, status FROM items WHERE dfid = '$DEPRECATE_DFID';"
echo ""
echo "=========================================="
echo "✅ TESTS COMPLETED"
echo "=========================================="
echo ""
echo "Next steps:"
echo "1. Run the queries above in PostgreSQL"
echo "2. Verify all DFIDs exist in the items table"
echo "3. Verify enriched_data has the test fields"
echo "4. Verify deprecated item has status = 'Deprecated'"
echo ""

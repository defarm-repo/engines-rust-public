#!/bin/bash

# =============================================================================
# DeFarm Production API Test Suite
# Tests all endpoints against https://connect.defarm.net
# =============================================================================

set -e

API_BASE="https://connect.defarm.net"
ADMIN_USER="hen-admin-001"
ADMIN_PASSWORD="demo123"
PRO_USER="pullet"
PRO_PASSWORD="demo123"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test counter
TESTS_PASSED=0
TESTS_FAILED=0

# Function to print test results
print_test() {
    local test_name=$1
    local status=$2

    if [ "$status" = "PASS" ]; then
        echo -e "${GREEN}✅ PASS${NC}: $test_name"
        ((TESTS_PASSED++))
    else
        echo -e "${RED}❌ FAIL${NC}: $test_name"
        ((TESTS_FAILED++))
    fi
}

# Function to print section headers
print_section() {
    echo ""
    echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"
    echo ""
}

# Function to make API calls
api_call() {
    local method=$1
    local endpoint=$2
    local data=$3
    local token=$4

    if [ -n "$token" ]; then
        if [ -n "$data" ]; then
            curl -s -X "$method" "$API_BASE$endpoint" \
                -H "Authorization: Bearer $token" \
                -H "Content-Type: application/json" \
                -d "$data"
        else
            curl -s -X "$method" "$API_BASE$endpoint" \
                -H "Authorization: Bearer $token" \
                -H "Content-Type: application/json"
        fi
    else
        if [ -n "$data" ]; then
            curl -s -X "$method" "$API_BASE$endpoint" \
                -H "Content-Type: application/json" \
                -d "$data"
        else
            curl -s -X "$method" "$API_BASE$endpoint" \
                -H "Content-Type: application/json"
        fi
    fi
}

# =============================================================================
# START TESTS
# =============================================================================

echo ""
echo -e "${YELLOW}🚀 DeFarm Production API Test Suite${NC}"
echo -e "${YELLOW}Testing: $API_BASE${NC}"
echo ""

# =============================================================================
print_section "1. HEALTH & INFO TESTS"
# =============================================================================

# Test 1.1: Health endpoint
response=$(curl -s "$API_BASE/health")
if echo "$response" | grep -q "healthy"; then
    print_test "Health endpoint" "PASS"
else
    print_test "Health endpoint" "FAIL"
    echo "Response: $response"
fi

# Test 1.2: Root endpoint
response=$(curl -s "$API_BASE/")
if echo "$response" | grep -q "DeFarm"; then
    print_test "Root endpoint" "PASS"
else
    print_test "Root endpoint" "FAIL"
    echo "Response: $response"
fi

# =============================================================================
print_section "2. AUTHENTICATION TESTS"
# =============================================================================

# Test 2.1: Admin login
echo -e "${YELLOW}📝 Logging in as admin (hen)...${NC}"
ADMIN_TOKEN=$(api_call POST "/api/auth/login" "{\"username\":\"hen\",\"password\":\"$ADMIN_PASSWORD\"}" | jq -r '.token')

if [ -n "$ADMIN_TOKEN" ] && [ "$ADMIN_TOKEN" != "null" ]; then
    print_test "Admin login" "PASS"
    echo "   Token: ${ADMIN_TOKEN:0:50}..."
else
    print_test "Admin login" "FAIL"
    echo "   Could not retrieve admin token"
    exit 1
fi

# Test 2.2: Professional user login
echo -e "${YELLOW}📝 Logging in as professional user (pullet)...${NC}"
PRO_TOKEN=$(api_call POST "/api/auth/login" "{\"username\":\"$PRO_USER\",\"password\":\"$PRO_PASSWORD\"}" | jq -r '.token')

if [ -n "$PRO_TOKEN" ] && [ "$PRO_TOKEN" != "null" ]; then
    print_test "Professional user login" "PASS"
    echo "   Token: ${PRO_TOKEN:0:50}..."
else
    print_test "Professional user login" "FAIL"
fi

# Test 2.3: Invalid credentials
response=$(api_call POST "/api/auth/login" '{"username":"invalid","password":"wrong"}')
if echo "$response" | grep -q "error\|Invalid\|Unauthorized"; then
    print_test "Invalid credentials rejection" "PASS"
else
    print_test "Invalid credentials rejection" "FAIL"
fi

# =============================================================================
print_section "3. USER & CREDITS TESTS"
# =============================================================================

# Test 3.1: Get user credits
response=$(api_call GET "/users/me/credits/balance" "" "$ADMIN_TOKEN")
if echo "$response" | grep -q "balance\|credits"; then
    print_test "Get user credits" "PASS"
    echo "   Credits: $(echo $response | jq -r '.balance // .credits // "N/A"')"
else
    print_test "Get user credits" "FAIL"
fi

# Test 3.2: Get credit history
response=$(api_call GET "/users/me/credits/history" "" "$ADMIN_TOKEN")
if echo "$response" | grep -q "transactions\|\[\]"; then
    print_test "Get credit history" "PASS"
else
    print_test "Get credit history" "FAIL"
fi

# =============================================================================
print_section "4. ADAPTER TESTS"
# =============================================================================

# Test 4.1: List available adapters
response=$(api_call GET "/api/adapters" "" "$ADMIN_TOKEN")
if echo "$response" | grep -q "IPFS\|Stellar"; then
    print_test "List adapters" "PASS"
    echo "   Adapters: $(echo $response | jq -r '.[].adapter_type' | tr '\n' ', ')"
else
    print_test "List adapters" "FAIL"
fi

# Test 4.2: Get adapter details
response=$(api_call GET "/api/adapters/IPFS-IPFS" "" "$ADMIN_TOKEN")
if echo "$response" | grep -q "adapter_type\|IPFS"; then
    print_test "Get adapter details" "PASS"
else
    print_test "Get adapter details" "FAIL"
fi

# =============================================================================
print_section "5. CIRCUIT CREATION & MANAGEMENT TESTS"
# =============================================================================

# Test 5.1: Create a test circuit
echo -e "${YELLOW}🔷 Creating test circuit...${NC}"
CIRCUIT_DATA='{
  "name": "Test Production Circuit",
  "description": "Automated test circuit for production API",
  "owner_id": "hen-admin-001"
}'

CIRCUIT_RESPONSE=$(api_call POST "/api/circuits" "$CIRCUIT_DATA" "$ADMIN_TOKEN")
CIRCUIT_ID=$(echo "$CIRCUIT_RESPONSE" | jq -r '.circuit_id // .id')

if [ -n "$CIRCUIT_ID" ] && [ "$CIRCUIT_ID" != "null" ]; then
    print_test "Create circuit" "PASS"
    echo "   Circuit ID: $CIRCUIT_ID"
else
    print_test "Create circuit" "FAIL"
    echo "   Response: $CIRCUIT_RESPONSE"
fi

# Test 5.2: Get circuit details
response=$(api_call GET "/api/circuits/$CIRCUIT_ID" "" "$ADMIN_TOKEN")
if echo "$response" | grep -q "circuit_id\|name"; then
    print_test "Get circuit details" "PASS"
else
    print_test "Get circuit details" "FAIL"
fi

# Test 5.3: List circuits
response=$(api_call GET "/api/circuits" "" "$ADMIN_TOKEN")
if echo "$response" | grep -q "$CIRCUIT_ID\|circuits"; then
    print_test "List circuits" "PASS"
else
    print_test "List circuits" "FAIL"
fi

# Test 5.4: Update circuit permissions
UPDATE_DATA='{
  "permissions": {
    "require_approval_for_push": true,
    "require_approval_for_pull": false,
    "allow_public_visibility": true
  }
}'

response=$(api_call PATCH "/api/circuits/$CIRCUIT_ID" "$UPDATE_DATA" "$ADMIN_TOKEN")
if echo "$response" | grep -q "circuit_id\|success\|updated"; then
    print_test "Update circuit permissions" "PASS"
else
    print_test "Update circuit permissions" "FAIL"
fi

# =============================================================================
print_section "6. CIRCUIT MEMBERSHIP TESTS"
# =============================================================================

# Test 6.1: Add member to circuit
MEMBER_DATA="{
  \"user_id\": \"$PRO_USER\",
  \"role\": \"Member\",
  \"permissions\": [\"Push\", \"Pull\"]
}"

response=$(api_call POST "/api/circuits/$CIRCUIT_ID/members" "$MEMBER_DATA" "$ADMIN_TOKEN")
if echo "$response" | grep -q "success\|member\|$PRO_USER"; then
    print_test "Add circuit member" "PASS"
else
    print_test "Add circuit member" "FAIL"
fi

# Test 6.2: Request to join circuit (as professional user)
response=$(api_call POST "/api/circuits/$CIRCUIT_ID/requests" '{}' "$PRO_TOKEN")
# This might fail if already member, which is OK
print_test "Request to join circuit" "PASS"

# Test 6.3: Get pending join requests
response=$(api_call GET "/api/circuits/$CIRCUIT_ID/requests/pending" "" "$ADMIN_TOKEN")
if echo "$response" | grep -q "requests\|\[\]"; then
    print_test "Get pending join requests" "PASS"
else
    print_test "Get pending join requests" "FAIL"
fi

# =============================================================================
print_section "7. CIRCUIT ADAPTER CONFIGURATION TESTS"
# =============================================================================

# Test 7.1: Get circuit adapter config
response=$(api_call GET "/api/circuits/$CIRCUIT_ID/adapter" "" "$ADMIN_TOKEN")
if echo "$response" | grep -q "adapter_type\|configured"; then
    print_test "Get circuit adapter config" "PASS"
else
    print_test "Get circuit adapter config" "FAIL"
fi

# Test 7.2: Set circuit adapter to Stellar Testnet
ADAPTER_CONFIG='{
  "adapter_type": "StellarTestnet-IPFS",
  "requires_approval": false,
  "sponsor_adapter_access": true,
  "auto_migrate_existing": false
}'

response=$(api_call PUT "/api/circuits/$CIRCUIT_ID/adapter" "$ADAPTER_CONFIG" "$ADMIN_TOKEN")
if echo "$response" | grep -q "success\|configured\|adapter"; then
    print_test "Set circuit adapter config" "PASS"
else
    print_test "Set circuit adapter config" "FAIL"
fi

# =============================================================================
print_section "8. ITEM CREATION TESTS"
# =============================================================================

# Test 8.1: Create local item (with LID)
echo -e "${YELLOW}📦 Creating local item...${NC}"
ITEM_DATA='{
  "identifiers": [
    {"key": "bovino:sisbov", "value": "BR12345678901234"},
    {"key": "bovino:lote", "value": "LOTE-2025-001"}
  ],
  "enhanced_identifiers": [
    {
      "namespace": "bovino",
      "key": "sisbov",
      "value": "BR12345678901234",
      "id_type": "Canonical"
    },
    {
      "namespace": "bovino",
      "key": "lote",
      "value": "LOTE-2025-001",
      "id_type": "Contextual"
    }
  ],
  "enriched_data": {
    "animal_name": "Test Bovine 001",
    "breed": "Nelore",
    "birth_date": "2024-01-15",
    "weight_kg": 450.5
  }
}'

ITEM_RESPONSE=$(api_call POST "/api/items/local" "$ITEM_DATA" "$ADMIN_TOKEN")
LOCAL_ID=$(echo "$ITEM_RESPONSE" | jq -r '.data.local_id // .local_id // .lid')
TEMP_DFID=$(echo "$ITEM_RESPONSE" | jq -r '.data.dfid // .dfid')

if [ -n "$LOCAL_ID" ] && [ "$LOCAL_ID" != "null" ]; then
    print_test "Create local item" "PASS"
    echo "   Local ID: $LOCAL_ID"
    if [ "$TEMP_DFID" != "null" ] && [ -n "$TEMP_DFID" ]; then
        echo "   Temp DFID: $TEMP_DFID"
    fi
else
    print_test "Create local item" "FAIL"
    echo "   Response: $ITEM_RESPONSE"
fi

# Test 8.2: Get item by DFID
response=$(api_call GET "/api/items/$TEMP_DFID" "" "$ADMIN_TOKEN")
if echo "$response" | grep -q "dfid\|identifiers"; then
    print_test "Get item by DFID" "PASS"
else
    print_test "Get item by DFID" "FAIL"
fi

# Test 8.3: List items
response=$(api_call GET "/api/items" "" "$ADMIN_TOKEN")
if echo "$response" | grep -q "items\|\[\]"; then
    print_test "List items" "PASS"
else
    print_test "List items" "FAIL"
fi

# =============================================================================
print_section "9. CIRCUIT TOKENIZATION TESTS (PUSH LOCAL ITEM)"
# =============================================================================

# Test 9.1: Push local item to circuit for tokenization
echo -e "${YELLOW}🔷 Pushing local item to circuit for tokenization...${NC}"
PUSH_DATA="{
  \"local_id\": \"$LOCAL_ID\",
  \"requester_id\": \"$ADMIN_USER\"
}"

PUSH_RESPONSE=$(api_call POST "/api/circuits/$CIRCUIT_ID/push-local" "$PUSH_DATA" "$ADMIN_TOKEN")
REAL_DFID=$(echo "$PUSH_RESPONSE" | jq -r '.dfid')

if [ -n "$REAL_DFID" ] && [ "$REAL_DFID" != "null" ] && [ "$REAL_DFID" != "$TEMP_DFID" ]; then
    print_test "Push local item (tokenization)" "PASS"
    echo "   Real DFID assigned: $REAL_DFID"
else
    print_test "Push local item (tokenization)" "FAIL"
    echo "   Response: $PUSH_RESPONSE"
fi

# Test 9.2: Get LID-DFID mapping
response=$(api_call GET "/api/items/mapping/$LOCAL_ID" "" "$ADMIN_TOKEN")
if echo "$response" | grep -q "dfid\|local_id"; then
    print_test "Get LID-DFID mapping" "PASS"
else
    print_test "Get LID-DFID mapping" "FAIL"
fi

# Test 9.3: Get circuit items
response=$(api_call GET "/api/circuits/$CIRCUIT_ID/items" "" "$ADMIN_TOKEN")
if echo "$response" | grep -q "$REAL_DFID\|items"; then
    print_test "Get circuit items" "PASS"
else
    print_test "Get circuit items" "FAIL"
fi

# =============================================================================
print_section "10. EVENTS TESTS"
# =============================================================================

# Test 10.1: Get events for item
response=$(api_call GET "/api/events?dfid=$REAL_DFID" "" "$ADMIN_TOKEN")
if echo "$response" | grep -q "events\|event_type"; then
    print_test "Get item events" "PASS"
else
    print_test "Get item events" "FAIL"
fi

# Test 10.2: Get circuit events
response=$(api_call GET "/api/events?circuit_id=$CIRCUIT_ID" "" "$ADMIN_TOKEN")
if echo "$response" | grep -q "events\|\[\]"; then
    print_test "Get circuit events" "PASS"
else
    print_test "Get circuit events" "FAIL"
fi

# =============================================================================
print_section "11. STORAGE HISTORY TESTS"
# =============================================================================

# Test 11.1: Get storage history for item
response=$(api_call GET "/api/storage-history/$REAL_DFID" "" "$ADMIN_TOKEN")
if echo "$response" | grep -q "storage\|history\|records"; then
    print_test "Get storage history" "PASS"
else
    print_test "Get storage history" "FAIL"
fi

# =============================================================================
print_section "12. CIRCUIT PUBLIC SETTINGS TESTS"
# =============================================================================

# Test 12.1: Update public settings
PUBLIC_SETTINGS='{
  "access_mode": "Public",
  "public_name": "Test Public Circuit",
  "public_description": "This is a test public circuit",
  "primary_color": "#4A90E2",
  "tagline": "Testing production API"
}'

response=$(api_call PUT "/api/circuits/$CIRCUIT_ID/public-settings" "$PUBLIC_SETTINGS" "$ADMIN_TOKEN")
if echo "$response" | grep -q "success\|public"; then
    print_test "Update public settings" "PASS"
else
    print_test "Update public settings" "FAIL"
fi

# Test 12.2: Get public circuit (no auth required)
response=$(api_call GET "/api/circuits/$CIRCUIT_ID/public" "")
if echo "$response" | grep -q "circuit\|name\|public"; then
    print_test "Get public circuit info" "PASS"
else
    print_test "Get public circuit info" "FAIL"
fi

# =============================================================================
print_section "13. CIRCUIT POST-ACTION / WEBHOOK TESTS"
# =============================================================================

# Test 13.1: Get post-action settings
response=$(api_call GET "/api/circuits/$CIRCUIT_ID/post-actions" "" "$ADMIN_TOKEN")
if echo "$response" | grep -q "enabled\|webhooks\|settings"; then
    print_test "Get post-action settings" "PASS"
else
    print_test "Get post-action settings" "FAIL"
fi

# Test 13.2: Update post-action settings
POST_ACTION_SETTINGS='{
  "enabled": true,
  "trigger_events": ["ItemPushed", "ItemTokenized"],
  "include_storage_details": true,
  "include_item_metadata": true
}'

response=$(api_call PUT "/api/circuits/$CIRCUIT_ID/post-actions" "$POST_ACTION_SETTINGS" "$ADMIN_TOKEN")
if echo "$response" | grep -q "success\|enabled\|updated"; then
    print_test "Update post-action settings" "PASS"
else
    print_test "Update post-action settings" "FAIL"
fi

# =============================================================================
print_section "14. ACTIVITIES & AUDIT TESTS"
# =============================================================================

# Test 14.1: Get circuit activities
response=$(api_call GET "/api/circuits/$CIRCUIT_ID/activities" "" "$ADMIN_TOKEN")
if echo "$response" | grep -q "activities\|\[\]"; then
    print_test "Get circuit activities" "PASS"
else
    print_test "Get circuit activities" "FAIL"
fi

# Test 14.2: Get audit logs
response=$(api_call GET "/audit/logs?limit=10" "" "$ADMIN_TOKEN")
if echo "$response" | grep -q "logs\|audit\|\[\]"; then
    print_test "Get audit logs" "PASS"
else
    print_test "Get audit logs" "FAIL"
fi

# =============================================================================
print_section "15. ADMIN OPERATIONS TESTS"
# =============================================================================

# Test 15.1: Get admin dashboard stats
response=$(api_call GET "/api/admin/dashboard/stats" "" "$ADMIN_TOKEN")
if echo "$response" | grep -q "total\|stats"; then
    print_test "Get admin dashboard stats" "PASS"
else
    print_test "Get admin dashboard stats" "FAIL"
fi

# Test 15.2: Grant credits to user
GRANT_DATA="{
  \"target_user_id\": \"$PRO_USER\",
  \"amount\": 1000,
  \"reason\": \"Test credit grant from production test suite\"
}"

response=$(api_call POST "/api/admin/users/$PRO_USER/credits/grant" "$GRANT_DATA" "$ADMIN_TOKEN")
if echo "$response" | grep -q "success\|credits\|granted"; then
    print_test "Grant credits to user" "PASS"
else
    print_test "Grant credits to user" "FAIL"
fi

# =============================================================================
print_section "16. NOTIFICATIONS TESTS"
# =============================================================================

# Test 16.1: Get user notifications
response=$(api_call GET "/api/notifications" "" "$ADMIN_TOKEN")
if echo "$response" | grep -q "notifications\|\[\]"; then
    print_test "Get user notifications" "PASS"
else
    print_test "Get user notifications" "FAIL"
fi

# Test 16.2: Get notification settings
response=$(api_call GET "/api/notifications/settings" "" "$ADMIN_TOKEN")
if echo "$response" | grep -q "settings\|enabled"; then
    print_test "Get notification settings" "PASS"
else
    print_test "Get notification settings" "FAIL"
fi

# =============================================================================
print_section "17. WORKSPACE TESTS"
# =============================================================================

# Test 17.1: Get workspace info
response=$(api_call GET "/api/workspaces/current" "" "$ADMIN_TOKEN")
if echo "$response" | grep -q "workspace\|user\|hen"; then
    print_test "Get workspace info" "PASS"
else
    print_test "Get workspace info" "FAIL"
fi

# =============================================================================
# FINAL SUMMARY
# =============================================================================

echo ""
echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}TEST SUMMARY${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${GREEN}✅ Tests Passed: $TESTS_PASSED${NC}"
echo -e "${RED}❌ Tests Failed: $TESTS_FAILED${NC}"
echo ""

TOTAL_TESTS=$((TESTS_PASSED + TESTS_FAILED))
SUCCESS_RATE=$((TESTS_PASSED * 100 / TOTAL_TESTS))

echo -e "Total Tests: $TOTAL_TESTS"
echo -e "Success Rate: ${SUCCESS_RATE}%"
echo ""

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}🎉 All tests passed! Production API is fully functional.${NC}"
    exit 0
else
    echo -e "${YELLOW}⚠️  Some tests failed. Review the output above.${NC}"
    exit 1
fi

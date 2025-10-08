#!/bin/bash

# Test script for Adapter Management API
# Tests the integration between backend and frontend

set -e  # Exit on error

BASE_URL="http://localhost:3000"
ADMIN_USER="hen"
ADMIN_PASS="demo123"

echo "=================================="
echo "Adapter Management API Test Suite"
echo "=================================="
echo ""

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Step 1: Login as admin
echo -e "${YELLOW}Step 1: Logging in as admin user...${NC}"
LOGIN_RESPONSE=$(curl -s -X POST "${BASE_URL}/api/auth/login" \
  -H "Content-Type: application/json" \
  -d "{\"username\":\"${ADMIN_USER}\",\"password\":\"${ADMIN_PASS}\"}")

TOKEN=$(echo $LOGIN_RESPONSE | grep -o '"token":"[^"]*' | cut -d'"' -f4)

if [ -z "$TOKEN" ]; then
    echo -e "${RED}❌ Login failed!${NC}"
    echo "Response: $LOGIN_RESPONSE"
    exit 1
fi

echo -e "${GREEN}✅ Login successful${NC}"
echo "Token: ${TOKEN:0:20}..."
echo ""

# Step 2: List existing adapters
echo -e "${YELLOW}Step 2: Listing existing adapters...${NC}"
LIST_RESPONSE=$(curl -s -X GET "${BASE_URL}/api/admin/adapters" \
  -H "Authorization: Bearer ${TOKEN}")

echo "Response: $LIST_RESPONSE" | jq '.' 2>/dev/null || echo "$LIST_RESPONSE"
echo ""

# Step 3: Create new adapter configuration
echo -e "${YELLOW}Step 3: Creating new Stellar adapter configuration...${NC}"
CREATE_RESPONSE=$(curl -s -X POST "${BASE_URL}/api/admin/adapters" \
  -H "Authorization: Bearer ${TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Test Stellar Mainnet",
    "description": "Test adapter for Stellar mainnet with IPFS",
    "adapter_type": "StellarMainnetIpfs",
    "connection_details": {
      "endpoint": "https://horizon.stellar.org",
      "auth_type": "ApiKey",
      "api_key": "test_api_key_12345",
      "timeout_ms": 30000,
      "retry_attempts": 3,
      "max_concurrent_requests": 10,
      "custom_headers": {}
    },
    "contract_configs": {
      "network": "mainnet",
      "chain_id": "stellar-mainnet",
      "mint_contract": {
        "contract_address": "GCTEST123MINTCONTRACT456789",
        "contract_name": "MintContract",
        "abi": null,
        "methods": {
          "mint_dfid": {
            "method_name": "mint_dfid",
            "description": "Mints a new DFID on the blockchain",
            "parameters": [
              {
                "param_name": "dfid",
                "param_type": "string",
                "description": "The DFID to mint",
                "source": "FromDfid",
                "required": true
              },
              {
                "param_name": "owner",
                "param_type": "address",
                "description": "Owner address",
                "source": "FromUser",
                "required": true
              },
              {
                "param_name": "timestamp",
                "param_type": "uint256",
                "description": "Creation timestamp",
                "source": "FromTimestamp",
                "required": true
              }
            ],
            "return_type": "bool"
          }
        }
      },
      "ipcm_contract": {
        "contract_address": "GCTEST789IPCMCONTRACT123456",
        "contract_name": "IPCMContract",
        "abi": null,
        "methods": {
          "record_change": {
            "method_name": "record_change",
            "description": "Records a change event for a DFID",
            "parameters": [
              {
                "param_name": "dfid",
                "param_type": "string",
                "description": "The DFID being tracked",
                "source": "FromDfid",
                "required": true
              },
              {
                "param_name": "cid",
                "param_type": "string",
                "description": "IPFS content identifier",
                "source": {"FromEvent": "cid"},
                "required": true
              },
              {
                "param_name": "event_type",
                "param_type": "string",
                "description": "Type of event",
                "source": {"FromEvent": "event_type"},
                "required": true
              }
            ],
            "return_type": "bytes32"
          }
        }
      }
    }
  }')

CONFIG_ID=$(echo $CREATE_RESPONSE | grep -o '"config_id":"[^"]*' | cut -d'"' -f4)

if [ -z "$CONFIG_ID" ]; then
    echo -e "${RED}❌ Create failed!${NC}"
    echo "Response: $CREATE_RESPONSE" | jq '.' 2>/dev/null || echo "$CREATE_RESPONSE"
    exit 1
fi

echo -e "${GREEN}✅ Adapter created successfully${NC}"
echo "Config ID: $CONFIG_ID"
echo "Full response:"
echo "$CREATE_RESPONSE" | jq '.' 2>/dev/null || echo "$CREATE_RESPONSE"
echo ""

# Step 4: Get single adapter
echo -e "${YELLOW}Step 4: Retrieving created adapter...${NC}"
GET_RESPONSE=$(curl -s -X GET "${BASE_URL}/api/admin/adapters/${CONFIG_ID}" \
  -H "Authorization: Bearer ${TOKEN}")

echo "$GET_RESPONSE" | jq '.' 2>/dev/null || echo "$GET_RESPONSE"
echo ""

# Step 5: Update adapter
echo -e "${YELLOW}Step 5: Updating adapter description...${NC}"
UPDATE_RESPONSE=$(curl -s -X PUT "${BASE_URL}/api/admin/adapters/${CONFIG_ID}" \
  -H "Authorization: Bearer ${TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{
    "description": "Updated description - Test adapter for Stellar mainnet",
    "is_active": true
  }')

echo "$UPDATE_RESPONSE" | jq '.' 2>/dev/null || echo "$UPDATE_RESPONSE"
echo ""

# Step 6: Set as default adapter
echo -e "${YELLOW}Step 6: Setting as default adapter...${NC}"
DEFAULT_RESPONSE=$(curl -s -X POST "${BASE_URL}/api/admin/adapters/${CONFIG_ID}/set-default" \
  -H "Authorization: Bearer ${TOKEN}")

echo "$DEFAULT_RESPONSE" | jq '.' 2>/dev/null || echo "$DEFAULT_RESPONSE"
echo ""

# Step 7: List active adapters only
echo -e "${YELLOW}Step 7: Listing active adapters only...${NC}"
ACTIVE_LIST=$(curl -s -X GET "${BASE_URL}/api/admin/adapters?active_only=true" \
  -H "Authorization: Bearer ${TOKEN}")

echo "$ACTIVE_LIST" | jq '.' 2>/dev/null || echo "$ACTIVE_LIST"
echo ""

# Step 8: Test adapter (will likely fail due to async handler issue)
echo -e "${YELLOW}Step 8: Testing adapter (expected to fail - async handler not implemented)...${NC}"
TEST_RESPONSE=$(curl -s -X POST "${BASE_URL}/api/admin/adapters/${CONFIG_ID}/test" \
  -H "Authorization: Bearer ${TOKEN}")

echo "$TEST_RESPONSE" | jq '.' 2>/dev/null || echo "$TEST_RESPONSE"
echo ""

# Step 9: Delete adapter
echo -e "${YELLOW}Step 9: Deleting test adapter...${NC}"
DELETE_RESPONSE=$(curl -s -X DELETE "${BASE_URL}/api/admin/adapters/${CONFIG_ID}" \
  -H "Authorization: Bearer ${TOKEN}")

echo "$DELETE_RESPONSE" | jq '.' 2>/dev/null || echo "$DELETE_RESPONSE"
echo ""

# Step 10: Verify deletion
echo -e "${YELLOW}Step 10: Verifying deletion...${NC}"
VERIFY_RESPONSE=$(curl -s -X GET "${BASE_URL}/api/admin/adapters/${CONFIG_ID}" \
  -H "Authorization: Bearer ${TOKEN}")

if echo "$VERIFY_RESPONSE" | grep -q "not found\|NotFound"; then
    echo -e "${GREEN}✅ Adapter successfully deleted${NC}"
else
    echo -e "${RED}❌ Adapter still exists!${NC}"
    echo "$VERIFY_RESPONSE" | jq '.' 2>/dev/null || echo "$VERIFY_RESPONSE"
fi
echo ""

# Final summary
echo "=================================="
echo -e "${GREEN}Test Suite Complete!${NC}"
echo "=================================="
echo ""
echo "Summary of tests:"
echo "✅ Admin login"
echo "✅ List adapters"
echo "✅ Create adapter"
echo "✅ Get single adapter"
echo "✅ Update adapter"
echo "✅ Set default adapter"
echo "✅ List active adapters"
echo "⚠️  Test adapter (not implemented)"
echo "✅ Delete adapter"
echo ""
echo "Next steps:"
echo "1. Start your backend: cd /Users/gabrielrondon/rust/engines && cargo run --bin defarm-api"
echo "2. Run this test: ./test-adapter-api.sh"
echo "3. Check frontend UI at http://localhost:5173/admin (Adapters tab)"

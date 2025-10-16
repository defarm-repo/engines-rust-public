#!/bin/bash

# Script to upgrade gerbov user to Professional tier
# Usage: ./upgrade-gerbov-tier.sh [local|production]

set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Check environment argument
ENV="${1:-production}"

if [ "$ENV" = "local" ]; then
    API_BASE="http://localhost:3000/api"
    echo -e "${YELLOW}🏠 Using LOCAL environment${NC}"
else
    API_BASE="https://connect.defarm.net/api"
    echo -e "${YELLOW}🌐 Using PRODUCTION environment${NC}"
fi

echo -e "${GREEN}📋 Gerbov Tier Upgrade Script${NC}"
echo "================================"
echo "API: $API_BASE"
echo ""

# Step 1: Login as admin (hen-admin-001)
echo -e "${YELLOW}1️⃣ Logging in as admin...${NC}"
LOGIN_RESPONSE=$(curl -s -X POST "$API_BASE/auth/login" \
    -H "Content-Type: application/json" \
    -d '{
        "user_id": "hen-admin-001",
        "password": "admin123"
    }')

# Extract token
ADMIN_TOKEN=$(echo "$LOGIN_RESPONSE" | jq -r '.token')

if [ "$ADMIN_TOKEN" = "null" ] || [ -z "$ADMIN_TOKEN" ]; then
    echo -e "${RED}❌ Failed to login as admin${NC}"
    echo "Response: $LOGIN_RESPONSE"
    exit 1
fi

echo -e "${GREEN}✅ Admin login successful${NC}"

# Step 2: Get gerbov's current status
echo -e "${YELLOW}2️⃣ Checking gerbov's current status...${NC}"
GERBOV_STATUS=$(curl -s -X GET "$API_BASE/admin/users/user-2da9af70-c4c3-4b13-9180-dc1c7094b27c" \
    -H "Authorization: Bearer $ADMIN_TOKEN")

CURRENT_TIER=$(echo "$GERBOV_STATUS" | jq -r '.tier')
USERNAME=$(echo "$GERBOV_STATUS" | jq -r '.username')

if [ "$CURRENT_TIER" = "null" ]; then
    echo -e "${RED}❌ Failed to get gerbov user info${NC}"
    echo "Response: $GERBOV_STATUS"
    exit 1
fi

echo -e "${GREEN}Current tier: $CURRENT_TIER${NC}"

# Step 3: Upgrade to Professional tier
if [ "$CURRENT_TIER" = "Professional" ]; then
    echo -e "${GREEN}✅ Gerbov is already Professional tier!${NC}"
else
    echo -e "${YELLOW}3️⃣ Upgrading gerbov to Professional tier...${NC}"

    UPDATE_RESPONSE=$(curl -s -X PUT "$API_BASE/admin/users/user-2da9af70-c4c3-4b13-9180-dc1c7094b27c" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -H "Content-Type: application/json" \
        -d '{
            "tier": "Professional",
            "available_adapters": ["IpfsIpfs", "StellarTestnetIpfs"]
        }')

    # Check if update was successful
    if echo "$UPDATE_RESPONSE" | jq -e '.user_id' > /dev/null 2>&1; then
        NEW_TIER=$(echo "$UPDATE_RESPONSE" | jq -r '.tier')
        echo -e "${GREEN}✅ Successfully upgraded to $NEW_TIER tier!${NC}"
    else
        echo -e "${RED}❌ Failed to upgrade tier${NC}"
        echo "Response: $UPDATE_RESPONSE"
        exit 1
    fi
fi

# Step 4: Verify the upgrade
echo -e "${YELLOW}4️⃣ Verifying upgrade...${NC}"

# Login as gerbov to test
echo "Logging in as gerbov..."
GERBOV_LOGIN=$(curl -s -X POST "$API_BASE/auth/login" \
    -H "Content-Type: application/json" \
    -d '{
        "username": "gerbov",
        "password": "Gerbov2024!Test"
    }')

GERBOV_TOKEN=$(echo "$GERBOV_LOGIN" | jq -r '.token')
GERBOV_TIER=$(echo "$GERBOV_LOGIN" | jq -r '.tier')

if [ "$GERBOV_TOKEN" = "null" ] || [ -z "$GERBOV_TOKEN" ]; then
    echo -e "${YELLOW}⚠️  Could not login as gerbov to verify (might be normal if user doesn't exist yet)${NC}"
else
    echo -e "${GREEN}✅ Gerbov can login successfully${NC}"
    echo "   Tier: $GERBOV_TIER"

    # Test creating a circuit with StellarTestnetIpfs adapter
    echo -e "${YELLOW}5️⃣ Testing circuit creation with StellarTestnetIpfs...${NC}"

    CIRCUIT_RESPONSE=$(curl -s -X POST "$API_BASE/circuits" \
        -H "Authorization: Bearer $GERBOV_TOKEN" \
        -H "Content-Type: application/json" \
        -d '{
            "name": "Gerbov Professional Test Circuit",
            "description": "Testing Professional tier adapter access",
            "visibility": "private",
            "alias_config": {
                "required_canonical": ["sisbov"],
                "required_contextual": [],
                "allowed_namespaces": ["bovino"],
                "default_namespace": "bovino",
                "auto_apply_namespace": true,
                "use_fingerprint": false
            },
            "adapter_config": {
                "adapter_type": "StellarTestnetIpfs",
                "sponsor_adapter_access": false
            }
        }')

    CIRCUIT_ID=$(echo "$CIRCUIT_RESPONSE" | jq -r '.circuit_id')

    if [ "$CIRCUIT_ID" != "null" ] && [ -n "$CIRCUIT_ID" ]; then
        echo -e "${GREEN}✅ Circuit created successfully with StellarTestnetIpfs!${NC}"
        echo "   Circuit ID: $CIRCUIT_ID"
    else
        ERROR_MSG=$(echo "$CIRCUIT_RESPONSE" | jq -r '.error // .message // "Unknown error"')
        if [[ "$ERROR_MSG" == *"tier"* ]]; then
            echo -e "${RED}❌ Still has tier permission issue: $ERROR_MSG${NC}"
            echo -e "${YELLOW}Note: In-memory storage means tier upgrades are lost on restart${NC}"
        else
            echo -e "${YELLOW}Circuit creation issue: $ERROR_MSG${NC}"
        fi
    fi
fi

echo ""
echo -e "${GREEN}================================${NC}"
echo -e "${GREEN}📊 Summary:${NC}"
echo "- User: gerbov (user-2da9af70-c4c3-4b13-9180-dc1c7094b27c)"
echo "- New Tier: Professional"
echo "- Available Adapters: IpfsIpfs, StellarTestnetIpfs"

if [ "$ENV" = "production" ]; then
    echo ""
    echo -e "${YELLOW}⚠️  IMPORTANT:${NC}"
    echo "Since the production API uses in-memory storage (PostgreSQL reverted),"
    echo "this tier upgrade will be lost when the API restarts."
    echo "To make it permanent, PostgreSQL persistence needs to be fixed."
fi

echo ""
echo -e "${GREEN}✅ Script completed!${NC}"
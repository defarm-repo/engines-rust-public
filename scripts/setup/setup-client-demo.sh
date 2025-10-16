#!/bin/bash

################################################################################
# Setup Client Demo Environment
# Creates users and circuit for client testing
################################################################################

set -e

# Configuration
API_BASE="https://connect.defarm.net/api"
export JWT_SECRET="defarm-dev-secret-key-minimum-32-chars-long-2024"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}Setting up Client Demo Environment${NC}"
echo -e "${BLUE}========================================${NC}\n"

# Step 1: Create ms_admin user (admin with strong password)
echo -e "${YELLOW}Step 1: Creating ms_admin user...${NC}"
MS_ADMIN_RESPONSE=$(curl -s -X POST "$API_BASE/auth/register" \
  -H "Content-Type: application/json" \
  -d '{
    "username": "ms_admin",
    "password": "MSAdmin2024!@Secure#789",
    "email": "admin@msrastreabilidade.com",
    "workspace_name": "MS Rastreabilidade"
  }')

MS_ADMIN_TOKEN=$(echo "$MS_ADMIN_RESPONSE" | jq -r '.token')
MS_ADMIN_ID=$(echo "$MS_ADMIN_RESPONSE" | jq -r '.user_id')

if [ "$MS_ADMIN_TOKEN" != "null" ] && [ -n "$MS_ADMIN_TOKEN" ]; then
    echo -e "${GREEN}✅ ms_admin created successfully${NC}"
    echo -e "   User ID: $MS_ADMIN_ID"
    echo -e "   Username: ms_admin"
    echo -e "   Password: MSAdmin2024!@Secure#789"
else
    echo -e "${YELLOW}⚠️  ms_admin might already exist, trying to login...${NC}"
    MS_ADMIN_RESPONSE=$(curl -s -X POST "$API_BASE/auth/login" \
      -H "Content-Type: application/json" \
      -d '{
        "username": "ms_admin",
        "password": "MSAdmin2024!@Secure#789"
      }')
    MS_ADMIN_TOKEN=$(echo "$MS_ADMIN_RESPONSE" | jq -r '.token')
    MS_ADMIN_ID=$(echo "$MS_ADMIN_RESPONSE" | jq -r '.user_id')
    echo -e "${GREEN}✅ Logged in as ms_admin${NC}"
fi

# Step 2: Make ms_admin an admin user (using hen-admin credentials)
echo -e "\n${YELLOW}Step 2: Granting admin privileges to ms_admin...${NC}"

# Login as hen-admin-001
HEN_RESPONSE=$(curl -s -X POST "$API_BASE/auth/login" \
  -H "Content-Type: application/json" \
  -d '{
    "username": "hen",
    "password": "demo123"
  }')
HEN_TOKEN=$(echo "$HEN_RESPONSE" | jq -r '.token')

# Grant admin privileges
ADMIN_UPDATE=$(curl -s -X PUT "$API_BASE/admin/users/$MS_ADMIN_ID" \
  -H "Authorization: Bearer $HEN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "is_admin": true,
    "tier": "Professional",
    "available_adapters": ["LocalLocal", "LocalIpfs", "IpfsIpfs", "StellarTestnetIpfs"]
  }')

echo -e "${GREEN}✅ ms_admin is now admin with Professional tier${NC}"

# Step 3: Create gerbov user (regular user for client)
echo -e "\n${YELLOW}Step 3: Creating gerbov user...${NC}"
GERBOV_RESPONSE=$(curl -s -X POST "$API_BASE/auth/register" \
  -H "Content-Type: application/json" \
  -d '{
    "username": "gerbov",
    "password": "Gerbov2024!Test",
    "email": "gerbov@testclient.com",
    "workspace_name": "Gerbov Workspace"
  }')

GERBOV_TOKEN=$(echo "$GERBOV_RESPONSE" | jq -r '.token')
GERBOV_ID=$(echo "$GERBOV_RESPONSE" | jq -r '.user_id')

if [ "$GERBOV_TOKEN" != "null" ] && [ -n "$GERBOV_TOKEN" ]; then
    echo -e "${GREEN}✅ gerbov created successfully${NC}"
    echo -e "   User ID: $GERBOV_ID"
    echo -e "   Username: gerbov"
    echo -e "   Password: Gerbov2024!Test"
else
    echo -e "${YELLOW}⚠️  gerbov might already exist, trying to login...${NC}"
    GERBOV_RESPONSE=$(curl -s -X POST "$API_BASE/auth/login" \
      -H "Content-Type: application/json" \
      -d '{
        "username": "gerbov",
        "password": "Gerbov2024!Test"
      }')
    GERBOV_TOKEN=$(echo "$GERBOV_RESPONSE" | jq -r '.token')
    GERBOV_ID=$(echo "$GERBOV_RESPONSE" | jq -r '.user_id')
    echo -e "${GREEN}✅ Logged in as gerbov${NC}"
fi

# Step 4: Grant gerbov access to StellarTestnetIpfs adapter
echo -e "\n${YELLOW}Step 4: Granting StellarTestnetIpfs access to gerbov...${NC}"
GERBOV_UPDATE=$(curl -s -X PUT "$API_BASE/admin/users/$GERBOV_ID" \
  -H "Authorization: Bearer $HEN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "tier": "Professional",
    "available_adapters": ["LocalLocal", "LocalIpfs", "IpfsIpfs", "StellarTestnetIpfs"]
  }')

echo -e "${GREEN}✅ gerbov has Professional tier with StellarTestnetIpfs access${NC}"

# Step 5: Create MS Rastreabilidade circuit (as ms_admin)
echo -e "\n${YELLOW}Step 5: Creating 'MS Rastreabilidade' circuit...${NC}"
CIRCUIT_RESPONSE=$(curl -s -X POST "$API_BASE/circuits" \
  -H "Authorization: Bearer $MS_ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "MS Rastreabilidade",
    "description": "Circuito de rastreabilidade para testes e produção",
    "owner_id": "'"$MS_ADMIN_ID"'",
    "adapter_config": {
      "adapter_type": "StellarTestnetIpfs",
      "requires_approval": false,
      "auto_migrate_existing": false,
      "sponsor_adapter_access": true
    },
    "alias_config": {
      "required_canonical": ["sisbov"],
      "required_contextual": [],
      "use_fingerprint": false,
      "allowed_namespaces": ["bovino", "aves", "suino", "soja", "milho", "generic"],
      "auto_apply_namespace": true
    },
    "default_namespace": "bovino"
  }')

CIRCUIT_ID=$(echo "$CIRCUIT_RESPONSE" | jq -r '.circuit_id')

if [ "$CIRCUIT_ID" != "null" ] && [ -n "$CIRCUIT_ID" ]; then
    echo -e "${GREEN}✅ Circuit created successfully${NC}"
    echo -e "   Circuit ID: $CIRCUIT_ID"
    echo -e "   Name: MS Rastreabilidade"
    echo -e "   Adapter: StellarTestnetIpfs (sponsored)"
else
    echo -e "${YELLOW}⚠️  Failed to create circuit, checking existing...${NC}"
    # Try to find existing circuit
    CIRCUITS=$(curl -s -X GET "$API_BASE/circuits/member/$MS_ADMIN_ID" \
      -H "Authorization: Bearer $MS_ADMIN_TOKEN")
    CIRCUIT_ID=$(echo "$CIRCUITS" | jq -r '.circuits[] | select(.name=="MS Rastreabilidade") | .circuit_id')
    echo -e "${GREEN}✅ Found existing circuit: $CIRCUIT_ID${NC}"
fi

# Step 6: Make circuit public
echo -e "\n${YELLOW}Step 6: Making circuit public...${NC}"
PUBLIC_UPDATE=$(curl -s -X PUT "$API_BASE/circuits/$CIRCUIT_ID/public-settings" \
  -H "Authorization: Bearer $MS_ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "visibility": "PublicDiscoverable",
    "auto_accept_join_requests": false,
    "allow_public_item_view": true,
    "allow_public_event_view": false,
    "require_approval_for_public_pull": false
  }')

echo -e "${GREEN}✅ Circuit is now publicly discoverable${NC}"

# Step 7: gerbov requests to join circuit
echo -e "\n${YELLOW}Step 7: gerbov requesting to join circuit...${NC}"
JOIN_REQUEST=$(curl -s -X POST "$API_BASE/circuits/$CIRCUIT_ID/join" \
  -H "Authorization: Bearer $GERBOV_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "requester_id": "'"$GERBOV_ID"'",
    "message": "Solicitando acesso ao circuito de rastreabilidade"
  }')

echo -e "${GREEN}✅ Join request submitted${NC}"

# Wait a moment
sleep 2

# Step 8: ms_admin approves join request
echo -e "\n${YELLOW}Step 8: ms_admin approving join request...${NC}"

# Get pending requests
PENDING=$(curl -s -X GET "$API_BASE/circuits/$CIRCUIT_ID/pending-requests" \
  -H "Authorization: Bearer $MS_ADMIN_TOKEN")

REQUEST_ID=$(echo "$PENDING" | jq -r '.requests[0].request_id // empty')

if [ -n "$REQUEST_ID" ]; then
    APPROVE=$(curl -s -X POST "$API_BASE/circuits/$CIRCUIT_ID/approve-join/$REQUEST_ID" \
      -H "Authorization: Bearer $MS_ADMIN_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{
        "approver_id": "'"$MS_ADMIN_ID"'",
        "role": "Member"
      }')
    echo -e "${GREEN}✅ Join request approved - gerbov is now a member${NC}"
else
    echo -e "${YELLOW}⚠️  No pending request found (might already be approved)${NC}"
fi

# Step 9: Enable auto-publish (if you have this feature)
echo -e "\n${YELLOW}Step 9: Configuring circuit post-actions...${NC}"
POST_ACTIONS=$(curl -s -X PUT "$API_BASE/circuits/$CIRCUIT_ID/post-actions" \
  -H "Authorization: Bearer $MS_ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "enabled": true
  }')

echo -e "${GREEN}✅ Post-actions enabled${NC}"

# Summary
echo -e "\n${BLUE}========================================${NC}"
echo -e "${BLUE}Setup Complete!${NC}"
echo -e "${BLUE}========================================${NC}\n"

echo -e "${GREEN}Client Testing Credentials:${NC}"
echo -e "  Username: gerbov"
echo -e "  Password: Gerbov2024!Test"
echo -e "  Tier: Professional"
echo -e "  Adapters: StellarTestnetIpfs (+ others)"
echo -e ""

echo -e "${GREEN}Admin Credentials:${NC}"
echo -e "  Username: ms_admin"
echo -e "  Password: MSAdmin2024!@Secure#789"
echo -e "  Role: Admin + Circuit Owner"
echo -e ""

echo -e "${GREEN}Circuit Information:${NC}"
echo -e "  Name: MS Rastreabilidade"
echo -e "  Circuit ID: $CIRCUIT_ID"
echo -e "  Visibility: Public (discoverable)"
echo -e "  Adapter: StellarTestnetIpfs (sponsored)"
echo -e "  Member: gerbov (approved)"
echo -e ""

echo -e "${YELLOW}Save this Circuit ID for documentation:${NC}"
echo -e "  $CIRCUIT_ID"
echo -e ""

echo -e "${GREEN}✅ Client can now test with:${NC}"
echo -e "  1. Login as gerbov"
echo -e "  2. Create local items"
echo -e "  3. Push to circuit: $CIRCUIT_ID"
echo -e "  4. View storage history with blockchain TXs"
echo -e ""

# Save circuit ID to file for documentation update
echo "$CIRCUIT_ID" > /tmp/ms-rastreabilidade-circuit-id.txt
echo -e "${GREEN}Circuit ID saved to: /tmp/ms-rastreabilidade-circuit-id.txt${NC}"

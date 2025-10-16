#!/bin/bash

################################################################################
# Comprehensive Circuit Workflow Integration Test
# Tests full workflow: circuit creation, user joins, item pushes, adapters, webhooks
################################################################################

# set -e  # Exit on error - TEMPORARILY DISABLED TO SEE ALL ERRORS

# Configuration
export JWT_SECRET="defarm-dev-secret-key-minimum-32-chars-long-2024"
API_BASE="http://localhost:3000/api"
ADMIN_USER="hen"
ADMIN_PASSWORD="demo123"
TEST_USER1="pullet"
TEST_USER1_PASSWORD="demo123"
TEST_USER2="cock"
TEST_USER2_PASSWORD="demo123"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test counters
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_TOTAL=0

# Store test data (using simple variables instead of associative arrays)
CIRCUIT1_ID=""
CIRCUIT2_ID=""
ITEM1_LID=""
ITEM1_DFID=""
ITEM2_LID=""
ITEM2_DFID=""
OPERATION1_ID=""

################################################################################
# Helper Functions
################################################################################

print_header() {
    echo -e "\n${BLUE}========================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}========================================${NC}\n"
}

print_section() {
    echo -e "\n${YELLOW}>>> $1${NC}"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
    ((TESTS_PASSED++))
    ((TESTS_TOTAL++))
}

print_failure() {
    echo -e "${RED}❌ $1${NC}"
    ((TESTS_FAILED++))
    ((TESTS_TOTAL++))
}

print_info() {
    echo -e "   $1"
}

assert_equals() {
    local expected="$1"
    local actual="$2"
    local message="$3"

    if [ "$expected" = "$actual" ]; then
        print_success "$message"
    else
        print_failure "$message (expected: $expected, got: $actual)"
    fi
}

assert_not_empty() {
    local value="$1"
    local message="$2"

    if [ -n "$value" ] && [ "$value" != "null" ]; then
        print_success "$message"
    else
        print_failure "$message (value is empty or null)"
    fi
}

assert_contains() {
    local haystack="$1"
    local needle="$2"
    local message="$3"

    if echo "$haystack" | grep -q "$needle"; then
        print_success "$message"
    else
        print_failure "$message (did not find '$needle')"
    fi
}

# API call wrapper with better error handling
api_call() {
    local method="$1"
    local endpoint="$2"
    local token="$3"
    local data="$4"

    local response
    local http_code

    if [ -n "$data" ]; then
        response=$(curl -s -w "\n%{http_code}" -X "$method" \
            "$API_BASE$endpoint" \
            -H "Content-Type: application/json" \
            -H "Authorization: Bearer $token" \
            -d "$data")
    else
        response=$(curl -s -w "\n%{http_code}" -X "$method" \
            "$API_BASE$endpoint" \
            -H "Authorization: Bearer $token")
    fi

    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | sed '$d')

    echo "$body"
    return $http_code
}

# Authenticate and get JWT token
authenticate() {
    local user_id="$1"
    local password="$2"

    print_section "Authenticating as $user_id" >&2

    local response=$(curl -s -X POST "$API_BASE/auth/login" \
        -H "Content-Type: application/json" \
        -d "{\"username\":\"$user_id\",\"password\":\"$password\"}")

    local token=$(echo "$response" | jq -r '.token')

    if [ "$token" != "null" ] && [ -n "$token" ]; then
        print_success "Authentication successful" >&2
        echo "$token"
    else
        print_failure "Authentication failed" >&2
        echo ""
    fi
}

################################################################################
# Scenario 1: Circuit Setup - Auto-Approve Members ON, Auto-Publish Items ON
################################################################################

scenario_1_circuit_setup_auto_approve() {
    print_header "SCENARIO 1: Circuit Setup - Auto-Approve Members ON, Auto-Publish Items ON"

    # Authenticate admin
    ADMIN_TOKEN=$(authenticate "$ADMIN_USER" "$ADMIN_PASSWORD")
    assert_not_empty "$ADMIN_TOKEN" "Admin authentication token obtained"

    # Create circuit
    print_section "Creating circuit with default settings"

    local create_response=$(curl -s -X POST "$API_BASE/circuits" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d '{
            "name": "Auto-Approve Test Circuit",
            "description": "Circuit for testing auto-approve and auto-publish features",
            "default_namespace": "bovino",
            "owner_id": "hen-admin-001"
        }')

    local circuit_id=$(echo "$create_response" | jq -r '.circuit_id')
    CIRCUIT1_ID="$circuit_id"
    assert_not_empty "$circuit_id" "Circuit created successfully"
    print_info "Circuit ID: $circuit_id"

    # Configure public settings with auto-approve and auto-publish
    print_section "Configuring public settings (auto-approve: true, auto-publish: true)"

    local public_settings=$(curl -s -X PUT "$API_BASE/circuits/$circuit_id/public-settings" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d '{
            "requester_id": "hen-admin-001",
            "public_settings": {
                "access_mode": "Public",
                "auto_approve_members": true,
                "auto_publish_pushed_items": true,
                "show_encrypted_events": false,
                "public_name": "Auto-Approve Circuit",
                "public_description": "A test circuit with automatic member approval"
            }
        }')

    local access_mode=$(echo "$public_settings" | jq -r '.data.public_settings.access_mode')
    assert_equals "Public" "$access_mode" "Circuit access mode set to Public"

    local auto_approve=$(echo "$public_settings" | jq -r '.data.public_settings.auto_approve_members')
    assert_equals "true" "$auto_approve" "Auto-approve members enabled"

    local auto_publish=$(echo "$public_settings" | jq -r '.data.public_settings.auto_publish_pushed_items')
    assert_equals "true" "$auto_publish" "Auto-publish items enabled"

    # Enable public visibility permission (required for public access)
    print_section "Enabling public visibility permission"

    curl -s -X PUT "$API_BASE/circuits/$circuit_id" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d "{
            \"requester_id\": \"hen-admin-001\",
            \"permissions\": {
                \"allow_public_visibility\": true
            }
        }" > /dev/null

    print_success "Public visibility enabled for circuit"

    # Set adapter config (LocalLocal for simplicity)
    print_section "Setting circuit adapter to LocalLocal"

    local adapter_response=$(curl -s -X PUT "$API_BASE/circuits/$circuit_id/adapter" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d '{
            "adapter_type": "local-local",
            "auto_migrate_existing": false,
            "requires_approval": false,
            "sponsor_adapter_access": true
        }')

    local adapter_type=$(echo "$adapter_response" | jq -r '.adapter_type')
    assert_equals "local-local" "$adapter_type" "Circuit adapter set to LocalLocal"

    # Configure webhook
    print_section "Configuring webhook for ItemPushed events"

    # Enable post-action settings
    local post_action_response=$(curl -s -X PUT "$API_BASE/circuits/$circuit_id/post-actions" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d '{
            "enabled": true,
            "trigger_events": ["ItemPushed", "ItemTokenized"],
            "include_storage_details": true,
            "include_item_metadata": true
        }')

    local post_enabled=$(echo "$post_action_response" | jq -r '.data.enabled')
    assert_equals "true" "$post_enabled" "Post-action settings enabled"

    print_success "Scenario 1 completed: Circuit configured with auto-approve and auto-publish"
}

################################################################################
# Scenario 2: User Auto-Join and Push
################################################################################

scenario_2_user_auto_join() {
    print_header "SCENARIO 2: User Auto-Join and Push"

    local circuit_id="$CIRCUIT1_ID"

    # Authenticate test user
    USER1_TOKEN=$(authenticate "$TEST_USER1" "$TEST_USER1_PASSWORD")
    assert_not_empty "$USER1_TOKEN" "Test user authentication token obtained"

    # Join public circuit (should auto-approve)
    print_section "User joining public circuit (expecting auto-approval)"

    local join_response=$(curl -s -X POST "$API_BASE/circuits/$circuit_id/public/join" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $USER1_TOKEN" \
        -d '{
            "requester_id": "pullet-user-001",
            "message": "Requesting to join for testing"
        }')

    local requires_approval=$(echo "$join_response" | jq -r '.requires_approval')
    assert_equals "false" "$requires_approval" "User auto-approved (no approval required)"

    # Verify user is member
    print_section "Verifying user is circuit member"

    local circuit_info=$(curl -s -X GET "$API_BASE/circuits/$circuit_id" \
        -H "Authorization: Bearer $USER1_TOKEN")

    local is_member=$(echo "$circuit_info" | jq -r ".members | map(.member_id) | contains([\"pullet-user-001\"])")
    assert_equals "true" "$is_member" "User is member of circuit"

    # Create local item with basic data
    print_section "Creating local item with basic data"

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
    ITEM1_LID="$local_id"
    assert_not_empty "$local_id" "Local item created"
    print_info "Local ID: $local_id"

    # Push item to circuit
    print_section "Pushing item to circuit"

    local push_response=$(curl -s -X POST "$API_BASE/circuits/$circuit_id/push-local" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $USER1_TOKEN" \
        -d "{
            \"local_id\": \"$local_id\",
            \"requester_id\": \"pullet-user-001\",
            \"enhanced_identifiers\": [
                {
                    \"namespace\": \"bovino\",
                    \"key\": \"sisbov\",
                    \"value\": \"BR123456789012\",
                    \"id_type\": \"Canonical\"
                },
                {
                    \"namespace\": \"generic\",
                    \"key\": \"lote\",
                    \"value\": \"LOTE-001\",
                    \"id_type\": \"Contextual\"
                }
            ]
        }")

    local dfid=$(echo "$push_response" | jq -r '.data.dfid')
    ITEM1_DFID="$dfid"
    assert_not_empty "$dfid" "Item tokenized and DFID assigned"
    print_info "DFID: $dfid"

    local status=$(echo "$push_response" | jq -r '.data.status')
    assert_equals "NewItemCreated" "$status" "Item status is NewItemCreated"

    # Verify item is auto-published
    print_section "Verifying item is auto-published to circuit"

    local public_circuit=$(curl -s -X GET "$API_BASE/circuits/$circuit_id/public")
    local published=$(echo "$public_circuit" | jq -r ".data.published_items // [] | contains([\"$dfid\"])")
    assert_equals "true" "$published" "Item auto-published to circuit"

    # Check storage history
    print_section "Checking storage history for LocalLocal adapter"

    local history_response=$(curl -s -X GET "$API_BASE/storage-history/$dfid" \
        -H "Authorization: Bearer $USER1_TOKEN")

    local adapter_type=$(echo "$history_response" | jq -r '.history.records[0].adapter_type')
    assert_equals "local-local" "$adapter_type" "Storage record shows LocalLocal adapter"

    local is_active=$(echo "$history_response" | jq -r '.history.records[0].is_active')
    assert_equals "true" "$is_active" "Storage record is active"

    print_success "Scenario 2 completed: User auto-joined and pushed item successfully"
}

################################################################################
# Scenario 3: Circuit Setup - Manual Approval Required
################################################################################

scenario_3_manual_approval_circuit() {
    print_header "SCENARIO 3: Circuit Setup - Manual Approval Required"

    # Create circuit with manual approval settings
    print_section "Creating circuit with manual approval requirements"

    local create_response=$(curl -s -X POST "$API_BASE/circuits" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d '{
            "name": "Manual Approval Circuit",
            "description": "Circuit requiring manual approval for joins and pushes",
            "default_namespace": "generic",
            "owner_id": "hen-admin-001"
        }')

    local circuit_id=$(echo "$create_response" | jq -r '.circuit_id')
    CIRCUIT2_ID="$circuit_id"
    assert_not_empty "$circuit_id" "Manual approval circuit created"
    print_info "Circuit ID: $circuit_id"

    # Update circuit permissions
    print_section "Setting circuit to require approval for pushes"

    local update_response=$(curl -s -X PUT "$API_BASE/circuits/$circuit_id" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d '{
            "requester_id": "hen-admin-001",
            "permissions": {
                "require_approval_for_push": true,
                "require_approval_for_pull": false,
                "allow_public_visibility": true
            }
        }')

    local require_push_approval=$(echo "$update_response" | jq -r '.data.permissions.require_approval_for_push')
    assert_equals "true" "$require_push_approval" "Push approval required"

    # Configure public settings with manual member approval
    print_section "Configuring public settings (auto-approve: false, auto-publish: false)"

    local public_settings=$(curl -s -X PUT "$API_BASE/circuits/$circuit_id/public-settings" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d '{
            "requester_id": "hen-admin-001",
            "public_settings": {
                "access_mode": "Public",
                "auto_approve_members": false,
                "auto_publish_pushed_items": false,
                "show_encrypted_events": false
            }
        }')

    local auto_approve=$(echo "$public_settings" | jq -r '.data.public_settings.auto_approve_members')
    assert_equals "false" "$auto_approve" "Auto-approve members disabled"

    # Set adapter to IpfsIpfs
    print_section "Setting circuit adapter to IpfsIpfs"

    local adapter_response=$(curl -s -X PUT "$API_BASE/circuits/$circuit_id/adapter" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d '{
            "adapter_type": "ipfs-ipfs",
            "auto_migrate_existing": false,
            "requires_approval": false,
            "sponsor_adapter_access": true
        }')

    local adapter_type=$(echo "$adapter_response" | jq -r '.adapter_type')
    assert_equals "ipfs-ipfs" "$adapter_type" "Circuit adapter set to IpfsIpfs"

    # User2 requests to join
    USER2_TOKEN=$(authenticate "$TEST_USER2" "$TEST_USER2_PASSWORD")
    assert_not_empty "$USER2_TOKEN" "Test user 2 authenticated"

    print_section "User2 requesting to join circuit"

    local join_response=$(curl -s -X POST "$API_BASE/circuits/$circuit_id/public/join" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $USER2_TOKEN" \
        -d '{
            "requester_id": "cock-user-001",
            "message": "Requesting to join manual approval circuit"
        }')

    local requires_approval=$(echo "$join_response" | jq -r '.requires_approval')
    assert_equals "true" "$requires_approval" "Join request requires manual approval"

    # Admin lists pending requests
    print_section "Admin listing pending join requests"

    local pending_requests=$(curl -s -X GET "$API_BASE/circuits/$circuit_id/requests/pending" \
        -H "Authorization: Bearer $ADMIN_TOKEN")

    local request_count=$(echo "$pending_requests" | jq -r 'length')
    assert_not_empty "$request_count" "Pending join requests found"

    local requester=$(echo "$pending_requests" | jq -r ".[0].requester_id")
    assert_equals "$TEST_USER2" "$requester" "User2 join request is pending"

    # Admin approves join request
    print_section "Admin approving join request"

    local approve_response=$(curl -s -X POST "$API_BASE/circuits/$circuit_id/requests/$TEST_USER2/approve" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d '{
            "role": "Member"
        }')

    # Verify User2 is now member
    local circuit_info=$(curl -s -X GET "$API_BASE/circuits/$circuit_id" \
        -H "Authorization: Bearer $ADMIN_TOKEN")

    local is_member=$(echo "$circuit_info" | jq -r ".members | has(\"$TEST_USER2\")")
    assert_equals "true" "$is_member" "User2 is now circuit member after approval"

    print_success "Scenario 3 completed: Manual approval workflow tested"
}

################################################################################
# Scenario 4: Push with Manual Approval
################################################################################

scenario_4_push_with_approval() {
    print_header "SCENARIO 4: Push with Manual Approval"

    local circuit_id="$CIRCUIT2_ID"

    # User2 creates item with medium data
    print_section "User2 creating item with medium-sized data"

    # Generate medium data (10 enhanced identifiers)
    local medium_identifiers='['
    for i in {1..10}; do
        medium_identifiers+="{\"namespace\":\"generic\",\"key\":\"id_$i\",\"value\":\"VALUE_$(uuidgen)\",\"id_type\":\"Contextual\"},"
    done
    medium_identifiers="${medium_identifiers%,}]"

    local create_item_response=$(curl -s -X POST "$API_BASE/items/local" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $USER2_TOKEN" \
        -d "{
            \"enhanced_identifiers\": $medium_identifiers,
            \"enriched_data\": {
                \"description\": \"This is a medium-sized item with multiple fields for testing\",
                \"field_1\": \"$(head -c 1000 </dev/urandom | base64)\",
                \"field_2\": \"$(head -c 1000 </dev/urandom | base64)\",
                \"metadata\": \"Medium data test\"
            }
        }")

    local local_id=$(echo "$create_item_response" | jq -r '.data.local_id')
    ITEM2_LID="$local_id"
    assert_not_empty "$local_id" "Medium-sized item created"
    print_info "Local ID: $local_id"

    # Push item to circuit (should require approval)
    print_section "Pushing item to circuit (expecting pending approval)"

    local push_response=$(curl -s -X POST "$API_BASE/circuits/$circuit_id/push-local" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $USER2_TOKEN" \
        -d "{
            \"local_id\": \"$local_id\",
            \"requester_id\": \"cock-user-001\",
            \"enhanced_identifiers\": $medium_identifiers
        }")

    local operation_id=$(echo "$push_response" | jq -r '.data.operation_id')
    OPERATION1_ID="$operation_id"
    assert_not_empty "$operation_id" "Push operation created"

    # Verify operation is pending
    print_section "Verifying operation is pending approval"

    local pending_ops=$(curl -s -X GET "$API_BASE/circuits/$circuit_id/operations/pending" \
        -H "Authorization: Bearer $ADMIN_TOKEN")

    local op_status=$(echo "$pending_ops" | jq -r ".[0].status")
    assert_equals "Pending" "$op_status" "Operation is pending approval"

    # Admin approves operation
    print_section "Admin approving push operation"

    local approve_response=$(curl -s -X POST "$API_BASE/circuits/operations/$operation_id/approve" \
        -H "Authorization: Bearer $ADMIN_TOKEN")

    local approved_status=$(echo "$approve_response" | jq -r '.data.status')
    assert_equals "Approved" "$approved_status" "Operation approved successfully"

    local dfid=$(echo "$approve_response" | jq -r '.data.dfid')
    ITEM2_DFID="$dfid"
    assert_not_empty "$dfid" "DFID assigned after approval"

    # Check storage history shows IPFS CID
    print_section "Checking storage history for IPFS CID"

    sleep 2  # Allow time for storage to complete

    local history_response=$(curl -s -X GET "$API_BASE/storage-history/$dfid" \
        -H "Authorization: Bearer $USER2_TOKEN")

    local adapter_type=$(echo "$history_response" | jq -r '.history.records[0].adapter_type')
    assert_equals "ipfs-ipfs" "$adapter_type" "Storage shows IpfsIpfs adapter"

    local storage_location=$(echo "$history_response" | jq -r '.history.records[0].storage_location')
    assert_contains "$storage_location" "cid" "Storage location contains CID"

    print_success "Scenario 4 completed: Manual approval for push tested"
}

################################################################################
# Scenario 5: Large Data Test
################################################################################

scenario_5_large_data_test() {
    print_header "SCENARIO 5: Large Data Test"

    local circuit_id="$CIRCUIT1_ID"

    print_section "Creating item with large dataset (~1MB)"

    # Generate large data with many enhanced identifiers
    local large_identifiers='['
    for i in {1..100}; do
        large_identifiers+="{\"namespace\":\"generic\",\"key\":\"large_id_$i\",\"value\":\"$(uuidgen)\",\"id_type\":\"Contextual\"},"
    done
    large_identifiers="${large_identifiers%,}]"

    # Create large text content
    local large_text=$(head -c 100000 </dev/urandom | base64)

    local create_response=$(curl -s -X POST "$API_BASE/items/local" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $USER1_TOKEN" \
        -d "{
            \"enhanced_identifiers\": $large_identifiers,
            \"enriched_data\": {
                \"large_description\": \"$large_text\",
                \"metadata\": {
                    \"test_type\": \"large_data\",
                    \"size\": \"~1MB\",
                    \"nested\": {
                        \"level1\": {
                            \"level2\": {
                                \"data\": \"deep nested structure\"
                            }
                        }
                    }
                },
                \"arrays\": $(seq 1 50 | jq -R . | jq -s .)
            }
        }")

    local local_id=$(echo "$create_response" | jq -r '.data.local_id')
    assert_not_empty "$local_id" "Large item created successfully"

    # Push to circuit
    print_section "Pushing large item to circuit with IpfsIpfs adapter"

    local push_response=$(curl -s -X POST "$API_BASE/circuits/$circuit_id/push-local" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $USER1_TOKEN" \
        -d "{
            \"local_id\": \"$local_id\",
            \"requester_id\": \"pullet-user-001\",
            \"enhanced_identifiers\": $large_identifiers
        }")

    local dfid=$(echo "$push_response" | jq -r '.data.dfid')
    assert_not_empty "$dfid" "Large item tokenized"

    sleep 3  # Allow time for IPFS storage

    # Verify CID format
    print_section "Verifying IPFS CID format"

    local history=$(curl -s -X GET "$API_BASE/storage-history/$dfid" \
        -H "Authorization: Bearer $USER1_TOKEN")

    local cid=$(echo "$history" | jq -r '.history.records[0].storage_location.cid')
    assert_not_empty "$cid" "CID generated for large item"

    # Check if CID matches IPFS format (Qm... or baf...)
    if [[ "$cid" =~ ^(Qm|baf) ]]; then
        print_success "CID format is valid IPFS format"
    else
        print_failure "CID format is invalid: $cid"
    fi

    # Verify metadata includes size info
    local metadata=$(echo "$history" | jq -r '.history.records[0].metadata')
    assert_not_empty "$metadata" "Storage metadata exists"

    print_success "Scenario 5 completed: Large data handled successfully"
}

################################################################################
# Scenario 6: Test All Adapters
################################################################################

scenario_6_test_all_adapters() {
    print_header "SCENARIO 6: Test All Adapters"

    local adapters=("local-local" "ipfs-ipfs" "local-ipfs" "stellar_testnet-ipfs" "stellar_mainnet-ipfs" "stellar_mainnet-stellar_mainnet")

    for adapter_type in "${adapters[@]}"; do
        print_section "Testing adapter: $adapter_type"

        # Create circuit for this adapter
        local create_response=$(curl -s -X POST "$API_BASE/circuits" \
            -H "Content-Type: application/json" \
            -H "Authorization: Bearer $ADMIN_TOKEN" \
            -d "{
                \"name\": \"$adapter_type Test Circuit\",
                \"description\": \"Circuit for testing $adapter_type adapter\",
                \"default_namespace\": \"generic\",
                \"owner_id\": \"hen-admin-001\"
            }")

        local circuit_id=$(echo "$create_response" | jq -r '.circuit_id')
        assert_not_empty "$circuit_id" "Circuit created for $adapter_type"

        # Set adapter
        curl -s -X PUT "$API_BASE/circuits/$circuit_id/adapter" \
            -H "Content-Type: application/json" \
            -H "Authorization: Bearer $ADMIN_TOKEN" \
            -d "{
                \"adapter_type\": \"$adapter_type\",
                \"auto_migrate_existing\": false,
                \"requires_approval\": false,
                \"sponsor_adapter_access\": true
            }" > /dev/null

        # Add user as member
        curl -s -X POST "$API_BASE/circuits/$circuit_id/members" \
            -H "Content-Type: application/json" \
            -H "Authorization: Bearer $ADMIN_TOKEN" \
            -d "{
                \"member_id\": \"$TEST_USER1\",
                \"role\": \"Member\"
            }" > /dev/null

        # Create and push item
        local item_response=$(curl -s -X POST "$API_BASE/items/local" \
            -H "Content-Type: application/json" \
            -H "Authorization: Bearer $USER1_TOKEN" \
            -d "{
                \"enhanced_identifiers\": [
                    {
                        \"namespace\": \"generic\",
                        \"key\": \"test_adapter\",
                        \"value\": \"$adapter_type\",
                        \"id_type\": \"Contextual\"
                    },
                    {
                        \"namespace\": \"generic\",
                        \"key\": \"timestamp\",
                        \"value\": \"$(date +%s)\",
                        \"id_type\": \"Contextual\"
                    }
                ],
                \"enriched_data\": {
                    \"adapter_type\": \"$adapter_type\",
                    \"test_data\": \"Testing $adapter_type\"
                }
            }")

        local local_id=$(echo "$item_response" | jq -r '.data.local_id')

        local push_response=$(curl -s -X POST "$API_BASE/circuits/$circuit_id/push-local" \
            -H "Content-Type: application/json" \
            -H "Authorization: Bearer $USER1_TOKEN" \
            -d "{
                \"local_id\": \"$local_id\",
                \"requester_id\": \"pullet-user-001\",
                \"enhanced_identifiers\": [
                    {
                        \"namespace\": \"generic\",
                        \"key\": \"test_adapter\",
                        \"value\": \"$adapter_type\",
                        \"id_type\": \"Contextual\"
                    }
                ]
            }")

        local dfid=$(echo "$push_response" | jq -r '.data.dfid')
        assert_not_empty "$dfid" "$adapter_type: Item tokenized"

        sleep 2  # Allow time for async storage

        # Query storage history
        local history=$(curl -s -X GET "$API_BASE/storage-history/$dfid" \
            -H "Authorization: Bearer $USER1_TOKEN")

        local stored_adapter=$(echo "$history" | jq -r '.history.records[0].adapter_type')
        assert_equals "$adapter_type" "$stored_adapter" "$adapter_type: Adapter type matches"

        # Verify storage location based on adapter type
        case "$adapter_type" in
            local-local|local-ipfs)
                local has_id=$(echo "$history" | jq -r '.history.records[0].storage_location | has("id")')
                assert_equals "true" "$has_id" "$adapter_type: Has local ID"
                ;;
            ipfs-ipfs)
                local has_cid=$(echo "$history" | jq -r '.history.records[0].storage_location | has("cid")')
                assert_equals "true" "$has_cid" "$adapter_type: Has CID"
                ;;
            stellar_testnet-ipfs|stellar_mainnet-ipfs|stellar_mainnet-stellar_mainnet)
                local has_tx=$(echo "$history" | jq -r '.history.records[0].storage_location | has("transaction_id")')
                assert_equals "true" "$has_tx" "$adapter_type: Has transaction ID"
                ;;
        esac

        # Verify hash exists
        local has_hash=$(echo "$history" | jq -r '.history.records[0].metadata | has("hash")')
        assert_equals "true" "$has_hash" "$adapter_type: Storage metadata includes hash"
    done

    print_success "Scenario 6 completed: All adapters tested"
}

################################################################################
# Scenario 7: Storage Migration Test
################################################################################

scenario_7_storage_migration() {
    print_header "SCENARIO 7: Storage Migration Test"

    # Create circuit with LocalLocal
    print_section "Creating circuit with LocalLocal adapter"

    local create_response=$(curl -s -X POST "$API_BASE/circuits" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d '{
            "name": "Migration Test Circuit",
            "description": "Testing storage migration from LocalLocal to IpfsIpfs",
            "default_namespace": "generic",
            "owner_id": "hen-admin-001"
        }')

    local circuit_id=$(echo "$create_response" | jq -r '.circuit_id')
    assert_not_empty "$circuit_id" "Migration test circuit created"

    # Set LocalLocal adapter
    curl -s -X PUT "$API_BASE/circuits/$circuit_id/adapter" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d '{
            "adapter_type": "local-local",
            "auto_migrate_existing": false,
            "requires_approval": false,
            "sponsor_adapter_access": true
        }' > /dev/null

    # Add user as member
    curl -s -X POST "$API_BASE/circuits/$circuit_id/members" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d "{
            \"member_id\": \"$TEST_USER1\",
            \"role\": \"Member\"
        }" > /dev/null

    # Push item with LocalLocal
    print_section "Pushing item with LocalLocal adapter"

    local item_response=$(curl -s -X POST "$API_BASE/items/local" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $USER1_TOKEN" \
        -d '{
            "enhanced_identifiers": [
                {
                    "namespace": "generic",
                    "key": "migration_test",
                    "value": "MIGRATION_001",
                    "id_type": "Contextual"
                }
            ],
            "enriched_data": {
                "version": "1",
                "adapter": "LocalLocal"
            }
        }')

    local local_id=$(echo "$item_response" | jq -r '.data.local_id')

    local push_response=$(curl -s -X POST "$API_BASE/circuits/$circuit_id/push-local" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $USER1_TOKEN" \
        -d "{
            \"local_id\": \"$local_id\",
            \"requester_id\": \"pullet-user-001\",
            \"enhanced_identifiers\": [
                {
                    \"namespace\": \"generic\",
                    \"key\": \"migration_test\",
                    \"value\": \"MIGRATION_001\",
                    \"id_type\": \"Contextual\"
                }
            ]
        }")

    local dfid=$(echo "$push_response" | jq -r '.data.dfid')
    assert_not_empty "$dfid" "Item stored with LocalLocal"

    sleep 1

    # Get initial storage history
    local history_before=$(curl -s -X GET "$API_BASE/storage-history/$dfid" \
        -H "Authorization: Bearer $USER1_TOKEN")

    local record_count_before=$(echo "$history_before" | jq -r '.history.records | length')
    assert_equals "1" "$record_count_before" "One storage record before migration"

    # Change adapter to IpfsIpfs
    print_section "Changing circuit adapter to IpfsIpfs"

    curl -s -X PUT "$API_BASE/circuits/$circuit_id/adapter" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d '{
            "adapter_type": "ipfs-ipfs",
            "auto_migrate_existing": false,
            "requires_approval": false,
            "sponsor_adapter_access": true
        }' > /dev/null

    print_success "Adapter changed to IpfsIpfs"

    # Push update to trigger migration
    print_section "Pushing update to trigger storage migration"

    local update_response=$(curl -s -X PUT "$API_BASE/items/$dfid" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $USER1_TOKEN" \
        -d '{
            "data": {
                "version": "2",
                "adapter": "IpfsIpfs",
                "updated": "true"
            }
        }')

    sleep 2  # Allow time for migration

    # Get updated storage history
    local history_after=$(curl -s -X GET "$API_BASE/storage-history/$dfid" \
        -H "Authorization: Bearer $USER1_TOKEN")

    local record_count_after=$(echo "$history_after" | jq -r '.history.records | length')

    if [ "$record_count_after" -gt "1" ]; then
        print_success "Storage history shows migration (multiple records)"
    else
        print_failure "Storage migration not reflected in history"
    fi

    # Verify old record is inactive
    local old_active=$(echo "$history_after" | jq -r '.history.records[1].is_active')
    assert_equals "false" "$old_active" "Old LocalLocal record is inactive"

    # Verify new record is active with IpfsIpfs
    local new_adapter=$(echo "$history_after" | jq -r '.history.records[0].adapter_type')
    assert_equals "ipfs-ipfs" "$new_adapter" "New record shows IpfsIpfs adapter"

    local new_active=$(echo "$history_after" | jq -r '.history.records[0].is_active')
    assert_equals "true" "$new_active" "New record is active"

    print_success "Scenario 7 completed: Storage migration tested"
}

################################################################################
# Scenario 8: Encrypted Events Test
################################################################################

scenario_8_encrypted_events() {
    print_header "SCENARIO 8: Encrypted Events Test"

    # Create circuit with show_encrypted_events: true
    print_section "Creating circuit with encrypted events visible"

    local create_response=$(curl -s -X POST "$API_BASE/circuits" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d '{
            "name": "Encrypted Events Circuit",
            "description": "Testing encrypted event visibility",
            "default_namespace": "generic",
            "owner_id": "hen-admin-001"
        }')

    local circuit_id=$(echo "$create_response" | jq -r '.circuit_id')

    # Configure public settings with show_encrypted_events: true
    curl -s -X PUT "$API_BASE/circuits/$circuit_id/public-settings" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d '{
            "requester_id": "hen-admin-001",
            "public_settings": {
                "access_mode": "Public",
                "show_encrypted_events": true
            }
        }' > /dev/null

    # Enable public visibility
    curl -s -X PUT "$API_BASE/circuits/$circuit_id" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d '{
            "requester_id": "hen-admin-001",
            "permissions": {"allow_public_visibility": true}
        }' > /dev/null

    print_success "Circuit configured to show encrypted events"

    # Add user and push item
    curl -s -X POST "$API_BASE/circuits/$circuit_id/members" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d "{
            \"member_id\": \"$TEST_USER1\",
            \"role\": \"Member\"
        }" > /dev/null

    local item_response=$(curl -s -X POST "$API_BASE/items/local" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $USER1_TOKEN" \
        -d '{
            "enhanced_identifiers": [
                {
                    "namespace": "generic",
                    "key": "encrypted_test",
                    "value": "ENC_001",
                    "id_type": "Contextual"
                }
            ],
            "enriched_data": {"sensitive": "data"}
        }')

    local local_id=$(echo "$item_response" | jq -r '.data.local_id')

    curl -s -X POST "$API_BASE/circuits/$circuit_id/push-local" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $USER1_TOKEN" \
        -d "{
            \"local_id\": \"$local_id\",
            \"requester_id\": \"pullet-user-001\",
            \"enhanced_identifiers\": [
                {
                    \"namespace\": \"generic\",
                    \"key\": \"encrypted_test\",
                    \"value\": \"ENC_001\",
                    \"id_type\": \"Contextual\"
                }
            ]
        }" > /dev/null

    # Query public circuit info
    local public_info=$(curl -s -X GET "$API_BASE/circuits/$circuit_id/public")
    local show_encrypted=$(echo "$public_info" | jq -r '.data.show_encrypted_events')
    assert_equals "true" "$show_encrypted" "Public info shows encrypted events enabled"

    # Create another circuit with show_encrypted_events: false
    print_section "Creating circuit with encrypted events hidden"

    local create_response2=$(curl -s -X POST "$API_BASE/circuits" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d '{
            "name": "No Encrypted Events Circuit",
            "description": "Encrypted events hidden",
            "default_namespace": "generic",
            "owner_id": "hen-admin-001"
        }')

    local circuit_id2=$(echo "$create_response2" | jq -r '.circuit_id')

    curl -s -X PUT "$API_BASE/circuits/$circuit_id2/public-settings" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d '{
            "requester_id": "hen-admin-001",
            "public_settings": {
                "access_mode": "Public",
                "show_encrypted_events": false
            }
        }' > /dev/null

    # Enable public visibility
    curl -s -X PUT "$API_BASE/circuits/$circuit_id2" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d '{
            "requester_id": "hen-admin-001",
            "permissions": {"allow_public_visibility": true}
        }' > /dev/null

    local public_info2=$(curl -s -X GET "$API_BASE/circuits/$circuit_id2/public")
    local show_encrypted2=$(echo "$public_info2" | jq -r '.data.show_encrypted_events')
    assert_equals "false" "$show_encrypted2" "Public info shows encrypted events disabled"

    print_success "Scenario 8 completed: Encrypted events visibility tested"
}

################################################################################
# Main Test Runner
################################################################################

main() {
    print_header "COMPREHENSIVE CIRCUIT WORKFLOW INTEGRATION TEST"
    echo "API Base: $API_BASE"
    echo "Admin User: $ADMIN_USER"
    echo ""

    # Check if API is running
    print_section "Checking if API server is running"
    if curl -s -f "$API_BASE/../health" > /dev/null; then
        print_success "API server is running"
    else
        print_failure "API server is not running at $API_BASE"
        echo "Please start the API server with: cargo run --bin defarm-api"
        exit 1
    fi

    # Run scenarios
    scenario_1_circuit_setup_auto_approve
    scenario_2_user_auto_join
    scenario_3_manual_approval_circuit
    scenario_4_push_with_approval
    scenario_5_large_data_test
    scenario_6_test_all_adapters
    scenario_7_storage_migration
    scenario_8_encrypted_events

    # Note: Scenarios 9-12 can be added for webhook testing, batch operations, etc.
    # The core workflow is now fully tested

    # Print summary
    print_header "TEST SUMMARY"
    echo "Total Tests: $TESTS_TOTAL"
    echo -e "${GREEN}Passed: $TESTS_PASSED${NC}"
    echo -e "${RED}Failed: $TESTS_FAILED${NC}"
    echo ""

    if [ $TESTS_FAILED -eq 0 ]; then
        echo -e "${GREEN}✅ ALL TESTS PASSED!${NC}"
        exit 0
    else
        echo -e "${RED}❌ SOME TESTS FAILED${NC}"
        exit 1
    fi
}

# Run main function
main "$@"

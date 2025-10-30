#!/bin/bash
#
# Demo Data Seeding Script
# Populates production with realistic demo circuits from demo_seed_data.json
#
# Usage: ./scripts/seed_demo_data.sh [API_BASE] [DATA_FILE]
#
# Examples:
#   ./scripts/seed_demo_data.sh
#   ./scripts/seed_demo_data.sh https://defarm-engines-api-production.up.railway.app
#   ./scripts/seed_demo_data.sh http://localhost:3000 /path/to/data.json

set -uo pipefail

# Configuration
API_BASE="${1:-https://defarm-engines-api-production.up.railway.app}"
DATA_FILE="${2:-/Users/gabrielrondon/Downloads/demo_seed_data.json}"
LOG_FILE="/tmp/seed_demo_data_$(date +%Y%m%d_%H%M%S).log"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Progress tracking
USERS_CREATED=0
CIRCUITS_CREATED=0
ITEMS_CREATED=0
EVENTS_CREATED=0
ERRORS=0

log() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $*" | tee -a "$LOG_FILE"
}

success() {
    echo -e "${GREEN}✅ $*${NC}" | tee -a "$LOG_FILE"
}

warn() {
    echo -e "${YELLOW}⚠️  $*${NC}" | tee -a "$LOG_FILE"
}

error() {
    echo -e "${RED}❌ $*${NC}" | tee -a "$LOG_FILE"
    ((ERRORS++)) || true
}

# Check prerequisites
if ! command -v jq &> /dev/null; then
    error "jq is required but not installed. Install with: brew install jq"
    exit 1
fi

if [ ! -f "$DATA_FILE" ]; then
    error "Data file not found: $DATA_FILE"
    exit 1
fi

log "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
log "Demo Data Seeding Script"
log "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
log "API Base: $API_BASE"
log "Data File: $DATA_FILE"
log "Log File: $LOG_FILE"
log ""

# Health check
log "Checking API health..."
if ! curl -sf "$API_BASE/health" > /dev/null; then
    error "API health check failed at $API_BASE/health"
    exit 1
fi
success "API is healthy"
log ""

# Parse JSON data
log "Parsing demo data..."
USERS=$(jq -r '.users' "$DATA_FILE")
CIRCUITS=$(jq -r '.circuits' "$DATA_FILE")
USER_COUNT=$(echo "$USERS" | jq 'length')
CIRCUIT_COUNT=$(echo "$CIRCUITS" | jq 'length')
log "Found: $USER_COUNT users, $CIRCUIT_COUNT circuits"
log ""

# Store user tokens (using parallel arrays for bash 3.x compatibility)
USER_NAMES=()
USER_TOKENS=()
USER_IDS=()

# Helper function to get token by username
get_user_token() {
    local username=$1
    local i
    for i in "${!USER_NAMES[@]}"; do
        if [ "${USER_NAMES[$i]}" = "$username" ]; then
            echo "${USER_TOKENS[$i]}"
            return 0
        fi
    done
    return 1
}

# Helper function to get user ID by username
get_user_id() {
    local username=$1
    local i
    for i in "${!USER_NAMES[@]}"; do
        if [ "${USER_NAMES[$i]}" = "$username" ]; then
            echo "${USER_IDS[$i]}"
            return 0
        fi
    done
    return 1
}

# Function: Create or login user
create_or_login_user() {
    local username=$1
    local password=$2
    local email=$3
    local tier=$4
    local workspace_name=$5

    log "Processing user: $username (tier: $tier)"

    # Try to login first
    local response
    response=$(curl -s -X POST "$API_BASE/api/auth/login" \
        -H "Content-Type: application/json" \
        -d "{\"username\":\"$username\",\"password\":\"$password\"}" 2>&1)

    local token
    token=$(echo "$response" | jq -r '.token // empty')

    if [ -n "$token" ]; then
        success "User $username already exists, logged in"
        USER_NAMES+=("$username")
        USER_TOKENS+=("$token")
        USER_IDS+=("")  # Placeholder, will be updated later

        # Get user ID from /users/me/profile
        local profile
        profile=$(curl -s -X GET "$API_BASE/api/users/me/profile" \
            -H "Authorization: Bearer $token")
        local user_id
        user_id=$(echo "$profile" | jq -r '.user_id // empty')
        # Update the user ID in the array
        local idx=-1
        local i
        for i in "${!USER_NAMES[@]}"; do
            if [ "${USER_NAMES[$i]}" = "$username" ]; then
                idx=$i
                break
            fi
        done
        if [ $idx -ne -1 ]; then
            USER_IDS[$idx]=$user_id
        fi
        return 0
    fi

    # User doesn't exist, try to create
    log "Creating new user: $username"
    response=$(curl -s -X POST "$API_BASE/api/auth/register" \
        -H "Content-Type: application/json" \
        -d "{
            \"username\":\"$username\",
            \"password\":\"$password\",
            \"email\":\"$email\",
            \"workspace_name\":\"$workspace_name\"
        }" 2>&1)

    token=$(echo "$response" | jq -r '.token // empty')

    if [ -n "$token" ]; then
        success "User $username created successfully"
        USER_NAMES+=("$username")
        USER_TOKENS+=("$token")
        USER_IDS+=("")  # Placeholder, will be updated later
        ((USERS_CREATED++))

        # Get user ID
        local profile
        profile=$(curl -s -X GET "$API_BASE/api/users/me/profile" \
            -H "Authorization: Bearer $token")
        local user_id
        user_id=$(echo "$profile" | jq -r '.user_id // empty')
        # Update the user ID in the array
        local idx=-1
        local i
        for i in "${!USER_NAMES[@]}"; do
            if [ "${USER_NAMES[$i]}" = "$username" ]; then
                idx=$i
                break
            fi
        done
        if [ $idx -ne -1 ]; then
            USER_IDS[$idx]=$user_id
        fi

        # Update tier if not basic
        if [ "$tier" != "basic" ]; then
            log "Upgrading user $username to tier: $tier"
            # Note: This requires admin privileges, may need to be done separately
            warn "Tier upgrade to $tier must be done via admin API"
        fi

        return 0
    fi

    # If signup failed, try login one more time
    log "Signup failed, attempting login..."
    response=$(curl -s -X POST "$API_BASE/api/auth/login" \
        -H "Content-Type: application/json" \
        -d "{\"username\":\"$username\",\"password\":\"$password\"}" 2>&1)

    token=$(echo "$response" | jq -r '.token // empty')

    if [ -n "$token" ]; then
        success "User $username logged in after signup failure"
        USER_NAMES+=("$username")
        USER_TOKENS+=("$token")
        USER_IDS+=("")  # Placeholder, will be updated later

        # Get user ID
        local profile
        profile=$(curl -s -X GET "$API_BASE/api/users/me/profile" \
            -H "Authorization: Bearer $token")
        local user_id
        user_id=$(echo "$profile" | jq -r '.user_id // empty')
        # Update the user ID in the array
        local idx=-1
        local i
        for i in "${!USER_NAMES[@]}"; do
            if [ "${USER_NAMES[$i]}" = "$username" ]; then
                idx=$i
                break
            fi
        done
        if [ $idx -ne -1 ]; then
            USER_IDS[$idx]=$user_id
        fi

        # Update tier if not basic
        if [ "$tier" != "basic" ]; then
            log "Upgrading user $username to tier: $tier"
            warn "Tier upgrade to $tier must be done via admin API"
        fi

        return 0
    fi

    error "Failed to create or login user $username"
    echo "$response" | tee -a "$LOG_FILE"
    return 1
}

# Function: Create circuit
create_circuit() {
    local circuit_json=$1
    local owner_username
    owner_username=$(echo "$circuit_json" | jq -r '.owner_username')
    local circuit_name
    circuit_name=$(echo "$circuit_json" | jq -r '.name')

    log "Creating circuit: $circuit_name (owner: $owner_username)"

    local token
    token=$(get_user_token "$owner_username")
    if [ -z "$token" ]; then
        error "No token found for user $owner_username"
        return 1
    fi

    # Extract circuit configuration
    local description
    description=$(echo "$circuit_json" | jq -r '.description')
    local visibility
    visibility=$(echo "$circuit_json" | jq -r '.visibility')
    local settings
    settings=$(echo "$circuit_json" | jq -c '.settings')
    local alias_config
    alias_config=$(echo "$circuit_json" | jq -c '.alias_config')

    # Create circuit
    local response
    # Determine if we should allow public visibility
    local allow_public_visibility="false"
    local public_settings=""

    if [ "$visibility" = "public" ]; then
        allow_public_visibility="true"
        # Add public settings when visibility is public
        public_settings=",\"public_settings\":{
            \"display_name\":\"$circuit_name\",
            \"description\":\"$description\",
            \"category\":\"agriculture\",
            \"tags\":[\"demo\",\"agriculture\",\"traceability\"]
        }"
    fi

    response=$(curl -s -X POST "$API_BASE/api/circuits" \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        -d "{
            \"name\":\"$circuit_name\",
            \"description\":\"$description\",
            \"allow_public_visibility\":$allow_public_visibility,
            \"require_approval_for_push\":$(echo "$settings" | jq '.require_approval_for_push'),
            \"require_approval_for_pull\":$(echo "$settings" | jq '.require_approval_for_pull'),
            \"auto_approve_members\":$(echo "$settings" | jq '.auto_approve_members'),
            \"alias_config\":$alias_config
            $public_settings
        }" 2>&1)

    local circuit_id
    circuit_id=$(echo "$response" | jq -r '.circuit_id // empty')

    if [ -z "$circuit_id" ]; then
        error "Failed to create circuit $circuit_name"
        echo "$response" | tee -a "$LOG_FILE"
        return 1
    fi

    success "Circuit created: $circuit_name (ID: $circuit_id)"
    ((CIRCUITS_CREATED++))

    # Configure adapter if specified
    local adapter_config
    adapter_config=$(echo "$circuit_json" | jq -c '.adapter_config // empty')
    if [ -n "$adapter_config" ] && [ "$adapter_config" != "null" ]; then
        log "Configuring adapter for circuit $circuit_name"
        local adapter_type
        adapter_type=$(echo "$adapter_config" | jq -r '.adapter_type')
        local sponsor_access
        sponsor_access=$(echo "$adapter_config" | jq -r '.sponsor_adapter_access')

        curl -s -X PUT "$API_BASE/api/circuits/$circuit_id/adapter" \
            -H "Authorization: Bearer $token" \
            -H "Content-Type: application/json" \
            -d "{
                \"adapter_type\":\"$adapter_type\",
                \"sponsor_adapter_access\":$sponsor_access
            }" > /dev/null

        success "Adapter configured: $adapter_type (sponsored: $sponsor_access)"
    fi

    # Add members if specified
    local members
    members=$(echo "$circuit_json" | jq -c '.members // []')
    local member_count
    member_count=$(echo "$members" | jq 'length')

    if [ "$member_count" -gt 0 ]; then
        log "Adding $member_count members to circuit $circuit_name"
        for i in $(seq 0 $((member_count - 1))); do
            local member
            member=$(echo "$members" | jq -c ".[$i]")
            local member_username
            member_username=$(echo "$member" | jq -r '.username')
            local member_role
            member_role=$(echo "$member" | jq -r '.role')
            local member_user_id
            member_user_id=$(get_user_id "$member_username")

            if [ -z "$member_user_id" ]; then
                warn "Cannot add member $member_username - user not found"
                continue
            fi

            curl -s -X POST "$API_BASE/api/circuits/$circuit_id/members" \
                -H "Authorization: Bearer $token" \
                -H "Content-Type: application/json" \
                -d "{
                    \"user_id\":\"$member_user_id\",
                    \"role\":\"$member_role\"
                }" > /dev/null

            success "Added member: $member_username ($member_role)"
        done
    fi

    # Create items in this circuit
    local items
    items=$(echo "$circuit_json" | jq -c '.items // []')
    local item_count
    item_count=$(echo "$items" | jq 'length')

    if [ "$item_count" -gt 0 ]; then
        log "Creating $item_count items in circuit $circuit_name"
        for i in $(seq 0 $((item_count - 1))); do
            local item
            item=$(echo "$items" | jq -c ".[$i]")
            create_item "$circuit_id" "$owner_username" "$item"
        done
    fi

    log ""
}

# Function: Create item with events
create_item() {
    local circuit_id=$1
    local owner_username=$2
    local item_json=$3

    local token
    token=$(get_user_token "$owner_username")
    local item_name
    item_name=$(echo "$item_json" | jq -r '.name')
    local item_description
    item_description=$(echo "$item_json" | jq -r '.description // ""')

    log "  Creating item: $item_name"

    # Extract identifiers and metadata
    local identifiers
    identifiers=$(echo "$item_json" | jq -c '.identifiers // {}')
    local metadata
    metadata=$(echo "$item_json" | jq -c '.metadata // {}')

    # Create local item first
    local response
    response=$(curl -s -X POST "$API_BASE/api/items/local" \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        -d "{
            \"name\":\"$item_name\",
            \"description\":\"$item_description\",
            \"identifiers\":$identifiers,
            \"metadata\":$metadata
        }" 2>&1)

    local local_id
    local_id=$(echo "$response" | jq -r '.data.local_id // .local_id // empty')

    if [ -z "$local_id" ]; then
        error "  Failed to create local item: $item_name"
        echo "$response" | tee -a "$LOG_FILE"
        return 1
    fi

    success "  Local item created with LID: $local_id"

    # Push to circuit to get DFID
    response=$(curl -s -X POST "$API_BASE/api/circuits/$circuit_id/push-local" \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        -d "{
            \"local_id\":\"$local_id\",
            \"requester_id\":\"$(get_user_id "$owner_username")\"
        }" 2>&1)

    local dfid
    dfid=$(echo "$response" | jq -r '.dfid // empty')

    if [ -z "$dfid" ]; then
        error "  Failed to push item to circuit: $item_name"
        echo "$response" | tee -a "$LOG_FILE"
        return 1
    fi

    success "  Item created: $item_name (DFID: $dfid)"
    ((ITEMS_CREATED++))

    # Create events for this item
    local events
    events=$(echo "$item_json" | jq -c '.events // []')
    local event_count
    event_count=$(echo "$events" | jq 'length')

    if [ "$event_count" -gt 0 ]; then
        log "    Creating $event_count events for item $item_name"
        for i in $(seq 0 $((event_count - 1))); do
            local event
            event=$(echo "$events" | jq -c ".[$i]")
            create_event "$dfid" "$owner_username" "$event"
        done
    fi
}

# Function: Create event
create_event() {
    local dfid=$1
    local owner_username=$2
    local event_json=$3

    local token
    token=$(get_user_token "$owner_username")
    local event_type
    event_type=$(echo "$event_json" | jq -r '.event_type')
    local event_description
    event_description=$(echo "$event_json" | jq -r '.description // ""')
    local event_timestamp
    event_timestamp=$(echo "$event_json" | jq -r '.timestamp')
    local event_visibility
    event_visibility=$(echo "$event_json" | jq -r '.visibility // "public"')
    local event_metadata
    event_metadata=$(echo "$event_json" | jq -c '.metadata // {}')

    # Create event
    local response
    response=$(curl -s -X POST "$API_BASE/api/events" \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        -d "{
            \"dfid\":\"$dfid\",
            \"event_type\":\"$event_type\",
            \"description\":\"$event_description\",
            \"timestamp\":\"$event_timestamp\",
            \"visibility\":\"$event_visibility\",
            \"metadata\":$event_metadata
        }" 2>&1)

    local event_id
    event_id=$(echo "$response" | jq -r '.event_id // empty')

    if [ -z "$event_id" ]; then
        warn "    Failed to create event: $event_type"
        return 1
    fi

    ((EVENTS_CREATED++))
}

# Main execution
log "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
log "Step 1: Creating/Authenticating Users"
log "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
log ""

for i in $(seq 0 $((USER_COUNT - 1))); do
    username=$(echo "$USERS" | jq -r ".[$i].username")
    password=$(echo "$USERS" | jq -r ".[$i].password")
    email=$(echo "$USERS" | jq -r ".[$i].email")
    tier=$(echo "$USERS" | jq -r ".[$i].tier")
    workspace_name=$(echo "$USERS" | jq -r ".[$i].workspace_name")

    create_or_login_user "$username" "$password" "$email" "$tier" "$workspace_name"
done

log ""
log "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
log "Step 2: Creating Circuits with Items and Events"
log "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
log ""

for i in $(seq 0 $((CIRCUIT_COUNT - 1))); do
    circuit=$(echo "$CIRCUITS" | jq -c ".[$i]")
    create_circuit "$circuit"
done

log ""
log "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
log "Summary"
log "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
success "Users created: $USERS_CREATED"
success "Circuits created: $CIRCUITS_CREATED"
success "Items created: $ITEMS_CREATED"
success "Events created: $EVENTS_CREATED"

if [ $ERRORS -gt 0 ]; then
    error "Errors encountered: $ERRORS"
    log ""
    warn "Check log file for details: $LOG_FILE"
    exit 1
else
    success "All demo data seeded successfully! 🎉"
    log ""
    log "Log file: $LOG_FILE"
    exit 0
fi

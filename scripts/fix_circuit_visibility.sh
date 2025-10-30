#!/bin/bash
#
# Fix Circuit Visibility Script
# Creates missing demo circuits and ensures they have public visibility
#

set -e

API_BASE="${API_BASE:-https://defarm-engines-api-production.up.railway.app}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1"
}

success() {
    echo -e "${GREEN}✅ $1${NC}"
}

error() {
    echo -e "${RED}❌ $1${NC}"
}

warn() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

# Demo users credentials (all use same password)
DEMO_PASSWORD="DemoPass123!"

# Get token for user
get_token() {
    local username=$1
    local password=$2

    local response
    response=$(curl -s -X POST "$API_BASE/api/auth/login" \
        -H "Content-Type: application/json" \
        -d "{\"username\":\"$username\",\"password\":\"$password\"}")

    echo "$response" | jq -r '.token // empty'
}

# Create circuit with public visibility
create_public_circuit() {
    local token=$1
    local name=$2
    local description=$3
    local adapter_type=$4
    local sponsor_access=$5

    log "Creating circuit: $name"

    local response
    response=$(curl -s -X POST "$API_BASE/api/circuits" \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        -d "{
            \"name\":\"$name\",
            \"description\":\"$description\",
            \"allow_public_visibility\":true,
            \"require_approval_for_push\":false,
            \"require_approval_for_pull\":false,
            \"auto_approve_members\":true,
            \"public_settings\":{
                \"access_mode\":\"Public\",
                \"public_name\":\"$name\",
                \"public_description\":\"$description\"
            },
            \"adapter_config\":{
                \"adapter_type\":\"$adapter_type\",
                \"auto_migrate_existing\":false,
                \"requires_approval\":false,
                \"sponsor_adapter_access\":$sponsor_access
            }
        }")

    local circuit_id
    circuit_id=$(echo "$response" | jq -r '.circuit_id // empty')

    if [ -n "$circuit_id" ]; then
        success "Circuit created: $name (ID: $circuit_id)"

        # Verify it's public
        local is_public
        is_public=$(echo "$response" | jq -r '.permissions.allow_public_visibility')
        if [ "$is_public" = "true" ]; then
            success "✓ Circuit is PUBLIC"
        else
            warn "Circuit created but NOT public, toggling visibility..."
            toggle_visibility "$token" "$circuit_id"
        fi

        echo "$circuit_id"
    else
        error "Failed to create circuit: $name"
        echo "$response" | jq '.'
        echo ""
    fi
}

# Toggle circuit visibility to public
toggle_visibility() {
    local token=$1
    local circuit_id=$2

    log "Toggling visibility for circuit: $circuit_id"

    local response
    response=$(curl -s -X PUT "$API_BASE/api/circuits/$circuit_id/visibility/toggle" \
        -H "Authorization: Bearer $token")

    local visibility
    visibility=$(echo "$response" | jq -r '.visibility // empty')

    if [ "$visibility" = "public" ]; then
        success "Circuit is now PUBLIC"
    else
        error "Failed to make circuit public"
        echo "$response" | jq '.'
    fi
}

# Check if circuit exists
list_circuits() {
    local token=$1

    curl -s "$API_BASE/api/circuits/list" \
        -H "Authorization: Bearer $token" | jq -r '.[].name // empty'
}

log "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
log "Circuit Visibility Fix Script"
log "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
log "API Base: $API_BASE"
log ""

# Circuit 1: Porco Preto - Portugal
log "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
log "Circuit 1: Porco Preto - Portugal"
log "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

TOKEN_PT=$(get_token "demo_owner_pt" "$DEMO_PASSWORD")
if [ -z "$TOKEN_PT" ]; then
    error "Failed to login as demo_owner_pt"
    exit 1
fi
success "Logged in as demo_owner_pt"

log "Checking existing circuits..."
existing_circuits=$(list_circuits "$TOKEN_PT")
if echo "$existing_circuits" | grep -q "Porco Preto"; then
    warn "Circuit 'Porco Preto - Portugal' already exists"
else
    create_public_circuit "$TOKEN_PT" \
        "Porco Preto - Portugal" \
        "Rastreabilidade do Porco Preto Alentejano DOP" \
        "stellar-testnet-ipfs" \
        "true"
fi
log ""

# Circuit 2: Unité de Transformation
log "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
log "Circuit 2: Unité de Transformation"
log "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

TOKEN_FR=$(get_token "demo_owner_fr" "$DEMO_PASSWORDfr]}")
if [ -z "$TOKEN_FR" ]; then
    error "Failed to login as demo_owner_fr"
    exit 1
fi
success "Logged in as demo_owner_fr"

log "Checking existing circuits..."
existing_circuits=$(list_circuits "$TOKEN_FR")
if echo "$existing_circuits" | grep -q "Unité de Transformation"; then
    warn "Circuit 'Unité de Transformation' already exists"
else
    create_public_circuit "$TOKEN_FR" \
        "Unité de Transformation" \
        "Traçabilité de la transformation agroalimentaire" \
        "ipfs-ipfs" \
        "true"
fi
log ""

# Circuit 3: Cadena de Jamón Artesanal
log "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
log "Circuit 3: Cadena de Jamón Artesanal"
log "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

TOKEN_ES=$(get_token "demo_owner_es" "$DEMO_PASSWORDes]}")
if [ -z "$TOKEN_ES" ]; then
    error "Failed to login as demo_owner_es"
    exit 1
fi
success "Logged in as demo_owner_es"

log "Checking existing circuits..."
existing_circuits=$(list_circuits "$TOKEN_ES")
if echo "$existing_circuits" | grep -q "Cadena de Jamón"; then
    warn "Circuit 'Cadena de Jamón Artesanal' already exists"
else
    create_public_circuit "$TOKEN_ES" \
        "Cadena de Jamón Artesanal" \
        "Trazabilidad del jamón ibérico de bellota" \
        "stellar-mainnet-ipfs" \
        "true"
fi
log ""

# Circuit 4: Selo do Boi
log "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
log "Circuit 4: Selo do Boi"
log "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

TOKEN_BR=$(get_token "demo_owner_br" "$DEMO_PASSWORDbr]}")
if [ -z "$TOKEN_BR" ]; then
    error "Failed to login as demo_owner_br"
    exit 1
fi
success "Logged in as demo_owner_br"

log "Checking existing circuits..."
existing_circuits=$(list_circuits "$TOKEN_BR")
if echo "$existing_circuits" | grep -q "Selo do Boi"; then
    warn "Circuit 'Selo do Boi' already exists"
else
    create_public_circuit "$TOKEN_BR" \
        "Selo do Boi" \
        "Rastreabilidade da cadeia bovina brasileira" \
        "ipfs-ipfs" \
        "false"
fi
log ""

log "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
log "Verification: Listing All Public Circuits"
log "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# List circuits for each user and check visibility
for username in demo_owner_pt demo_owner_fr demo_owner_es demo_owner_br; do
    token=$(get_token "$username" "$DEMO_PASSWORD")
    if [ -n "$token" ]; then
        log "Circuits for $username:"
        curl -s "$API_BASE/api/circuits/list" -H "Authorization: Bearer $token" | \
            jq -r '.[] | "  - \(.name): public=\(.permissions.allow_public_visibility)"'
    fi
done

log ""
success "✅ Circuit visibility fix complete!"
log ""
log "To verify, check individual circuit public endpoints:"
log "  GET $API_BASE/api/circuits/{circuit_id}/public"

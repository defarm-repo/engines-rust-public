#!/bin/bash

# Railway Environment Variables Manager
# This script helps manage environment variables for DeFarm services
# Usage: RAILWAY_TOKEN=xxx ./railway-vars.sh [command] [options]

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Default service
SERVICE="defarm-engines-api"

function show_help() {
    echo "Railway Variables Manager for DeFarm"
    echo ""
    echo "Usage: RAILWAY_TOKEN=xxx ./railway-vars.sh [command] [options]"
    echo ""
    echo "Commands:"
    echo "  list [service]    - List all variables for a service"
    echo "  get KEY [service] - Get specific variable value"
    echo "  set KEY=VALUE [service] - Set a variable"
    echo "  backup [service]  - Backup variables to .env.backup"
    echo "  compare          - Compare variables across services"
    echo ""
    echo "Services: defarm-engines-api, ipcm-event-listener, postgres, redis"
    echo ""
    echo "Examples:"
    echo "  ./railway-vars.sh list"
    echo "  ./railway-vars.sh get DATABASE_URL"
    echo "  ./railway-vars.sh set LOG_LEVEL=debug"
    echo "  ./railway-vars.sh backup defarm-engines-api"
}

function check_token() {
    if [ -z "$RAILWAY_TOKEN" ]; then
        echo -e "${RED}❌ Error: RAILWAY_TOKEN not set${NC}"
        echo "Get token from: https://railway.com/account/tokens"
        exit 1
    fi
}

function list_vars() {
    local service=${1:-$SERVICE}
    echo -e "${BLUE}📋 Variables for service: $service${NC}"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    if railway variables --service "$service" 2>/dev/null; then
        echo -e "${GREEN}✅ Variables listed successfully${NC}"
    else
        echo -e "${RED}❌ Failed to list variables${NC}"
    fi
}

function get_var() {
    local key=$1
    local service=${2:-$SERVICE}

    if [ -z "$key" ]; then
        echo -e "${RED}❌ Error: Variable key required${NC}"
        exit 1
    fi

    echo -e "${BLUE}🔍 Getting $key from $service${NC}"
    value=$(railway variables --service "$service" 2>/dev/null | grep "^$key=" | cut -d'=' -f2-)

    if [ -n "$value" ]; then
        echo -e "${GREEN}$key=$value${NC}"
    else
        echo -e "${YELLOW}⚠️  Variable $key not found${NC}"
    fi
}

function set_var() {
    local keyvalue=$1
    local service=${2:-$SERVICE}

    if [ -z "$keyvalue" ] || [[ ! "$keyvalue" =~ ^[^=]+=.+$ ]]; then
        echo -e "${RED}❌ Error: Invalid format. Use KEY=VALUE${NC}"
        exit 1
    fi

    echo -e "${BLUE}⚙️  Setting $keyvalue for $service${NC}"

    if railway variables --service "$service" --set "$keyvalue" --skip-deploys 2>/dev/null; then
        echo -e "${GREEN}✅ Variable set successfully${NC}"
        echo -e "${YELLOW}⚠️  Note: Redeploy required for changes to take effect${NC}"
        echo "Run: railway redeploy --service $service"
    else
        echo -e "${RED}❌ Failed to set variable${NC}"
    fi
}

function backup_vars() {
    local service=${1:-$SERVICE}
    local backup_file=".env.backup.${service}.$(date +%Y%m%d_%H%M%S)"

    echo -e "${BLUE}💾 Backing up variables for $service${NC}"

    if railway variables --service "$service" --kv 2>/dev/null > "$backup_file"; then
        echo -e "${GREEN}✅ Variables backed up to: $backup_file${NC}"
        echo "File contains $(wc -l < "$backup_file") variables"
    else
        echo -e "${RED}❌ Backup failed${NC}"
        rm -f "$backup_file"
    fi
}

function compare_services() {
    echo -e "${BLUE}🔄 Comparing variables across services${NC}"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    services=("defarm-engines-api" "ipcm-event-listener")

    for service in "${services[@]}"; do
        echo ""
        echo -e "${YELLOW}Service: $service${NC}"
        railway variables --service "$service" 2>/dev/null | grep -E "DATABASE_URL|JWT_SECRET|STELLAR" | head -5 || echo "No critical vars found"
    done
}

# Main execution
check_token

case "$1" in
    list)
        list_vars "$2"
        ;;
    get)
        get_var "$2" "$3"
        ;;
    set)
        set_var "$2" "$3"
        ;;
    backup)
        backup_vars "$2"
        ;;
    compare)
        compare_services
        ;;
    help|--help|-h|"")
        show_help
        ;;
    *)
        echo -e "${RED}Unknown command: $1${NC}"
        show_help
        exit 1
        ;;
esac
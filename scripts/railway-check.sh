#!/bin/bash

# Railway Service Health Check Script
# This script checks the status of all DeFarm services on Railway
# Usage: RAILWAY_TOKEN=xxx ./railway-check.sh

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🚂 Railway DeFarm Services Health Check"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Check if RAILWAY_TOKEN is set
if [ -z "$RAILWAY_TOKEN" ]; then
    echo -e "${RED}❌ Error: RAILWAY_TOKEN not set${NC}"
    echo "Please set: export RAILWAY_TOKEN=your-token"
    echo "Get token from: https://railway.com/account/tokens"
    exit 1
fi

# Services to check
SERVICES=("defarm-engines-api" "ipcm-event-listener" "postgres" "redis")
API_URL="https://defarm-engines-api-production.up.railway.app"

echo "📊 Checking Railway CLI Access..."
if railway status 2>/dev/null | grep -q "Project:"; then
    echo -e "${GREEN}✅ Railway CLI authenticated${NC}"
    railway status
else
    echo -e "${RED}❌ Railway CLI authentication failed${NC}"
fi

echo ""
echo "🔍 Checking Service Status..."
echo "--------------------------------"

# Check each service logs
for service in "${SERVICES[@]}"; do
    echo ""
    echo "📦 Service: $service"

    # Try to get last log entry
    if railway logs --service "$service" 2>/dev/null | tail -1 | grep -q .; then
        echo -e "${GREEN}✅ Service is running${NC}"
        echo "Last log entry:"
        railway logs --service "$service" 2>/dev/null | tail -3
    else
        echo -e "${YELLOW}⚠️  No recent logs or service not found${NC}"
    fi
done

echo ""
echo "🌐 Checking API Endpoint..."
echo "--------------------------------"

# Check API health with longer timeout
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" -m 60 "$API_URL/health" 2>/dev/null || echo "000")

if [ "$HTTP_CODE" = "200" ]; then
    echo -e "${GREEN}✅ API is healthy (HTTP $HTTP_CODE)${NC}"
    echo "API Response:"
    curl -s "$API_URL/health" | jq '.' 2>/dev/null || curl -s "$API_URL/health"
elif [ "$HTTP_CODE" = "000" ]; then
    echo -e "${RED}❌ API timeout or connection failed${NC}"
    echo "Possible causes:"
    echo "  - Service is hibernated (free tier)"
    echo "  - Service crashed"
    echo "  - Network issues"
else
    echo -e "${YELLOW}⚠️  API returned HTTP $HTTP_CODE${NC}"
fi

echo ""
echo "🔧 Environment Variables Check..."
echo "--------------------------------"

# Check if we can read environment variables
if railway variables --service defarm-engines-api 2>/dev/null | grep -q "JWT_SECRET"; then
    echo -e "${GREEN}✅ Can read environment variables${NC}"
    echo "Key variables configured:"
    railway variables --service defarm-engines-api 2>/dev/null | grep -E "DATABASE_URL|JWT_SECRET|STELLAR" | head -5
else
    echo -e "${YELLOW}⚠️  Cannot read environment variables${NC}"
fi

echo ""
echo "📈 Quick Actions:"
echo "--------------------------------"
echo "• View logs:     railway logs --service defarm-engines-api"
echo "• SSH access:    railway ssh --service defarm-engines-api"
echo "• Redeploy:      railway redeploy --service defarm-engines-api"
echo "• Open dashboard: railway open"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
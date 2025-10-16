#!/bin/bash
# Health check script for Railway deployment

set -e

API_URL="${1:-https://defarm-engines-api-production.up.railway.app}"

echo "🏥 Checking API health at $API_URL"
echo ""

# Check basic health endpoint
echo "1️⃣ Checking /health endpoint..."
HEALTH_RESPONSE=$(curl -s -w "\nHTTP_CODE:%{http_code}" "$API_URL/health")
HTTP_CODE=$(echo "$HEALTH_RESPONSE" | grep "HTTP_CODE" | cut -d: -f2)
RESPONSE_BODY=$(echo "$HEALTH_RESPONSE" | sed '/HTTP_CODE/d')

if [ "$HTTP_CODE" = "200" ]; then
    echo "✅ Health endpoint OK"
    echo "   Response: $RESPONSE_BODY"
else
    echo "❌ Health endpoint failed with HTTP $HTTP_CODE"
    exit 1
fi

echo ""

# Check database health endpoint
echo "2️⃣ Checking /health/db endpoint..."
DB_HEALTH_RESPONSE=$(curl -s -w "\nHTTP_CODE:%{http_code}" "$API_URL/health/db")
DB_HTTP_CODE=$(echo "$DB_HEALTH_RESPONSE" | grep "HTTP_CODE" | cut -d: -f2)
DB_RESPONSE_BODY=$(echo "$DB_HEALTH_RESPONSE" | sed '/HTTP_CODE/d')

if [ "$DB_HTTP_CODE" = "200" ]; then
    echo "✅ Database health endpoint OK"
    echo "   Response: $DB_RESPONSE_BODY"
elif [ "$DB_HTTP_CODE" = "503" ]; then
    echo "⚠️  Database not connected (using in-memory storage)"
    echo "   Response: $DB_RESPONSE_BODY"
else
    echo "❌ Database health check failed with HTTP $DB_HTTP_CODE"
fi

echo ""

# Test authentication
echo "3️⃣ Testing authentication..."
AUTH_RESPONSE=$(curl -s -X POST "$API_URL/api/auth/login" \
    -H "Content-Type: application/json" \
    -d '{"username":"hen","password":"demo123"}' \
    -w "\nHTTP_CODE:%{http_code}")

AUTH_HTTP_CODE=$(echo "$AUTH_RESPONSE" | grep "HTTP_CODE" | cut -d: -f2)
AUTH_BODY=$(echo "$AUTH_RESPONSE" | sed '/HTTP_CODE/d')

if [ "$AUTH_HTTP_CODE" = "200" ]; then
    echo "✅ Authentication OK"
    TOKEN=$(echo "$AUTH_BODY" | grep -o '"token":"[^"]*' | cut -d'"' -f4)
    if [ -n "$TOKEN" ]; then
        echo "   Token received: ${TOKEN:0:50}..."
    fi
else
    echo "❌ Authentication failed with HTTP $AUTH_HTTP_CODE"
    echo "   Response: $AUTH_BODY"
    exit 1
fi

echo ""
echo "🎉 All health checks passed!"
echo ""
echo "📊 Summary:"
echo "   • API Status: ✅ Operational"
echo "   • Database: $([ "$DB_HTTP_CODE" = "200" ] && echo "✅ Connected" || echo "⚠️  In-Memory")"
echo "   • Authentication: ✅ Working"
echo ""
echo "Ready for frontend integration!"

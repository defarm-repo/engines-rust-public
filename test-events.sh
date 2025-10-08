#!/bin/bash

# Event Testing Script for Frontend
# Usage: ./test-events.sh [DFID]

DFID=${1:-"DFID-20251006-000001-8AA5"}
BASE_URL="http://localhost:3000"

echo "========================================"
echo "  DeFarm Event Testing Script"
echo "========================================"
echo ""
echo "Testing DFID: $DFID"
echo "Base URL: $BASE_URL"
echo ""

# Step 1: Login to get JWT token
echo "1️⃣  Logging in as admin (hen)..."
LOGIN_RESPONSE=$(curl -s -X POST "$BASE_URL/api/auth/login" \
  -H "Content-Type: application/json" \
  -d '{
    "username": "hen",
    "password": "demo123"
  }')

TOKEN=$(echo "$LOGIN_RESPONSE" | grep -o '"token":"[^"]*' | cut -d'"' -f4)

if [ -z "$TOKEN" ]; then
  echo "❌ Login failed!"
  echo "Response: $LOGIN_RESPONSE"
  exit 1
fi

echo "✅ Login successful"
echo "Token: ${TOKEN:0:50}..."
echo ""

# Step 2: Check if item exists
echo "2️⃣  Checking if item exists..."
ITEM_RESPONSE=$(curl -s "$BASE_URL/api/items/$DFID" \
  -H "Authorization: Bearer $TOKEN")

if echo "$ITEM_RESPONSE" | grep -q '"dfid"'; then
  echo "✅ Item exists"
  echo ""
  echo "Item details:"
  echo "$ITEM_RESPONSE" | jq '.' 2>/dev/null || echo "$ITEM_RESPONSE"
else
  echo "❌ Item NOT found"
  echo "Response: $ITEM_RESPONSE"
  echo ""
  echo "Creating test item to demonstrate event creation..."

  # Create a test item
  CREATE_RESPONSE=$(curl -s -X POST "$BASE_URL/api/items" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $TOKEN" \
    -d '{
      "identifiers": [
        {"key": "lot_number", "value": "TEST-'$(date +%s)'"}
      ],
      "enriched_data": {
        "name": "Test Item for Event Testing",
        "description": "Created by test script to verify event creation"
      }
    }')

  NEW_DFID=$(echo "$CREATE_RESPONSE" | grep -o '"dfid":"[^"]*' | cut -d'"' -f4)

  if [ -n "$NEW_DFID" ]; then
    echo "✅ Created test item: $NEW_DFID"
    DFID="$NEW_DFID"
  else
    echo "❌ Failed to create test item"
    echo "Response: $CREATE_RESPONSE"
    exit 1
  fi
fi
echo ""

# Step 3: Check events for item
echo "3️⃣  Checking events for item..."
EVENTS_RESPONSE=$(curl -s "$BASE_URL/api/events/item/$DFID" \
  -H "Authorization: Bearer $TOKEN")

if echo "$EVENTS_RESPONSE" | grep -q "error"; then
  echo "❌ Error fetching events"
  echo "Response: $EVENTS_RESPONSE"
  exit 1
fi

EVENT_COUNT=$(echo "$EVENTS_RESPONSE" | jq '. | length' 2>/dev/null || echo "0")

echo "📊 Found $EVENT_COUNT events"
echo ""

if [ "$EVENT_COUNT" -gt 0 ]; then
  echo "✅ EVENTS ARE BEING CREATED AND STORED!"
  echo ""
  echo "Event details:"
  echo "$EVENTS_RESPONSE" | jq '.' 2>/dev/null || echo "$EVENTS_RESPONSE"
  echo ""

  # Show event types
  echo "Event types found:"
  echo "$EVENTS_RESPONSE" | jq -r '.[].event_type' 2>/dev/null | sort | uniq -c
  echo ""

  # Show visibility
  echo "Event visibility:"
  echo "$EVENTS_RESPONSE" | jq -r '.[].visibility' 2>/dev/null | sort | uniq -c
  echo ""

  # Show encryption status
  echo "Encryption status:"
  echo "$EVENTS_RESPONSE" | jq -r 'if .[].is_encrypted then "Encrypted" else "Not Encrypted" end' 2>/dev/null | sort | uniq -c

else
  echo "❌ NO EVENTS FOUND"
  echo ""
  echo "This indicates a problem with event creation."
  echo "Check: src/items_engine.rs - verify events are created when items are created"
fi

echo ""
echo "========================================"
echo "  Test Complete"
echo "========================================"

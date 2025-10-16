# Frontend Event Testing Guide

**Date**: 2025-10-06
**Status**: Testing Instructions

## Important: No SQL Database

Our system uses **in-memory storage**, not a SQL database. The SQL queries from the frontend message won't work. Instead, use these HTTP endpoints.

## Test DFID from Frontend Message

Testing for: `DFID-20251006-000001-8AA5`

### Step 1: Check if Item Exists

```bash
curl http://localhost:3000/api/items/DFID-20251006-000001-8AA5
```

**Expected Response**:
```json
{
  "dfid": "DFID-20251006-000001-8AA5",
  "identifiers": [...],
  "enriched_data": {...},
  "creation_timestamp": 1728240000,
  "last_modified": 1728240000,
  "source_entries": [...],
  "confidence_score": 1.0,
  "status": "Active"
}
```

**If 404**: Item doesn't exist in system

### Step 2: Check Events for Item

```bash
curl http://localhost:3000/api/events/item/DFID-20251006-000001-8AA5
```

**Expected Response** (should show 2 events as mentioned by frontend):
```json
[
  {
    "event_id": "uuid-1",
    "dfid": "DFID-20251006-000001-8AA5",
    "event_type": "Created",
    "timestamp": "2025-10-06T18:00:00Z",
    "source": "user-123",
    "metadata": {},
    "is_encrypted": false,
    "visibility": "Public",
    "content_hash": "blake3_hash..."
  },
  {
    "event_id": "uuid-2",
    "dfid": "DFID-20251006-000001-8AA5",
    "event_type": "Enriched",
    "timestamp": "2025-10-06T18:01:00Z",
    "source": "user-123",
    "metadata": {},
    "is_encrypted": false,
    "visibility": "Public",
    "content_hash": "blake3_hash..."
  }
]
```

**If empty array `[]`**: Events weren't saved when item was created

### Step 3: Check Public Circuit Events (if applicable)

If this item is in a public circuit:

```bash
curl http://localhost:3000/api/circuits/{circuit_id}/public
```

Look for `DFID-20251006-000001-8AA5` in the `published_items` array.

## Diagnostic Scenarios

### Scenario A: Item exists, no events
```bash
# Item exists
curl http://localhost:3000/api/items/DFID-20251006-000001-8AA5
# → 200 OK with item data

# But events are empty
curl http://localhost:3000/api/events/item/DFID-20251006-000001-8AA5
# → [] (empty array)
```

**Diagnosis**: Events not created during item creation
**Check**: `items_engine.rs` - verify event creation in `create_item()`

### Scenario B: Events exist, but not in public endpoint
```bash
# Events exist
curl http://localhost:3000/api/events/item/DFID-20251006-000001-8AA5
# → [event1, event2]

# But public circuit doesn't include them
curl http://localhost:3000/api/circuits/{circuit_id}/public
# → published_items has DFID but no events
```

**Diagnosis**: Public endpoint not including events
**Check**: `circuits_engine.rs:1164` - `get_public_circuit_info()` method

### Scenario C: Everything exists, but visibility filtered
```bash
# Events exist
curl http://localhost:3000/api/events/item/DFID-20251006-000001-8AA5
# → [event1, event2]

# But visibility != Public
# Check event.visibility field in response
```

**Diagnosis**: Events have Private/CircuitOnly/Direct visibility
**Solution**: Update event visibility when creating or use `show_encrypted_events` flag

## Creating Test Data

If no items exist, create a test item:

```bash
# Login first to get JWT
curl -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "username": "hen",
    "password": "demo123"
  }'
# → Save the "token" from response

# Create item
TOKEN="<token-from-above>"
curl -X POST http://localhost:3000/api/items \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "identifiers": [
      {"key": "lot_number", "value": "TEST-001"}
    ],
    "enriched_data": {
      "name": "Test Item",
      "description": "Testing event creation"
    }
  }'
# → Save the "dfid" from response

# Check events for new item
DFID="<dfid-from-above>"
curl http://localhost:3000/api/events/item/$DFID
# → Should see Created event
```

## Quick Test Script

Save this as `test-events.sh`:

```bash
#!/bin/bash

DFID=${1:-"DFID-20251006-000001-8AA5"}
BASE_URL="http://localhost:3000"

echo "=== Testing Events for DFID: $DFID ==="
echo ""

echo "1. Checking if item exists..."
ITEM_RESPONSE=$(curl -s "$BASE_URL/api/items/$DFID")
if echo "$ITEM_RESPONSE" | grep -q "dfid"; then
  echo "✅ Item exists"
else
  echo "❌ Item not found"
  exit 1
fi
echo ""

echo "2. Checking events..."
EVENTS_RESPONSE=$(curl -s "$BASE_URL/api/events/item/$DFID")
EVENT_COUNT=$(echo "$EVENTS_RESPONSE" | jq '. | length' 2>/dev/null || echo "0")

if [ "$EVENT_COUNT" -gt 0 ]; then
  echo "✅ Found $EVENT_COUNT events"
  echo ""
  echo "Events:"
  echo "$EVENTS_RESPONSE" | jq '.' 2>/dev/null || echo "$EVENTS_RESPONSE"
else
  echo "❌ No events found"
  echo ""
  echo "Diagnosis: Events weren't created for this item"
  echo "Action: Check item creation flow in items_engine.rs"
fi
```

Usage:
```bash
chmod +x test-events.sh
./test-events.sh                                      # Test default DFID
./test-events.sh DFID-20251006-000001-8AA5           # Test specific DFID
```

## Expected Behavior

When an item is created:
1. ItemsEngine creates the item
2. EventsEngine creates a "Created" event
3. Event is stored with BLAKE3 content_hash
4. Event is accessible via `/api/events/item/{dfid}`

When an item is enriched:
1. ItemsEngine updates the item
2. EventsEngine creates an "Enriched" event
3. New event added to item's event history

## Troubleshooting

### "No events found"
1. Check if EventsEngine is called during item creation
2. Verify events are saved to storage
3. Check event visibility settings

### "Events exist but not in public endpoint"
1. Verify event visibility is "Public"
2. Check `show_encrypted_events` circuit setting
3. Ensure `get_public_circuit_info()` includes events

### "Events show different DFID"
1. Check DFID generation consistency
2. Verify item and event use same DFID
3. Check for DFID conflicts or duplicates

## Response to Frontend

**Re: SQL Query Request**

We don't use SQL database - the system uses in-memory storage. Use these HTTP endpoints instead:

```bash
# Check events for the DFID you mentioned
curl http://localhost:3000/api/events/item/DFID-20251006-000001-8AA5
```

**If you see 2 events**: ✅ Backend is working, events are saved
**If you see empty array**: ❌ Events not created, need to investigate item creation flow
**If you get 404**: ❌ Item doesn't exist, check if it was created properly

The system is currently running on port 3000. Test it now and let me know what you find.

---

**Files to Check if Events Missing**:
- `src/items_engine.rs` - Item creation should trigger event creation
- `src/events_engine.rs` - Event storage logic
- `src/storage.rs` - In-memory storage implementation

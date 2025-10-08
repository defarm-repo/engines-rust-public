# ISSUE: Events Stored in Item Data, Not EventsEngine

**Date**: 2025-10-06
**Severity**: Medium
**Status**: Identified

## Problem Summary

The frontend is storing events inside `item.enriched_data.events[]` instead of using the EventsEngine. This means:

❌ Events are not tracked by EventsEngine
❌ Event synchronization doesn't work
❌ Event visibility filtering doesn't apply
❌ Content-based deduplication doesn't work
❌ Event querying by type/visibility doesn't work

## Test Results

### Item DFID: `DFID-20251006-000001-8AA5`

**Item Data** (from `/api/items/{dfid}`):
```json
{
  "enriched_data": {
    "events": [
      {
        "author": "pullet-user-001",
        "content": "teste",
        "event_type": "feeding",
        "is_encrypted": true,
        "metadata": {},
        "source": "pullet-user-001",
        "timestamp": 1759781123,
        "type": "feeding",
        "visibility": "Private"
      },
      {
        "author": "pullet-user-001",
        "content": "teste",
        "event_type": "story",
        "is_encrypted": false,
        "metadata": {},
        "source": "pullet-user-001",
        "timestamp": 1759781219,
        "type": "story",
        "visibility": "Public"
      }
    ]
  }
}
```

**Events Count** (from `/api/events/item/{dfid}`):
```json
[]  // ❌ Zero events!
```

## Root Cause

Frontend is using **legacy event storage pattern** where events are embedded in item data:
```
Item.enriched_data.events[]
```

Instead of using the **proper EventsEngine pattern**:
```
EventsEngine.create_event() → Separate Event table/storage
```

## Impact

1. **Event Synchronization Won't Work**: Our new push/pull system expects events in EventsEngine
2. **No Event History**: Can't query event timeline for an item
3. **No Visibility Filtering**: Events in enriched_data don't respect visibility rules
4. **No Deduplication**: Same events can be duplicated in enriched_data
5. **No Cross-References**: Can't find all events of a type across items

## Migration Path

### Option 1: Backend Migration (Recommended)

When an item is loaded, migrate embedded events to EventsEngine:

```rust
// In items_engine.rs or API layer
pub fn migrate_embedded_events(item: &Item, events_engine: &mut EventsEngine) {
    if let Some(events_array) = item.enriched_data.get("events") {
        if let Some(events) = events_array.as_array() {
            for event_data in events {
                // Create proper Event object
                let event = Event::new(
                    item.dfid.clone(),
                    parse_event_type(event_data),
                    event_data["source"].as_str().unwrap().to_string(),
                    parse_visibility(event_data)
                );

                // Store in EventsEngine
                events_engine.create_event(event)?;
            }

            // Remove from enriched_data
            item.enriched_data.remove("events");
        }
    }
}
```

### Option 2: Frontend Migration

Update frontend to use EventsEngine API:

**Before** (current):
```typescript
// Creating item with embedded events
POST /api/items
{
  "identifiers": [...],
  "enriched_data": {
    "title": "...",
    "events": [...]  // ❌ Don't do this
  }
}
```

**After** (correct):
```typescript
// Step 1: Create item
const item = await POST('/api/items', {
  identifiers: [...],
  enriched_data: { title: "..." }
});

// Step 2: Create events separately
for (const event of events) {
  await POST('/api/events', {
    dfid: item.dfid,
    event_type: event.type,
    source: currentUserId,
    visibility: event.visibility,
    is_encrypted: event.is_encrypted,
    metadata: event.metadata
  });
}
```

### Option 3: Hybrid Approach

1. Keep embedded events for backward compatibility
2. Also create proper Event objects
3. Gradually phase out embedded events

## Immediate Actions

1. ✅ **Document the issue** (this file)
2. 📋 **Share findings with frontend team**
3. 🤝 **Decide on migration strategy** (frontend vs backend)
4. 🔧 **Implement migration**
5. ✅ **Test event synchronization** after migration
6. 📝 **Update documentation**

## Testing After Fix

After migration, this should pass:

```bash
# Item should exist
curl http://localhost:3000/api/items/DFID-20251006-000001-8AA5
# → Returns item

# Events should exist in EventsEngine
curl http://localhost:3000/api/events/item/DFID-20251006-000001-8AA5 \
  -H "Authorization: Bearer $TOKEN"
# → Returns 2 events (same ones from enriched_data)

# Event synchronization should work
curl http://localhost:3000/api/circuits/{circuit_id}/items?include_events=true
# → Returns items with events from EventsEngine
```

## Decision Required

**Which migration approach should we take?**

- [ ] Option 1: Backend auto-migration
- [ ] Option 2: Frontend API update
- [ ] Option 3: Hybrid approach

## Related Files

- `src/items_engine.rs` - Item creation
- `src/events_engine.rs` - Event storage
- `src/api/items.rs` - Items API
- `src/api/events.rs` - Events API (created but not used by frontend)

## Communication to Frontend

Hi Frontend Team,

We found the issue! 🎯

**Problem**: Events are being stored in `item.enriched_data.events[]` instead of using the EventsEngine.

**Why it matters**:
- Event synchronization (push/pull) won't work with embedded events
- Event visibility filtering doesn't apply
- Can't query events across items

**What we found**:
- Item `DFID-20251006-000001-8AA5` has 2 events in `enriched_data.events`
- But `/api/events/item/{dfid}` returns 0 events
- Events exist in item data but not in event tracking system

**Solution Options**:
1. Backend can auto-migrate embedded events to EventsEngine
2. Frontend can update to use `/api/events` endpoint
3. Hybrid: support both during transition

**Which do you prefer?**

Let's discuss the migration path. The EventsEngine is ready to use, we just need to route events through it instead of embedding them in items.

---

**Test Script Available**: `test-events.sh`
Run it to verify events after migration.

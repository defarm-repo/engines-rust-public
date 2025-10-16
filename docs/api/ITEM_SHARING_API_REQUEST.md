# Item Sharing API Endpoints Request

**To: Backend Team**
**From: Frontend Team**
**Date: 2025-09-27**
**Priority: Medium - Required for completing localStorage migration**

## Summary

While completing the migration from localStorage to API for items, we've identified that **item sharing functionality** requires dedicated API endpoints. Currently `ItemShare.tsx` and `CircuitPublicPage.tsx` use localStorage for sharing workflows, but this needs to be migrated to the API to complete the integration.

## Current localStorage Operations That Need API

### 1. Item Sharing Operations (from `ItemShare.tsx`)

```typescript
// Current localStorage operations:
itemStorage.loadByCreator(currentUser)           // Get user's items for sharing
itemShareStorage.loadForUser(currentUser)       // Get items shared WITH user
itemShareStorage.loadAll()                      // Check for existing shares
itemShareStorage.save(share)                    // Create new share
itemStorage.load(selectedItem)                  // Get item details for display
itemShareStorage.loadByItem(item.id)            // Get all shares for an item
```

### 2. Circuit Public Page Access (from `CircuitPublicPage.tsx`)

```typescript
// Current localStorage operations:
itemShareStorage.loadForUser(currentUser)       // Check user's shared access
```

## Proposed API Endpoints

### Item Sharing Management

```typescript
// 1. Get user's own items (for sharing)
GET /api/items?creator={userId}
// Response: ItemData[] - user's items available for sharing

// 2. Share an item with a user
POST /api/items/{dfid}/share
// Request body: { recipient_user_id: string, permissions?: string[] }
// Response: { share_id: string, dfid: string, recipient_user_id: string, shared_at: timestamp }

// 3. Get items shared WITH current user
GET /api/users/{userId}/shared-items
// Response: Array<{ share_id: string, item: ItemData, shared_by: string, shared_at: timestamp }>

// 4. Get all shares for a specific item (admin/owner view)
GET /api/items/{dfid}/shares
// Response: Array<{ share_id: string, recipient_user_id: string, shared_by: string, shared_at: timestamp }>

// 5. Remove/revoke a share
DELETE /api/items/{dfid}/share/{share_id}
// Response: { success: boolean }

// 6. Check if item is shared with specific user
GET /api/items/{dfid}/shared-with/{userId}
// Response: { is_shared: boolean, share_id?: string, shared_at?: timestamp }
```

## Required Data Structures

### Share Response Type
```typescript
interface ItemShare {
  share_id: string;           // Unique share identifier
  dfid: string;              // Item being shared
  shared_by: string;         // User who created the share
  recipient_user_id: string; // User receiving access
  shared_at: number;         // Unix timestamp
  permissions?: string[];    // Optional: specific permissions granted
}

interface SharedItemResponse {
  share_id: string;
  item: ItemData;            // Full item data with enriched_data
  shared_by: string;
  shared_at: number;
  permissions?: string[];
}
```

## Use Cases to Support

### 1. Share Item Workflow
```
User A creates item → User A shares with User B → User B gets access
- POST /api/items/{dfid}/share with recipient_user_id
- User B can then GET /api/users/{userB}/shared-items to see it
```

### 2. View Shared Items
```
User B wants to see all items shared with them
- GET /api/users/{userB}/shared-items
- Returns items with full enriched_data from entity resolution
```

### 3. Manage Item Shares (Admin View)
```
User A wants to see who has access to their item
- GET /api/items/{dfid}/shares
- Returns list of all users with access
```

### 4. Circuit Public Access Control
```
Circuit public page checks if user has shared access to specific items
- GET /api/items/{dfid}/shared-with/{userId}
- Quick boolean check for access control
```

## Integration Benefits

### ✅ **With Item Sharing API:**
- All sharing data stored centrally on server
- Shares work with entity-resolved items (progressive enrichment)
- Better security and access control
- Audit trail for all sharing operations
- Scales with multi-user environments

### ❌ **Current localStorage Limitations:**
- Sharing data only exists locally
- No cross-device synchronization
- Limited to current browser session
- No audit trail or management capabilities

## Technical Notes

### Entity Resolution Integration
- Item shares should reference DFIDs (not internal IDs)
- When items get enriched through entity resolution, shared access is preserved
- Shared item queries should return latest enriched data

### Security Considerations
- Verify user has permission to share items (owns or has share permissions)
- Prevent sharing items that user doesn't have access to
- Consider implementing share permissions (read-only, full access, etc.)

### Performance
- Consider caching frequently accessed shares
- Implement pagination for users with many shared items
- Optimize queries for "shared-with" checks

## Implementation Priority

### Phase 1 (Required for localStorage migration)
- ✅ `POST /api/items/{dfid}/share` - Create shares
- ✅ `GET /api/users/{userId}/shared-items` - View received shares
- ✅ `GET /api/items/{dfid}/shared-with/{userId}` - Access check

### Phase 2 (Enhanced management)
- ⭐ `GET /api/items/{dfid}/shares` - Admin view of shares
- ⭐ `DELETE /api/items/{dfid}/share/{share_id}` - Revoke shares
- ⭐ Share permissions system

## Testing Examples

```bash
# Share cow_001 with user456
curl -X POST http://localhost:3000/api/items/DFID-20250927-000003-DC57/share \
  -H "Content-Type: application/json" \
  -d '{"recipient_user_id": "user456"}'

# Check what's shared with user456
curl http://localhost:3000/api/users/user456/shared-items

# Check if cow_001 is shared with user456
curl http://localhost:3000/api/items/DFID-20250927-000003-DC57/shared-with/user456
```

## Next Steps

1. **Backend**: Implement Phase 1 endpoints
2. **Frontend**: Update ItemShare.tsx to use new API
3. **Frontend**: Update CircuitPublicPage.tsx access checks
4. **Testing**: Verify sharing workflows work with entity resolution

---

**Frontend Team**: Ready to begin integration as soon as Phase 1 endpoints are available
**Current Status**: PublicLandingPage.tsx already migrated to items API successfully
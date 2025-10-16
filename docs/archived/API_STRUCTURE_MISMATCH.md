# API Structure Mismatch - Circuit Items & Public Settings

## Progress Update
✅ **Major Progress**: Circuit items endpoint now returns data instead of empty array!
✅ **Permissions Fixed**: `require_approval_for_push: false` is working

## Remaining Issues

### 1. Circuit Items Structure - ✅ RESOLVED

**Frontend Expectation** (based on types.ts):
```typescript
interface CircuitItem {
  circuit_id: string;
  item_dfid: string;        // ← Backend uses 'dfid' instead
  pushed_by: string;
  pushed_at: number;
  permissions: string[];
  status: 'active' | 'removed';  // ← Backend doesn't include this field
}
```

**Actual Backend Response** ✅:
```javascript
// GET /circuits/{id}/items returns:
{
  dfid: "DFID-20250928-000004-4C7F",           // ✅ (field name: 'dfid' not 'item_dfid')
  circuit_id: "f574c18b-45b9-40d2-b50b-9b4024d1772e",
  circuit_name: "Circuit f574c18b-45b9-40d2-b50b-9b4024d1772e",
  pushed_by: "hen",                            // ✅
  pushed_at: 1759091326,                       // ✅
  permissions: ["read", "verify"]              // ✅
  // Note: No 'status' field - if item exists, it's considered active
}
```

**Frontend Fix**: Updated to use `item.dfid` instead of `item.item_dfid` and removed status check.

### 2. Public Settings Structure Mismatch

**Frontend Expectation**:
```javascript
circuit.public_settings.auto_publish_pushed_items  // boolean
```

**Actual Backend Response**:
```javascript
circuit.public_settings: undefined  // Field doesn't exist
```

**Expected Location**: The auto-publish setting should be in `circuit.public_settings.auto_publish_pushed_items`

## Test Data
- **Circuit ID**: `f574c18b-45b9-40d2-b50b-9b4024d1772e`
- **Item DFID**: `DFID-20250928-000004-4C7F`
- **Expected Behavior**: Item should be found in circuit items with status 'active'

## Questions for Backend Team

1. **Circuit Items Structure**: What fields does the `/circuits/{id}/items` endpoint actually return?
   - Does it return `item_dfid` or `dfid`?
   - Does it include `status` field?
   - What's the complete object structure?

2. **Public Settings Location**: Where is the auto-publish setting stored?
   - Is it in `circuit.public_settings`?
   - Or in a different field like `circuit.settings`?
   - What's the complete public settings structure?

3. **API Documentation**: Can you provide the actual response schemas for:
   - `GET /circuits/{id}/items`
   - `GET /circuits/{id}` (full circuit data)

## Remaining Issue: Auto-Publish for Public Pages

**Current Status**: Items are successfully pushed to circuits and appear in circuit items list, but auto-publish to public pages is not working.

**Root Issue**: The `auto_publish_pushed_items` setting is saved by the frontend but not available when fetching circuit data.

**Questions for Backend Team**:
1. **Where are public settings stored?** Frontend saves to `/circuits/{id}/public-settings` but they don't appear in circuit response
2. **Is auto-publish implemented?** When `auto_publish_pushed_items: true`, should pushed items automatically appear in `published_items`?
3. **Alternative approach**: Should there be a manual "publish item" endpoint if auto-publish isn't ready?

**Test Status**:
- ✅ 4/6 steps passing
- ❌ Only auto-publish/public page verification failing

## Next Steps
1. **Share this file** with backend team to clarify public settings structure
2. **Or ask for manual publish endpoint** as interim solution
3. **Complete push-to-public workflow** once auto-publish is working
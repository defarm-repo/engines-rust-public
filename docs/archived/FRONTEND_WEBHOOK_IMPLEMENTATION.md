# Frontend Implementation Guide: Circuit Post-Action Webhooks

## Feature Overview

Implement a webhook management system that allows circuit owners and admins to configure HTTP callbacks that trigger after item push operations. This is a **completely optional** feature that circuit managers can enable to integrate circuits with external systems (ERPs, notification services, analytics platforms, etc.).

## User Persona

**Who uses this?** Circuit owners and administrators who want to:
- Send item data to their ERP system when items are pushed
- Trigger notifications in Slack/Discord/Teams
- Update external databases or analytics platforms
- Integrate with custom business logic systems

## Core Concepts

### Post-Action Settings (Circuit-Level)
- **Enabled/Disabled**: Master switch for the webhook system
- **Trigger Events**: Which operations trigger webhooks
  - `item_pushed`: Any item pushed to circuit
  - `item_tokenized`: New item created (first-time push with LID)
  - `item_approved`: Item approved by admin
  - `item_published`: Item made public
- **Include Storage Details**: Whether to send IPFS/blockchain location info
- **Include Item Metadata**: Whether to send full item data

### Webhooks
- Multiple webhooks can be configured per circuit
- Each webhook has its own URL, authentication, and enabled state
- Webhooks deliver JSON payloads via HTTP POST/PUT/PATCH

## API Endpoints

### Authentication
All endpoints require JWT authentication. Include in header:
```
Authorization: Bearer <jwt_token>
```

### 1. Get Post-Action Settings
```http
GET /api/circuits/:circuit_id/post-actions
```

**Response:**
```json
{
  "success": true,
  "data": {
    "enabled": true,
    "trigger_events": ["item_pushed", "item_tokenized"],
    "include_storage_details": true,
    "include_item_metadata": true,
    "webhooks": [
      {
        "id": "uuid-here",
        "name": "My ERP System",
        "url": "https://api.myerp.com/webhooks/defarm",
        "method": "Post",
        "headers": {
          "X-Custom-Header": "value"
        },
        "auth_type": "BearerToken",
        "auth_credentials": "***hidden***",
        "enabled": true,
        "retry_config": {
          "max_retries": 3,
          "initial_delay_ms": 1000,
          "max_delay_ms": 30000,
          "backoff_multiplier": 2.0
        },
        "created_at": "2025-10-08T10:00:00Z",
        "updated_at": "2025-10-08T10:00:00Z"
      }
    ]
  }
}
```

### 2. Update Post-Action Settings
```http
PUT /api/circuits/:circuit_id/post-actions
Content-Type: application/json
```

**Request Body:**
```json
{
  "enabled": true,
  "trigger_events": ["item_pushed", "item_tokenized"],
  "include_storage_details": true,
  "include_item_metadata": true
}
```

**Response:**
```json
{
  "success": true,
  "message": "Post-action settings updated",
  "data": { /* updated settings */ }
}
```

### 3. Create Webhook
```http
POST /api/circuits/:circuit_id/post-actions/webhooks
Content-Type: application/json
```

**Request Body:**
```json
{
  "name": "My ERP Webhook",
  "url": "https://api.myerp.com/webhooks/defarm",
  "method": "POST",  // Optional: "POST" | "PUT" | "PATCH"
  "headers": {       // Optional
    "X-API-Version": "v1"
  },
  "auth_type": "BearerToken",  // Optional: "None" | "BearerToken" | "ApiKey" | "BasicAuth" | "CustomHeader"
  "auth_credentials": "my-secret-token",  // Optional
  "enabled": true    // Optional, defaults to true
}
```

**Response:**
```json
{
  "success": true,
  "message": "Webhook created successfully",
  "data": { /* webhook object with generated id */ }
}
```

**Error Response (400 - Bad Request):**
```json
{
  "error": "Localhost URLs are not allowed"
}
```

### 4. Get Webhook Details
```http
GET /api/circuits/:circuit_id/post-actions/webhooks/:webhook_id
```

### 5. Update Webhook
```http
PUT /api/circuits/:circuit_id/post-actions/webhooks/:webhook_id
Content-Type: application/json
```

**Request Body (all fields optional):**
```json
{
  "name": "Updated Name",
  "url": "https://new-url.com/webhook",
  "enabled": false
}
```

### 6. Delete Webhook
```http
DELETE /api/circuits/:circuit_id/post-actions/webhooks/:webhook_id
```

**Response:**
```json
{
  "success": true,
  "message": "Webhook deleted successfully"
}
```

### 7. Test Webhook
```http
POST /api/circuits/:circuit_id/post-actions/webhooks/:webhook_id/test
```

**Response:**
```json
{
  "success": true,
  "message": "Webhook test initiated",
  "webhook": {
    "id": "uuid",
    "name": "My Webhook",
    "url": "https://..."
  }
}
```

### 8. Get Delivery History
```http
GET /api/circuits/:circuit_id/post-actions/deliveries?limit=50
```

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "id": "delivery-uuid",
      "webhook_id": "webhook-uuid",
      "circuit_id": "circuit-uuid",
      "trigger_event": "ItemPushed",
      "payload": { /* full webhook payload */ },
      "status": "Delivered",  // "Pending" | "InProgress" | "Delivered" | "Failed" | "Retrying"
      "attempts": 1,
      "response_code": 200,
      "response_body": "OK",
      "created_at": "2025-10-08T10:00:00Z",
      "delivered_at": "2025-10-08T10:00:05Z",
      "error_message": null,
      "next_retry_at": null
    }
  ],
  "count": 1
}
```

## UI/UX Requirements

### Page Structure

Create a new section in the Circuit Settings page:

```
Circuit Settings
├── General
├── Members & Roles
├── Permissions
├── Adapter Configuration
└── Post-Action Webhooks  ← NEW
```

### Post-Action Webhooks Page Layout

#### 1. Header Section
```
┌─────────────────────────────────────────────────┐
│ Post-Action Webhooks                   [Toggle] │
│                                                  │
│ Configure HTTP callbacks that trigger when      │
│ items are pushed to this circuit.               │
└─────────────────────────────────────────────────┘
```

**Toggle Behavior:**
- When OFF: Show disabled state, hide webhook list
- When ON: Show configuration options

#### 2. Configuration Section (when enabled)
```
┌─────────────────────────────────────────────────┐
│ Trigger Events                                  │
│ ☑ Item Pushed        (any item pushed)         │
│ ☑ Item Tokenized     (new item created)        │
│ ☐ Item Approved      (admin approved)          │
│ ☐ Item Published     (made public)             │
│                                                  │
│ Payload Options                                 │
│ ☑ Include storage details (IPFS/blockchain)    │
│ ☑ Include item metadata                        │
│                                                  │
│ [Save Configuration]                            │
└─────────────────────────────────────────────────┘
```

#### 3. Webhooks List
```
┌─────────────────────────────────────────────────┐
│ Configured Webhooks              [+ Add Webhook]│
│                                                  │
│ ┌───────────────────────────────────────────┐  │
│ │ My ERP System               [Toggle On]   │  │
│ │ https://api.myerp.com/...                 │  │
│ │ 🔒 Bearer Token • POST                    │  │
│ │ Last delivery: 2 min ago (Success)        │  │
│ │ [Test] [Edit] [View History] [Delete]    │  │
│ └───────────────────────────────────────────┘  │
│                                                  │
│ ┌───────────────────────────────────────────┐  │
│ │ Slack Notifications         [Toggle Off]  │  │
│ │ https://hooks.slack.com/...               │  │
│ │ 🔓 No Auth • POST                         │  │
│ │ Last delivery: 1 hour ago (Failed)        │  │
│ │ [Test] [Edit] [View History] [Delete]    │  │
│ └───────────────────────────────────────────┘  │
└─────────────────────────────────────────────────┘
```

#### 4. Add/Edit Webhook Modal
```
┌─────────────────────────────────────────────────┐
│ Add Webhook                             [Close] │
├─────────────────────────────────────────────────┤
│                                                  │
│ Name *                                          │
│ [My ERP System                              ]   │
│                                                  │
│ Webhook URL *                                   │
│ [https://api.example.com/webhook            ]   │
│ ⚠️ Cannot use localhost or private IPs          │
│                                                  │
│ HTTP Method                                     │
│ ( ) POST  ( ) PUT  ( ) PATCH                    │
│                                                  │
│ Custom Headers (optional)                       │
│ [+ Add Header]                                  │
│ Key: [X-API-Version    ] Value: [v1         ]   │
│                                                  │
│ Authentication                                  │
│ [Dropdown: Bearer Token ▼]                      │
│                                                  │
│ Token/Credential *                              │
│ [••••••••••••••••                           ]   │
│ [Show]                                          │
│                                                  │
│ ☑ Enable webhook immediately                   │
│                                                  │
│ [Cancel]                    [Create Webhook]    │
└─────────────────────────────────────────────────┘
```

**Validation Rules:**
- URL must start with `http://` or `https://`
- Cannot contain `localhost`, `127.0.0.1`, `0.0.0.0`, or private IP ranges
- Name required (max 255 chars)
- Auth credentials required if auth type != None

#### 5. Delivery History Modal
```
┌─────────────────────────────────────────────────┐
│ Delivery History: My ERP System         [Close]│
├─────────────────────────────────────────────────┤
│                                                  │
│ Showing last 50 deliveries                      │
│                                                  │
│ ┌───────────────────────────────────────────┐  │
│ │ ✅ Delivered • 2 minutes ago              │  │
│ │ Event: item_pushed                        │  │
│ │ Response: 200 OK (1 attempt)              │  │
│ │ [View Payload] [View Response]            │  │
│ └───────────────────────────────────────────┘  │
│                                                  │
│ ┌───────────────────────────────────────────┐  │
│ │ ❌ Failed • 1 hour ago                    │  │
│ │ Event: item_tokenized                     │  │
│ │ Error: Network timeout (3 attempts)       │  │
│ │ [View Payload] [View Error]               │  │
│ └───────────────────────────────────────────┘  │
│                                                  │
│ ┌───────────────────────────────────────────┐  │
│ │ 🔄 Retrying • 5 minutes ago               │  │
│ │ Event: item_pushed                        │  │
│ │ Next retry: in 10 seconds (2 attempts)    │  │
│ │ [View Payload]                            │  │
│ └───────────────────────────────────────────┘  │
│                                                  │
│ [Load More]                                     │
└─────────────────────────────────────────────────┘
```

**Status Icons:**
- ✅ Delivered (green)
- ❌ Failed (red)
- 🔄 Retrying (yellow)
- ⏳ Pending (gray)
- 🚀 In Progress (blue)

### User Flows

#### Flow 1: Enable Webhooks and Add First Webhook
1. User navigates to Circuit Settings → Post-Action Webhooks
2. User sees toggle in OFF state with explanation
3. User clicks toggle to ON
4. Configuration section appears
5. User selects trigger events (pre-check "Item Pushed")
6. User clicks "Save Configuration"
7. User clicks "+ Add Webhook" button
8. Modal opens with form
9. User fills in webhook details:
   - Name: "My ERP"
   - URL: `https://api.myerp.com/webhook`
   - Auth: Bearer Token
   - Token: `secret-token-123`
10. User clicks "Create Webhook"
11. Modal closes, webhook appears in list
12. Success toast: "Webhook created successfully"

#### Flow 2: Test Webhook
1. User clicks "Test" button on webhook card
2. Loading indicator appears
3. API call sends test payload
4. Success toast: "Test webhook sent! Check delivery history for results."
5. User clicks "View History"
6. Modal opens showing test delivery at top

#### Flow 3: Disable Specific Webhook
1. User clicks toggle on webhook card
2. Toggle switches to OFF, card shows disabled state
3. Success toast: "Webhook disabled"
4. No deliveries triggered for this webhook until re-enabled

#### Flow 4: View Failed Delivery Details
1. User clicks "View History" on webhook
2. Modal shows delivery list
3. User clicks "View Error" on failed delivery
4. Expandable section shows:
   ```
   Error: Connection timeout after 30s
   Attempts: 3/3
   Status Code: N/A
   Last Attempt: 2025-10-08T10:30:45Z
   ```

## Component Structure (Suggested)

```typescript
components/
├── circuits/
│   └── settings/
│       └── webhooks/
│           ├── PostActionWebhooksPage.tsx      // Main page
│           ├── WebhookConfigSection.tsx        // Trigger events & options
│           ├── WebhookList.tsx                 // List of webhooks
│           ├── WebhookCard.tsx                 // Individual webhook card
│           ├── WebhookFormModal.tsx            // Add/Edit modal
│           ├── DeliveryHistoryModal.tsx        // History modal
│           ├── DeliveryListItem.tsx            // Individual delivery
│           └── WebhookPayloadViewer.tsx        // JSON payload viewer
```

## State Management

### Main State
```typescript
interface PostActionState {
  settings: PostActionSettings | null;
  webhooks: Webhook[];
  deliveries: WebhookDelivery[];
  loading: boolean;
  error: string | null;
}

interface PostActionSettings {
  enabled: boolean;
  trigger_events: TriggerEvent[];
  include_storage_details: boolean;
  include_item_metadata: boolean;
}

interface Webhook {
  id: string;
  name: string;
  url: string;
  method: 'Post' | 'Put' | 'Patch';
  headers: Record<string, string>;
  auth_type: 'None' | 'BearerToken' | 'ApiKey' | 'BasicAuth' | 'CustomHeader';
  auth_credentials?: string; // Hidden from display
  enabled: boolean;
  retry_config: RetryConfig;
  created_at: string;
  updated_at: string;
}

type TriggerEvent = 'item_pushed' | 'item_approved' | 'item_tokenized' | 'item_published';
```

## Error Handling

### API Error Responses

**403 Forbidden:**
```json
{
  "error": "Permission denied"
}
```
**Action:** Show error toast, disable all write operations, show permission message

**400 Bad Request:**
```json
{
  "error": "Localhost URLs are not allowed"
}
```
**Action:** Show inline validation error in form

**404 Not Found:**
```json
{
  "error": "Circuit not found"
}
```
**Action:** Redirect to circuits list, show error toast

**500 Internal Server Error:**
```json
{
  "error": "Failed to store circuit: ..."
}
```
**Action:** Show error toast, allow retry

## Security Considerations

### Frontend Validation
1. **URL Validation:**
   - Must start with `http://` or `https://`
   - Block: `localhost`, `127.0.0.1`, `0.0.0.0`
   - Block: `192.168.*`, `10.*`, `172.16.*`
   - Show warning for `http://` (prefer `https://`)

2. **Credential Display:**
   - Never log credentials
   - Mask in UI: `auth_credentials: "***hidden***"`
   - Show/hide toggle for credentials during entry
   - Clear from memory after save

3. **Permission Checks:**
   - Only show page if user has `ManagePermissions` on circuit
   - Disable edit/delete if user is not owner/admin

## Testing Scenarios

### Manual Testing Checklist
- [ ] Enable/disable webhook system
- [ ] Create webhook with all auth types
- [ ] Test webhook (should see test delivery)
- [ ] Edit webhook name and URL
- [ ] Toggle individual webhook on/off
- [ ] Delete webhook with confirmation
- [ ] View delivery history with pagination
- [ ] Expand/collapse payload JSON
- [ ] Filter deliveries by status
- [ ] Validate URL patterns (reject localhost)
- [ ] Handle network errors gracefully
- [ ] Permission denied error handling

### Edge Cases
1. **No webhooks configured:** Show empty state with "Add Webhook" CTA
2. **All webhooks disabled:** Show warning "All webhooks are disabled"
3. **Failed delivery:** Show retry count and next retry time
4. **Very long URLs:** Truncate in card, show full in modal
5. **Large payloads:** Paginate or lazy-load JSON viewer
6. **Concurrent edits:** Handle optimistic locking (last-write-wins)

## Example Webhook Payload (for reference)

This is what your configured webhooks will receive:

```json
{
  "event_type": "item_pushed",
  "circuit_id": "123e4567-e89b-12d3-a456-426614174000",
  "circuit_name": "My Supply Chain Circuit",
  "timestamp": "2025-10-08T10:15:30Z",
  "item": {
    "dfid": "DFID-20251008-001-ABCD",
    "local_id": "789e0123-e89b-12d3-a456-426614174999",
    "identifiers": [
      {
        "key": "sisbov",
        "value": "BR12345678901234"
      },
      {
        "key": "farm_id",
        "value": "FARM-001"
      }
    ],
    "pushed_by": "user-123"
  },
  "storage": {
    "adapter_type": "IPFS",
    "location": "ipfs://QmXxxx...",
    "hash": "blake3-hash-here",
    "cid": "QmXxxx...",
    "metadata": {
      "size": 1024,
      "uploaded_at": "2025-10-08T10:15:25Z"
    }
  },
  "operation_id": "op-456e7890-e89b-12d3-a456-426614174000",
  "status": "completed"
}
```

## Implementation Priority

### Phase 1 - MVP (Week 1)
- [ ] Basic settings page with enable/disable toggle
- [ ] Trigger event selection
- [ ] Add webhook modal (URL + name only)
- [ ] Webhook list display
- [ ] Delete webhook

### Phase 2 - Enhanced (Week 2)
- [ ] Authentication options
- [ ] Custom headers
- [ ] Edit webhook
- [ ] Enable/disable individual webhooks
- [ ] Test webhook button

### Phase 3 - Monitoring (Week 3)
- [ ] Delivery history modal
- [ ] Payload viewer
- [ ] Status filters
- [ ] Retry information
- [ ] Real-time updates (optional)

## Questions for Backend Team

1. ✅ Do credentials need client-side encryption before sending?
   - **Answer:** No, handled server-side
2. ✅ Is there rate limiting on webhook operations?
   - **Answer:** Standard API rate limits apply
3. ✅ Should we implement real-time delivery status updates?
   - **Answer:** No, polling is sufficient
4. ✅ Maximum number of webhooks per circuit?
   - **Answer:** No hard limit, but recommend 10 max
5. ✅ Can we edit retry configuration per webhook?
   - **Answer:** No, uses default. Future enhancement.

## Success Metrics

Track these metrics for product analytics:
- % of circuits with webhooks enabled
- Average webhooks per circuit
- Webhook success rate
- Most common auth types used
- Time to first webhook configuration
- Test webhook usage rate

---

**Need Help?** Contact the backend team or check the [Backend API Documentation](./CLAUDE.md)

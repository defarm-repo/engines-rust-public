# API Requests for Circuit Public Settings

The frontend has implemented a comprehensive Circuit Public Settings Editor that requires additional API endpoints to support the full functionality. Currently, only the basic public visibility toggle is working through the existing circuit update endpoint.

## Currently Working
- ✅ `PUT /circuits/{circuit_id}` - Updates `permissions.allow_public_visibility`

## Required New Endpoints

### 1. Public Circuit Configuration
**Endpoint:** `PUT /circuits/{circuit_id}/public-settings`

**Request Body:**
```json
{
  "requester_id": "string",
  "public_settings": {
    "access_mode": "public" | "protected" | "scheduled",
    "scheduled_date": "ISO8601 timestamp", // Only for scheduled mode
    "access_password": "string", // Only for protected mode
    "public_name": "string",
    "public_description": "string",
    "primary_color": "#hexcolor",
    "secondary_color": "#hexcolor",
    "published_items": ["item_id1", "item_id2"],
    "auto_approve_members": boolean,
    "auto_publish_pushed_items": boolean,
    "show_encrypted_events": boolean,
    "required_event_types": "string", // Comma-separated list
    "data_quality_rules": "string",
    "export_permissions": "admin" | "members" | "public"
  }
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    // Updated circuit data including new public_settings
  }
}
```

### 2. Get Public Circuit Data (for public page)
**Endpoint:** `GET /circuits/{circuit_id}/public`

**Description:** Returns publicly accessible circuit information based on the configured public settings.

**Response:**
```json
{
  "success": true,
  "data": {
    "circuit_id": "string",
    "public_name": "string",
    "public_description": "string",
    "primary_color": "#hexcolor",
    "secondary_color": "#hexcolor",
    "member_count": number,
    "published_items": [
      {
        "item_id": "string",
        "name": "string",
        "description": "string",
        "published_at": "ISO8601 timestamp"
      }
    ],
    "recent_activity": [
      // Only if show_encrypted_events is true and access_mode allows
    ],
    "access_mode": "public" | "protected" | "scheduled",
    "requires_password": boolean,
    "is_currently_accessible": boolean
  }
}
```

### 3. Join Public Circuit
**Endpoint:** `POST /circuits/{circuit_id}/public/join`

**Request Body:**
```json
{
  "requester_id": "string",
  "access_password": "string", // Only for protected circuits
  "message": "string" // Optional join message
}
```

**Response:**
```json
{
  "success": true,
  "message": "Join request submitted" | "Automatically approved",
  "requires_approval": boolean
}
```

## Frontend Implementation Status

### ✅ Implemented Features
- Public visibility toggle
- Public URL generation and display
- Access modes (Public/Protected/Scheduled) with conditional fields
- Page customization (colors, name, description)
- Items to publish selection (UI ready)
- Advanced options (auto-approval, export permissions, etc.)
- Unified state management
- Form validation and error handling

### 🔄 Waiting for API
- Saving all public settings (currently only saves public visibility)
- Loading existing public settings
- Public circuit page route functionality
- Items selection from actual circuit items

## Technical Notes

1. **State Management**: The frontend uses a unified `settings` object that tracks all public configuration options.

2. **Backward Compatibility**: The existing `permissions.allow_public_visibility` should remain as the master switch for public access.

3. **Security**: The public endpoint should respect the access mode and only return appropriate data based on the current configuration.

4. **Performance**: Consider caching public circuit data since it will be accessed by non-authenticated users.

## Frontend Integration

Once the API endpoints are available, the frontend will need minimal changes:
1. Update the save function to use the new endpoint
2. Add loading of existing public settings on component mount
3. Create the public circuit page component and route

The current implementation is designed to be easily integrated with these new endpoints without requiring major architectural changes.
# API Keys Management - Frontend Integration Guide

## Overview

The API Keys management system is now implemented and ready for frontend integration. All endpoints are protected by JWT authentication and support complete CRUD operations for API keys.

## Recent Fixes

1. **Fixed UUID compatibility** - System now supports both UUID and string-based user IDs (like "hen-admin-001")
2. **Deterministic UUID generation** - Uses BLAKE3 hashing to convert string user IDs to UUIDs consistently
3. **All handlers updated** - Create, list, get, update, delete, revoke, and usage stats endpoints fixed

## API Endpoints

All endpoints require JWT authentication via `Authorization: Bearer {token}` header.

### 1. Create API Key
```http
POST /api/api-keys
Authorization: Bearer {jwt_token}
Content-Type: application/json

{
  "name": "My API Key",
  "organization_type": "Producer",  // One of: Admin, Producer, Association, Enterprise, Government, External
  "permissions": {
    "read": true,
    "write": true,
    "admin": false,
    "custom": {}  // Optional custom permissions
  },
  "rate_limit_per_hour": 1000,
  "expires_in_days": 30,  // Optional
  "notes": "Description of the API key",  // Optional
  "organization_id": null,  // Optional UUID
  "allowed_endpoints": [],  // Optional array of endpoint patterns
  "allowed_ips": []  // Optional array of IP addresses
}
```

**Response:**
```json
{
  "api_key": "dfm_abc123...",  // Full key shown ONLY once
  "metadata": {
    "id": "uuid",
    "name": "My API Key",
    "key_prefix": "dfm_abc1",
    "organization_type": "Producer",
    "permissions": {...},
    "is_active": true,
    "last_used_at": null,
    "usage_count": 0,
    "rate_limit_per_hour": 1000,
    "created_at": "2025-10-28T...",
    "expires_at": "2025-11-27T..."
  },
  "warning": "Save this API key securely. You won't be able to see it again."
}
```

### 2. List API Keys
```http
GET /api/api-keys?include_inactive=false
Authorization: Bearer {jwt_token}
```

**Response:**
```json
[
  {
    "metadata": {
      "id": "uuid",
      "name": "My API Key",
      "key_prefix": "dfm_abc1",
      "organization_type": "Producer",
      "permissions": {...},
      "is_active": true,
      "last_used_at": "2025-10-28T...",
      "usage_count": 42,
      "rate_limit_per_hour": 1000,
      "created_at": "2025-10-01T...",
      "expires_at": "2025-11-27T..."
    }
  }
]
```

### 3. Get API Key Details
```http
GET /api/api-keys/{key_id}
Authorization: Bearer {jwt_token}
```

**Response:** Same as metadata in create response.

### 4. Update API Key
```http
PATCH /api/api-keys/{key_id}
Authorization: Bearer {jwt_token}
Content-Type: application/json

{
  "name": "Updated Name",  // Optional
  "permissions": {...},  // Optional
  "is_active": false,  // Optional
  "rate_limit_per_hour": 2000,  // Optional
  "notes": "Updated notes"  // Optional
}
```

**Response:** Updated API key metadata.

### 5. Revoke API Key
```http
POST /api/api-keys/{key_id}/revoke
Authorization: Bearer {jwt_token}
```

Sets `is_active` to `false`. Returns updated metadata.

### 6. Delete API Key
```http
DELETE /api/api-keys/{key_id}
Authorization: Bearer {jwt_token}
```

**Response:** `204 No Content`

### 7. Get Usage Statistics
```http
GET /api/api-keys/{key_id}/usage?days=7
Authorization: Bearer {jwt_token}
```

**Response:**
```json
{
  "total_requests": 1234,
  "successful_requests": 1200,
  "failed_requests": 34,
  "avg_response_time_ms": 125.5,
  "last_used_at": "2025-10-28T...",
  "daily_usage": [
    {
      "date": "2025-10-28",
      "requests": 234,
      "errors": 5
    }
  ]
}
```

## Authentication with API Keys

Once created, API keys can be used to authenticate requests instead of JWT tokens:

```http
GET /api/circuits
X-API-Key: dfm_abc123...
```

OR

```http
GET /api/circuits
Authorization: Bearer dfm_abc123...
```

## Frontend Integration Checklist

- [ ] **API Key Creation Form**
  - Name input
  - Organization type dropdown
  - Permissions checkboxes (read, write, admin)
  - Rate limit input
  - Expiration days input
  - Notes textarea
  - **IMPORTANT:** Show full API key only once with copy button

- [ ] **API Keys List View**
  - Table/cards showing all API keys
  - Display: name, prefix, status, usage count, created date
  - Filter by active/inactive
  - Sort by creation date, last used, usage count

- [ ] **API Key Details View**
  - View all metadata
  - Show usage statistics chart
  - Edit button → update form
  - Revoke button with confirmation
  - Delete button with confirmation

- [ ] **Usage Statistics**
  - Total requests counter
  - Success/failure ratio chart
  - Average response time metric
  - Daily usage chart (last 7/30 days)

- [ ] **Security Features**
  - Mask API key in lists (show only prefix)
  - Copy to clipboard functionality
  - Revoke confirmation dialog
  - Delete confirmation dialog
  - Expiration warning badges

## Error Handling

### Common Errors

- `401 Unauthorized` - Invalid or missing JWT token
- `403 Forbidden` - User doesn't own the API key
- `404 Not Found` - API key doesn't exist
- `422 Unprocessable Entity` - Invalid request payload
- `500 Internal Server Error` - Server error

### Example Error Response
```json
"Invalid user ID format"
```

## Testing

Use the provided test script to verify all endpoints:

```bash
./test-api-keys.sh
```

This tests:
1. Authentication
2. API key creation
3. Listing keys
4. Getting key details
5. Updating keys
6. Authentication with API key
7. Usage statistics
8. Revoking keys
9. Deleting keys

## Demo Accounts

Test with these accounts:

| Username | Password | User ID | Tier |
|----------|----------|---------|------|
| hen | demo123 | hen-admin-001 | Admin |
| chick | Demo123! | user-uuid | Basic |
| gerbov | Gerbov2024!Test | user-uuid | Professional |

## Deployment Status

**Current Status:** Code is implemented and tested locally.

**To deploy:**
```bash
git add src/api/api_keys.rs
git commit -m "fix: Support string user IDs in API key management"
git push origin main
```

Railway will automatically deploy the changes.

## Notes

1. **User ID Compatibility:** The system now supports both UUID and string-based user IDs using deterministic UUID generation via BLAKE3 hashing
2. **Organization Types:** Must be one of: Admin, Producer, Association, Enterprise, Government, External
3. **Permissions:** All permissions must include the `custom` field (can be empty object)
4. **Security:** Full API key is shown ONLY once during creation
5. **Rate Limiting:** Configured per key, defaults to tier limits if not specified

## Next Steps

1. Deploy the fixed code to production
2. Implement frontend UI components
3. Test with real user accounts
4. Add monitoring and analytics for API key usage
5. Consider adding API key rotation functionality

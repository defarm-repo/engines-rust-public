# API Request/Response Schemas

**Last Updated**: 2025-10-19
**Purpose**: Prevent deserialization errors by documenting exact field names

## Authentication

### POST /api/auth/login
```json
// Request
{
  "username": "string",      // NOT user_id!
  "password": "string",
  "workspace_id": "string?"  // optional
}

// Response
{
  "token": "string",
  "user_id": "string",
  "workspace_id": "string?",
  "expires_at": 1234567890
}
```

### POST /api/auth/register
```json
// Request
{
  "username": "string",
  "password": "string",
  "email": "string",
  "workspace_name": "string?"  // optional
}
```

## Items

### POST /api/items/local
```json
// Request
{
  "identifiers": {...}?,       // optional
  "enhanced_identifiers": [...]?,  // optional
  "enriched_data": {...}?      // optional
}
// NOTE: requester_id is extracted from JWT token automatically

// Response
{
  "success": true,
  "data": {
    "local_id": "uuid",
    "identifiers": {...}
  }
}
```

## Circuits

### POST /api/circuits
```json
// Request
{
  "name": "string",
  "description": "string",
  "adapter_config": {...}?,     // optional
  "alias_config": {...}?,       // optional
  "allow_public_visibility": bool?  // optional
}
// NOTE: owner_id and created_by are extracted from JWT automatically

// Response
{
  "circuit_id": "uuid",
  "name": "string",
  ...
}
```

### POST /api/circuits/:id/push-local
```json
// Request
{
  "local_id": "uuid",
  "identifiers": [...]?,        // optional
  "enriched_data": {...}?       // optional
}
// NOTE: requester_id is extracted from JWT automatically

// Response
{
  "success": true,
  "dfid": "DFID-...",
  "operation_id": "uuid"
}
```

### PUT /api/circuits/:id/adapter
```json
// Request
{
  "adapter_type": "stellar_testnet-ipfs",
  "sponsor_adapter_access": bool
}
```

## Common Mistakes to Avoid

1. ❌ Using `user_id` instead of `username` in login
2. ❌ Including `requester_id` in request body (it's extracted from JWT)
3. ❌ Including `owner_id` or `created_by` in circuit creation
4. ❌ Sending empty objects `{}` when fields are optional - use `null` or omit

## How to Use This Document

Before making an API request:
1. Find the endpoint in this document
2. Use the EXACT field names shown
3. Check which fields are optional (marked with `?`)
4. Remember that user identity comes from JWT, not request body

## Updating This Document

When adding/modifying endpoints:
1. Read the Rust struct definition in `src/api/*.rs`
2. Note which fields are `Option<T>` (optional)
3. Check for comments about JWT extraction
4. Add example request/response to this document

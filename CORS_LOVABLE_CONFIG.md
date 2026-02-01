# ✅ CORS Configuration for Lovable Domains

## 📋 Summary

Added CORS support for Lovable frontend builder domains to enable seamless integration with the DeFarm API.

**Date:** 2026-01-28
**Commit:** bc3b97f
**Status:** ✅ Deployed to Production

## 🔧 Changes Made

### 1. Custom Origin Validation Function

Added `is_origin_allowed()` function in `/src/bin/api.rs` that supports:
- Exact domain matching
- Wildcard subdomain matching (*.lovableproject.com, *.lovable.app)
- Protocol-agnostic matching (http/https)

### 2. Allowed Origins

#### DeFarm Domains
- `circuits.defarm.net` - Frontend application
- `connect.defarm.net` - API endpoint
- `defarm.net` - Main domain
- `www.defarm.net` - WWW subdomain

#### Lovable Domains (Wildcards)
- `*.lovableproject.com` - Lovable project domains
- `*.lovable.app` - Lovable app domains
- `*.lovable.dev` - Lovable development domains

#### Development
- `localhost` - Local development
- `127.0.0.1` - Local IP

### 3. CORS Configuration

```rust
let cors = CorsLayer::new()
    .allow_origin(AllowOrigin::predicate(
        |origin: &HeaderValue, _request_parts| {
            if let Ok(origin_str) = origin.to_str() {
                is_origin_allowed(origin_str)
            } else {
                false
            }
        },
    ))
    .allow_methods([
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::DELETE,
        Method::PATCH,
        Method::OPTIONS,
    ])
    .allow_headers([
        header::AUTHORIZATION,
        header::CONTENT_TYPE,
        header::ACCEPT,
        "x-api-key",
    ])
    .allow_credentials(true);
```

## 🎯 Features

### Wildcard Support
- Matches any subdomain under Lovable domains
- Example: `my-app.lovableproject.com` ✅ Allowed
- Example: `demo-123.lovable.app` ✅ Allowed

### Security
- Explicit whitelist (no blanket CORS)
- Domain validation on every request
- Credentials support for authenticated requests

### Methods Allowed
- GET - Read operations
- POST - Create operations
- PUT - Update operations
- DELETE - Delete operations
- PATCH - Partial updates
- OPTIONS - Preflight requests

### Headers Allowed
- `Authorization` - JWT tokens
- `Content-Type` - JSON payloads
- `Accept` - Response format
- `x-api-key` - API key authentication

## 🧪 Testing

### Test from Lovable

```javascript
// In your Lovable project, test API access:
fetch('https://defarm-engines-api-production.up.railway.app/health', {
  method: 'GET',
  credentials: 'include',
})
  .then(res => res.json())
  .then(data => console.log('API Health:', data))
  .catch(err => console.error('CORS Error:', err));
```

### Test Authentication

```javascript
// Login test
fetch('https://defarm-engines-api-production.up.railway.app/api/auth/login', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
  },
  credentials: 'include',
  body: JSON.stringify({
    username: 'gerbov',
    password: 'Gerbov2024!Test'
  }),
})
  .then(res => res.json())
  .then(data => console.log('Login Success:', data))
  .catch(err => console.error('Error:', err));
```

### Test Circuit Access

```javascript
// Get circuit data (authenticated)
const token = 'your-jwt-token';
fetch('https://defarm-engines-api-production.up.railway.app/api/circuits/4eb4e8da-12f7-4bfb-9610-686e9c21c1a2', {
  method: 'GET',
  headers: {
    'Authorization': `Bearer ${token}`,
    'Content-Type': 'application/json',
  },
  credentials: 'include',
})
  .then(res => res.json())
  .then(data => console.log('Circuit Data:', data))
  .catch(err => console.error('Error:', err));
```

## 🔍 Verification

### Check CORS Headers

```bash
curl -I -X OPTIONS \
  -H "Origin: https://my-app.lovableproject.com" \
  -H "Access-Control-Request-Method: GET" \
  https://defarm-engines-api-production.up.railway.app/health
```

Expected response:
```
access-control-allow-origin: https://my-app.lovableproject.com
access-control-allow-methods: GET, POST, PUT, DELETE, PATCH, OPTIONS
access-control-allow-headers: authorization, content-type, accept, x-api-key
access-control-allow-credentials: true
```

### Test from Browser Console

```javascript
// Open browser dev tools on any Lovable page
// Paste and run:
fetch('https://defarm-engines-api-production.up.railway.app/health')
  .then(r => r.json())
  .then(console.log)
  .catch(console.error);
```

If successful, you'll see:
```json
{
  "status": "healthy",
  "timestamp": "2026-01-28T...",
  "uptime": "System operational"
}
```

If CORS error, you'll see:
```
CORS policy: No 'Access-Control-Allow-Origin' header
```

## 📊 Deployment Status

### Git
```
Commit: bc3b97f
Branch: main
Pushed: ✅ Yes
```

### Railway
```
Service: defarm-engines-api
Environment: production
URL: https://defarm-engines-api-production.up.railway.app
Auto-deploy: ✅ Enabled (triggered on git push)
Status: ✅ Healthy
```

### API Endpoints
```
Health: https://defarm-engines-api-production.up.railway.app/health
Auth: https://defarm-engines-api-production.up.railway.app/api/auth/*
Circuits: https://defarm-engines-api-production.up.railway.app/api/circuits/*
Items: https://defarm-engines-api-production.up.railway.app/api/items/*
```

## 🐛 Troubleshooting

### CORS Error: "Origin not allowed"

**Cause:** Domain not in whitelist
**Solution:** Check that your Lovable domain matches one of:
- `*.lovableproject.com`
- `*.lovable.app`
- `*.lovable.dev`

### CORS Error: "Credentials not allowed"

**Cause:** Missing credentials in fetch
**Solution:** Add `credentials: 'include'` to fetch options

### CORS Error: "Method not allowed"

**Cause:** Using unsupported HTTP method
**Solution:** Ensure method is one of: GET, POST, PUT, DELETE, PATCH, OPTIONS

### CORS Error: "Header not allowed"

**Cause:** Custom header not in whitelist
**Solution:** Only use: Authorization, Content-Type, Accept, x-api-key

## 📚 Code Reference

### Modified Files
- `/src/bin/api.rs` - CORS configuration and origin validation

### Functions
- `is_origin_allowed(origin: &str) -> bool` - Origin validation with wildcard support
- Lines 68-105 in `/src/bin/api.rs`

### CORS Setup
- Lines 421-448 in `/src/bin/api.rs`

## 🔐 Security Considerations

### What's Protected
✅ Explicit whitelist (no `*` wildcard)
✅ Domain validation on every request
✅ Supports credentials for authenticated requests
✅ Limited to specific HTTP methods
✅ Limited to specific headers

### What's Not Protected
⚠️ Any subdomain under Lovable domains is allowed
- This is intentional for Lovable's multi-tenant architecture
- Each Lovable user gets a unique subdomain

### Recommendations
- Monitor API usage for abuse
- Consider rate limiting per origin
- Log CORS rejections for security analysis

## ✅ Testing Checklist

- [x] Code compiled without errors
- [x] Git commit created
- [x] Pushed to main branch
- [x] Railway auto-deploy triggered
- [x] API health check passing
- [ ] Test from Lovable project (pending user testing)
- [ ] Test authentication flow (pending user testing)
- [ ] Test circuit access (pending user testing)

## 📖 Next Steps

1. **Create Lovable Project**
   - Build frontend in Lovable
   - Use DeFarm API for backend

2. **Test Integration**
   - Verify CORS works from Lovable domain
   - Test all API endpoints
   - Verify authentication flow

3. **Monitor Usage**
   - Check Railway logs for CORS rejections
   - Monitor API performance
   - Watch for abuse patterns

---

**Status:** ✅ DEPLOYED AND READY
**API:** https://defarm-engines-api-production.up.railway.app
**Domains:** Lovable (*.lovableproject.com, *.lovable.app, *.lovable.dev)

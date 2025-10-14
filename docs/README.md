# DeFarm Engines API Documentation

Welcome to the DeFarm Engines API documentation! This folder contains everything you need to integrate with our blockchain-based farm asset tokenization platform.

## 📖 Documentation Overview

### 1. Interactive API Documentation (Recommended Starting Point)

**File**: `index.html`

Open this file in your browser for **interactive API testing**:

```bash
# Option 1: Open directly
open docs/index.html

# Option 2: Serve via HTTP server
cd docs
python3 -m http.server 8000
# Then open http://localhost:8000 in browser
```

**Features**:
- ✅ Try out all API endpoints directly in browser
- ✅ Automatic request/response examples
- ✅ Built-in authentication flow
- ✅ Schema validation
- ✅ Copy-paste curl commands

### 2. Integration Quickstart Guide

**File**: `../INTEGRATION_QUICKSTART.md`

**5-minute quickstart** with code examples in:
- curl (command line)
- Python
- JavaScript/Node.js

**Includes**:
- Complete workflow examples
- Authentication setup
- Error handling
- Webhook configuration
- Troubleshooting guide

**Read it here**: [INTEGRATION_QUICKSTART.md](../INTEGRATION_QUICKSTART.md)

### 3. OpenAPI Specifications

#### External Integration API
**File**: `openapi-external.yaml`

For **farm management systems** and external integrators.

**Endpoints included**:
- Authentication (login, register)
- Item creation (local items)
- Circuit operations (push to blockchain)
- Storage history (blockchain transactions)
- Adapters (available storage backends)
- Webhooks (event notifications)

#### Internal/Admin API (Coming Soon)
**File**: `openapi-internal.yaml`

For **internal developers** and system administrators.

**Will include**:
- All external endpoints PLUS
- User management
- Tier configuration
- Credit management
- Adapter configuration
- System administration

## 🚀 Quick Start for Different Audiences

### For Farm Management Systems / External Integrators

**Goal**: Integrate your system to tokenize farm assets on blockchain

1. **Read**: [INTEGRATION_QUICKSTART.md](../INTEGRATION_QUICKSTART.md)
2. **Test**: Open `index.html` in browser and try the API
3. **Code**: Use Python/JavaScript examples from quickstart guide
4. **Deploy**: Follow integration patterns for production

**Key Endpoints**:
- `POST /api/auth/login` - Get JWT token
- `POST /api/items/local` - Create item with identifiers
- `POST /api/circuits/{id}/push-local` - Tokenize to blockchain
- `GET /api/items/{dfid}/storage-history` - View blockchain TXs

### For Internal Developers

**Goal**: Understand all available endpoints for building internal tools

1. **Explore**: Open `index.html` and switch to "Internal/Admin API" (coming soon)
2. **Test**: Use Swagger UI to experiment with admin endpoints
3. **Reference**: Use OpenAPI spec for generating client SDKs

### For Mobile/Frontend Developers

**Goal**: Build user-facing applications

1. **Authentication**: Use JWT tokens from `/api/auth/login`
2. **Local Items**: Create items with `/api/items/local`
3. **Status Checking**: Poll `/api/items/mapping/{local_id}` for DFID
4. **Webhooks**: Set up real-time notifications for UI updates

### For DevOps / Infrastructure

**Goal**: Deploy and monitor API in production

1. **Deployment**: See [PRODUCTION_DEPLOYMENT.md](../PRODUCTION_DEPLOYMENT.md)
2. **Monitoring**: Check health endpoints (`/health`, `/health/db`)
3. **Rate Limits**: Monitor headers for tier limits
4. **Logs**: Configure logging level via environment variables

## 🔧 Using the OpenAPI Specs

### Generate Client SDKs

Use OpenAPI Generator to create type-safe clients:

```bash
# Python client
openapi-generator-cli generate \
  -i docs/openapi-external.yaml \
  -g python \
  -o sdk/python

# JavaScript/TypeScript client
openapi-generator-cli generate \
  -i docs/openapi-external.yaml \
  -g typescript-axios \
  -o sdk/typescript

# Go client
openapi-generator-cli generate \
  -i docs/openapi-external.yaml \
  -g go \
  -o sdk/go
```

### Import into Postman

1. Open Postman
2. File → Import
3. Select `openapi-external.yaml`
4. Auto-generates complete collection with examples

### Import into Insomnia

1. Open Insomnia
2. Application → Import → From File
3. Select `openapi-external.yaml`

### Validate Requests

Use the spec for request validation:

```python
from openapi_core import create_spec
from openapi_core.validation.request import openapi_request_validator

spec = create_spec('docs/openapi-external.yaml')
validator = openapi_request_validator.RequestValidator(spec)
result = validator.validate(request)
```

## 📊 API Endpoints Summary

### Public Endpoints (No Auth)
- `GET /health` - Health check
- `POST /api/auth/login` - User login
- `POST /api/auth/register` - User registration

### Protected Endpoints (Requires Auth)

#### Items
- `POST /api/items/local` - Create local item
- `GET /api/items/mapping/{local_id}` - Get LID-DFID mapping
- `GET /api/items/{dfid}/storage-history` - Get blockchain transactions

#### Circuits
- `POST /api/circuits/{id}/push-local` - Push item to circuit (tokenization)

#### Adapters
- `GET /api/adapters` - List available storage adapters

#### Webhooks
- `GET /api/circuits/{id}/post-actions` - Get webhook settings
- `PUT /api/circuits/{id}/post-actions` - Update webhook settings
- `POST /api/circuits/{id}/post-actions/webhooks` - Create webhook
- `GET /api/circuits/{id}/post-actions/webhooks/{webhook_id}` - Get webhook
- `PUT /api/circuits/{id}/post-actions/webhooks/{webhook_id}` - Update webhook
- `DELETE /api/circuits/{id}/post-actions/webhooks/{webhook_id}` - Delete webhook
- `POST /api/circuits/{id}/post-actions/webhooks/{webhook_id}/test` - Test webhook
- `GET /api/circuits/{id}/post-actions/deliveries` - Get delivery history

## 🔐 Authentication Methods

### JWT Token (Recommended for Web/Mobile Apps)

```bash
# Login
POST /api/auth/login
Body: {"username": "...", "password": "..."}

# Use token
Authorization: Bearer eyJ0eXAiOiJKV1Qi...
```

**Expires**: 24 hours
**Refresh**: `POST /api/auth/refresh`

### API Key (Recommended for Server-to-Server)

```bash
# Get key from administrator

# Use in header
X-API-Key: dfm_your32characterkeyhere
```

**Expires**: Configurable
**Permissions**: Scoped per key

## 🌍 Environments

### Production
- **Base URL**: `https://connect.defarm.net`
- **Network**: Stellar Testnet + IPFS (Pinata)
- **Use for**: Live production data

### Staging (Coming Soon)
- **Base URL**: TBD
- **Network**: Stellar Testnet + IPFS
- **Use for**: Integration testing

### Local Development
- **Base URL**: `http://localhost:3000`
- **Network**: Configurable via environment variables
- **Use for**: Development and testing

## 📝 Rate Limits

| Tier | Requests/Hour | Requests/Day |
|------|--------------|--------------|
| Basic | 100 | 1,000 |
| Professional | 1,000 | 10,000 |
| Enterprise | 10,000 | 100,000 |

**Headers**:
```
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 85
X-RateLimit-Reset: 1760460000
```

**429 Response**:
```json
{
  "error": "Rate limit exceeded. Please try again in 3600 seconds."
}
```

## 🐛 Common Issues

### CORS Errors (Browser)

**Problem**: Browser blocks requests due to CORS policy

**Solution**:
- Use proper authentication headers
- For local dev: Configure CORS in server
- For production: Requests should work (CORS enabled)

### SSL Certificate Errors

**Problem**: SSL verification fails

**Solution**:
```python
# Python - disable verification (dev only!)
requests.post(url, verify=False)

# Production - use proper certificates
requests.post(url, verify=True)
```

### Token Expired

**Problem**: 401 Unauthorized after 24 hours

**Solution**:
```bash
POST /api/auth/refresh
Authorization: Bearer OLD_TOKEN
```

## 📞 Support

- **Documentation Issues**: Open issue on GitHub
- **API Questions**: support@defarm.io
- **Integration Help**: integration@defarm.io
- **Emergency**: Call your account manager

## 🔗 Additional Resources

- **Architecture Overview**: [../CLAUDE.md](../CLAUDE.md)
- **Production Deployment**: [../PRODUCTION_DEPLOYMENT.md](../PRODUCTION_DEPLOYMENT.md)
- **Railway Deployment**: [../RAILWAY_DEPLOYMENT.md](../RAILWAY_DEPLOYMENT.md)
- **Tokenization Requirements**: [../BACKEND_TOKENIZATION_REQUIREMENTS.md](../BACKEND_TOKENIZATION_REQUIREMENTS.md)

## 📅 Version History

- **v1.0.0** (2025-10-14): Initial external integration API documentation
  - OpenAPI 3.0 spec with all essential endpoints
  - Interactive Swagger UI documentation
  - Integration quickstart guide with code examples
  - Webhook system documentation

---

**Last Updated**: October 14, 2025
**Maintained By**: DeFarm Engineering Team

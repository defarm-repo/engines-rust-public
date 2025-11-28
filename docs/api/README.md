# API Documentation

This directory contains comprehensive API documentation for the DeFarm Engines system.

## 🌟 Quick Start

### For New Users
Start here: **[API_GUIDE.md](./API_GUIDE.md)** - Complete bilingual guide (English/Portuguese) with:
- Authentication methods (JWT + API Keys)
- Common workflows and examples
- Error handling and best practices
- Step-by-step tutorials

### For Client-Specific Documentation
- **[GERBOV_INTEGRATION.md](./GERBOV_INTEGRATION.md)** - Complete integration guide for Gerbov client (Portuguese)

### For API Testing
- **[Postman Collection](./defarm-api-collection.json)** - Import into Postman for testing all endpoints
- **[Environment Files](./defarm-api-environments.json)** - Pre-configured environments (Production, Development, Gerbov Test)
- **[API Analysis Report](./API_ANALYSIS_REPORT.md)** - Latest API analysis with test results

## 📄 Documentation Files

### **API_GUIDE.md** - Comprehensive API Guide ⭐
Complete bilingual documentation (English and Portuguese) covering:
- 🔐 Dual authentication (JWT tokens + API Keys)
- 📋 Step-by-step workflows
- 🔑 Complete API Keys documentation
- 🎯 Core concepts (Items, DFIDs, Circuits, Identifiers)
- ❌ Error handling and troubleshooting
- ⚡ Rate limiting and best practices
- 💡 Real-world examples with bash scripts

**Perfect for:**
- New developers getting started
- Integration developers
- Frontend/mobile app developers
- Third-party integrators

### **GERBOV_INTEGRATION.md** - Client Integration Guide
Consolidated Portuguese documentation for Gerbov client including:
- Credenciais de acesso e autenticação
- Fluxo completo de autenticação JWT
- Gerenciamento de chaves de API
- Criação e tokenização de itens
- Operações de circuito
- Histórico e rastreabilidade
- Códigos de erro com soluções
- Scripts bash completos
- Tier limitations documentation

### **defarm-api-collection.json** - Postman Collection
Ready-to-import Postman collection with:
- 80+ API requests organized by module
- Pre-request scripts for token management
- Test scripts for response validation
- Environment variable integration

### **defarm-api-environments.json** - Postman Environments
Three pre-configured environments:
- Production (connect.defarm.net)
- Development (localhost:3000)
- Gerbov Test (with test credentials)

### **API_ANALYSIS_REPORT.md** - API Analysis Report
Comprehensive analysis including:
- Live testing results
- Documentation accuracy audit
- Issues found and recommendations
- Test tokens and data generated

### **openapi.yaml** - OpenAPI 3.0 Specification
The authoritative API contract between frontend and backend.

**What it contains:**
- ✅ All API operations and endpoint paths
- ✅ Complete request/response schemas
- ✅ Authentication requirements (JWT + API Key)
- ✅ API tags and modules
- ✅ Data schemas and validation rules
- ✅ Error formats and status codes
- ✅ Example requests and responses

**How to use:**

#### **View in Swagger UI Online**
1. Visit [Swagger Editor](https://editor.swagger.io/)
2. Copy/paste the contents of `openapi.yaml`
3. See interactive documentation with "Try it out" functionality

#### **Import into API Testing Tools**
- **Postman**: File → Import → Select `openapi.yaml`
- **Insomnia**: Application → Import → OpenAPI 3.0
- **Thunder Client** (VS Code): Import → OpenAPI

#### **Generate Client SDKs**
```bash
# JavaScript/TypeScript
npx openapi-typescript-codegen --input openapi.yaml --output ./src/api

# Python
openapi-generator-cli generate -i openapi.yaml -g python -o ./api-client

# Go
openapi-generator-cli generate -i openapi.yaml -g go -o ./api-client
```

#### **Validate OpenAPI Spec**
```bash
# Install validator
npm install -g @apidevtools/swagger-cli

# Validate spec
swagger-cli validate openapi.yaml
```

### **Additional Documentation**

#### **API_REQUESTS.md** - Circuit Public Settings
Documents the circuit public settings feature including:
- Public visibility configuration
- Access modes (public/protected/scheduled)
- Public circuit page customization
- Join request workflows

#### **JWT_AUTHENTICATION_GUIDE.md** - JWT Authentication Details
Deep dive into JWT authentication:
- Token structure and claims
- Token lifecycle and refresh
- Security considerations

#### **BACKEND_API_SPEC.md** - Legacy Backend Specification
Historical API specification (superseded by openapi.yaml and API_GUIDE.md)

### **archive/** - Historical Documentation
Contains superseded documentation and unimplemented feature requests:
- Historical API specifications
- Feature request documents
- Legacy guides

See `archive/README.md` for details on archived documents.

---

## 🎯 Frontend-Backend Interface

The frontend **ONLY** needs to know about:

1. **API Endpoints** (defined in `openapi.yaml`)
   - What paths exist
   - What HTTP methods are supported
   - What authentication is required

2. **Request Schemas** (defined in `openapi.yaml`)
   - What JSON structure to send
   - What fields are required
   - What validation rules apply

3. **Response Schemas** (defined in `openapi.yaml`)
   - What JSON structure to expect
   - What fields will be present
   - What data types are used

4. **Authentication Flow** (defined in `openapi.yaml`)
   - How to get a JWT token (`POST /auth/login`)
   - How to include it in requests (`Authorization: Bearer <token>`)
   - Or how to use API keys (`X-API-Key: dfm_xxx`)

The frontend should **NEVER** need to know:
- ❌ Internal engine implementations
- ❌ Database schema details
- ❌ Business logic internals
- ❌ Storage backend configuration

---

## 🚀 Quick Start for Frontend Developers

### 1. Authentication
```javascript
// Login to get JWT token
const response = await fetch('https://connect.defarm.net/api/auth/login', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    username: 'user@example.com',
    password: 'SecurePass123!'
  })
});

const { token } = await response.json();
```

### 2. Use Token for API Calls
```javascript
// All authenticated endpoints need the JWT token
const circuits = await fetch('https://connect.defarm.net/api/circuits', {
  headers: {
    'Authorization': `Bearer ${token}`
  }
});
```

### 3. Create a Circuit
```javascript
// Simple circuit (no blockchain)
const circuit = await fetch('https://connect.defarm.net/api/circuits', {
  method: 'POST',
  headers: {
    'Authorization': `Bearer ${token}`,
    'Content-Type': 'application/json'
  },
  body: JSON.stringify({
    name: 'My Supply Chain',
    description: 'Track organic products'
  })
});

// Or with blockchain adapter (requires appropriate tier)
const circuitWithAdapter = await fetch('https://connect.defarm.net/api/circuits', {
  method: 'POST',
  headers: {
    'Authorization': `Bearer ${token}`,
    'Content-Type': 'application/json'
  },
  body: JSON.stringify({
    name: 'My Supply Chain',
    description: 'Track organic products',
    adapter_config: {
      adapter_type: 'StellarTestnetIpfs',  // or 'StellarMainnetIpfs' for Enterprise tier
      requires_approval: false,             // REQUIRED field
      auto_migrate_existing: false,         // REQUIRED field
      sponsor_adapter_access: true
    }
  })
});
```

### 4. Create and Tokenize an Item
```javascript
// Step 1: Create local item (gets LID)
const localItem = await fetch('https://connect.defarm.net/api/items/local', {
  method: 'POST',
  headers: {
    'Authorization': `Bearer ${token}`,
    'Content-Type': 'application/json'
  },
  body: JSON.stringify({
    identifiers: [
      {
        namespace: 'bovino',
        key: 'sisbov',
        value: 'BR12345678901234',
        id_type: 'Canonical',
        verified: false
      }
    ],
    enriched_data: {
      weight: '500kg',
      breed: 'Angus'
    }
  })
});

const { data: { local_id } } = await localItem.json();

// Step 2: Push to circuit (gets DFID)
const tokenized = await fetch(`https://connect.defarm.net/api/circuits/${circuitId}/push-local`, {
  method: 'POST',
  headers: {
    'Authorization': `Bearer ${token}`,
    'Content-Type': 'application/json'
  },
  body: JSON.stringify({
    local_id: local_id
  })
});

const { data: { dfid } } = await tokenized.json();
console.log('Item tokenized with DFID:', dfid);
```

---

## 📚 API Modules

The API is organized into 16 modules (tags):

1. **Authentication** - Login, register, logout
2. **Circuits** - Circuit CRUD, push/pull operations
3. **Items** - Item creation, tokenization, management
4. **Workspaces** - Workspace management
5. **API Keys** - API key creation and management
6. **Events** - Event tracking and history
7. **Activities** - Activity logs and audit trail
8. **Storage History** - Item storage locations
9. **Adapters** - Storage adapter configuration
10. **Receipts** - Receipt management
11. **Notifications** - Real-time notifications
12. **User Credits** - Credit management
13. **Audit** - Audit logs
14. **Admin** - Administrative operations
15. **ZK Proofs** - Zero-knowledge proofs
16. **Test Blockchain** - Testing utilities

---

## 🔐 Authentication Methods

### JWT Token (Recommended for Web Apps)
```
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

- Get token from `POST /auth/login`
- Include in all authenticated requests
- Token expires after configured duration
- Includes user_id and workspace_id claims

### API Key (Recommended for Server-to-Server)
```
X-API-Key: dfm_a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6
```

- Create via `POST /api-keys`
- Long-lived, doesn't expire unless configured
- Can have IP and endpoint restrictions
- Supports rate limiting per key

---

## 📊 Response Format

All successful responses follow this format:
```json
{
  "success": true,
  "data": { ... },
  "message": "Operation completed successfully"
}
```

All error responses follow this format:
```json
{
  "error": "AUTHENTICATION_FAILED",
  "message": "Invalid credentials provided",
  "details": { ... },
  "suggestions": [
    "Verify your username and password",
    "Check if your account is active"
  ]
}
```

---

## 🎨 OpenAPI Visualization Tools

### Online Viewers
- [Swagger Editor](https://editor.swagger.io/) - Interactive editor with live preview
- [Redoc](https://redocly.github.io/redoc/) - Beautiful three-panel documentation
- [Stoplight Studio](https://stoplight.io/studio) - Visual API designer

### VS Code Extensions
- **OpenAPI (Swagger) Editor** - Syntax highlighting and validation
- **Swagger Viewer** - Live preview in VS Code
- **REST Client** - Test APIs directly from VS Code

---

## 📖 Additional Resources

- **CLAUDE_INSTRUCTIONS.md** - System architecture principles
- **Implementation Code** - See `src/api/*.rs` for actual implementations
- **Type Definitions** - See `src/types.rs` for data structures

---

## ✅ Validation Checklist

When working with the API:

- [ ] Read `openapi.yaml` to understand endpoint structure
- [ ] Use Swagger Editor to visualize the API
- [ ] Import into Postman/Insomnia for testing
- [ ] Generate TypeScript types from OpenAPI spec
- [ ] Never hardcode API URLs (use environment variables)
- [ ] Always handle errors with proper user feedback
- [ ] Implement token refresh logic for expired JWTs
- [ ] Use appropriate authentication method (JWT vs API Key)
- [ ] Follow rate limiting guidelines
- [ ] Log API errors for debugging

---

**Need help?** Contact the API team or refer to the OpenAPI specification for complete details.

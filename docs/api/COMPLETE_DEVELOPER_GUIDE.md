# DeFarm Engines - Complete Developer Guide

Welcome to the complete developer ecosystem for DeFarm Engines! This guide brings together all documentation, SDKs, tools, and resources in one place.

## 📚 Table of Contents

1. [Quick Start](#quick-start)
2. [Documentation](#documentation)
3. [SDKs](#sdks)
4. [CLI Tool](#cli-tool)
5. [Interactive API Documentation](#interactive-api-documentation)
6. [Example Projects](#example-projects)
7. [Support](#support)

---

## 🚀 Quick Start

### Option 1: Using the CLI (Easiest)

```bash
# Install CLI
npm install -g defarm-cli

# Login
defarm login

# Create item
defarm items create --key product --value ABC123

# List circuits
defarm circuits list
```

### Option 2: Using TypeScript SDK

```bash
# Install SDK
npm install @defarm/sdk

# Use in code
import { DefarmClient } from '@defarm/sdk';

const client = new DefarmClient({
  BASE: 'https://connect.defarm.net'
});

const { token } = await client.auth.login({
  username: 'your_username',
  password: 'your_password'
});
```

### Option 3: Using Python SDK

```bash
# Install SDK
pip install defarm-sdk

# Use in code
from defarm import ApiClient, Configuration
from defarm.api import AuthApi

config = Configuration(host="https://connect.defarm.net")
with ApiClient(config) as api_client:
    auth_api = AuthApi(api_client)
    response = auth_api.login({
        "username": "your_username",
        "password": "your_password"
    })
```

### Option 4: Direct API Calls (curl)

```bash
# Login
TOKEN=$(curl -s -X POST https://connect.defarm.net/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"hen","password":"demo123"}' | jq -r '.token')

# List items
curl -s https://connect.defarm.net/api/items \
  -H "Authorization: Bearer $TOKEN"
```

---

## 📖 Documentation

### Main Documentation Files

| Document | Description | Location |
|----------|-------------|----------|
| **API Guide** | Complete API reference with examples | `docs/api/API_GUIDE.md` |
| **Advanced Concepts** | Identifiers, deduplication, blockchain, events, webhooks | `docs/api/ADVANCED_CONCEPTS.md` |
| **API Guide Additions** | New sections (Snapshots, Timeline, Merkle, Credits, ZK Proofs) | `docs/api/API_GUIDE_ADDITIONS.md` |
| **OpenAPI Spec** | Machine-readable API specification | `docs/api/openapi.yaml` |
| **Client Integration** | Portuguese guide for Gerbov client | `docs/api/GERBOV_INTEGRATION.md` |
| **JWT Auth Guide** | Deep dive into JWT authentication | `docs/api/JWT_AUTHENTICATION_GUIDE.md` |

### Architecture Documentation

| Document | Description |
|----------|-------------|
| **System Principles** | Engine architecture and design principles |
| **Concurrency Model** | Thread safety and async patterns |
| **Railway Deployment** | Cloud deployment guide |

---

## 🛠 SDKs

### TypeScript/JavaScript SDK

**Installation:**
```bash
npm install @defarm/sdk
```

**Generate from OpenAPI:**
```bash
cd sdk
./generate-typescript-sdk.sh
```

**Features:**
- ✅ Full TypeScript type definitions
- ✅ Auto-generated from OpenAPI spec
- ✅ Axios-based HTTP client
- ✅ Promise-based async/await API
- ✅ Comprehensive error handling

**Example:**
```typescript
import { DefarmClient } from '@defarm/sdk';

const client = new DefarmClient({
  BASE: 'https://connect.defarm.net',
  TOKEN: 'your-jwt-token'
});

// Create item
const item = await client.items.createLocalItem({
  identifiers: [{
    namespace: 'bovino',
    key: 'sisbov',
    value: 'BR12345678901234',
    id_type: 'Canonical',
    verified: false
  }]
});

// Push to circuit
const result = await client.circuits.pushLocalItemToCircuit({
  circuitId: 'circuit-uuid',
  requestBody: { local_id: item.local_id }
});

console.log('Tokenized:', result.dfid);
```

**Location:** `sdk/typescript/`

---

### Python SDK

**Installation:**
```bash
pip install defarm-sdk
```

**Generate from OpenAPI:**
```bash
cd sdk
./generate-python-sdk.sh
```

**Features:**
- ✅ Python type hints
- ✅ Auto-generated from OpenAPI spec
- ✅ Urllib3-based HTTP client
- ✅ Exception handling
- ✅ Async support (optional)

**Example:**
```python
from defarm import ApiClient, Configuration
from defarm.api import ItemsApi, CircuitsApi

config = Configuration(host="https://connect.defarm.net")
config.access_token = "your-jwt-token"

with ApiClient(config) as api_client:
    items_api = ItemsApi(api_client)
    circuits_api = CircuitsApi(api_client)

    # Create item
    item = items_api.create_local_item({
        "identifiers": [{
            "namespace": "bovino",
            "key": "sisbov",
            "value": "BR12345678901234",
            "id_type": "Canonical",
            "verified": False
        }]
    })

    # Push to circuit
    result = circuits_api.push_local_item_to_circuit(
        circuit_id="circuit-uuid",
        body={"local_id": item.local_id}
    )

    print(f"Tokenized: {result.dfid}")
```

**Location:** `sdk/python/`

---

## 💻 CLI Tool

### Installation

```bash
npm install -g defarm-cli
```

Or use without installing:
```bash
npx defarm-cli <command>
```

### Available Commands

#### Authentication
```bash
defarm login                    # Interactive login
defarm login -u user -p pass    # Direct login
defarm whoami                   # Show current user
defarm logout                   # Clear credentials
```

#### Items
```bash
defarm items list                           # List all items
defarm items list --limit 10                # Limit results
defarm items create --key sisbov --value BR123
defarm items get DFID-20251203-000001-40BA
defarm items timeline DFID-20251203-000001-40BA
defarm items storage DFID-20251203-000001-40BA
```

#### Circuits
```bash
defarm circuits list
defarm circuits create "Supply Chain" --public
defarm circuits create "Supply Chain" --adapter StellarTestnetIpfs
defarm circuits get <circuit-id>
defarm circuits push <circuit-id> <local-id>
defarm circuits items <circuit-id>
defarm circuits members <circuit-id>
```

#### Events
```bash
defarm events list DFID-20251203-000001-40BA
defarm events create DFID-20251203-000001-40BA \
  --type Enriched \
  --visibility Public \
  --metadata '{"action":"test"}'
```

#### Merkle Tree
```bash
defarm merkle item-root DFID-20251203-000001-40BA
defarm merkle circuit-root <circuit-id>
defarm merkle verify proof.json
```

#### Configuration
```bash
defarm config set api_url https://connect.defarm.net
defarm config get token
defarm config list
```

### Environment Variables

```bash
export DEFARM_API_URL="https://connect.defarm.net"
export DEFARM_TOKEN="your-jwt-token"
export DEFARM_API_KEY="dfm_your_api_key"

# Now use CLI without login
defarm items list
```

### JSON Output

```bash
defarm items list --json | jq '.[0].dfid'
defarm circuits list --json | jq '.[] | .name'
```

**Location:** `cli/`

---

## 🌐 Interactive API Documentation

### Swagger UI

Open `docs/api/swagger-ui.html` in your browser for interactive API documentation.

**Features:**
- ✅ Try out API calls directly from browser
- ✅ See request/response examples
- ✅ Test with demo credentials
- ✅ Generate code snippets
- ✅ Explore all endpoints

**Demo Credentials:**
- Admin: `hen` / `demo123`
- Basic: `chick` / `Demo123!`
- Professional: `pullet` / `demo123`
- Enterprise: `cock` / `demo123`

**How to use:**
1. Open `swagger-ui.html`
2. Click "Authorize"
3. Login at `POST /api/auth/login` with demo credentials
4. Copy the token from response
5. Click "Authorize" again and paste token
6. Now you can try any endpoint!

**Location:** `docs/api/swagger-ui.html`

---

## 📦 Example Projects

### Complete Supply Chain Example (Bash)

```bash
#!/bin/bash
# Complete supply chain tracking with blockchain verification

TOKEN="your_jwt_token"
BASE_URL="https://connect.defarm.net"

# Create circuit with blockchain
CIRCUIT_ID=$(curl -s -X POST "$BASE_URL/api/circuits" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Organic Coffee Supply Chain",
    "adapter_config": {
      "adapter_type": "StellarTestnetIpfs",
      "requires_approval": false,
      "auto_migrate_existing": false,
      "sponsor_adapter_access": true
    }
  }' | jq -r '.data.circuit_id')

# Create local item
LOCAL_ID=$(curl -s -X POST "$BASE_URL/api/items/local" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "identifiers": [{
      "namespace": "coffee",
      "key": "batch",
      "value": "FARM-2025-001",
      "id_type": "Contextual"
    }],
    "enriched_data": {
      "farm": "Green Mountain Farm",
      "variety": "Arabica",
      "harvest_date": "2025-01-15"
    }
  }' | jq -r '.data.local_id')

# Push to circuit (tokenization + blockchain)
RESULT=$(curl -s -X POST "$BASE_URL/api/circuits/$CIRCUIT_ID/push-local" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"local_id\": \"$LOCAL_ID\"}")

DFID=$(echo "$RESULT" | jq -r '.data.dfid')
CID=$(echo "$RESULT" | jq -r '.data.storage.cid')
TX_HASH=$(echo "$RESULT" | jq -r '.data.storage.transaction_hash')

echo "✓ Item tokenized: $DFID"
echo "✓ IPFS CID: $CID"
echo "✓ Stellar TX: $TX_HASH"

# Add events
curl -s -X POST "$BASE_URL/api/events" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"dfid\": \"$DFID\",
    \"event_type\": \"Enriched\",
    \"visibility\": \"Public\",
    \"metadata\": {
      \"action\": \"wet_milling\",
      \"date\": \"$(date -I)\",
      \"operator\": \"farm_worker_1\"
    }
  }"

# Get Merkle proof
curl -s -X GET "$BASE_URL/api/merkle/items/$DFID/merkle-root" \
  -H "Authorization: Bearer $TOKEN" | jq '.'

echo "✓ Complete supply chain tracking enabled!"
```

### TypeScript/React Example

```typescript
import { useEffect, useState } from 'react';
import { DefarmClient } from '@defarm/sdk';

function ItemTracker() {
  const [client] = useState(() => new DefarmClient({
    BASE: 'https://connect.defarm.net'
  }));

  const [items, setItems] = useState([]);

  useEffect(() => {
    async function loadItems() {
      // Login
      const { token } = await client.auth.login({
        username: 'demo_user',
        password: 'demo_pass'
      });

      client.request.config.TOKEN = token;

      // Fetch items
      const itemList = await client.items.listItems();
      setItems(itemList);
    }

    loadItems();
  }, []);

  return (
    <div>
      <h1>My Items</h1>
      {items.map(item => (
        <div key={item.dfid}>
          <h3>{item.dfid}</h3>
          <p>Status: {item.status}</p>
        </div>
      ))}
    </div>
  );
}
```

### Python Flask Example

```python
from flask import Flask, jsonify
from defarm import ApiClient, Configuration
from defarm.api import ItemsApi

app = Flask(__name__)

config = Configuration(host="https://connect.defarm.net")
config.access_token = "your_token"

@app.route('/api/items')
def list_items():
    with ApiClient(config) as api_client:
        items_api = ItemsApi(api_client)
        items = items_api.list_items()
        return jsonify([{
            'dfid': item.dfid,
            'status': item.status,
            'created': item.creation_timestamp
        } for item in items])

if __name__ == '__main__':
    app.run(debug=True)
```

---

## 🎓 Learning Resources

### Tutorials

1. **Getting Started** - `docs/api/API_GUIDE.md`
2. **Advanced Workflows** - `docs/api/API_GUIDE_ADDITIONS.md`
3. **Client Integration** - `docs/api/GERBOV_INTEGRATION.md`
4. **Railway Deployment** - `CLAUDE.md` (Railway section)

### Video Tutorials (Coming Soon)

- Introduction to DeFarm Engines
- Building a supply chain app
- Blockchain integration guide
- Merkle tree verification

### Blog Posts (Coming Soon)

- Understanding DFIDs and Tokenization
- Circuit-based Data Sharing
- Cryptographic Verification with Merkle Trees
- Real-world Use Cases

---

## 📊 API Coverage

### Complete Endpoint List

| Module | Endpoints | Description |
|--------|-----------|-------------|
| Authentication | 6 | Login, register, password reset |
| Items | 27 | Local items, tokenized items, search |
| Circuits | 40+ | CRUD, operations, webhooks, public settings |
| Events | 12 | Create, query, timeline |
| Merkle Tree | 11 | Proofs, verification, roots |
| Snapshots | 8 | Historical state capture |
| Admin | 10 | User management, statistics |
| API Keys | 7 | Create, manage, revoke |
| Notifications | 6 | REST + WebSocket |
| Workspaces | 11 | Multi-tenant management |
| Audit | 17 | Compliance, security, logs |
| **Total** | **200+** | Complete API coverage |

---

## 🔧 Development Tools

### Generate SDKs

```bash
# TypeScript SDK
cd sdk
./generate-typescript-sdk.sh

# Python SDK
./generate-python-sdk.sh
```

### Validate OpenAPI Spec

```bash
npm install -g @apidevtools/swagger-cli
swagger-cli validate docs/api/openapi.yaml
```

### Test API

```bash
# Use Postman collection
# Import: docs/api/defarm-api-collection.json

# Or use CLI
defarm login
defarm items list
```

---

## 🆘 Support

### Documentation

- **API Guide**: `docs/api/API_GUIDE.md`
- **Interactive Docs**: Open `docs/api/swagger-ui.html`
- **OpenAPI Spec**: `docs/api/openapi.yaml`

### Community

- **GitHub Issues**: https://github.com/defarm/engines/issues
- **Discord**: https://discord.gg/defarm
- **Forum**: https://forum.defarm.net

### Contact

- **Email**: support@defarm.net
- **Documentation**: https://connect.defarm.net/docs
- **API Status**: https://status.defarm.net

---

## 📝 Quick Reference Card

### Authentication

```bash
# JWT
curl -X POST https://connect.defarm.net/api/auth/login \
  -d '{"username":"user","password":"pass"}'

# Use token
curl -H "Authorization: Bearer $TOKEN" \
  https://connect.defarm.net/api/items

# API Key
curl -H "X-API-Key: dfm_your_key" \
  https://connect.defarm.net/api/items
```

### Core Operations

```bash
# Create local item
POST /api/items/local

# Push to circuit (tokenization)
POST /api/circuits/{id}/push-local

# Create event
POST /api/events

# Get Merkle proof
GET /api/merkle/items/{dfid}/merkle-root
```

### Response Format

```json
{
  "success": true,
  "data": { ... },
  "message": "Operation completed"
}
```

### Error Format

```json
{
  "error": "ERROR_CODE",
  "message": "Human readable message",
  "suggestions": ["Try this...", "Or this..."]
}
```

---

**Last Updated**: 2025-01-24
**API Version**: v1.0
**Production URL**: https://connect.defarm.net

🌱 **Happy Building with DeFarm Engines!**

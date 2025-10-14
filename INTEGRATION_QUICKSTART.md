# DeFarm Engines - Integration Quickstart Guide

Welcome to the DeFarm Engines API! This guide will help you integrate your farm management system with our blockchain-based tokenization platform in less than 30 minutes.

## 📋 Table of Contents

1. [Overview](#overview)
2. [Prerequisites](#prerequisites)
3. [5-Minute Quickstart](#5-minute-quickstart)
4. [Authentication](#authentication)
5. [Core Workflow](#core-workflow)
6. [Code Examples](#code-examples)
7. [Webhooks](#webhooks)
8. [Troubleshooting](#troubleshooting)

---

## 🌟 Overview

The DeFarm API enables you to:
- **Tokenize farm assets** (cattle, crops, equipment) on blockchain
- **Store data** on IPFS (decentralized, immutable storage)
- **Track provenance** through supply chain via NFTs
- **Receive notifications** via webhooks when events occur

### Key Concepts

- **Local Item**: Item created in your system (not yet tokenized)
- **LID (Local ID)**: UUID assigned when you create a local item
- **Circuit**: Shared workspace where items are tokenized
- **Tokenization**: Process of minting NFT + storing on IPFS
- **DFID**: Universal identifier assigned after tokenization
- **Adapter**: Storage backend (e.g., StellarTestnetIpfs, StellarMainnetIpfs)

---

## ⚙️ Prerequisites

1. **API Credentials**: Username and password (or API key)
2. **Circuit ID**: Get from your DeFarm administrator
3. **HTTP Client**: curl, Python, Node.js, or your preferred language
4. **Network Access**: Firewall rules allowing HTTPS to production API

### API Endpoints

- **Production**: `https://defarm-engines-api-production.up.railway.app`
- **Local Development**: `http://localhost:3000`

---

## ⚡ 5-Minute Quickstart

### Step 1: Login and Get Token

```bash
curl -X POST https://defarm-engines-api-production.up.railway.app/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "username": "your_username",
    "password": "your_password"
  }'
```

**Response:**
```json
{
  "token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "user_id": "user-xxx",
  "workspace_id": "workspace-xxx",
  "expires_at": 1760457014
}
```

💡 **Save the token** - you'll use it in all subsequent requests.

### Step 2: Create a Local Item

```bash
curl -X POST https://defarm-engines-api-production.up.railway.app/api/items/local \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "enhanced_identifiers": [
      {
        "namespace": "bovino",
        "key": "sisbov",
        "value": "BR921180523565",
        "id_type": "Canonical"
      }
    ],
    "enriched_data": {
      "breed": "Nelore",
      "birth_date": "2023-05-15",
      "weight_kg": 450,
      "farm_id": "FARM-001"
    }
  }'
```

**Response:**
```json
{
  "success": true,
  "data": {
    "local_id": "12a0b56d-f478-4547-b796-792d6e0e81eb",
    "status": "LocalOnly"
  }
}
```

💡 **Save the local_id** - you'll use it to push to circuit.

### Step 3: Push to Circuit (Tokenization)

```bash
curl -X POST https://defarm-engines-api-production.up.railway.app/api/circuits/f17266f6-f4a1-44f7-87df-118194b20828/push-local \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "local_id": "12a0b56d-f478-4547-b796-792d6e0e81eb",
    "requester_id": "user-xxx"
  }'
```

**Response:**
```json
{
  "success": true,
  "data": {
    "dfid": "DFID-20251013-000001-CEEF",
    "status": "NewItemCreated",
    "operation_id": "b67f14c7-9ec0-49ff-ae3d-b5cf04b2de0e",
    "local_id": "12a0b56d-f478-4547-b796-792d6e0e81eb"
  }
}
```

### Step 4: Get Blockchain Transaction Hashes

```bash
curl https://defarm-engines-api-production.up.railway.app/api/items/DFID-20251013-000001-CEEF/storage-history \
  -H "Authorization: Bearer YOUR_TOKEN"
```

**Response:**
```json
{
  "success": true,
  "dfid": "DFID-20251013-000001-CEEF",
  "records": [
    {
      "adapter_type": "StellarTestnetIpfs",
      "network": "stellar-testnet",
      "nft_mint_tx": "c81ce1ccbdf4ffa334ab81b050ec670ba4277c877cae00d550932b9bd12fb272",
      "ipcm_update_tx": "88fb0f1a3a5039ea52f3237f988442be11de4225fdfc8ea79ad803370a27be04",
      "ipfs_cid": "QmPmK1dJR5TJaiknnM1gAHpUXM7eK9921kdc7TsuPW6Zsr",
      "ipfs_pinned": true,
      "stored_at": "2025-10-13T15:50:24.345425842+00:00",
      "triggered_by": "circuit_push",
      "is_active": true
    }
  ]
}
```

🎉 **Success!** Your item is now:
- ✅ Stored on IPFS with CID: `QmPmK1dJR5TJaiknnM1gAHpUXM7eK9921kdc7TsuPW6Zsr`
- ✅ Minted as NFT with transaction: `c81ce1cc...`
- ✅ Registered in IPCM with transaction: `88fb0f1a...`
- ✅ Assigned permanent DFID: `DFID-20251013-000001-CEEF`

---

## 🔐 Authentication

### Option 1: JWT Token (Recommended for Web Apps)

**Login to get token:**
```bash
POST /api/auth/login
Content-Type: application/json

{
  "username": "your_username",
  "password": "your_password"
}
```

**Use token in requests:**
```bash
Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...
```

**Token expires in 24 hours** - use `/api/auth/refresh` to renew.

### Option 2: API Key (Recommended for System Integration)

**Get API key from administrator**, then:

```bash
X-API-Key: dfm_your32characterapikeyhere12345
```

Or:

```bash
Authorization: Bearer dfm_your32characterapikeyhere12345
```

### New User Registration

```bash
POST /api/auth/register
Content-Type: application/json

{
  "username": "farm_user_01",
  "password": "SecurePass123!",
  "email": "user@farm.com",
  "workspace_name": "MyFarm"
}
```

**Password requirements:**
- Minimum 8 characters
- At least one uppercase letter
- At least one lowercase letter
- At least one digit
- At least one special character

---

## 🔄 Core Workflow

### Complete Integration Flow

```
1. Create Local Item
   ↓
2. Store in Your Database (with LID)
   ↓
3. Push to Circuit when ready
   ↓
4. Receive DFID from response
   ↓
5. Update Your Database (LID → DFID mapping)
   ↓
6. Query Storage History for blockchain TXs
   ↓
7. (Optional) Set up Webhooks for real-time updates
```

### When to Create Local Items vs Push to Circuit

**Create Local Item when:**
- User is offline or blockchain network unavailable
- You want to batch multiple items before tokenization
- You need to validate data before committing to blockchain
- You want to defer tokenization costs

**Push to Circuit when:**
- User explicitly requests tokenization
- Item reaches specific workflow stage (e.g., "Ready to ship")
- Batch processing window closes
- Compliance requires immediate blockchain registration

---

## 💻 Code Examples

### Python Example

```python
import requests
import json
from typing import Optional, Dict, Any

class DeFarmClient:
    def __init__(self, base_url: str, username: str, password: str):
        self.base_url = base_url
        self.token: Optional[str] = None
        self.user_id: Optional[str] = None
        self.login(username, password)

    def login(self, username: str, password: str):
        """Authenticate and get JWT token"""
        response = requests.post(
            f"{self.base_url}/api/auth/login",
            json={"username": username, "password": password}
        )
        response.raise_for_status()
        data = response.json()
        self.token = data['token']
        self.user_id = data['user_id']
        print(f"✅ Logged in as {self.user_id}")

    def _headers(self) -> Dict[str, str]:
        """Get authorization headers"""
        return {
            "Authorization": f"Bearer {self.token}",
            "Content-Type": "application/json"
        }

    def create_local_item(self, identifiers: list, data: dict) -> str:
        """Create a local item and return its LID"""
        payload = {
            "enhanced_identifiers": identifiers,
            "enriched_data": data
        }
        response = requests.post(
            f"{self.base_url}/api/items/local",
            headers=self._headers(),
            json=payload
        )
        response.raise_for_status()
        result = response.json()
        local_id = result['data']['local_id']
        print(f"✅ Created local item: {local_id}")
        return local_id

    def push_to_circuit(self, circuit_id: str, local_id: str) -> Dict[str, Any]:
        """Push local item to circuit for tokenization"""
        payload = {
            "local_id": local_id,
            "requester_id": self.user_id
        }
        response = requests.post(
            f"{self.base_url}/api/circuits/{circuit_id}/push-local",
            headers=self._headers(),
            json=payload
        )
        response.raise_for_status()
        result = response.json()
        dfid = result['data']['dfid']
        status = result['data']['status']
        print(f"✅ Tokenized! DFID: {dfid} (Status: {status})")
        return result['data']

    def get_storage_history(self, dfid: str) -> Dict[str, Any]:
        """Get blockchain transactions for an item"""
        response = requests.get(
            f"{self.base_url}/api/items/{dfid}/storage-history",
            headers=self._headers()
        )
        response.raise_for_status()
        result = response.json()

        if result['records']:
            record = result['records'][0]
            print(f"✅ Storage History for {dfid}:")
            print(f"   NFT TX: {record.get('nft_mint_tx')}")
            print(f"   IPCM TX: {record.get('ipcm_update_tx')}")
            print(f"   IPFS CID: {record.get('ipfs_cid')}")

        return result

# Usage Example
if __name__ == "__main__":
    # Initialize client
    client = DeFarmClient(
        base_url="https://defarm-engines-api-production.up.railway.app",
        username="your_username",
        password="your_password"
    )

    # Example 1: Cattle with SISBOV
    cattle_identifiers = [
        {
            "namespace": "bovino",
            "key": "sisbov",
            "value": "BR921180523565",
            "id_type": "Canonical"
        }
    ]
    cattle_data = {
        "breed": "Nelore",
        "birth_date": "2023-05-15",
        "weight_kg": 450,
        "farm_id": "FARM-001"
    }

    # Create local item
    local_id = client.create_local_item(cattle_identifiers, cattle_data)

    # Push to circuit
    result = client.push_to_circuit(
        circuit_id="f17266f6-f4a1-44f7-87df-118194b20828",
        local_id=local_id
    )

    # Get blockchain transactions
    history = client.get_storage_history(result['dfid'])

    # Example 2: Soybean batch
    soybean_identifiers = [
        {
            "namespace": "soja",
            "key": "lote",
            "value": "LOTE-2024-001",
            "id_type": "Contextual"
        },
        {
            "namespace": "soja",
            "key": "safra",
            "value": "2024",
            "id_type": "Contextual"
        }
    ]
    soybean_data = {
        "variety": "Monsoy 8372",
        "harvest_date": "2024-03-20",
        "quantity_kg": 50000,
        "certifications": ["organic", "non-gmo"]
    }

    local_id_2 = client.create_local_item(soybean_identifiers, soybean_data)
    result_2 = client.push_to_circuit("f17266f6-f4a1-44f7-87df-118194b20828", local_id_2)
```

### JavaScript/Node.js Example

```javascript
const axios = require('axios');

class DeFarmClient {
    constructor(baseUrl, username, password) {
        this.baseUrl = baseUrl;
        this.token = null;
        this.userId = null;
        this.login(username, password);
    }

    async login(username, password) {
        try {
            const response = await axios.post(`${this.baseUrl}/api/auth/login`, {
                username,
                password
            });
            this.token = response.data.token;
            this.userId = response.data.user_id;
            console.log(`✅ Logged in as ${this.userId}`);
        } catch (error) {
            console.error('❌ Login failed:', error.response?.data || error.message);
            throw error;
        }
    }

    getHeaders() {
        return {
            'Authorization': `Bearer ${this.token}`,
            'Content-Type': 'application/json'
        };
    }

    async createLocalItem(identifiers, data) {
        try {
            const response = await axios.post(
                `${this.baseUrl}/api/items/local`,
                {
                    enhanced_identifiers: identifiers,
                    enriched_data: data
                },
                { headers: this.getHeaders() }
            );
            const localId = response.data.data.local_id;
            console.log(`✅ Created local item: ${localId}`);
            return localId;
        } catch (error) {
            console.error('❌ Create item failed:', error.response?.data || error.message);
            throw error;
        }
    }

    async pushToCircuit(circuitId, localId) {
        try {
            const response = await axios.post(
                `${this.baseUrl}/api/circuits/${circuitId}/push-local`,
                {
                    local_id: localId,
                    requester_id: this.userId
                },
                { headers: this.getHeaders() }
            );
            const { dfid, status } = response.data.data;
            console.log(`✅ Tokenized! DFID: ${dfid} (Status: ${status})`);
            return response.data.data;
        } catch (error) {
            console.error('❌ Push to circuit failed:', error.response?.data || error.message);
            throw error;
        }
    }

    async getStorageHistory(dfid) {
        try {
            const response = await axios.get(
                `${this.baseUrl}/api/items/${dfid}/storage-history`,
                { headers: this.getHeaders() }
            );

            if (response.data.records && response.data.records.length > 0) {
                const record = response.data.records[0];
                console.log(`✅ Storage History for ${dfid}:`);
                console.log(`   NFT TX: ${record.nft_mint_tx}`);
                console.log(`   IPCM TX: ${record.ipcm_update_tx}`);
                console.log(`   IPFS CID: ${record.ipfs_cid}`);
            }

            return response.data;
        } catch (error) {
            console.error('❌ Get storage history failed:', error.response?.data || error.message);
            throw error;
        }
    }
}

// Usage Example
(async () => {
    // Initialize client
    const client = new DeFarmClient(
        'https://defarm-engines-api-production.up.railway.app',
        'your_username',
        'your_password'
    );

    // Wait for login
    await new Promise(resolve => setTimeout(resolve, 1000));

    // Example: Cattle with SISBOV
    const cattleIdentifiers = [
        {
            namespace: 'bovino',
            key: 'sisbov',
            value: 'BR921180523565',
            id_type: 'Canonical'
        }
    ];

    const cattleData = {
        breed: 'Nelore',
        birth_date: '2023-05-15',
        weight_kg: 450,
        farm_id: 'FARM-001'
    };

    try {
        // Create local item
        const localId = await client.createLocalItem(cattleIdentifiers, cattleData);

        // Push to circuit
        const result = await client.pushToCircuit('f17266f6-f4a1-44f7-87df-118194b20828', localId);

        // Get blockchain transactions
        await client.getStorageHistory(result.dfid);
    } catch (error) {
        console.error('Error in workflow:', error);
    }
})();
```

---

## 🔔 Webhooks

Set up webhooks to receive real-time notifications when items are tokenized.

### Creating a Webhook

```bash
curl -X POST https://defarm-engines-api-production.up.railway.app/api/circuits/f17266f6-f4a1-44f7-87df-118194b20828/post-actions/webhooks \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "ERP System Notification",
    "url": "https://your-erp.com/api/defarm-webhook",
    "events": ["ItemPushed", "ItemTokenized"],
    "enabled": true,
    "auth_type": "BearerToken",
    "auth_config": {
      "token": "your_secret_token"
    },
    "include_storage_details": true,
    "include_item_metadata": true
  }'
```

### Webhook Payload Example

When an item is tokenized, you'll receive:

```json
{
  "event_type": "ItemTokenized",
  "circuit_id": "8c453dd7-3996-49a3-828a-27c155cff82a",
  "circuit_name": "Production Circuit",
  "timestamp": "2025-10-13T15:50:24.345Z",
  "item": {
    "dfid": "DFID-20251013-000001-CEEF",
    "local_id": "12a0b56d-f478-4547-b796-792d6e0e81eb",
    "identifiers": [
      {
        "namespace": "bovino",
        "key": "sisbov",
        "value": "BR921180523565"
      }
    ],
    "pushed_by": "user-xxx"
  },
  "storage": {
    "adapter_type": "StellarTestnetIpfs",
    "location": "Stellar Testnet",
    "nft_transaction": "c81ce1ccbdf4ffa334ab81b050ec670ba4277c877cae00d550932b9bd12fb272",
    "ipcm_transaction": "88fb0f1a3a5039ea52f3237f988442be11de4225fdfc8ea79ad803370a27be04",
    "ipfs_cid": "QmPmK1dJR5TJaiknnM1gAHpUXM7eK9921kdc7TsuPW6Zsr",
    "metadata": {
      "nft_contract": "CDOZEG35YQ7KYASQBUW2DVV7CIQZB5HMWAB2PWPUCHSTKSCD5ZUTPUW3",
      "ipcm_contract": "CAALVDSF7RLM7IRGE3GQKPRHWWZSPDSNHOBEIEDJU5MAM4I4PVFWJXLS"
    }
  },
  "operation_id": "b67f14c7-9ec0-49ff-ae3d-b5cf04b2de0e",
  "status": "Completed"
}
```

### Webhook Best Practices

1. **Return 200 OK quickly** - Process async if needed
2. **Validate webhook signature** - Use auth_type for security
3. **Handle retries gracefully** - DeFarm retries up to 3 times
4. **Log all webhook deliveries** - For debugging and audit
5. **Use HTTPS only** - Never accept webhooks over HTTP in production

---

## 🐛 Troubleshooting

### Common Issues

#### 401 Unauthorized

**Problem**: Token expired or invalid

**Solution**:
```bash
# Refresh token
POST /api/auth/refresh
Authorization: Bearer YOUR_OLD_TOKEN
```

#### 403 Forbidden - Adapter Access Denied

**Problem**: Your tier doesn't have access to the adapter

**Error**:
```json
{
  "error": "Permission denied: Your tier (basic) does not have access to the StellarMainnetIpfs adapter"
}
```

**Solution**: Contact administrator to upgrade tier or request adapter access

#### 404 Not Found - Circuit or Item

**Problem**: Circuit ID or DFID doesn't exist

**Solution**: Verify IDs with administrator

#### 429 Rate Limit Exceeded

**Problem**: Too many requests

**Response Headers**:
```
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 0
X-RateLimit-Reset: 1760460000
Retry-After: 3600
```

**Solution**: Wait for reset time or upgrade tier for higher limits

### Rate Limits by Tier

| Tier | Requests/Hour | Requests/Day |
|------|--------------|--------------|
| Basic | 100 | 1,000 |
| Professional | 1,000 | 10,000 |
| Enterprise | 10,000 | 100,000 |

### Debug Mode

Enable verbose logging in your client:

```python
import logging
logging.basicConfig(level=logging.DEBUG)
```

```javascript
axios.interceptors.request.use(request => {
    console.log('Request:', request);
    return request;
});
```

### Getting Help

1. **Check API Documentation**: Open `docs/index.html` in browser
2. **View Interactive Examples**: Use Swagger UI "Try it out" feature
3. **Contact Support**: support@defarm.io
4. **Check Status Page**: https://status.defarm.io

---

## 📚 Additional Resources

- **Full API Reference**: [docs/openapi-external.yaml](docs/openapi-external.yaml)
- **Interactive Documentation**: [docs/index.html](docs/index.html)
- **Architecture Guide**: [CLAUDE.md](CLAUDE.md)
- **Production Deployment**: [PRODUCTION_DEPLOYMENT.md](PRODUCTION_DEPLOYMENT.md)

---

## 🎯 Next Steps

1. ✅ Complete the 5-minute quickstart
2. ✅ Test with your own data
3. ✅ Set up webhooks for production
4. ✅ Implement error handling and retries
5. ✅ Monitor rate limits
6. ✅ Request production credentials from administrator

**Questions?** Contact your DeFarm integration specialist or email support@defarm.io

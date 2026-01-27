# API Guide Additions - New Sections

## 📸 Snapshots

Snapshots capture the complete state of items or circuits at a specific point in time, enabling historical analysis and state recovery.

### What are Snapshots?

A snapshot is an immutable record of:
- **Item Snapshot**: Complete item state including identifiers, enriched data, events, and storage history
- **Circuit Snapshot**: Complete circuit state including members, permissions, items, and configuration

Snapshots are automatically created when:
- Items are pushed to circuits
- Circuit configuration changes
- Events are added to items
- Manual snapshot requests

### Snapshot Endpoints

#### List Item Snapshots
**Endpoint:** `GET /api/snapshots/items/:dfid/snapshots`

```bash
curl -X GET "https://connect.defarm.net/api/snapshots/items/DFID-20251203-000001-40BA/snapshots" \
  -H "Authorization: Bearer $TOKEN"
```

**Response:**
```json
{
  "success": true,
  "data": {
    "snapshots": [
      {
        "snapshot_id": "snap-uuid-123",
        "dfid": "DFID-20251203-000001-40BA",
        "created_at": "2025-01-21T10:00:00Z",
        "event_count": 5,
        "storage_count": 2,
        "created_by": "user-123"
      }
    ],
    "total": 10
  }
}
```

#### Get Latest Item Snapshot
**Endpoint:** `GET /api/snapshots/items/:dfid/snapshots/latest`

```bash
curl -X GET "https://connect.defarm.net/api/snapshots/items/DFID-20251203-000001-40BA/snapshots/latest" \
  -H "Authorization: Bearer $TOKEN"
```

**Response includes:**
- Complete item data
- All events at snapshot time
- Storage history at snapshot time
- Identifiers and enriched data

#### Get Specific Snapshot
**Endpoint:** `GET /api/snapshots/items/:dfid/snapshots/:snapshot_id`

```bash
curl -X GET "https://connect.defarm.net/api/snapshots/items/DFID-20251203-000001-40BA/snapshots/snap-uuid-123" \
  -H "Authorization: Bearer $TOKEN"
```

#### Circuit Snapshots

**Endpoint:** `GET /api/snapshots/circuits/:circuit_id/snapshots`

```bash
curl -X GET "https://connect.defarm.net/api/snapshots/circuits/circuit-uuid/snapshots" \
  -H "Authorization: Bearer $TOKEN"
```

**Circuit snapshot includes:**
- Circuit configuration
- Member list with roles
- All items in circuit
- Permission settings
- Public settings

### Public Snapshots

For public items/circuits, snapshots can be accessed without authentication:

**Endpoint:** `GET /api/public/snapshots/items/:dfid/snapshots`

```bash
curl -X GET "https://connect.defarm.net/api/public/snapshots/items/DFID-20251203-000001-40BA/snapshots"
```

### Use Cases

1. **Historical Analysis**: Compare item state across time
2. **Audit Trail**: Prove item state at specific moment
3. **Data Recovery**: Restore previous state if needed
4. **Compliance**: Meet regulatory requirements for data retention
5. **Conflict Resolution**: Determine when changes occurred

---

## 📅 Timeline

The Timeline API provides a chronological view of all activities related to an item, combining events, circuit operations, and storage changes into a unified timeline.

### Timeline Endpoints

#### Get Item Timeline
**Endpoint:** `GET /api/timeline/items/:dfid`

```bash
curl -X GET "https://connect.defarm.net/api/timeline/items/DFID-20251203-000001-40BA?limit=50" \
  -H "Authorization: Bearer $TOKEN"
```

**Query Parameters:**
- `limit` (optional): Maximum entries to return (default: 50)
- `offset` (optional): Pagination offset
- `from_date` (optional): Filter from date (ISO8601)
- `to_date` (optional): Filter to date (ISO8601)
- `event_types` (optional): Filter by event types (comma-separated)

**Response:**
```json
{
  "success": true,
  "data": {
    "timeline": [
      {
        "entry_id": "timeline-uuid-1",
        "timestamp": "2025-01-21T10:00:00Z",
        "entry_type": "Event",
        "event_type": "Created",
        "source": "user-123",
        "metadata": {
          "action": "initial_registration"
        }
      },
      {
        "entry_id": "timeline-uuid-2",
        "timestamp": "2025-01-21T11:00:00Z",
        "entry_type": "CircuitOperation",
        "operation": "PushedToCircuit",
        "circuit_id": "circuit-uuid",
        "circuit_name": "Supply Chain A"
      },
      {
        "entry_id": "timeline-uuid-3",
        "timestamp": "2025-01-21T12:00:00Z",
        "entry_type": "StorageChange",
        "storage_type": "StellarTestnetIpfs",
        "cid": "Qm...",
        "transaction_hash": "abc123..."
      }
    ],
    "total_entries": 25,
    "has_more": false
  }
}
```

#### Get Single Timeline Entry
**Endpoint:** `GET /api/timeline/entries/:entry_id`

```bash
curl -X GET "https://connect.defarm.net/api/timeline/entries/timeline-uuid-1" \
  -H "Authorization: Bearer $TOKEN"
```

#### Timeline Indexing Progress
**Endpoint:** `GET /api/timeline/indexing-progress`

```bash
curl -X GET "https://connect.defarm.net/api/timeline/indexing-progress" \
  -H "Authorization: Bearer $TOKEN"
```

**Response:**
```json
{
  "success": true,
  "data": {
    "total_items": 1000,
    "indexed_items": 950,
    "progress_percentage": 95.0,
    "last_updated": "2025-01-21T13:00:00Z"
  }
}
```

### Timeline Entry Types

1. **Event**: Item lifecycle events (Created, Enriched, etc.)
2. **CircuitOperation**: Push/Pull operations
3. **StorageChange**: Blockchain/storage updates
4. **IdentifierChange**: Identifier additions/modifications
5. **MerkleAnchor**: Merkle root blockchain anchors (future)

### Use Cases

1. **Complete Traceability**: Full item journey from creation to current state
2. **Audit Reports**: Generate comprehensive audit trails
3. **Supply Chain Visibility**: Track item movement through circuits
4. **Compliance Documentation**: Prove chain of custody
5. **Analytics**: Analyze patterns in item lifecycles

---

## 🔐 Merkle State Tree

The Merkle State Tree provides cryptographic verification of data integrity using BLAKE3 hashing. It enables trustless verification of events and items without accessing the full dataset.

### How Merkle Trees Work

DeFarm uses a three-level Merkle tree hierarchy:

```
Circuit Merkle Root
    ├── Item 1 Merkle Root
    │   ├── Event 1 Hash
    │   ├── Event 2 Hash
    │   └── Event 3 Hash
    ├── Item 2 Merkle Root
    │   ├── Event 4 Hash
    │   └── Event 5 Hash
    └── Item 3 Merkle Root
        └── Event 6 Hash
```

**Event Hash** = BLAKE3(event_id + dfid + event_type + source + timestamp + metadata)
**Item Merkle Root** = Merkle root of all event hashes for that item
**Circuit Merkle Root** = Merkle root of all item Merkle roots in circuit

### Merkle Endpoints (Authenticated)

#### Get Item Merkle Root
**Endpoint:** `GET /api/merkle/items/:dfid/merkle-root`

```bash
curl -X GET "https://connect.defarm.net/api/merkle/items/DFID-20251203-000001-40BA/merkle-root" \
  -H "Authorization: Bearer $TOKEN"
```

**Response:**
```json
{
  "success": true,
  "data": {
    "dfid": "DFID-20251203-000001-40BA",
    "merkle_root": "f8b11708805ac8e29f4a63f9092ee44cf05361da8ea457380a3d2a81a4d00f05",
    "event_count": 5,
    "computed_at": "2025-01-21T13:00:00Z"
  }
}
```

#### Generate Event Proof
**Endpoint:** `GET /api/merkle/items/:dfid/merkle-proof/:event_id`

```bash
curl -X GET "https://connect.defarm.net/api/merkle/items/DFID-20251203-000001-40BA/merkle-proof/event-uuid-123" \
  -H "Authorization: Bearer $TOKEN"
```

**Response:**
```json
{
  "success": true,
  "data": {
    "proof": {
      "event_id": "event-uuid-123",
      "dfid": "DFID-20251203-000001-40BA",
      "event_hash": "abc123...",
      "merkle_root": "f8b117...",
      "proof_hashes": ["hash1", "hash2", "hash3"],
      "proof_positions": ["left", "right", "left"]
    }
  }
}
```

#### Get Circuit Merkle Root
**Endpoint:** `GET /api/merkle/circuits/:circuit_id/merkle-root`

```bash
curl -X GET "https://connect.defarm.net/api/merkle/circuits/circuit-uuid/merkle-root" \
  -H "Authorization: Bearer $TOKEN"
```

**Response:**
```json
{
  "success": true,
  "data": {
    "circuit_id": "circuit-uuid",
    "merkle_root": "2c1ee7e929a1a90db8b6db7b3795dbb2dd0caa206a6438c19d62f846fca85b14",
    "item_count": 7,
    "items": [
      {
        "dfid": "DFID-20251203-000001-40BA",
        "event_count": 5,
        "merkle_root": "f8b11708..."
      }
    ],
    "computed_at": "2025-01-21T13:00:00Z"
  }
}
```

#### Generate Item Proof
**Endpoint:** `GET /api/merkle/circuits/:circuit_id/merkle-proof/:dfid`

```bash
curl -X GET "https://connect.defarm.net/api/merkle/circuits/circuit-uuid/merkle-proof/DFID-20251203-000001-40BA" \
  -H "Authorization: Bearer $TOKEN"
```

#### Verify Merkle Proof
**Endpoint:** `POST /api/merkle/verify-proof`

```bash
curl -X POST "https://connect.defarm.net/api/merkle/verify-proof" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "proof": {
      "event_hash": "abc123...",
      "merkle_root": "f8b117...",
      "proof_hashes": ["hash1", "hash2"],
      "proof_positions": ["left", "right"]
    }
  }'
```

**Response:**
```json
{
  "success": true,
  "data": {
    "is_valid": true,
    "verified_at": "2025-01-21T13:00:00Z"
  }
}
```

### Public Merkle Endpoints (No Authentication)

All Merkle endpoints are available publicly for public items/circuits:

**Base Path:** `/api/public/merkle`

```bash
# Public item Merkle root
curl -X GET "https://connect.defarm.net/api/public/merkle/items/DFID-20251203-000001-40BA/merkle-root"

# Public event proof
curl -X GET "https://connect.defarm.net/api/public/merkle/items/DFID-20251203-000001-40BA/merkle-proof/event-uuid"

# Public circuit Merkle root
curl -X GET "https://connect.defarm.net/api/public/merkle/circuits/circuit-uuid/merkle-root"

# Public item proof
curl -X GET "https://connect.defarm.net/api/public/merkle/circuits/circuit-uuid/merkle-proof/DFID-20251203-000001-40BA"

# Public proof verification
curl -X POST "https://connect.defarm.net/api/public/merkle/verify-proof" \
  -H "Content-Type: application/json" \
  -d '{"proof": {...}}'
```

### Use Cases

1. **Third-Party Verification**: External auditors can verify data integrity without API access
2. **Tamper Detection**: Detect if any event or item has been modified
3. **Lightweight Verification**: Verify specific events without downloading entire dataset
4. **Blockchain Anchoring** (Future): Anchor circuit roots to blockchain for immutability
5. **Compliance**: Cryptographic proof for regulatory requirements

### Future: Blockchain Anchoring

**Planned Feature:**
- Periodic commitment of circuit Merkle roots to Stellar IPCM contract
- Creates immutable timestamp proving circuit state at specific moment
- Enables verification against on-chain anchors
- Suggested interval: configurable per circuit (hourly/daily/on-demand)

---

## 💳 User Credits

The User Credits system manages tier-based operations, ensuring users stay within their subscription limits.

### How Credits Work

Credits are consumed for:
- Creating items
- Pushing items to circuits
- Creating events
- Creating circuits
- Storage operations (blockchain)

Credit limits vary by tier:
- **Free**: 100 credits/month
- **Basic**: 1,000 credits/month
- **Professional**: 10,000 credits/month
- **Enterprise**: Unlimited

### Credit Endpoints

#### Get Credit Balance
**Endpoint:** `GET /api/user-credits/balance`

```bash
curl -X GET "https://connect.defarm.net/api/user-credits/balance" \
  -H "Authorization: Bearer $TOKEN"
```

**Response:**
```json
{
  "success": true,
  "data": {
    "user_id": "user-123",
    "tier": "Professional",
    "balance": 8453,
    "monthly_limit": 10000,
    "used_this_month": 1547,
    "reset_date": "2025-02-01T00:00:00Z"
  }
}
```

#### Get Credit Transaction History
**Endpoint:** `GET /api/user-credits/transactions`

```bash
curl -X GET "https://connect.defarm.net/api/user-credits/transactions?limit=20" \
  -H "Authorization: Bearer $TOKEN"
```

**Response:**
```json
{
  "success": true,
  "data": {
    "transactions": [
      {
        "transaction_id": "tx-uuid-1",
        "timestamp": "2025-01-21T10:00:00Z",
        "operation": "CREATE_ITEM",
        "credits_used": 1,
        "balance_after": 8453,
        "metadata": {
          "dfid": "DFID-20251203-000001-40BA"
        }
      },
      {
        "transaction_id": "tx-uuid-2",
        "timestamp": "2025-01-21T11:00:00Z",
        "operation": "PUSH_TO_CIRCUIT",
        "credits_used": 2,
        "balance_after": 8451,
        "metadata": {
          "circuit_id": "circuit-uuid",
          "adapter": "StellarTestnetIpfs"
        }
      }
    ],
    "total": 1547
  }
}
```

### Credit Operations and Costs

| Operation | Free | Basic | Professional | Enterprise |
|-----------|------|-------|--------------|------------|
| Create Local Item | 0 | 0 | 0 | 0 |
| Create Item with DFID | 1 | 1 | 1 | 0 |
| Push to Circuit (no blockchain) | 1 | 1 | 1 | 0 |
| Push to Circuit (blockchain) | 5 | 5 | 2 | 0 |
| Create Event | 0 | 0 | 0 | 0 |
| Create Circuit | 10 | 5 | 2 | 0 |

### Admin Credit Management

Admins can adjust user credits:

**Endpoint:** `POST /api/admin/users/:user_id/credits`

```bash
curl -X POST "https://connect.defarm.net/api/admin/users/user-123/credits" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "amount": 1000,
    "operation": "add",
    "reason": "Promotional bonus"
  }'
```

**Bulk Grant Credits:**

**Endpoint:** `POST /api/admin/users/credits/bulk-grant`

```bash
curl -X POST "https://connect.defarm.net/api/admin/users/credits/bulk-grant" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "user_ids": ["user-1", "user-2", "user-3"],
    "amount": 500,
    "reason": "Beta testing reward"
  }'
```

### Handling Insufficient Credits

When credits are insufficient, the API returns:

```json
{
  "error": "INSUFFICIENT_CREDITS",
  "message": "Not enough credits to complete this operation",
  "details": {
    "required": 5,
    "available": 2,
    "tier": "Basic"
  },
  "suggestions": [
    "Upgrade to Professional tier for more credits",
    "Wait until monthly reset on 2025-02-01",
    "Contact admin for credit adjustment"
  ]
}
```

---

## 🔒 Zero-Knowledge Proofs (ZK Proofs)

Zero-Knowledge Proofs allow verification of data properties without revealing the data itself.

### What are ZK Proofs?

ZK Proofs enable:
- Prove item exists without showing identifiers
- Prove event occurred without revealing metadata
- Prove item in circuit without exposing circuit details
- Age verification without showing exact date
- Quantity verification without revealing amount

### ZK Proof Endpoints

**Note:** This is an advanced feature currently in development. Contact the API team for early access.

#### Generate ZK Proof
**Endpoint:** `POST /api/zk-proofs/generate`

```bash
curl -X POST "https://connect.defarm.net/api/zk-proofs/generate" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "proof_type": "item_exists",
    "dfid": "DFID-20251203-000001-40BA",
    "properties": {
      "min_events": 5
    }
  }'
```

#### Verify ZK Proof
**Endpoint:** `POST /api/zk-proofs/verify`

```bash
curl -X POST "https://connect.defarm.net/api/zk-proofs/verify" \
  -H "Content-Type: application/json" \
  -d '{
    "proof": "...",
    "public_inputs": {...}
  }'
```

### Planned ZK Proof Types

1. **Item Existence**: Prove item exists without revealing identifiers
2. **Event Count**: Prove minimum number of events without showing them
3. **Age Verification**: Prove item older than X days
4. **Circuit Membership**: Prove item in circuit without naming circuit
5. **Value Range**: Prove value in range without revealing exact value

---

## 🚀 Advanced Workflows

### Workflow 3: Complete Supply Chain Tracking

This workflow demonstrates a complete supply chain from farm to consumer with blockchain verification.

```bash
#!/bin/bash
TOKEN="your_jwt_token"
BASE_URL="https://connect.defarm.net"

# Step 1: Create Supply Chain Circuit with Blockchain
echo "Creating supply chain circuit..."
CIRCUIT_RESPONSE=$(curl -s -X POST "$BASE_URL/api/circuits" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Organic Coffee Supply Chain",
    "description": "Track organic coffee from farm to roaster",
    "adapter_config": {
      "adapter_type": "StellarTestnetIpfs",
      "requires_approval": false,
      "auto_migrate_existing": false,
      "sponsor_adapter_access": true
    },
    "permissions": {
      "require_approval_for_push": false,
      "require_approval_for_pull": false,
      "allow_public_visibility": true
    }
  }')

CIRCUIT_ID=$(echo "$CIRCUIT_RESPONSE" | jq -r '.data.circuit_id')
echo "Circuit created: $CIRCUIT_ID"

# Step 2: Create Coffee Batch at Farm
echo "Creating coffee batch..."
LOCAL_RESPONSE=$(curl -s -X POST "$BASE_URL/api/items/local" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "identifiers": [
      {
        "namespace": "coffee",
        "key": "batch",
        "value": "FARM-2025-001",
        "id_type": "Contextual",
        "verified": false
      },
      {
        "namespace": "coffee",
        "key": "organic_cert",
        "value": "USDA-ORG-12345",
        "id_type": "Canonical",
        "verified": true
      }
    ],
    "enriched_data": {
      "farm": "Green Mountain Farm",
      "variety": "Arabica",
      "harvest_date": "2025-01-15",
      "weight_kg": 1000,
      "altitude_m": 1500
    }
  }')

LOCAL_ID=$(echo "$LOCAL_RESPONSE" | jq -r '.data.local_id')
echo "Local batch created: $LOCAL_ID"

# Step 3: Push to Circuit (Tokenization + Blockchain Storage)
echo "Tokenizing batch on blockchain..."
PUSH_RESPONSE=$(curl -s -X POST "$BASE_URL/api/circuits/$CIRCUIT_ID/push-local" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"local_id\": \"$LOCAL_ID\"
  }")

DFID=$(echo "$PUSH_RESPONSE" | jq -r '.data.dfid')
CID=$(echo "$PUSH_RESPONSE" | jq -r '.data.storage.cid')
TX_HASH=$(echo "$PUSH_RESPONSE" | jq -r '.data.storage.transaction_hash')

echo "Batch tokenized: $DFID"
echo "IPFS CID: $CID"
echo "Stellar TX: $TX_HASH"

# Step 4: Add Processing Event
echo "Adding processing event..."
EVENT_RESPONSE=$(curl -s -X POST "$BASE_URL/api/events" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"dfid\": \"$DFID\",
    \"event_type\": \"Enriched\",
    \"visibility\": \"Public\",
    \"metadata\": {
      \"action\": \"wet_milling\",
      \"date\": \"2025-01-16\",
      \"operator\": \"farm_worker_1\",
      \"notes\": \"Wet milling completed, beans in fermentation\"
    }
  }")

echo "Processing event added"

# Step 5: Transfer to Processor
echo "Adding transfer event..."
curl -s -X POST "$BASE_URL/api/events" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"dfid\": \"$DFID\",
    \"event_type\": \"Transferred\",
    \"visibility\": \"Public\",
    \"metadata\": {
      \"from\": \"Green Mountain Farm\",
      \"to\": \"Mountain Roasters Inc\",
      \"date\": \"2025-01-20\",
      \"transport\": \"refrigerated_truck\",
      \"weight_kg\": 950
    }
  }" > /dev/null

echo "Transfer event added"

# Step 6: Get Complete Timeline
echo "Fetching complete timeline..."
curl -s -X GET "$BASE_URL/api/timeline/items/$DFID" \
  -H "Authorization: Bearer $TOKEN" | jq '.data.timeline'

# Step 7: Get Storage History
echo "Fetching storage history..."
curl -s -X GET "$BASE_URL/api/items/$DFID/storage-history" \
  -H "Authorization: Bearer $TOKEN" | jq '.data'

# Step 8: Generate Merkle Proof
echo "Generating cryptographic proof..."
PROOF=$(curl -s -X GET "$BASE_URL/api/merkle/circuits/$CIRCUIT_ID/merkle-proof/$DFID" \
  -H "Authorization: Bearer $TOKEN")

echo "Merkle proof generated"
echo "$PROOF" | jq '.data.proof.merkle_root'

# Step 9: Make Circuit Public
echo "Publishing circuit to public..."
curl -s -X PUT "$BASE_URL/api/circuits/$CIRCUIT_ID/public-settings" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "access_mode": "Public",
    "public_name": "Organic Coffee Traceability",
    "public_description": "Track our organic coffee from farm to your cup",
    "published_items": ["'$DFID'"],
    "auto_publish_pushed_items": true,
    "show_encrypted_events": false
  }' > /dev/null

echo "Circuit is now public!"
echo "Public URL: https://connect.defarm.net/public/circuits/$CIRCUIT_ID"

# Step 10: Anyone can now verify (no auth needed)
echo "Public verification (no auth)..."
curl -s -X GET "$BASE_URL/api/public/merkle/items/$DFID/merkle-root" | jq '.'
```

**Result:** Complete supply chain tracking with:
- ✅ Blockchain storage (IPFS + Stellar)
- ✅ Cryptographic verification (Merkle proofs)
- ✅ Public transparency (anyone can verify)
- ✅ Complete audit trail (all events tracked)
- ✅ Tamper-proof (blockchain anchored)

### Workflow 4: Multi-Organization Collaboration

```bash
#!/bin/bash
TOKEN_ORG_A="org_a_token"
TOKEN_ORG_B="org_b_token"
BASE_URL="https://connect.defarm.net"

# Organization A creates circuit and invites Organization B
CIRCUIT_ID=$(curl -s -X POST "$BASE_URL/api/circuits" \
  -H "Authorization: Bearer $TOKEN_ORG_A" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Multi-Org Collaboration",
    "description": "Shared supply chain tracking",
    "permissions": {
      "require_approval_for_push": true,
      "require_approval_for_pull": false
    }
  }' | jq -r '.data.circuit_id')

# Add Organization B as member
curl -s -X POST "$BASE_URL/api/circuits/$CIRCUIT_ID/members" \
  -H "Authorization: Bearer $TOKEN_ORG_A" \
  -H "Content-Type: application/json" \
  -d '{
    "member_id": "org-b-user-id",
    "role": "Member"
  }'

# Organization B creates and pushes item
LOCAL_ID=$(curl -s -X POST "$BASE_URL/api/items/local" \
  -H "Authorization: Bearer $TOKEN_ORG_B" \
  -H "Content-Type: application/json" \
  -d '{
    "identifiers": [{"namespace": "product", "key": "sku", "value": "SKU-123", "id_type": "Contextual"}]
  }' | jq -r '.data.local_id')

# Push requires approval
OPERATION_ID=$(curl -s -X POST "$BASE_URL/api/circuits/$CIRCUIT_ID/push-local" \
  -H "Authorization: Bearer $TOKEN_ORG_B" \
  -H "Content-Type: application/json" \
  -d "{\"local_id\": \"$LOCAL_ID\"}" | jq -r '.data.operation_id')

# Organization A approves
curl -s -X POST "$BASE_URL/api/circuits/operations/$OPERATION_ID/approve" \
  -H "Authorization: Bearer $TOKEN_ORG_A" \
  -H "Content-Type: application/json" \
  -d '{
    "approved": true,
    "notes": "Verified and approved"
  }'

echo "Multi-org workflow complete!"
```

### Workflow 5: API Key for IoT Device

```bash
#!/bin/bash
TOKEN="your_jwt_token"
BASE_URL="https://connect.defarm.net"

# Create API key for IoT device
API_KEY_RESPONSE=$(curl -s -X POST "$BASE_URL/api/api-keys" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Warehouse IoT Sensor",
    "permissions": {
      "read": true,
      "write": true,
      "admin": false
    },
    "allowed_endpoints": [
      "/api/items/local",
      "/api/events",
      "/api/circuits/*/push-local"
    ],
    "rate_limit_per_hour": 500,
    "expires_in_days": 365,
    "notes": "Temperature and humidity sensor for warehouse monitoring"
  }')

API_KEY=$(echo "$API_KEY_RESPONSE" | jq -r '.data.api_key')
echo "API Key for IoT Device: $API_KEY"
echo "⚠️  Save this key securely - it won't be shown again!"

# IoT device can now use API key directly (no JWT needed)
# Example: IoT device reports temperature reading
curl -X POST "$BASE_URL/api/events" \
  -H "X-API-Key: $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "dfid": "DFID-WAREHOUSE-PALLET-001",
    "event_type": "Enriched",
    "visibility": "Private",
    "metadata": {
      "sensor_id": "TEMP-SENSOR-42",
      "temperature_c": 18.5,
      "humidity_percent": 65,
      "timestamp": "2025-01-21T14:30:00Z"
    }
  }'
```

---

## 📖 Additional Resources

### Interactive Documentation
- **Swagger UI**: https://connect.defarm.net/docs (coming soon)
- **OpenAPI Spec**: `docs/api/openapi.yaml`

### SDKs
- **TypeScript SDK**: `npm install @defarm/sdk` (coming soon)
- **Python SDK**: `pip install defarm-sdk` (coming soon)

### Tools
- **CLI Tool**: `npm install -g defarm-cli` (coming soon)
- **Postman Collection**: `docs/api/defarm-api-collection.json`

### Support
- **Documentation**: https://connect.defarm.net/docs
- **API Status**: https://status.defarm.net
- **Support Email**: support@defarm.net

---

**Last Updated**: 2025-01-24
**API Version**: v1.0
**Production URL**: https://connect.defarm.net

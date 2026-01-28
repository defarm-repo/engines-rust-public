# DeFarm Engines - Advanced Concepts Guide

**Complete guide to understanding DeFarm's core architecture, deduplication, blockchain storage, and advanced features.**

---

## 🌐 Language | Idioma

- [🇺🇸 English](#english-version)
- [🇧🇷 Português](#versão-em-português)

---

# English Version

## 📋 Table of Contents

1. [Identifiers & Deduplication Strategy](#identifiers--deduplication-strategy)
2. [Namespace System](#namespace-system)
3. [Complete Tokenization Flow](#complete-tokenization-flow)
4. [Blockchain Storage & Adapters](#blockchain-storage--adapters)
5. [Events System Deep Dive](#events-system-deep-dive)
6. [Item Lifecycle Management](#item-lifecycle-management)
7. [Circuit Roles & Permissions](#circuit-roles--permissions)
8. [Circuit Alias Configuration](#circuit-alias-configuration)
9. [Post-Action Webhooks](#post-action-webhooks)
10. [External Aliases System](#external-aliases-system)

---

## Identifiers & Deduplication Strategy

### Overview

DeFarm uses a sophisticated identifier system to prevent duplicate records across the entire ecosystem. Understanding how identifiers work is **critical** for proper data management.

### Identifier Types

#### 1. Canonical Identifiers

**Definition:** Globally unique identifiers within their registry system.

**Characteristics:**
- ✅ Globally unique (one entity per identifier worldwide)
- ✅ Enable cross-user deduplication without fingerprints
- ✅ Format validated by system against registry rules
- ✅ Examples: SISBOV (cattle), CPF (person), CAR (rural property), RFID tags

**Example:**
```json
{
  "namespace": "bovino",
  "key": "sisbov",
  "value": "BR12345678901234",
  "id_type": "Canonical",
  "verified": false
}
```

**Use Cases:**
- Government-issued IDs (CPF, CAR, RENASEM)
- Industry standards (SISBOV for cattle)
- RFID/NFC tags with unique serial numbers
- Blockchain addresses

#### 2. Contextual Identifiers

**Definition:** Identifiers that are only unique within a specific user/organization context.

**Characteristics:**
- ⚠️ Locally unique within organization only
- ⚠️ Require fingerprint for cross-user deduplication
- ⚠️ Examples: Batch numbers, farm IDs, internal SKUs

**Example:**
```json
{
  "namespace": "soja",
  "key": "lote",
  "value": "123",
  "id_type": "Contextual",
  "verified": false
}
```

**Use Cases:**
- Batch/lot numbers (Lote 123, Safra 2024/25)
- Internal farm IDs (Granja G5, Talhão A3)
- Local product codes (SKU-ABC-123)
- Temporary identifiers

### Deduplication Strategy

When you push an item to a circuit, DeFarm follows this priority sequence to determine if the item already exists:

```
┌─────────────────────────────────────────┐
│   Item Pushed to Circuit                │
└──────────────┬──────────────────────────┘
               ↓
┌──────────────────────────────────────────┐
│ Priority 1: Check Canonical Identifiers  │
│ (SISBOV, CPF, CAR, RFID, etc.)          │
└──────────────┬───────────────────────────┘
               ↓
         Match Found? ─── YES → Enrich existing item
               │
               NO
               ↓
┌──────────────────────────────────────────┐
│ Priority 2: Check Fingerprint            │
│ (if circuit has use_fingerprint=true)    │
└──────────────┬───────────────────────────┘
               ↓
         Match Found? ─── YES → Enrich existing item
               │
               NO
               ↓
┌──────────────────────────────────────────┐
│ Priority 3: Create New DFID              │
│ This is a new unique entity              │
└──────────────────────────────────────────┘
```

### Fingerprint Generation

**When is a fingerprint used?**
- When you have only contextual identifiers (no canonical IDs)
- When the circuit has `use_fingerprint: true` configured

**How is it generated?**
```
fingerprint = BLAKE3(
  user_id +
  local_id +
  timestamp_nanoseconds +
  sorted_identifiers
)
```

**Key Properties:**
- Scoped per circuit (prevents cross-contamination)
- Includes timestamp (prevents collisions)
- Deterministic within same user and identifier set
- Different users with "Lote 123" get different fingerprints

**Example Scenario:**

```bash
# Farm A creates item with contextual ID
Farm A: "Lote 123" → Fingerprint A → DFID-001

# Farm B creates item with same contextual ID
Farm B: "Lote 123" → Fingerprint B → DFID-002

# Different DFIDs because different farms (correct behavior!)
```

### Best Practices

| Scenario | Recommended Identifier | Reason |
|----------|----------------------|---------|
| Cattle tracking | SISBOV (Canonical) | Government registry, globally unique |
| Person identity | CPF (Canonical) | Government registry, prevents duplicates |
| Batch of crops | Batch number (Contextual) + Farm CPF | Batch unique within farm |
| RFID-tagged asset | RFID serial (Canonical) | Globally unique hardware ID |
| Internal product | SKU (Contextual) + Company CNPJ | SKU unique within company |

---

## Namespace System

### Why Namespaces?

Namespaces prevent identifier collisions across different value chains and industries.

**Problem without namespaces:**
```
Farm A: lote:123 (soybeans)
Farm B: lote:123 (cattle feed)
→ Collision! Are these the same entity?
```

**Solution with namespaces:**
```
Farm A: soja:lote:123 (soybeans)
Farm B: aves:lote:123 (poultry)
→ No collision! Clearly different entities.
```

### Standard Namespaces

| Namespace | Industry | Example Use Cases |
|-----------|----------|-------------------|
| `bovino` | Cattle | SISBOV, ear tags, cattle tracking |
| `aves` | Poultry | Granja IDs, chicken batch numbers |
| `suino` | Swine | Farm IDs, pig tracking |
| `soja` | Soybean | Batch numbers, harvest tracking |
| `milho` | Corn | Lot numbers, grain tracking |
| `algodao` | Cotton | Bale numbers, cotton tracking |
| `cafe` | Coffee | Batch IDs, coffee tracking |
| `leite` | Dairy | Tank numbers, milk tracking |
| `generic` | Multi-purpose | General use cases |

### Namespace Format

**Full identifier format:**
```
namespace:key:value
```

**Examples:**
```
bovino:sisbov:BR12345678901234
pessoa:cpf:12345678901
soja:lote:FARM-2024-001
cafe:batch:ORGANIC-BATCH-42
```

### Circuit-Level Namespace Configuration

Circuits can specify a `default_namespace` that is automatically applied to identifiers:

```json
{
  "circuit_id": "uuid",
  "name": "Coffee Supply Chain",
  "adapter_config": {
    "default_namespace": "cafe",
    "auto_apply_namespace": true
  }
}
```

**Behavior with auto_apply_namespace:**

```json
// User submits identifier without namespace
{
  "key": "batch",
  "value": "ORGANIC-001"
}

// System automatically applies circuit namespace
{
  "namespace": "cafe",
  "key": "batch",
  "value": "ORGANIC-001"
}
// Final format: cafe:batch:ORGANIC-001
```

### Custom Namespaces

For specialized industries not covered by standard namespaces, you can use custom namespaces:

```
floricultura:lote:ROSES-2024-001
aquicultura:tanque:TANK-5-TILAPIA
silvicultura:talhao:EUCALYPTUS-A3
```

**Best Practice:** Use descriptive, industry-standard Portuguese terms for custom namespaces.

---

## Complete Tokenization Flow

### Overview

Tokenization is the process of converting a local item (with LID) into a globally recognized item (with DFID) by pushing it to a circuit.

### Step-by-Step Flow

```
┌─────────────────────────────────────────────────────┐
│  1. CREATE LOCAL ITEM (Workspace-Private)           │
│     POST /api/items/local                           │
│     → Returns LID (UUID)                            │
│     → Item stored as "LID-{uuid}"                   │
└────────────────────┬────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────────┐
│  2. PUSH TO CIRCUIT (Tokenization)                  │
│     POST /api/circuits/{id}/push-local              │
│     → Send LID + identifiers                        │
└────────────────────┬────────────────────────────────┘
                     ↓
         ┌───────────────────────┐
         │  Circuit Validation   │
         ├───────────────────────┤
         │ • Check namespaces    │
         │ • Validate IDs format │
         │ • Check permissions   │
         │ • Verify adapter      │
         └───────────┬───────────┘
                     ↓
         ┌───────────────────────┐
         │  Deduplication Check  │
         ├───────────────────────┤
         │ Priority 1: Canonical │
         │ Priority 2: Fingerprint│
         │ Priority 3: New DFID  │
         └───────────┬───────────┘
                     ↓
                Match Found?
                /          \
              YES           NO
               ↓             ↓
    ┌──────────────┐  ┌──────────────┐
    │ Enrich Item  │  │ Create New   │
    │ (Same DFID)  │  │ (New DFID)   │
    └──────┬───────┘  └──────┬───────┘
           │                  │
           └────────┬─────────┘
                    ↓
         ┌───────────────────────┐
         │  Blockchain Storage   │
         ├───────────────────────┤
         │ 1. Store in IPFS      │
         │    → Get CID          │
         │ 2. Submit to Stellar  │
         │    → Get TX Hash      │
         └───────────┬───────────┘
                     ↓
         ┌───────────────────────┐
         │  Store LID→DFID Map   │
         │  Create Events        │
         │  Send Notifications   │
         └───────────┬───────────┘
                     ↓
┌─────────────────────────────────────────────────────┐
│  3. RETURN RESULT                                   │
│     → DFID                                          │
│     → CID (IPFS hash)                               │
│     → Transaction Hash (Stellar)                    │
│     → Storage metadata                              │
└─────────────────────────────────────────────────────┘
```

### Practical Example

```bash
TOKEN="your_jwt_token"
BASE_URL="https://connect.defarm.net"

# Step 1: Create local item
echo "Creating local item..."
LOCAL_RESPONSE=$(curl -s -X POST "$BASE_URL/api/items/local" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "identifiers": [
      {
        "namespace": "bovino",
        "key": "sisbov",
        "value": "BR98765432109876",
        "id_type": "Canonical",
        "verified": false
      }
    ],
    "enriched_data": {
      "breed": "Nelore",
      "birth_date": "2024-03-15",
      "weight_kg": 180
    }
  }')

LOCAL_ID=$(echo "$LOCAL_RESPONSE" | jq -r '.data.local_id')
echo "✓ Local item created: $LOCAL_ID"
echo "  (Item is workspace-private at this stage)"

# Step 2: Push to circuit for tokenization
echo ""
echo "Pushing to circuit for tokenization..."
PUSH_RESPONSE=$(curl -s -X POST "$BASE_URL/api/circuits/your-circuit-id/push-local" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"local_id\": \"$LOCAL_ID\"}")

DFID=$(echo "$PUSH_RESPONSE" | jq -r '.data.dfid')
CID=$(echo "$PUSH_RESPONSE" | jq -r '.data.storage.cid')
TX_HASH=$(echo "$PUSH_RESPONSE" | jq -r '.data.storage.transaction_hash')

echo "✓ Tokenization complete!"
echo "  DFID: $DFID"
echo "  IPFS CID: $CID"
echo "  Stellar TX: $TX_HASH"

# Step 3: Query by either LID or DFID
echo ""
echo "Querying item..."
curl -s "$BASE_URL/api/items/$DFID" \
  -H "Authorization: Bearer $TOKEN" | jq '.data | {dfid, status, identifiers}'

# Step 4: Check LID-DFID mapping
echo ""
echo "Checking LID mapping..."
curl -s "$BASE_URL/api/items/mapping/$LOCAL_ID" \
  -H "Authorization: Bearer $TOKEN" | jq '.'
```

### What Happens During Tokenization?

| Step | Description | Time |
|------|-------------|------|
| Validation | Check identifiers, namespaces, permissions | ~50ms |
| Deduplication | Search for existing items by canonical ID or fingerprint | ~100ms |
| DFID Assignment | Generate new DFID or use existing | ~10ms |
| IPFS Storage | Upload data to IPFS, get CID | ~500ms |
| Stellar Submit | Submit CID to blockchain, get TX hash | ~5000ms |
| Event Creation | Create ItemTokenized and ItemPushed events | ~50ms |
| **Total** | **Complete tokenization** | **~5.7s** |

**Note:** Times are approximate. IPFS and Stellar operations are the slowest parts.

---

## Blockchain Storage & Adapters

### Overview

DeFarm uses a hybrid storage approach: **IPFS for data** + **Stellar blockchain for immutable references**.

### Storage Architecture

```
┌─────────────────────────────────────────────────────┐
│  Item Data                                          │
│  (identifiers, enriched_data, events)               │
└────────────────────┬────────────────────────────────┘
                     ↓
         ┌───────────────────────┐
         │   Serialize to JSON   │
         └───────────┬───────────┘
                     ↓
         ┌───────────────────────┐
         │   Upload to IPFS      │
         │   → Get CID           │
         │   (Content Hash)      │
         └───────────┬───────────┘
                     ↓
         ┌───────────────────────┐
         │   Submit to Stellar   │
         │   IPCM Contract       │
         │   store(CID, metadata)│
         └───────────┬───────────┘
                     ↓
         ┌───────────────────────┐
         │   Get TX Hash         │
         │   (Blockchain Proof)  │
         └───────────────────────┘
```

### Adapter Types

#### 1. StellarTestnetIpfs (Development)

**Use for:** Development, testing, demos

**Characteristics:**
- ✅ Free to use (testnet XLM)
- ✅ Full blockchain features
- ✅ IPFS storage included
- ⚠️ Testnet data not permanent
- ⚠️ Not for production

**Example:**
```json
{
  "adapter_type": "StellarTestnetIpfs",
  "requires_approval": false,
  "sponsor_adapter_access": true
}
```

#### 2. StellarMainnetIpfs (Production)

**Use for:** Production applications

**Characteristics:**
- ✅ Production-grade reliability
- ✅ Permanent blockchain records
- ✅ IPFS storage included
- 💰 Requires XLM for fees
- 🔒 Enterprise SLA available

**Example:**
```json
{
  "adapter_type": "StellarMainnetIpfs",
  "requires_approval": true,
  "sponsor_adapter_access": false
}
```

#### 3. IpfsOnly (No Blockchain)

**Use for:** IPFS-only storage without blockchain

**Characteristics:**
- ✅ Lower cost (no blockchain fees)
- ✅ Fast storage (~500ms)
- ❌ No blockchain proof
- ❌ No immutable timestamp

**Example:**
```json
{
  "adapter_type": "IpfsOnly",
  "requires_approval": false
}
```

#### 4. LocalStorage (Development Only)

**Use for:** Local development without external services

**Characteristics:**
- ✅ Zero external dependencies
- ✅ Instant storage (<10ms)
- ❌ No persistence across restarts
- ❌ Not for production

### Circuit Adapter Configuration

#### Setting Up a Circuit with Blockchain

```bash
TOKEN="your_jwt_token"

# Create circuit with Stellar + IPFS
curl -X POST "https://connect.defarm.net/api/circuits" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Organic Coffee Supply Chain",
    "description": "Track coffee from farm to cup",
    "adapter_config": {
      "adapter_type": "StellarTestnetIpfs",
      "requires_approval": false,
      "auto_migrate_existing": false,
      "sponsor_adapter_access": true
    },
    "visibility": "Public"
  }'
```

#### Adapter Sponsorship

**What is adapter sponsorship?**

When a circuit has `sponsor_adapter_access: true`, the circuit **sponsors blockchain access** for all members. This means:

| Without Sponsorship | With Sponsorship |
|---------------------|------------------|
| Users need Premium/Enterprise tier | All members can use blockchain |
| Each user pays blockchain fees | Circuit owner pays fees |
| Members blocked if tier insufficient | All members can push freely |

**Use Cases:**
- 🏢 **Enterprise circuits** - Company sponsors access for suppliers
- 🎓 **Educational circuits** - School sponsors access for students
- 🔬 **Research circuits** - Project sponsors access for researchers

**Example: Sponsored Circuit**
```json
{
  "circuit_name": "Enterprise Supply Chain",
  "adapter_config": {
    "adapter_type": "StellarMainnetIpfs",
    "sponsor_adapter_access": true
  }
}
```

Result: Basic tier users can push items to this circuit despite not having blockchain access in their tier.

### Retrieving Circuit Adapter Configuration

```bash
# Get adapter config (authenticated)
curl "https://connect.defarm.net/api/circuits/{circuit_id}/adapter" \
  -H "Authorization: Bearer $TOKEN"

# Response
{
  "adapter_type": "StellarTestnetIpfs",
  "sponsor_adapter_access": true,
  "requires_approval": false
}
```

### Storage Response Fields

When you push an item to a circuit with blockchain enabled, you receive:

```json
{
  "dfid": "DFID-20250128-000042-7A3F",
  "storage": {
    "adapter_type": "StellarTestnetIpfs",
    "location": "ipfs",
    "hash": "QmT5NvUtoM5nW...",
    "cid": "QmT5NvUtoM5nW...",
    "transaction_hash": "a3f7c2e1b9d8...",
    "stellar_network": "testnet",
    "contract_id": "CAIFDKP...",
    "metadata": {
      "stored_at": "2025-01-28T14:30:00Z",
      "size_bytes": 1024
    }
  }
}
```

**Fields explained:**
- `cid` - IPFS Content Identifier (use to retrieve from IPFS)
- `transaction_hash` - Stellar transaction ID (verify on blockchain explorer)
- `stellar_network` - "testnet" or "mainnet"
- `contract_id` - IPCM smart contract address

### Verifying Storage on Blockchain

```bash
# View transaction on Stellar Explorer
# Testnet: https://stellar.expert/explorer/testnet/tx/{transaction_hash}
# Mainnet: https://stellar.expert/explorer/public/tx/{transaction_hash}

# Retrieve from IPFS
curl "https://ipfs.io/ipfs/{cid}"

# Or using local IPFS node
ipfs cat {cid}
```

---

## Events System Deep Dive

### Overview

Events provide a complete, immutable audit trail of everything that happens to an item throughout its lifecycle.

### Event Visibility Levels

#### 1. Public Events

**Characteristics:**
- ✅ Visible to everyone (even unauthenticated users)
- ✅ Included in public API endpoints
- ✅ Used for transparency and public audit trails
- ✅ Example: Organic certification timestamp

**Use Cases:**
- Product certifications
- Public milestones (harvest date, processing date)
- Transparency events for consumers

**Example:**
```json
{
  "dfid": "DFID-...",
  "event_type": "Enriched",
  "visibility": "Public",
  "metadata": {
    "action": "organic_certification",
    "certifier": "ECOCERT",
    "certificate_id": "BR-2024-12345"
  }
}
```

#### 2. Private Events (Encrypted)

**Characteristics:**
- 🔒 Encrypted at rest
- 🔒 Only visible to item owner and authorized circuit members
- 🔒 Used for sensitive business data
- 🔒 Example: Cost information, supplier contracts

**Use Cases:**
- Pricing information
- Supplier agreements
- Internal quality issues
- Confidential business data

**Example:**
```json
{
  "dfid": "DFID-...",
  "event_type": "Enriched",
  "visibility": "Private",
  "metadata": {
    "purchase_price_usd": 450.00,
    "supplier": "Farm XYZ",
    "contract_id": "CNT-2024-789"
  }
}
```

#### 3. Circuit-Only Events

**Characteristics:**
- 👥 Visible only to circuit members
- 👥 Not included in public endpoints
- 👥 Shared within trusted group
- 👥 Example: Internal processing notes

**Use Cases:**
- Internal circuit operations
- Shared processing notes
- Circuit-specific metadata
- Collaborative tracking

### Event Deduplication

**Problem:** Multiple systems might report the same event multiple times.

**Solution:** Content-hash based deduplication.

**How it works:**

```
Content Hash = BLAKE3(dfid + event_type + source + metadata)
(Timestamp excluded to enable proper deduplication)
```

**Example Scenario:**

```bash
# System A reports event
POST /api/events
{
  "dfid": "DFID-001",
  "event_type": "Enriched",
  "metadata": {"action": "harvested"}
}
→ Creates event with ID abc-123

# System B reports same event (duplicate)
POST /api/events
{
  "dfid": "DFID-001",
  "event_type": "Enriched",
  "metadata": {"action": "harvested"}
}
→ Returns existing event abc-123
→ Response includes: was_deduplicated: true
```

### Local Events

**What are local events?**

Events created before the item has a DFID (still local-only).

**Use Cases:**
- Offline data collection
- Pre-tokenization tracking
- Workspace-private logging

**Workflow:**

```bash
# 1. Create local event (no DFID yet)
POST /api/events/local
{
  "event_type": "Created",
  "metadata": {"action": "item_prepared"}
}
→ Returns local_event_id: "uuid-456"
→ Stored as "LOCAL-EVENT-{uuid}"

# 2. Later: Push to circuit with events
POST /api/circuits/{id}/push-events
{
  "local_event_ids": ["uuid-456"]
}
→ Event gets real DFID
→ "LOCAL-EVENT-uuid" → "DFID-20250128-..."
```

### Event Auto-Merge

When duplicate events are detected during push, metadata is **non-destructively merged**:

**Original Event:**
```json
{
  "metadata": {
    "action": "processed",
    "location": "Factory A"
  }
}
```

**Duplicate Push with New Metadata:**
```json
{
  "metadata": {
    "quality_score": 95,
    "inspector": "John Doe"
  }
}
```

**Merged Result:**
```json
{
  "metadata": {
    "action": "processed",
    "location": "Factory A",
    "quality_score": 95,
    "inspector": "John Doe"
  }
}
```

**Note:** Only new keys are added. Existing keys are never overwritten.

### Event Security

**Source Attribution:**

Every event includes a `source` field that is **automatically populated** from authentication context:

- JWT Token → `source = user_id from token`
- API Key → `source = user_id from API key`
- **User cannot override this field**

This prevents audit trail tampering by malicious actors.

**Example:**
```json
{
  "event_id": "evt-123",
  "dfid": "DFID-001",
  "event_type": "Enriched",
  "source": "user-abc-def",  // ← Auto-populated, tamper-proof
  "timestamp": "2025-01-28T14:30:00Z",
  "metadata": {...}
}
```

---

## Item Lifecycle Management

### Item States

```
┌──────────┐
│  ACTIVE  │ ← Initial state (normal operations)
└────┬─────┘
     │
     ├─────→ ┌──────────┐
     │       │  MERGED  │ ← Duplicates consolidated
     │       └──────────┘
     │
     ├─────→ ┌──────────┐
     │       │  SPLIT   │ ← Separated into multiple items
     │       └──────────┘
     │
     └─────→ ┌──────────────┐
             │ DEPRECATED   │ ← No longer valid
             └──────────────┘
```

### 1. Active State

**Description:** Normal operating state for items.

**Operations Allowed:**
- ✅ Create events
- ✅ Enrich with data
- ✅ Push to circuits
- ✅ Pull from circuits

### 2. Merged State

**When:** Two or more items determined to be duplicates of the same real-world entity.

**What Happens:**
```
Item A (DFID-001) + Item B (DFID-002)
          ↓
     Merge Operation
          ↓
Item A (DFID-001) = ACTIVE (primary)
Item B (DFID-002) = MERGED → points to DFID-001
```

**Example API Call:**
```bash
POST /api/items/merge
{
  "primary_dfid": "DFID-001",
  "secondary_dfid": "DFID-002",
  "reason": "Same SISBOV detected after manual review"
}
```

**Result:**
- Item DFID-001 remains Active
- Item DFID-002 marked as Merged
- All events from DFID-002 now associated with DFID-001
- Queries for DFID-002 automatically redirect to DFID-001

### 3. Split State

**When:** One item determined to actually represent multiple separate entities.

**What Happens:**
```
Item A (DFID-001)
     ↓
Split Operation
     ↓
Item A (DFID-001) = SPLIT (archived)
Item B (DFID-003) = ACTIVE (new entity 1)
Item C (DFID-004) = ACTIVE (new entity 2)
```

**Example Scenario:**

A batch originally thought to be uniform is discovered to contain products from two different sources.

### 4. Deprecated State

**When:** Item no longer valid but must be kept for audit trail.

**What Happens:**
- Item marked as Deprecated
- Still queryable for historical analysis
- Cannot create new events
- Cannot push to new circuits

**Use Cases:**
- Recalled products
- Expired certifications
- Canceled batches

---

## Circuit Roles & Permissions

### Complete Permission Matrix

| Permission | Owner | Admin | Member | Viewer |
|------------|-------|-------|--------|--------|
| **Circuit Management** |
| Create circuit | ✅ | ❌ | ❌ | ❌ |
| Edit circuit settings | ✅ | ❌ | ❌ | ❌ |
| Delete circuit | ✅ | ❌ | ❌ | ❌ |
| Deactivate circuit | ✅ | ❌ | ❌ | ❌ |
| **Member Management** |
| Add members | ✅ | ✅ | ❌ | ❌ |
| Remove members | ✅ | ✅ | ❌ | ❌ |
| Change member roles | ✅ | ✅ | ❌ | ❌ |
| View members | ✅ | ✅ | ✅ | ✅ |
| **Adapter Configuration** |
| Configure adapter | ✅ | ✅ | ❌ | ❌ |
| Enable/disable sponsorship | ✅ | ✅ | ❌ | ❌ |
| View adapter config | ✅ | ✅ | ✅ | ✅ |
| **Item Operations** |
| Push items | ✅ | ✅ | ✅* | ❌ |
| Pull items | ✅ | ✅ | ✅* | ❌ |
| View items | ✅ | ✅ | ✅ | ✅ |
| Approve operations | ✅ | ✅ | ❌ | ❌ |
| **Webhooks** |
| Create webhooks | ✅ | ✅ | ❌ | ❌ |
| Edit webhooks | ✅ | ✅ | ❌ | ❌ |
| Delete webhooks | ✅ | ✅ | ❌ | ❌ |
| View webhook history | ✅ | ✅ | ❌ | ❌ |

**\*Note:** Member push/pull requires `allow_member_push`/`allow_member_pull` circuit settings enabled.

### Role-Based API Examples

#### Owner Operations

```bash
# Create circuit
POST /api/circuits
{
  "name": "Supply Chain",
  "visibility": "Private"
}

# Configure adapter
PUT /api/circuits/{id}/adapter
{
  "adapter_type": "StellarTestnetIpfs",
  "sponsor_adapter_access": true
}

# Add member
POST /api/circuits/{id}/members
{
  "user_id": "user-123",
  "role": "Member"
}
```

#### Admin Operations

```bash
# Add member (same as owner)
POST /api/circuits/{id}/members

# Approve push operation
POST /api/circuits/{id}/operations/{op_id}/approve

# Configure webhooks
POST /api/circuits/{id}/post-actions/webhooks
{
  "url": "https://example.com/webhook",
  "events": ["ItemPushed", "ItemTokenized"]
}
```

#### Member Operations

```bash
# Push item (if allowed)
POST /api/circuits/{id}/push-local
{
  "local_id": "lid-123"
}

# Pull item (if allowed)
POST /api/circuits/{id}/pull
{
  "dfid": "DFID-001"
}

# View circuit items
GET /api/circuits/{id}/items
```

#### Viewer Operations

```bash
# View items (read-only)
GET /api/circuits/{id}/items

# View members
GET /api/circuits/{id}/members

# View circuit details
GET /api/circuits/{id}
```

---

## Circuit Alias Configuration

### What are Circuit Aliases?

Circuit aliases are **data requirements** that circuits can enforce to ensure data quality and consistency.

### Configurable Requirements

```json
{
  "adapter_config": {
    "required_canonical_identifiers": ["sisbov", "cpf"],
    "required_contextual_identifiers": ["lote", "safra"],
    "allowed_namespaces": ["bovino", "pessoa"],
    "default_namespace": "bovino",
    "auto_apply_namespace": true,
    "use_fingerprint": true
  }
}
```

### Required Canonical Identifiers

**Purpose:** Enforce that items must have specific globally unique IDs.

**Example:**
```json
{
  "required_canonical_identifiers": ["sisbov"]
}
```

**Behavior:**
```bash
# This push will SUCCEED
POST /api/circuits/{id}/push-local
{
  "identifiers": [
    {
      "namespace": "bovino",
      "key": "sisbov",
      "value": "BR12345678901234",
      "id_type": "Canonical"
    }
  ]
}

# This push will FAIL (missing required SISBOV)
POST /api/circuits/{id}/push-local
{
  "identifiers": [
    {
      "namespace": "bovino",
      "key": "ear_tag",
      "value": "123",
      "id_type": "Contextual"
    }
  ]
}
→ Error: "Missing required canonical identifier: sisbov"
```

### Required Contextual Identifiers

**Purpose:** Enforce additional context identifiers.

**Example:**
```json
{
  "required_contextual_identifiers": ["lote", "safra"]
}
```

**Use Case:** Soybean supply chain requiring batch number and harvest season.

### Allowed Namespaces

**Purpose:** Restrict which value chains can use the circuit.

**Example:**
```json
{
  "allowed_namespaces": ["bovino", "pessoa"]
}
```

**Behavior:**
- ✅ Items with `bovino:*` identifiers → Accepted
- ✅ Items with `pessoa:*` identifiers → Accepted
- ❌ Items with `soja:*` identifiers → Rejected

### Auto-Apply Namespace

**Purpose:** Automatically add circuit's default namespace to identifiers.

**Example:**
```json
{
  "default_namespace": "cafe",
  "auto_apply_namespace": true
}
```

**Behavior:**

```json
// User submits
{
  "key": "batch",
  "value": "ORGANIC-001"
}

// System stores
{
  "namespace": "cafe",  // ← Auto-applied
  "key": "batch",
  "value": "ORGANIC-001"
}
```

### Use Fingerprint

**Purpose:** Enable fingerprint-based deduplication for contextual identifiers.

**When to enable:**
- Circuit accepts items without canonical IDs
- Need cross-user deduplication for contextual IDs
- Want to prevent duplicate entries from different users

**When to disable:**
- Circuit requires canonical IDs only
- Want each user to maintain separate instances
- Local-only tracking (no cross-user dedup)

---

## Post-Action Webhooks

### Overview

Webhooks allow circuits to notify external systems when item operations complete.

### Webhook Events

| Event Type | Triggered When | Payload Includes |
|------------|----------------|------------------|
| `ItemPushed` | Item pushed to circuit | DFID, identifiers, storage |
| `ItemApproved` | Push operation approved by admin | DFID, approver, timestamp |
| `ItemTokenized` | LID converted to DFID | DFID, LID, mapping |
| `ItemPublished` | Item made public in circuit | DFID, visibility level |

### Creating a Webhook

```bash
POST /api/circuits/{circuit_id}/post-actions/webhooks
Authorization: Bearer {token}
Content-Type: application/json

{
  "url": "https://your-system.com/webhook/defarm",
  "events": ["ItemPushed", "ItemTokenized"],
  "auth_type": "BearerToken",
  "auth_config": {
    "token": "your-secret-token"
  },
  "include_storage_details": true,
  "include_item_metadata": true,
  "enabled": true,
  "retry_config": {
    "max_retries": 3,
    "initial_delay_ms": 1000,
    "max_delay_ms": 30000,
    "backoff_multiplier": 2.0
  }
}
```

### Webhook Payload Example

```json
{
  "event_type": "ItemPushed",
  "circuit_id": "circuit-uuid",
  "circuit_name": "Coffee Supply Chain",
  "timestamp": "2025-01-28T14:30:00Z",
  "item": {
    "dfid": "DFID-20250128-000042-7A3F",
    "local_id": "lid-abc-123",
    "identifiers": [
      {
        "namespace": "cafe",
        "key": "batch",
        "value": "ORGANIC-2025-001"
      }
    ],
    "pushed_by": "user-xyz"
  },
  "storage": {
    "adapter_type": "StellarTestnetIpfs",
    "location": "ipfs",
    "cid": "QmT5NvUtoM5nW...",
    "transaction_hash": "a3f7c2e1b9d8...",
    "metadata": {
      "stored_at": "2025-01-28T14:30:00Z",
      "size_bytes": 1024
    }
  },
  "operation_id": "op-456",
  "status": "Completed"
}
```

### Authentication Methods

#### 1. Bearer Token
```json
{
  "auth_type": "BearerToken",
  "auth_config": {
    "token": "your-secret-token"
  }
}
```

HTTP Header: `Authorization: Bearer your-secret-token`

#### 2. API Key
```json
{
  "auth_type": "ApiKey",
  "auth_config": {
    "header_name": "X-API-Key",
    "api_key": "your-api-key"
  }
}
```

HTTP Header: `X-API-Key: your-api-key`

#### 3. Basic Auth
```json
{
  "auth_type": "BasicAuth",
  "auth_config": {
    "username": "user",
    "password": "pass"
  }
}
```

HTTP Header: `Authorization: Basic base64(user:pass)`

#### 4. Custom Header
```json
{
  "auth_type": "CustomHeader",
  "auth_config": {
    "header_name": "X-Custom-Auth",
    "header_value": "secret-value"
  }
}
```

HTTP Header: `X-Custom-Auth: secret-value`

### Retry Logic

Webhooks automatically retry on failure with exponential backoff:

```
Attempt 1: Send immediately → Failed
Wait 1s
Attempt 2: Send → Failed
Wait 2s (1s × 2.0 multiplier)
Attempt 3: Send → Failed
Wait 4s (2s × 2.0 multiplier)
Attempt 4: Send → Success! ✓

Max retries reached → Give up, log failure
```

### Testing Webhooks

```bash
POST /api/circuits/{circuit_id}/post-actions/webhooks/{webhook_id}/test
Authorization: Bearer {token}

# Sends test payload to webhook URL
# Returns success/failure result immediately
```

### Viewing Delivery History

```bash
GET /api/circuits/{circuit_id}/post-actions/deliveries
Authorization: Bearer {token}

# Response
{
  "deliveries": [
    {
      "delivery_id": "del-123",
      "webhook_id": "wh-456",
      "event_type": "ItemPushed",
      "status": "Success",
      "attempts": 1,
      "response_code": 200,
      "response_body": "OK",
      "delivered_at": "2025-01-28T14:30:05Z"
    },
    {
      "delivery_id": "del-124",
      "webhook_id": "wh-456",
      "event_type": "ItemTokenized",
      "status": "Failed",
      "attempts": 3,
      "response_code": 500,
      "error_message": "Internal Server Error",
      "last_attempt_at": "2025-01-28T14:31:00Z"
    }
  ]
}
```

---

## External Aliases System

### Overview

External aliases allow items to track identifiers from multiple external systems (certifiers, ERPs, government registries).

### Use Cases

1. **Multi-Certifier Tracking**
   - Organic certification from ECOCERT
   - Fair Trade certification from FLO-CERT
   - Rainforest Alliance certification

2. **ERP Integration**
   - SAP product code
   - Oracle inventory ID
   - Internal SKU

3. **Government Registries**
   - SISBOV (cattle registry)
   - RENASEM (seed registry)
   - CAR (rural property registry)

### Alias Structure

```json
{
  "scheme": "certification",
  "value": "ECOCERT-BR-2024-12345",
  "issuer": "ECOCERT Brazil",
  "issued_at": "2024-01-15T10:00:00Z",
  "expires_at": "2025-01-15T10:00:00Z",
  "evidence_hash": "blake3_hash_of_certificate_pdf",
  "metadata": {
    "certification_type": "Organic",
    "scope": "Coffee Production",
    "auditor": "John Smith"
  }
}
```

### Adding External Aliases

```bash
POST /api/items/{dfid}/aliases
Authorization: Bearer {token}
Content-Type: application/json

{
  "scheme": "erp",
  "value": "SAP-12345",
  "issuer": "Company ERP System",
  "evidence_hash": "blake3_hash_of_erp_export",
  "metadata": {
    "system": "SAP S/4HANA",
    "created_by": "integration_bot"
  }
}
```

### Querying by External Alias

```bash
GET /api/items/by-alias?scheme=certification&value=ECOCERT-BR-2024-12345
Authorization: Bearer {token}

# Returns item(s) with matching alias
{
  "items": [
    {
      "dfid": "DFID-001",
      "aliases": [
        {
          "scheme": "certification",
          "value": "ECOCERT-BR-2024-12345",
          "issuer": "ECOCERT Brazil"
        }
      ]
    }
  ]
}
```

### Conflict Detection

**Scenario:** Two different issuers provide conflicting aliases for the same item.

```json
// Alias 1
{
  "scheme": "weight",
  "value": "450kg",
  "issuer": "Farm Scale System"
}

// Alias 2 (CONFLICT!)
{
  "scheme": "weight",
  "value": "455kg",
  "issuer": "Processing Plant Scale"
}
```

**System Response:**
- Both aliases are stored
- Conflict flagged with `conflict: true`
- Manual review required to resolve
- Audit trail shows both values with issuers

### Best Practices

1. **Use Evidence Hashes**
   - Always include `evidence_hash` for verifiability
   - Hash the source document (PDF, CSV, etc.)
   - Enables proof of origin

2. **Include Issuer Information**
   - Full issuer name
   - Issuer authority type (certifier, ERP, government)
   - Contact information in metadata

3. **Track Expiration**
   - Set `expires_at` for time-limited aliases
   - System can automatically flag expired aliases
   - Useful for certifications and permits

4. **Namespace Aliases by Scheme**
   - Use consistent scheme names: `certification`, `erp`, `government_registry`
   - Prevents confusion across different types

---

# Versão em Português

## 📋 Índice

1. [Identificadores & Estratégia de Deduplicação](#identificadores--estratégia-de-deduplicação)
2. [Sistema de Namespaces](#sistema-de-namespaces)
3. [Fluxo Completo de Tokenização](#fluxo-completo-de-tokenização)
4. [Armazenamento Blockchain & Adaptadores](#armazenamento-blockchain--adaptadores)
5. [Sistema de Eventos Aprofundado](#sistema-de-eventos-aprofundado)
6. [Gerenciamento do Ciclo de Vida de Itens](#gerenciamento-do-ciclo-de-vida-de-itens)
7. [Funções e Permissões de Circuitos](#funções-e-permissões-de-circuitos)
8. [Configuração de Aliases de Circuitos](#configuração-de-aliases-de-circuitos)
9. [Webhooks de Pós-Ação](#webhooks-de-pós-ação)
10. [Sistema de Aliases Externos](#sistema-de-aliases-externos)

---

## Identificadores & Estratégia de Deduplicação

### Visão Geral

O DeFarm usa um sistema sofisticado de identificadores para prevenir registros duplicados em todo o ecossistema. Entender como os identificadores funcionam é **crítico** para o gerenciamento adequado dos dados.

### Tipos de Identificadores

#### 1. Identificadores Canônicos

**Definição:** Identificadores globalmente únicos dentro do seu sistema de registro.

**Características:**
- ✅ Globalmente únicos (uma entidade por identificador no mundo todo)
- ✅ Permitem deduplicação entre usuários sem fingerprints
- ✅ Formato validado pelo sistema contra regras de registro
- ✅ Exemplos: SISBOV (gado), CPF (pessoa), CAR (propriedade rural), tags RFID

**Exemplo:**
```json
{
  "namespace": "bovino",
  "key": "sisbov",
  "value": "BR12345678901234",
  "id_type": "Canonical",
  "verified": false
}
```

**Casos de Uso:**
- IDs emitidos pelo governo (CPF, CAR, RENASEM)
- Padrões da indústria (SISBOV para gado)
- Tags RFID/NFC com números de série únicos
- Endereços blockchain

#### 2. Identificadores Contextuais

**Definição:** Identificadores que são únicos apenas dentro de um contexto específico de usuário/organização.

**Características:**
- ⚠️ Localmente únicos apenas dentro da organização
- ⚠️ Requerem fingerprint para deduplicação entre usuários
- ⚠️ Exemplos: Números de lote, IDs de fazenda, SKUs internos

**Exemplo:**
```json
{
  "namespace": "soja",
  "key": "lote",
  "value": "123",
  "id_type": "Contextual",
  "verified": false
}
```

**Casos de Uso:**
- Números de lote/safra (Lote 123, Safra 2024/25)
- IDs internos de fazenda (Granja G5, Talhão A3)
- Códigos de produtos locais (SKU-ABC-123)
- Identificadores temporários

### Estratégia de Deduplicação

Quando você envia um item para um circuito, o DeFarm segue esta sequência de prioridade para determinar se o item já existe:

```
┌─────────────────────────────────────────┐
│   Item Enviado para Circuito            │
└──────────────┬──────────────────────────┘
               ↓
┌──────────────────────────────────────────┐
│ Prioridade 1: Verificar IDs Canônicos   │
│ (SISBOV, CPF, CAR, RFID, etc.)          │
└──────────────┬───────────────────────────┘
               ↓
         Encontrou? ─── SIM → Enriquecer item existente
               │
               NÃO
               ↓
┌──────────────────────────────────────────┐
│ Prioridade 2: Verificar Fingerprint     │
│ (se circuito tem use_fingerprint=true)  │
└──────────────┬───────────────────────────┘
               ↓
         Encontrou? ─── SIM → Enriquecer item existente
               │
               NÃO
               ↓
┌──────────────────────────────────────────┐
│ Prioridade 3: Criar Novo DFID           │
│ Esta é uma nova entidade única          │
└──────────────────────────────────────────┘
```

### Geração de Fingerprint

**Quando um fingerprint é usado?**
- Quando você tem apenas identificadores contextuais (sem IDs canônicos)
- Quando o circuito tem `use_fingerprint: true` configurado

**Como é gerado?**
```
fingerprint = BLAKE3(
  user_id +
  local_id +
  timestamp_nanoseconds +
  identificadores_ordenados
)
```

**Propriedades-Chave:**
- Escopo por circuito (previne contaminação cruzada)
- Inclui timestamp (previne colisões)
- Determinístico dentro do mesmo usuário e conjunto de identificadores
- Usuários diferentes com "Lote 123" obtêm fingerprints diferentes

**Cenário de Exemplo:**

```bash
# Fazenda A cria item com ID contextual
Fazenda A: "Lote 123" → Fingerprint A → DFID-001

# Fazenda B cria item com mesmo ID contextual
Fazenda B: "Lote 123" → Fingerprint B → DFID-002

# DFIDs diferentes porque são fazendas diferentes (comportamento correto!)
```

### Melhores Práticas

| Cenário | Identificador Recomendado | Motivo |
|---------|---------------------------|--------|
| Rastreamento de gado | SISBOV (Canônico) | Registro governamental, globalmente único |
| Identidade de pessoa | CPF (Canônico) | Registro governamental, previne duplicatas |
| Lote de culturas | Número de lote (Contextual) + CPF da Fazenda | Lote único dentro da fazenda |
| Ativo com RFID | Serial RFID (Canônico) | ID de hardware globalmente único |
| Produto interno | SKU (Contextual) + CNPJ da Empresa | SKU único dentro da empresa |

---

## Sistema de Namespaces

### Por Que Namespaces?

Namespaces previnem colisões de identificadores entre diferentes cadeias de valor e indústrias.

**Problema sem namespaces:**
```
Fazenda A: lote:123 (soja)
Fazenda B: lote:123 (ração bovina)
→ Colisão! São a mesma entidade?
```

**Solução com namespaces:**
```
Fazenda A: soja:lote:123 (soja)
Fazenda B: aves:lote:123 (aves)
→ Sem colisão! Claramente entidades diferentes.
```

### Namespaces Padrão

| Namespace | Indústria | Casos de Uso Exemplo |
|-----------|-----------|---------------------|
| `bovino` | Gado | SISBOV, brincos, rastreamento bovino |
| `aves` | Aves | IDs de granja, lotes de frango |
| `suino` | Suínos | IDs de fazenda, rastreamento de porcos |
| `soja` | Soja | Números de lote, rastreamento de safra |
| `milho` | Milho | Números de lote, rastreamento de grãos |
| `algodao` | Algodão | Números de fardo, rastreamento de algodão |
| `cafe` | Café | IDs de lote, rastreamento de café |
| `leite` | Laticínios | Números de tanque, rastreamento de leite |
| `generic` | Múltiplos | Casos de uso gerais |

### Formato de Namespace

**Formato completo do identificador:**
```
namespace:key:value
```

**Exemplos:**
```
bovino:sisbov:BR12345678901234
pessoa:cpf:12345678901
soja:lote:FAZENDA-2024-001
cafe:batch:ORGANICO-LOTE-42
```

### Configuração de Namespace em Nível de Circuito

Circuitos podem especificar um `default_namespace` que é automaticamente aplicado aos identificadores:

```json
{
  "circuit_id": "uuid",
  "name": "Cadeia de Café",
  "adapter_config": {
    "default_namespace": "cafe",
    "auto_apply_namespace": true
  }
}
```

**Comportamento com auto_apply_namespace:**

```json
// Usuário envia identificador sem namespace
{
  "key": "batch",
  "value": "ORGANICO-001"
}

// Sistema aplica automaticamente o namespace do circuito
{
  "namespace": "cafe",
  "key": "batch",
  "value": "ORGANICO-001"
}
// Formato final: cafe:batch:ORGANICO-001
```

### Namespaces Personalizados

Para indústrias especializadas não cobertas por namespaces padrão, você pode usar namespaces personalizados:

```
floricultura:lote:ROSAS-2024-001
aquicultura:tanque:TANQUE-5-TILAPIA
silvicultura:talhao:EUCALIPTO-A3
```

**Melhor Prática:** Use termos descritivos em português padrão da indústria para namespaces personalizados.

---

## Fluxo Completo de Tokenização

### Visão Geral

Tokenização é o processo de converter um item local (com LID) em um item reconhecido globalmente (com DFID) ao enviá-lo para um circuito.

### Fluxo Passo a Passo

```
┌─────────────────────────────────────────────────────┐
│  1. CRIAR ITEM LOCAL (Privado no Workspace)         │
│     POST /api/items/local                           │
│     → Retorna LID (UUID)                            │
│     → Item armazenado como "LID-{uuid}"            │
└────────────────────┬────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────────┐
│  2. ENVIAR PARA CIRCUITO (Tokenização)              │
│     POST /api/circuits/{id}/push-local              │
│     → Envia LID + identificadores                   │
└────────────────────┬────────────────────────────────┘
                     ↓
         ┌───────────────────────┐
         │  Validação Circuito   │
         ├───────────────────────┤
         │ • Verificar namespaces│
         │ • Validar formato IDs │
         │ • Verificar permissões│
         │ • Verificar adaptador │
         └───────────┬───────────┘
                     ↓
         ┌───────────────────────┐
         │  Verificação Dedup    │
         ├───────────────────────┤
         │ Prioridade 1: Canônico│
         │ Prioridade 2: Fingerpr│
         │ Prioridade 3: Novo DFID│
         └───────────┬───────────┘
                     ↓
                Encontrou?
                /          \
              SIM           NÃO
               ↓             ↓
    ┌──────────────┐  ┌──────────────┐
    │ Enriquecer   │  │ Criar Novo   │
    │ (Mesmo DFID) │  │ (Novo DFID)  │
    └──────┬───────┘  └──────┬───────┘
           │                  │
           └────────┬─────────┘
                    ↓
         ┌───────────────────────┐
         │  Armazenamento Blockch│
         ├───────────────────────┤
         │ 1. Armazenar no IPFS  │
         │    → Obter CID        │
         │ 2. Enviar para Stellar│
         │    → Obter TX Hash    │
         └───────────┬───────────┘
                     ↓
         ┌───────────────────────┐
         │  Armazenar Map LID→DFID│
         │  Criar Eventos        │
         │  Enviar Notificações  │
         └───────────┬───────────┘
                     ↓
┌─────────────────────────────────────────────────────┐
│  3. RETORNAR RESULTADO                              │
│     → DFID                                          │
│     → CID (hash IPFS)                               │
│     → Hash da Transação (Stellar)                   │
│     → Metadados de armazenamento                    │
└─────────────────────────────────────────────────────┘
```

### Exemplo Prático

```bash
TOKEN="seu_token_jwt"
BASE_URL="https://connect.defarm.net"

# Passo 1: Criar item local
echo "Criando item local..."
LOCAL_RESPONSE=$(curl -s -X POST "$BASE_URL/api/items/local" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "identifiers": [
      {
        "namespace": "bovino",
        "key": "sisbov",
        "value": "BR98765432109876",
        "id_type": "Canonical",
        "verified": false
      }
    ],
    "enriched_data": {
      "raca": "Nelore",
      "data_nascimento": "2024-03-15",
      "peso_kg": 180
    }
  }')

LOCAL_ID=$(echo "$LOCAL_RESPONSE" | jq -r '.data.local_id')
echo "✓ Item local criado: $LOCAL_ID"
echo "  (Item privado no workspace nesta fase)"

# Passo 2: Enviar para circuito para tokenização
echo ""
echo "Enviando para circuito para tokenização..."
PUSH_RESPONSE=$(curl -s -X POST "$BASE_URL/api/circuits/id-do-circuito/push-local" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"local_id\": \"$LOCAL_ID\"}")

DFID=$(echo "$PUSH_RESPONSE" | jq -r '.data.dfid')
CID=$(echo "$PUSH_RESPONSE" | jq -r '.data.storage.cid')
TX_HASH=$(echo "$PUSH_RESPONSE" | jq -r '.data.storage.transaction_hash')

echo "✓ Tokenização completa!"
echo "  DFID: $DFID"
echo "  CID IPFS: $CID"
echo "  TX Stellar: $TX_HASH"

# Passo 3: Consultar por LID ou DFID
echo ""
echo "Consultando item..."
curl -s "$BASE_URL/api/items/$DFID" \
  -H "Authorization: Bearer $TOKEN" | jq '.data | {dfid, status, identifiers}'

# Passo 4: Verificar mapeamento LID-DFID
echo ""
echo "Verificando mapeamento LID..."
curl -s "$BASE_URL/api/items/mapping/$LOCAL_ID" \
  -H "Authorization: Bearer $TOKEN" | jq '.'
```

### O Que Acontece Durante a Tokenização?

| Passo | Descrição | Tempo |
|-------|-----------|-------|
| Validação | Verificar identificadores, namespaces, permissões | ~50ms |
| Deduplicação | Buscar itens existentes por ID canônico ou fingerprint | ~100ms |
| Atribuição DFID | Gerar novo DFID ou usar existente | ~10ms |
| Armazenamento IPFS | Enviar dados para IPFS, obter CID | ~500ms |
| Envio Stellar | Enviar CID para blockchain, obter hash TX | ~5000ms |
| Criação Evento | Criar eventos ItemTokenized e ItemPushed | ~50ms |
| **Total** | **Tokenização completa** | **~5.7s** |

**Nota:** Tempos são aproximados. Operações IPFS e Stellar são as partes mais lentas.

---

## Armazenamento Blockchain & Adaptadores

### Visão Geral

O DeFarm usa uma abordagem de armazenamento híbrido: **IPFS para dados** + **Blockchain Stellar para referências imutáveis**.

### Arquitetura de Armazenamento

```
┌─────────────────────────────────────────────────────┐
│  Dados do Item                                      │
│  (identificadores, enriched_data, eventos)          │
└────────────────────┬────────────────────────────────┘
                     ↓
         ┌───────────────────────┐
         │   Serializar para JSON│
         └───────────┬───────────┘
                     ↓
         ┌───────────────────────┐
         │   Enviar para IPFS    │
         │   → Obter CID         │
         │   (Hash de Conteúdo)  │
         └───────────┬───────────┘
                     ↓
         ┌───────────────────────┐
         │   Enviar para Stellar │
         │   Contrato IPCM       │
         │   store(CID, metadata)│
         └───────────┬───────────┘
                     ↓
         ┌───────────────────────┐
         │   Obter Hash TX       │
         │   (Prova Blockchain)  │
         └───────────────────────┘
```

### Tipos de Adaptadores

#### 1. StellarTestnetIpfs (Desenvolvimento)

**Usar para:** Desenvolvimento, testes, demos

**Características:**
- ✅ Gratuito (XLM testnet)
- ✅ Recursos completos de blockchain
- ✅ Armazenamento IPFS incluído
- ⚠️ Dados testnet não permanentes
- ⚠️ Não para produção

**Exemplo:**
```json
{
  "adapter_type": "StellarTestnetIpfs",
  "requires_approval": false,
  "sponsor_adapter_access": true
}
```

#### 2. StellarMainnetIpfs (Produção)

**Usar para:** Aplicações de produção

**Características:**
- ✅ Confiabilidade nível produção
- ✅ Registros blockchain permanentes
- ✅ Armazenamento IPFS incluído
- 💰 Requer XLM para taxas
- 🔒 SLA empresarial disponível

**Exemplo:**
```json
{
  "adapter_type": "StellarMainnetIpfs",
  "requires_approval": true,
  "sponsor_adapter_access": false
}
```

#### 3. IpfsOnly (Sem Blockchain)

**Usar para:** Armazenamento somente IPFS sem blockchain

**Características:**
- ✅ Menor custo (sem taxas blockchain)
- ✅ Armazenamento rápido (~500ms)
- ❌ Sem prova blockchain
- ❌ Sem timestamp imutável

**Exemplo:**
```json
{
  "adapter_type": "IpfsOnly",
  "requires_approval": false
}
```

#### 4. LocalStorage (Apenas Desenvolvimento)

**Usar para:** Desenvolvimento local sem serviços externos

**Características:**
- ✅ Zero dependências externas
- ✅ Armazenamento instantâneo (<10ms)
- ❌ Sem persistência entre reinicializações
- ❌ Não para produção

### Configuração de Adaptador de Circuito

#### Configurando um Circuito com Blockchain

```bash
TOKEN="seu_token_jwt"

# Criar circuito com Stellar + IPFS
curl -X POST "https://connect.defarm.net/api/circuits" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Cadeia de Café Orgânico",
    "description": "Rastrear café da fazenda à xícara",
    "adapter_config": {
      "adapter_type": "StellarTestnetIpfs",
      "requires_approval": false,
      "auto_migrate_existing": false,
      "sponsor_adapter_access": true
    },
    "visibility": "Public"
  }'
```

#### Patrocínio de Adaptador

**O que é patrocínio de adaptador?**

Quando um circuito tem `sponsor_adapter_access: true`, o circuito **patrocina acesso blockchain** para todos os membros. Isso significa:

| Sem Patrocínio | Com Patrocínio |
|----------------|----------------|
| Usuários precisam tier Premium/Empresarial | Todos membros podem usar blockchain |
| Cada usuário paga taxas blockchain | Proprietário do circuito paga taxas |
| Membros bloqueados se tier insuficiente | Todos membros podem enviar livremente |

**Casos de Uso:**
- 🏢 **Circuitos empresariais** - Empresa patrocina acesso para fornecedores
- 🎓 **Circuitos educacionais** - Escola patrocina acesso para alunos
- 🔬 **Circuitos de pesquisa** - Projeto patrocina acesso para pesquisadores

**Exemplo: Circuito Patrocinado**
```json
{
  "circuit_name": "Cadeia Empresarial",
  "adapter_config": {
    "adapter_type": "StellarMainnetIpfs",
    "sponsor_adapter_access": true
  }
}
```

Resultado: Usuários tier básico podem enviar itens para este circuito apesar de não ter acesso blockchain em seu tier.

### Recuperando Configuração de Adaptador do Circuito

```bash
# Obter config adaptador (autenticado)
curl "https://connect.defarm.net/api/circuits/{circuit_id}/adapter" \
  -H "Authorization: Bearer $TOKEN"

# Resposta
{
  "adapter_type": "StellarTestnetIpfs",
  "sponsor_adapter_access": true,
  "requires_approval": false
}
```

### Campos de Resposta de Armazenamento

Quando você envia um item para um circuito com blockchain habilitado, você recebe:

```json
{
  "dfid": "DFID-20250128-000042-7A3F",
  "storage": {
    "adapter_type": "StellarTestnetIpfs",
    "location": "ipfs",
    "hash": "QmT5NvUtoM5nW...",
    "cid": "QmT5NvUtoM5nW...",
    "transaction_hash": "a3f7c2e1b9d8...",
    "stellar_network": "testnet",
    "contract_id": "CAIFDKP...",
    "metadata": {
      "stored_at": "2025-01-28T14:30:00Z",
      "size_bytes": 1024
    }
  }
}
```

**Campos explicados:**
- `cid` - Identificador de Conteúdo IPFS (use para recuperar do IPFS)
- `transaction_hash` - ID da transação Stellar (verificar no explorador blockchain)
- `stellar_network` - "testnet" ou "mainnet"
- `contract_id` - Endereço do contrato inteligente IPCM

### Verificando Armazenamento no Blockchain

```bash
# Ver transação no Explorador Stellar
# Testnet: https://stellar.expert/explorer/testnet/tx/{transaction_hash}
# Mainnet: https://stellar.expert/explorer/public/tx/{transaction_hash}

# Recuperar do IPFS
curl "https://ipfs.io/ipfs/{cid}"

# Ou usando nó IPFS local
ipfs cat {cid}
```

---

**Last Updated:** 2025-01-28
**API Version:** v1.0
**Production URL:** https://connect.defarm.net

---

🌱 **Documentação completa para desenvolvedores avançados do DeFarm Engines!**

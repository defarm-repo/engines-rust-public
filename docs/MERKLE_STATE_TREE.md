# Merkle State Tree (MST) System

The Merkle State Tree system provides cryptographic verification of item and circuit states, enabling efficient sync verification, proof of inclusion, and change detection.

## Architecture

### Three-Level Hierarchy

```
          Circuit Root
              |
    +---------+----------+
    |         |          |
 Item 1    Item 2     Item 3
  Root      Root       Root
    |         |          |
  Events   Events     Events
```

1. **Circuit Level**: Root hash computed from all item roots in a circuit
2. **Item Level**: Root hash computed from all events for an item
3. **Event Level**: Leaf nodes containing event hashes

### Hash Algorithm

All hashes use **BLAKE3** producing 64-character hex strings.

**Event Hash Formula:**
```
BLAKE3(event_id|event_type|timestamp_nanos|source|metadata_json)
```

## Core Components

### MerkleTree (`src/merkle_tree.rs`)

Core data structure for building and verifying Merkle trees.

```rust
use defarm_engine::{MerkleTree, MerkleProof};

// Build tree from leaves
let leaves = vec!["hash1".to_string(), "hash2".to_string()];
let tree = MerkleTree::from_leaves(leaves);

// Get root hash
let root = tree.root(); // Option<&str>

// Generate proof for leaf at index 0
let proof = tree.generate_proof(0)?;

// Verify proof
let valid = MerkleTree::verify_proof(&proof, &expected_root);
```

### MerkleEngine (`src/merkle_engine.rs`)

Higher-level engine for building trees from storage data.

```rust
use defarm_engine::{MerkleEngine, hash_event};

// Create engine with storage
let engine = MerkleEngine::new(storage);

// Get item's Merkle root
let response = engine.get_item_root("DFID-20240101-000001-ABC1")?;
// Returns: { dfid, merkle_root, event_count, computed_at }

// Get circuit's Merkle root
let response = engine.get_circuit_root(&circuit_id)?;
// Returns: { circuit_id, merkle_root, item_count, items[], computed_at }

// Generate proof that event exists in item
let proof = engine.prove_event_in_item(&dfid, &event_id)?;

// Generate proof that item exists in circuit
let proof = engine.prove_item_in_circuit(&circuit_id, &dfid)?;

// Compare two circuit states
let comparison = engine.compare_circuit_states(
    &circuit_id,
    &local_root,
    &remote_root,
    Some(&remote_items),
)?;
```

## API Endpoints

All endpoints require authentication (JWT or API key).

### Root Hash Endpoints

#### GET `/api/merkle/items/:dfid/merkle-root`

Get the Merkle root hash for an item's events.

**Response:**
```json
{
  "success": true,
  "data": {
    "dfid": "DFID-20240101-000001-ABC1",
    "merkle_root": "abc123...",
    "event_count": 5,
    "computed_at": "2024-01-01T12:00:00Z"
  }
}
```

#### GET `/api/merkle/circuits/:circuit_id/merkle-root`

Get the Merkle root hash for a circuit's items.

**Response:**
```json
{
  "success": true,
  "data": {
    "circuit_id": "uuid-here",
    "merkle_root": "def456...",
    "item_count": 10,
    "items": [
      { "dfid": "DFID-...", "merkle_root": "...", "event_count": 3 }
    ],
    "computed_at": "2024-01-01T12:00:00Z"
  }
}
```

### Proof Endpoints

#### GET `/api/merkle/items/:dfid/merkle-proof/:event_id`

Generate a proof that an event exists in an item's Merkle tree.

**Response:**
```json
{
  "success": true,
  "proof": {
    "leaf_hash": "...",
    "siblings": [
      { "hash": "...", "position": "Left" }
    ],
    "root": "..."
  },
  "item_dfid": "DFID-...",
  "event_id": "uuid"
}
```

#### GET `/api/merkle/circuits/:circuit_id/merkle-proof/:dfid`

Generate a proof that an item exists in a circuit's Merkle tree.

**Response:**
```json
{
  "success": true,
  "proof": {
    "leaf_hash": "...",
    "siblings": [...],
    "root": "..."
  },
  "circuit_id": "uuid",
  "item_dfid": "DFID-..."
}
```

### Verification Endpoint

#### POST `/api/merkle/verify-proof`

Verify a Merkle proof against an expected root.

**Request:**
```json
{
  "proof": {
    "leaf_hash": "...",
    "siblings": [...],
    "root": "..."
  },
  "expected_root": "abc123..."
}
```

**Response:**
```json
{
  "valid": true,
  "message": "Proof is valid - leaf exists in tree with expected root"
}
```

### Sync Comparison Endpoint

#### POST `/api/merkle/circuits/:circuit_id/sync-check`

Compare local and remote circuit states to detect differences.

**Request:**
```json
{
  "local_root": "abc123...",
  "remote_root": "def456...",
  "remote_items": [
    { "dfid": "DFID-...", "merkle_root": "...", "event_count": 3 }
  ]
}
```

**Response:**
```json
{
  "in_sync": false,
  "local_root": "abc123...",
  "remote_root": "def456...",
  "message": "States differ - 3 items affected",
  "differing_items": ["DFID-1", "DFID-2", "DFID-3"],
  "local_only": ["DFID-1"],
  "remote_only": ["DFID-2"],
  "modified": ["DFID-3"]
}
```

## Use Cases

### 1. Sync Verification

Quickly verify if two users have the same circuit state:

```bash
# User A gets their root
curl -H "Authorization: Bearer $TOKEN_A" \
  "$API/api/merkle/circuits/$CIRCUIT_ID/merkle-root"

# User B compares with their state
curl -X POST -H "Authorization: Bearer $TOKEN_B" \
  -H "Content-Type: application/json" \
  -d '{"local_root": "...", "remote_root": "user_a_root"}' \
  "$API/api/merkle/circuits/$CIRCUIT_ID/sync-check"
```

### 2. Proof of Inclusion

Prove an event exists without revealing other events:

```bash
# Generate proof
curl -H "Authorization: Bearer $TOKEN" \
  "$API/api/merkle/items/$DFID/merkle-proof/$EVENT_ID"

# Verify proof (can be done offline)
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"proof": {...}, "expected_root": "..."}' \
  "$API/api/merkle/verify-proof"
```

### 3. Change Detection

Find exactly which items differ between two states:

```bash
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "local_root": "...",
    "remote_root": "...",
    "remote_items": [...]
  }' \
  "$API/api/merkle/circuits/$CIRCUIT_ID/sync-check"
```

## Properties

### Determinism
- Same events always produce the same tree
- Leaves are sorted before tree construction
- Event order doesn't affect final root

### Efficiency
- O(log n) proof size
- O(log n) verification time
- Single hash comparison for sync check

### Security
- BLAKE3 cryptographic hash function
- Collision resistance
- Pre-image resistance
- Tamper-evident

## Data Types

### MerkleProof

```rust
pub struct MerkleProof {
    pub leaf_hash: String,
    pub siblings: Vec<ProofSibling>,
    pub root: String,
}

pub struct ProofSibling {
    pub hash: String,
    pub position: SiblingPosition,
}

pub enum SiblingPosition {
    Left,
    Right,
}
```

### ItemMerkleEntry

```rust
pub struct ItemMerkleEntry {
    pub dfid: String,
    pub merkle_root: String,
    pub event_count: usize,
}
```

### SyncComparison

```rust
pub struct SyncComparison {
    pub in_sync: bool,
    pub local_root: String,
    pub remote_root: String,
    pub differing_items: Vec<String>,
    pub local_only: Vec<String>,
    pub remote_only: Vec<String>,
    pub modified: Vec<String>,
}
```

## Error Handling

| Error | HTTP Status | Description |
|-------|-------------|-------------|
| EmptyTree | 404 | No events/items found |
| InvalidProof | 400 | Malformed proof data |
| StorageError | 500 | Storage backend failure |
| Unauthorized | 401 | Missing authentication |

## Testing

Run Merkle-specific tests:

```bash
cargo test merkle_tree
cargo test merkle_engine
```

## Related Documentation

- [Snapshot System](./SNAPSHOT_SYSTEM.md) - For full state snapshots
- [Events Engine](./EVENTS_ENGINE.md) - Event creation and management
- [Circuits Engine](./CIRCUITS_ENGINE.md) - Circuit operations

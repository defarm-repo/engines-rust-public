# Adapter Bridge & Storage Consolidation System

**Status**: Proposal - Not Yet Implemented
**Created**: 2025-10-06
**Priority**: Medium (after event synchronization is validated)

## Problem Statement

When users with different adapter configurations push items to the same circuit, we need to handle:

1. **Cross-adapter transfers**: Moving items between storage backends
2. **Identifier conflicts**: Same identifier, different items, different adapters
3. **Cost management**: Who pays for circuit storage operations?
4. **Storage consolidation**: Managing items in multiple locations
5. **Adapter compatibility**: Which adapters can transfer to which?

## Current State

### ✅ What Works
- Conflict detection for duplicate identifiers
- Storage history tracking
- Circuit adapter configuration API

### ❌ What's Missing
- `store_item_to_circuit_adapter()` is a stub (circuits_engine.rs:369)
- No adapter-to-adapter transfer implementation
- No cost calculation or sponsorship model
- No storage consolidation policies
- No adapter compatibility matrix

## Scenarios Analysis

### Scenario 1: Same Identifier, Different Items, Different Adapters

**Setup:**
- User A: Creates item with `lot_number: "ABC123"` → Stored on **Polygon** → DFID-A
- User B: Creates item with `lot_number: "ABC123"` → Stored on **Stellar** → DFID-B
- Both push to Circuit C which uses **IPFS**

**Current Behavior:**
```
1. Conflict Detection Triggers:
   - System detects: lot_number "ABC123" → [DFID-A, DFID-B]
   - Creates PendingItem with reason: Conflict
   - Status: Requires manual resolution

2. Storage Operations (NOT IMPLEMENTED):
   - Should: DFID-A: Polygon → IPFS (circuit's adapter)
   - Should: DFID-B: Stellar → IPFS (circuit's adapter)
   - Actually: Items remain in original locations only

3. Resolution Options:
   a) They are duplicates → Merge into single DFID
   b) They are different → Keep both, update identifiers
   c) One is correct → Deprecate the other
```

### Scenario 2: Same Item (DFID), Multiple Storage Locations

**Setup:**
- User A pushes DFID-X (stored on Polygon) to Circuit C
- User B pulls DFID-X from somewhere, stores on Stellar, pushes to same Circuit C
- Circuit C uses **IPFS**

**Expected Behavior:**
```
1. First Push (User A):
   - DFID-X: Polygon → IPFS
   - Storage History: [Polygon, IPFS]

2. Second Push (User B):
   - System recognizes DFID-X already in circuit
   - Operation: Deduplication (same content_hash)
   - DFID-X: Stellar location added to history
   - Storage History: [Polygon, IPFS, Stellar]

Result: Item exists in 3 places, circuit uses IPFS version
```

### Scenario 3: Circuit Adapter Mismatch

**Setup:**
- User A: Local-Local adapter
- User B: IPFS-IPFS adapter
- Circuit C: Stellar-IPFS adapter

**Expected Behavior:**
```
User A pushes DFID-X:
├─ Source: Local storage
├─ Target: Circuit (Stellar-IPFS)
├─ Operation: Cross-adapter transfer
└─ Result:
   ├─ Metadata → Stellar
   ├─ Events → IPFS
   └─ Storage History updated

User B pushes DFID-Y:
├─ Source: IPFS storage
├─ Target: Circuit (Stellar-IPFS)
├─ Operation: Partial match (IPFS events already compatible)
└─ Result:
   ├─ Metadata → Stellar (new)
   ├─ Events → IPFS (reuse existing)
   └─ Cheaper operation (only metadata transfer)
```

## Real-World Use Cases

### Use Case A: Agriculture Supply Chain

```
Coffee Farm (User A):
├─ Adapter: Local-Local (cheap, offline-first)
├─ Item: lot_number "2025-HARVEST-001"
└─ Storage: Local file system

Export Company (User B):
├─ Adapter: Stellar-IPFS (verified, public)
├─ Item: lot_number "2025-HARVEST-001"
└─ Storage: Stellar metadata + IPFS events

Certification Circuit:
├─ Adapter: IPFS-IPFS (neutral, accessible)
└─ Members: Farm, Export Co, Certifier

Workflow:
1. Both push to circuit
2. System detects potential duplicate (same identifier)
3. Certifier reviews both items
4. If same: Merge DFIDs, consolidate storage history
5. If different: Flag as fraud/error
6. Circuit stores canonical version on IPFS
7. All members can verify via IPFS hashes
```

### Use Case B: Cross-Border Trade

```
Manufacturer (China):
├─ Adapter: Polygon-IPFS
├─ Item: product_id "WIDGET-5000"
└─ Registers trademark on Polygon

Distributor (USA):
├─ Adapter: Stellar-IPFS
├─ Item: product_id "WIDGET-5000"
└─ Registers import license on Stellar

Trade Circuit:
├─ Adapter: Dual (Polygon + Stellar recognition)
├─ Rule: Accept items from either blockchain
└─ IPFS as common event storage

Workflow:
1. Both push to circuit
2. System recognizes SAME product (DFID match)
3. Storage history: [Polygon, Stellar, IPFS]
4. Circuit maintains BOTH blockchain anchors
5. Different jurisdictions can verify their preferred chain
6. IPFS provides neutral ground for events
```

### Use Case C: Privacy-First Research

```
Researcher A:
├─ Adapter: Local-Local (data privacy)
├─ Item: patient_id "P-12345" (encrypted)
└─ Cannot use public blockchains (HIPAA)

Researcher B:
├─ Adapter: IPFS-IPFS (sharing preference)
├─ Item: patient_id "P-67890" (encrypted)
└─ Willing to use distributed storage

Research Circuit:
├─ Adapter: Local-IPFS (hybrid)
├─ Rule: Metadata stays local, only encrypted events to IPFS
└─ Sponsorship: Circuit pays IPFS costs

Workflow:
1. Researcher A pushes → Metadata stays local, encrypted events to IPFS
2. Researcher B pushes → Already on IPFS, just add to circuit
3. Circuit never exposes raw patient data
4. All events encrypted with circuit key
5. Circuit sponsors IPFS storage (research grant funded)
```

## Proposed Implementation

### 1. Adapter Bridge Trait

```rust
pub trait AdapterBridge {
    /// Check if transfer is possible between adapters
    async fn can_transfer_between(
        source: &AdapterType,
        target: &AdapterType
    ) -> bool;

    /// Calculate cost of transfer
    async fn transfer_cost(
        source: &AdapterType,
        target: &AdapterType,
        item_size: usize
    ) -> Result<u64, BridgeError>;

    /// Execute the transfer
    async fn execute_transfer(
        item: &Item,
        events: &[Event],
        source: &AdapterType,
        target: &AdapterType
    ) -> Result<TransferReceipt, BridgeError>;
}
```

### 2. Adapter Compatibility Matrix

```rust
pub struct AdapterCompatibilityMatrix {
    rules: HashMap<(AdapterType, AdapterType), CompatibilityRule>,
}

pub enum CompatibilityRule {
    Direct,              // Can transfer directly
    RequiresBridge(Box<dyn AdapterBridge>), // Needs translation
    Prohibited(String),  // Cannot transfer (with reason)
}

// Example:
// Local → IPFS: Direct
// Polygon → Stellar: RequiresBridge (blockchain bridge)
// IPFS → Local: Prohibited ("Cannot guarantee data availability")
```

### 3. Circuit Sponsorship Model

```rust
pub struct CircuitAdapterConfig {
    pub adapter: AdapterType,
    pub sponsorship_mode: SponsorshipMode,
    pub cost_sharing: CostSharing,
    pub storage_limits: StorageLimits,
}

pub enum SponsorshipMode {
    UserPays,           // User pays for circuit storage
    CircuitSponsors,    // Circuit pays from treasury
    Hybrid(f64),        // Split costs (percentage to circuit)
}

pub struct CostSharing {
    pub circuit_pays_percentage: f64,
    pub max_circuit_contribution: Option<u64>,
    pub fallback_to_user: bool,
}

pub struct StorageLimits {
    pub max_item_size: usize,
    pub max_events_per_item: usize,
    pub max_total_storage: usize,
}
```

### 4. Conflict Resolution Strategies

```rust
pub enum ConflictStrategy {
    AutoMerge {
        confidence_threshold: f64,
        require_approval: bool,
    },
    ManualReview {
        assign_to: Vec<UserId>,
        timeout: Duration,
        default_action: ConflictAction,
    },
    BlockDuplicates {
        allow_same_source: bool,
        allow_enrichment: bool,
    },
}

pub enum ConflictAction {
    MergeItems,
    KeepBoth,
    RejectNew,
    RejectOld,
}
```

### 5. Storage Consolidation Policy

```rust
pub struct StorageConsolidationPolicy {
    /// Remove duplicates across storage locations
    pub deduplication: bool,

    /// Keep most recent version only
    pub prefer_latest: bool,

    /// Keep version from most trusted adapter
    pub prefer_adapter: Option<AdapterType>,

    /// Maintain copies in all locations
    pub maintain_all_copies: bool,

    /// Sync changes across all locations
    pub sync_updates: bool,

    /// Garbage collect old versions
    pub gc_old_versions: Option<Duration>,
}
```

### 6. Transfer Receipt & Audit Trail

```rust
pub struct TransferReceipt {
    pub transfer_id: Uuid,
    pub item_dfid: String,
    pub source_adapter: AdapterType,
    pub target_adapter: AdapterType,
    pub source_location: StorageLocation,
    pub target_location: StorageLocation,
    pub timestamp: DateTime<Utc>,
    pub cost: u64,
    pub paid_by: UserId,
    pub status: TransferStatus,
    pub verification_hash: String,
}

pub enum TransferStatus {
    Pending,
    InProgress,
    Completed,
    Failed(String),
    Rolled_back,
}
```

## Implementation Phases

### Phase 1: Foundation (Week 1-2)
- [ ] Implement `AdapterBridge` trait
- [ ] Build adapter compatibility matrix
- [ ] Add cost estimation functions
- [ ] Create transfer receipt system

### Phase 2: Basic Transfers (Week 3-4)
- [ ] Implement Local ↔ IPFS transfers
- [ ] Implement IPFS ↔ Stellar transfers
- [ ] Implement IPFS ↔ Polygon transfers
- [ ] Add transfer validation and rollback

### Phase 3: Sponsorship (Week 5-6)
- [ ] Implement sponsorship modes
- [ ] Add circuit treasury management
- [ ] Create cost tracking and billing
- [ ] Add usage limits and quotas

### Phase 4: Advanced Features (Week 7-8)
- [ ] Implement storage consolidation
- [ ] Add conflict resolution strategies
- [ ] Create adapter bridge for blockchain transfers
- [ ] Add garbage collection

### Phase 5: Testing & Optimization (Week 9-10)
- [ ] End-to-end scenario testing
- [ ] Performance optimization
- [ ] Cost estimation accuracy
- [ ] Security audit

## Open Questions

1. **Blockchain Bridges**: How to handle Polygon ↔ Stellar transfers? External bridge service?
2. **Cost Pricing**: How to price different adapter operations fairly?
3. **Failed Transfers**: Rollback strategy if transfer fails mid-way?
4. **Privacy**: How to transfer encrypted items without exposing keys?
5. **Verification**: How to verify items match after transfer?
6. **Concurrency**: What if item is modified during transfer?

## Dependencies

- Event synchronization system (✅ completed)
- Circuit adapter configuration (✅ completed)
- Credit/tier system (✅ completed)
- Storage history manager (✅ completed)

## Success Metrics

- [ ] 99% successful transfers between compatible adapters
- [ ] < 5s average transfer time for small items
- [ ] Zero data loss during transfers
- [ ] Accurate cost estimation (±10%)
- [ ] Conflict resolution without data duplication

## Related Documents

- Circuit adapter permissions (implemented)
- Storage history management (implemented)
- Event synchronization system (implemented)
- Credit/tier system (implemented)

---

**Decision Required**: Approve implementation plan or revise priorities

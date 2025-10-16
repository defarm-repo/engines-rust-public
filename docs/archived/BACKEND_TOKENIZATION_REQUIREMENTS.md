# Backend Implementation Requirements - Circuit Tokenization System

## Overview

The frontend has been fully implemented for the circuit-based tokenization system. This document specifies the backend endpoints and functionality needed to support the new workflow.

---

## 🎯 Summary of Changes

The tokenization system replaces immediate DFID assignment with a two-stage process:

1. **Local Item Creation** - Items are created with a LID (Local ID) and stored temporarily
2. **Circuit Push (Tokenization)** - Items are pushed to a circuit, where they receive a DFID or match existing items

---

## 📡 Required Backend Endpoints

### 1. Create Local Item

**Endpoint:** `POST /api/items/local`

**Purpose:** Create a local item that hasn't been tokenized yet (no DFID assigned)

**Request Body:**
```typescript
{
  identifiers?: Identifier[];  // Legacy format (optional, for backward compatibility)
  enhanced_identifiers?: EnhancedIdentifier[];  // New format (preferred)
  enriched_data?: Record<string, any>;
}

interface EnhancedIdentifier {
  namespace: string;        // Value chain: bovino, aves, soja, cafe, generic
  key: string;              // Identifier name: sisbov, cpf, lote, peso, etc.
  value: string;            // Actual identifier value
  id_type: "Canonical" | "Contextual";  // Type of identifier
}
```

**Response:**
```typescript
{
  success: true,
  data: {
    local_id: string;  // UUID assigned to this local item
    status: "LocalOnly";
  }
}
```

**Backend Logic:**
- Generate a UUID for `local_id`
- Store item with `LocalOnly` status
- DO NOT assign DFID yet
- Store enhanced_identifiers if provided
- Return local_id for future reference

**Example Request:**
```json
{
  "enhanced_identifiers": [
    {
      "namespace": "bovino",
      "key": "sisbov",
      "value": "BR1234567890123",
      "id_type": "Canonical"
    },
    {
      "namespace": "bovino",
      "key": "peso",
      "value": "450kg",
      "id_type": "Contextual"
    }
  ],
  "enriched_data": {
    "title": "My Cattle",
    "creator": "user123"
  }
}
```

---

### 2. Push Item to Circuit (Tokenization)

**Endpoint:** `POST /api/circuits/{circuit_id}/push`

**Purpose:** Push a local item to a circuit for tokenization. This assigns a DFID or matches to an existing item.

**Request Body:**
```typescript
{
  local_id: string;  // UUID from local item creation
  identifiers?: EnhancedIdentifier[];  // Optional: additional identifiers to add
  enriched_data?: Record<string, any>;  // Optional: additional data
}
```

**Response:**
```typescript
{
  success: true,
  data: {
    dfid: string;  // DFID assigned (new) or matched (existing)
    status: "NewItemCreated" | "ExistingItemEnriched";
    operation_id: string;  // ID of this push operation
    local_id: string;  // Original local_id
  }
}
```

**Backend Logic:**

1. **Validate Circuit Membership:**
   - Check user is member of circuit
   - Check user has permission to push items

2. **Load Circuit Configuration:**
   - Get circuit's `alias_config` (if exists)
   - Get circuit's `default_namespace`

3. **Validate Identifiers Against Circuit Requirements:**
   - Check all `required_canonical` identifiers are present
   - Check all `required_contextual` identifiers are present
   - Validate identifier formats (SISBOV, CPF, etc.)
   - Check namespaces are allowed (if `allowed_namespaces` specified)

4. **Apply Circuit Rules:**
   - If `auto_apply_namespace` is true, apply `default_namespace` to identifiers missing namespace
   - Combine identifiers from local item + additional identifiers from request

5. **Deduplication Check:**
   - Search for existing items in circuit with matching canonical identifiers
   - If `use_fingerprint` is true and no canonical match, check fingerprint match
   - If match found: **Enrich existing item** (status: "ExistingItemEnriched")
   - If no match: **Create new item** (status: "NewItemCreated")

6. **Assign DFID:**
   - If new item: Generate new DFID
   - If matched: Use existing DFID

7. **Update Item Status:**
   - Change status from "LocalOnly" to "Tokenized"
   - Store DFID in item record
   - Link local_id to DFID in mapping table

8. **Add to Circuit:**
   - Add item to circuit's items list
   - If circuit has `require_approval_for_push`, set status to "Pending"
   - Otherwise, add directly to circuit

**Example Request:**
```json
{
  "local_id": "550e8400-e29b-41d4-a716-446655440000",
  "identifiers": [
    {
      "namespace": "bovino",
      "key": "fazenda",
      "value": "Fazenda Santa Maria",
      "id_type": "Contextual"
    }
  ]
}
```

**Example Response (New Item):**
```json
{
  "success": true,
  "data": {
    "dfid": "DFID-20250108-000042-A7B3",
    "status": "NewItemCreated",
    "operation_id": "op_550e8400",
    "local_id": "550e8400-e29b-41d4-a716-446655440000"
  }
}
```

**Example Response (Duplicate Found):**
```json
{
  "success": true,
  "data": {
    "dfid": "DFID-20250107-000015-C2F1",
    "status": "ExistingItemEnriched",
    "operation_id": "op_550e8401",
    "local_id": "550e8400-e29b-41d4-a716-446655440000"
  }
}
```

---

### 3. Get LID-DFID Mapping

**Endpoint:** `GET /api/items/mapping/{local_id}`

**Purpose:** Query the mapping between a Local ID and its DFID (if tokenized)

**Response:**
```typescript
{
  success: true,
  data: {
    local_id: string;
    dfid: string | null;  // null if not yet tokenized
    status: "LocalOnly" | "Tokenized" | "Pending";
  }
}
```

**Example Response (Not Tokenized):**
```json
{
  "success": true,
  "data": {
    "local_id": "550e8400-e29b-41d4-a716-446655440000",
    "dfid": null,
    "status": "LocalOnly"
  }
}
```

**Example Response (Tokenized):**
```json
{
  "success": true,
  "data": {
    "local_id": "550e8400-e29b-41d4-a716-446655440000",
    "dfid": "DFID-20250108-000042-A7B3",
    "status": "Tokenized"
  }
}
```

---

### 4. Update Circuit Endpoint (Enhancement)

**Endpoint:** `GET /api/circuits/{circuit_id}`

**Current Response Enhancement:**
Add these fields to the existing CircuitData response:

```typescript
{
  // ... existing fields ...
  default_namespace?: string;  // NEW: Default namespace for this circuit
  alias_config?: {             // NEW: Tokenization configuration
    required_canonical: string[];      // e.g., ["sisbov"]
    required_contextual: string[];     // e.g., ["lote", "safra"]
    allowed_namespaces?: string[];     // e.g., ["bovino"]
    auto_apply_namespace: boolean;     // Auto-fill empty namespaces
    use_fingerprint: boolean;          // Use fingerprint for dedup
  };
}
```

**Example:**
```json
{
  "success": true,
  "data": {
    "circuit_id": "circuit_123",
    "name": "Rastreabilidade Bovina",
    "description": "Circuit for cattle traceability",
    "default_namespace": "bovino",
    "alias_config": {
      "required_canonical": ["sisbov"],
      "required_contextual": ["fazenda"],
      "allowed_namespaces": ["bovino"],
      "auto_apply_namespace": true,
      "use_fingerprint": false
    },
    // ... other existing fields ...
  }
}
```

---

### 5. Update ItemData Response (Enhancement)

**All Item Endpoints** (`GET /api/items`, `GET /api/items/{dfid}`, etc.)

Add these fields to ItemData:

```typescript
{
  // ... existing fields ...
  local_id?: string;  // NEW: UUID of local item (if created via new flow)
  enhanced_identifiers?: EnhancedIdentifier[];  // NEW: Enhanced identifiers
  item_status?: "LocalOnly" | "Tokenized" | "Pending";  // NEW: Tokenization status
}
```

---

## 🔧 Database Schema Changes

### New Tables/Fields Needed:

#### 1. items Table Updates:
```sql
ALTER TABLE items ADD COLUMN local_id UUID;
ALTER TABLE items ADD COLUMN enhanced_identifiers JSONB;
ALTER TABLE items ADD COLUMN item_status VARCHAR(20) DEFAULT 'Tokenized';
```

#### 2. circuits Table Updates:
```sql
ALTER TABLE circuits ADD COLUMN default_namespace VARCHAR(50);
ALTER TABLE circuits ADD COLUMN alias_config JSONB;
```

#### 3. LID-DFID Mapping Table (Optional):
```sql
CREATE TABLE lid_dfid_mappings (
  local_id UUID PRIMARY KEY,
  dfid VARCHAR(50),
  status VARCHAR(20),
  created_at TIMESTAMP DEFAULT NOW(),
  tokenized_at TIMESTAMP
);
```

---

## ✅ Validation Rules

### Identifier Format Validation:

The frontend validates these formats - backend should enforce them too:

```typescript
const VALIDATION_RULES = {
  sisbov: /^BR\d{13}$/,      // SISBOV: BR + 13 digits
  cpf: /^\d{11}$/,            // CPF: 11 digits
  cnpj: /^\d{14}$/,           // CNPJ: 14 digits
  // CAR, GTA, SIF: varies by state/region
};
```

### Circuit Requirements Validation:

```typescript
// Example circuit config
{
  "required_canonical": ["sisbov"],
  "required_contextual": ["fazenda", "lote"],
  "allowed_namespaces": ["bovino"],
  "auto_apply_namespace": true,
  "use_fingerprint": false
}

// Validation logic:
// 1. All required_canonical identifiers must be present
// 2. All required_contextual identifiers must be present
// 3. If allowed_namespaces is set, all identifiers must use allowed namespaces
// 4. Canonical identifiers must match format validation
```

---

## 🚨 Error Responses

### Validation Error:
```json
{
  "success": false,
  "error": "ValidationError",
  "message": "Required canonical identifier 'sisbov' not provided"
}
```

### Permission Error:
```json
{
  "success": false,
  "error": "PermissionDenied",
  "message": "User does not have permission to push to this circuit"
}
```

### Invalid Identifier Format:
```json
{
  "success": false,
  "error": "ValidationError",
  "message": "Invalid SISBOV format. Expected: BR + 13 digits. Example: BR1234567890123"
}
```

### Namespace Not Allowed:
```json
{
  "success": false,
  "error": "ValidationError",
  "message": "Namespace 'aves' not allowed in this circuit. Allowed: bovino"
}
```

---

## 📊 Deduplication Algorithm

### Canonical Identifier Matching:

```python
def find_duplicate(new_item, circuit_items):
    """
    Find existing item in circuit with matching canonical identifiers
    """
    # Extract canonical identifiers from new item
    new_canonical = [id for id in new_item.enhanced_identifiers
                     if id.id_type == "Canonical"]

    # Search circuit items for matches
    for existing_item in circuit_items:
        existing_canonical = [id for id in existing_item.enhanced_identifiers
                             if id.id_type == "Canonical"]

        # Check if any canonical identifier matches
        for new_id in new_canonical:
            for existing_id in existing_canonical:
                if (new_id.namespace == existing_id.namespace and
                    new_id.key == existing_id.key and
                    new_id.value == existing_id.value):
                    # Found duplicate!
                    return existing_item

    return None  # No duplicate found
```

### Fingerprint Matching (if use_fingerprint = true):

```python
def calculate_fingerprint(item):
    """
    Create a fingerprint from all identifiers for deduplication
    when no canonical identifiers exist
    """
    # Sort identifiers for consistent hashing
    sorted_ids = sorted(item.enhanced_identifiers,
                       key=lambda x: (x.namespace, x.key, x.value))

    # Create string representation
    fingerprint_str = "|".join([
        f"{id.namespace}:{id.key}:{id.value}"
        for id in sorted_ids
    ])

    # Hash it
    return hashlib.sha256(fingerprint_str.encode()).hexdigest()
```

---

## 🧪 Test Scenarios

### Test 1: Create and Push New Item
```bash
# Step 1: Create local item
POST /api/items/local
{
  "enhanced_identifiers": [
    {"namespace": "bovino", "key": "sisbov", "value": "BR1234567890123", "id_type": "Canonical"}
  ]
}
# Response: { local_id: "uuid-123", status: "LocalOnly" }

# Step 2: Push to circuit
POST /api/circuits/circuit_456/push
{
  "local_id": "uuid-123"
}
# Response: { dfid: "DFID-001", status: "NewItemCreated" }
```

### Test 2: Deduplication
```bash
# Step 1: Create first item
POST /api/items/local → { local_id: "uuid-111" }
POST /api/circuits/circuit_456/push
{ "local_id": "uuid-111" }
# Response: { dfid: "DFID-001", status: "NewItemCreated" }

# Step 2: Create second item with SAME SISBOV
POST /api/items/local → { local_id: "uuid-222" }
POST /api/circuits/circuit_456/push
{ "local_id": "uuid-222" }
# Response: { dfid: "DFID-001", status: "ExistingItemEnriched" }  ← Same DFID!
```

### Test 3: Validation Error
```bash
POST /api/circuits/circuit_456/push
{
  "local_id": "uuid-123",
  "identifiers": [
    {"namespace": "bovino", "key": "peso", "value": "450kg", "id_type": "Contextual"}
  ]
}
# Response: ERROR - "Required canonical identifier 'sisbov' not provided"
```

---

## 📝 Implementation Checklist

### Phase 1: Basic Endpoints
- [ ] Implement `POST /api/items/local`
- [ ] Implement `POST /api/circuits/{id}/push` (without deduplication)
- [ ] Implement `GET /api/items/mapping/{local_id}`
- [ ] Add `default_namespace` and `alias_config` to circuits table
- [ ] Add `local_id`, `enhanced_identifiers`, `item_status` to items table

### Phase 2: Validation
- [ ] Circuit requirements validation
- [ ] Identifier format validation (SISBOV, CPF, etc.)
- [ ] Namespace restriction enforcement
- [ ] Auto-apply namespace logic

### Phase 3: Deduplication
- [ ] Canonical identifier matching algorithm
- [ ] Fingerprint-based matching (if use_fingerprint = true)
- [ ] Item enrichment logic (merge data from duplicate)
- [ ] Return correct status (NewItemCreated vs ExistingItemEnriched)

### Phase 4: Testing
- [ ] Unit tests for validation
- [ ] Integration tests for push workflow
- [ ] Deduplication test cases
- [ ] Error handling test cases

---

## 🔮 Future Enhancements (Post-MVP)

- Batch push operations (push multiple local items at once)
- Edit identifiers after creation but before tokenization
- Pending approval workflow for local items
- Push history tracking
- Allow one item to be pushed to multiple circuits
- Identifier templates per namespace
- Bulk identifier import from CSV

---

## 📞 Frontend Contact Points

The frontend is ready and waiting for these endpoints. Key integration points:

1. **ItemComposer.tsx** (line 102) - Calls `items.createLocal()`
2. **CircuitPushDialog.tsx** (line 163) - Calls `circuitApi.pushToCircuit()`
3. **API types** - All TypeScript interfaces in `src/lib/api/types.ts`

---

## 🎯 Success Criteria

Backend implementation is complete when:

✅ Users can create local items with enhanced identifiers
✅ Local items can be pushed to circuits for tokenization
✅ Deduplication correctly identifies matching items
✅ Circuit requirements are validated before push
✅ Proper error messages for validation failures
✅ Frontend receives correct response formats
✅ All test scenarios pass

---

**Questions?** Reach out to the frontend team with any clarifications needed!

**Implementation Priority:** High - Frontend is complete and blocked on these endpoints.

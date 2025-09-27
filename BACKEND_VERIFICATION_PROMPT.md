# DeFarm Engine Architecture & Principles

**Version**: 2025-09-27
**Status**: Active Implementation

## System Overview

The DeFarm Engine is a comprehensive traceability and receipt management system built in Rust, designed to handle farm operations, supply chain tracking, and circuit-based collaboration networks. The system implements a data-lake-first architecture with progressive entity resolution and permanent identifier assignment.

## Core Architecture Principles

### 1. Data Lake First Architecture
- **Principle**: All incoming data is immediately preserved in its original form
- **Implementation**: Every submission creates a permanent source entry with UUID tracking
- **Benefit**: No data loss, complete audit trail, ability to reprocess with improved algorithms

### 2. Progressive Entity Resolution
- **Principle**: Items accumulate identifiers and data over time through intelligent matching
- **Implementation**:
  ```
  Data Submission → Data Lake Storage → Entity Resolution → [Match: Enrich] OR [No Match: Create]
  ```
- **Benefit**: Reduces duplicates, enriches existing data, maintains data integrity

### 3. Server-Controlled Resource Identification
- **Principle**: All resource identifiers (DFIDs) are generated server-side after verification
- **Implementation**: Clients submit data without identifiers; server assigns after duplicate checking
- **Benefit**: Prevents identifier conflicts, ensures uniqueness, maintains security

## DFID Generation Workflow

### Core Process
1. **Data Submission**: Client submits identifiers + enriched data + source entry UUID
2. **Entity Resolution**: System checks existing items for matching identifiers
3. **Decision Point**:
   - **Match Found**: Enrich existing item, add new identifiers, return existing DFID
   - **No Match**: Generate new DFID, create new item, store all data
4. **Response**: Return item with DFID and all associated data

### DFID Format
- **Pattern**: `DFID-YYYYMMDD-NNNNNN-CHECKSUM`
- **Properties**: Sequential, date-based, includes verification checksum
- **Uniqueness**: Guaranteed unique across system lifetime
- **Immutability**: Once assigned, never changes or reused

### Verification Timing
DFIDs are generated ONLY after entity resolution confirms the item is genuinely new. This ensures:
- No wasted identifiers on duplicate submissions
- Consistent entity consolidation
- Proper enrichment of existing items

## API Architecture

### Items Engine Endpoints
```
POST   /api/items                    # Create item (auto-DFID generation)
GET    /api/items                    # List items (with filtering)
GET    /api/items/{dfid}             # Get specific item
PUT    /api/items/{dfid}             # Update item (add identifiers/data)
DELETE /api/items/{dfid}             # Deprecate item
POST   /api/items/{dfid}/merge       # Merge two items
POST   /api/items/{dfid}/split       # Split item into multiple
GET    /api/items/search             # Search items by criteria
GET    /api/items/stats              # System statistics
GET    /api/items/identifier/{k}/{v} # Find by specific identifier
```

### Circuits Engine Endpoints
```
POST   /api/circuits                 # Create collaboration circuit
GET    /api/circuits                 # List accessible circuits
GET    /api/circuits/{id}            # Get circuit details
PUT    /api/circuits/{id}            # Update circuit settings
DELETE /api/circuits/{id}            # Deprecate circuit
POST   /api/circuits/{id}/join       # Request to join circuit
PUT    /api/circuits/{id}/approve    # Approve join request
POST   /api/circuits/{id}/pull       # Pull item from circuit
POST   /api/circuits/{id}/push       # Push item to circuit
GET    /api/circuits/{id}/items      # List circuit items
```

### Receipts Engine Endpoints
```
POST   /api/receipts                 # Create receipt
GET    /api/receipts                 # List receipts
GET    /api/receipts/{id}            # Get receipt details
PUT    /api/receipts/{id}            # Update receipt
POST   /api/receipts/{id}/verify     # Verify receipt integrity
GET    /api/receipts/chain/{id}      # Get receipt chain
```

## Data Schema Definitions

### Item Schema
```rust
struct Item {
    dfid: String,                                    // Server-generated unique ID
    identifiers: Vec<Identifier>,                    // All known identifiers
    enriched_data: HashMap<String, serde_json::Value>, // Progressive data accumulation
    creation_timestamp: DateTime<Utc>,               // First creation time
    last_modified: DateTime<Utc>,                    // Latest modification
    source_entries: Vec<Uuid>,                       // All contributing submissions
    status: ItemStatus,                              // Active | Deprecated | Merged | Split
    confidence_score: f64,                           // Quality/reliability score
}

struct Identifier {
    key: String,        // Identifier type (e.g., "lot_number", "barcode")
    value: String,      // Identifier value
}

enum ItemStatus {
    Active,      // Normal operational state
    Deprecated,  // Marked for removal
    Merged,      // Consolidated into another item
    Split,       // Divided into multiple items
}
```

### Circuit Schema
```rust
struct Circuit {
    id: String,                                      // Unique circuit identifier
    name: String,                                    // Human-readable name
    description: String,                             // Purpose description
    participants: Vec<Participant>,                  // Authorized members
    settings: CircuitSettings,                       // Operational parameters
    status: CircuitStatus,                           // Active | Inactive | Deprecated
}

struct CircuitSettings {
    public_settings: HashMap<String, serde_json::Value>, // Visible to all
    private_settings: HashMap<String, serde_json::Value>, // Creator only
    permissions: PermissionMatrix,                       // Access controls
}
```

## Entity Resolution Algorithm

### Duplicate Detection Logic
1. **Exact Identifier Match**: Any shared identifier indicates potential duplicate
2. **Confidence Scoring**: Evaluate match strength based on identifier types
3. **Enrichment Strategy**: Merge data from multiple sources intelligently
4. **Conflict Resolution**: Handle contradictory data with source tracking

### Progressive Enrichment
- Items start with minimal data and grow over time
- Each submission adds new identifiers and data points
- Source tracking maintains provenance for all data elements
- Confidence scores reflect data quality and reliability

## System Benefits

### For Developers
- **REST Compliance**: Standard HTTP methods and status codes
- **Type Safety**: Rust's type system prevents runtime errors
- **Comprehensive API**: Full CRUD operations with advanced features
- **Consistent Patterns**: Similar endpoint structures across engines

### For Operations
- **Data Integrity**: No data loss, complete audit trails
- **Scalability**: Efficient storage and retrieval patterns
- **Reliability**: Rust's memory safety and error handling
- **Monitoring**: Built-in logging and statistics collection

### For Business
- **Traceability**: Complete supply chain tracking
- **Collaboration**: Circuit-based partner networks
- **Compliance**: Audit-ready data preservation
- **Growth**: Progressive data enrichment over time

## Implementation Status

### Completed Features
- Core Items Engine with DFID generation
- Complete REST API for items management
- Entity resolution with progressive enrichment
- Circuit creation and management system
- Circuit participant permission controls
- Receipt generation and verification
- Comprehensive logging and error handling
- Multi-engine API server with routing

### Current Capabilities
- Server-side DFID generation after entity resolution
- Progressive item enrichment through identifier matching
- Circuit-based collaboration networks
- Receipt chain verification
- Comprehensive API coverage for all major operations
- Type-safe Rust implementation with error handling

This architecture provides a robust foundation for farm traceability, supply chain management, and collaborative network operations while maintaining data integrity and system reliability.
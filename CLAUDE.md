# Claude Instructions

## System Documentation Rule
Every new feature or update must be documented by updating existing principles or appending new ones in the appropriate section. Keep principles simple - one row per principle maximum.

## Reception Engine Principles

### Data Reception
1. Any data can be sent to the system
2. The data must contain at least one identifier
3. At reception, a receipt will be created containing BLAKE3 hash, timestamp, UUID, and identifiers
4. The receipt serves as cryptographic proof of data reception
5. Data integrity can be verified by comparing original data hash with receipt hash

### Identifiers
1. Identifiers are key-value pairs that provide context about the data
2. Multiple identifiers can be associated with a single data submission
3. Identifiers enable cross-referencing and lookup of receipts
4. Both keys and values can vary in structure and format

## Logging Engine Principles

### Event Logging
1. All system events are logged with timestamps, levels, and contextual data
2. Log entries contain engine source, event type, message, and key-value context
3. Log levels include Info, Warn, and Error for appropriate categorization
4. Logs are searchable by engine, event type, level, and time range

### Integration
1. Each engine maintains its own logging instance for isolated event tracking
2. Logging is automatically integrated into all engine operations
3. Success and failure events are logged with relevant context and metadata

## Storage Engine Principles

### Privacy-First Design
1. Original data is never stored, only cryptographic hashes are persisted
2. Identifiers can be stored encrypted or hashed for privacy protection
3. Storage backends are configurable for different deployment models
4. Data separation enables multi-tenant and on-premise deployments

### Storage Backends
1. In-memory storage for development and testing
2. Encrypted file storage for single-node deployments
3. Database storage with encryption-at-rest for production
4. Customer-controlled storage for on-premise deployments

### Data Security
1. AES-256-GCM encryption protects data at rest when enabled
2. Customer-generated encryption keys ensure data privacy
3. Unencrypted storage available for non-sensitive deployments
4. Storage backend separation enables multi-tenant isolation

## DFID Engine Principles

### DFID Generation
1. DFIDs use format: DFID-{timestamp}-{sequence}-{checksum}
2. Each DFID is globally unique with built-in validation
3. Timestamp component enables chronological sorting
4. Sequence counter prevents collisions within same timestamp
5. Checksum provides basic integrity verification

## Verification Engine Principles

### Data Processing Flow
1. Data flows from Receipt Engine → Data Lake → Verification → Items
2. Verification engine processes pending data lake entries automatically
3. Identifier analysis determines if data creates new item or enriches existing
4. All verification decisions are logged with full context

### Deduplication Logic
1. New identifiers create new items with generated DFIDs
2. Existing identifiers enrich corresponding items
3. Conflicting identifiers trigger conflict resolution
4. Auto-resolution attempts confidence-based and temporal strategies

### Conflict Resolution
1. Multiple identifiers mapping to different DFIDs require resolution
2. System attempts automatic resolution using confidence scores
3. Complex conflicts are flagged for manual review
4. All conflict decisions maintain audit trail

## Items Engine Principles

### Item Management
1. Items represent deduplicated, canonical records with DFIDs
2. Items can be enriched with additional data from multiple sources
3. Item operations include creation, enrichment, merging, and splitting
4. All item changes maintain source entry references for traceability

### Item Lifecycle
1. Items start as Active when created
2. Items can be Merged when duplicates are consolidated
3. Items can be Split when separation is required
4. Items can be Deprecated when no longer valid

## Events Engine Principles

### Event Tracking
1. All item lifecycle changes create events with timestamps and metadata
2. Events support public, private, and circuit-only visibility levels
3. Private events are encrypted for confidentiality protection
4. Events maintain full audit trail of item history

### Event Types
1. Created events track new item creation with identifiers
2. Enriched events track data additions to existing items
3. Merged and Split events track item consolidation operations
4. Circuit events track push/pull operations between items and circuits

### Event Storage
1. Events are stored with DFID references for item association
2. Event metadata includes contextual information about changes
3. Time-range queries enable historical analysis of item changes
4. Event visibility controls access to sensitive operations

## Circuits Engine Principles

### Circuit Management
1. Circuits are permission-controlled repositories for sharing items
2. Circuit owners have full control over membership and permissions
3. Circuit members have role-based access to push/pull operations
4. Circuit permissions control approval requirements for operations

### Circuit Membership
1. Owner role has all permissions including circuit management
2. Admin role can manage members and approve operations
3. Member role can push and pull items based on circuit settings
4. Viewer role has read-only access to circuit items

### Circuit Operations
1. Push operations share items from user to circuit
2. Pull operations retrieve items from circuit to user
3. Operations can require approval based on circuit permissions
4. All operations create events for audit trail

### Circuit Security
1. Circuit visibility controls event publication (public vs private)
2. Circuit permissions enable fine-grained access control
3. Operation approval workflow prevents unauthorized access
4. Circuit deactivation disables all operations while preserving data
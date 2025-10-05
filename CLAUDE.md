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

### Circuit Adapter Permissions
1. Circuits can specify required adapter type for push operations via adapter_config
2. Circuits can sponsor adapter access for members (sponsor_adapter_access flag)
3. When sponsored, any member with Push permission can push regardless of their adapter access
4. When not sponsored, users must have the required adapter in their available_adapters
5. Adapter permissions checked against user's custom adapters or tier defaults
6. Push operations fail with clear error when adapter access denied
7. Error messages guide users to request adapter access from administrator
8. Circuit adapter config changes trigger CircuitAdapterConfigUpdated notifications
9. GET /api/circuits/:id/adapter retrieves circuit adapter configuration with JWT authentication
10. PUT /api/circuits/:id/adapter sets/updates configuration, validates requester tier has adapter access
11. Only circuit owner or admins can configure adapter settings

## API Key Engine Principles

### API Key Generation
1. API keys use format: dfm_{32-character-random-string}
2. Keys are hashed using BLAKE3 before storage
3. Only key prefix (first 8 characters) is stored for identification
4. Full key is shown only once at creation time
5. Keys are cryptographically random and globally unique

### API Key Authentication
1. Keys can be provided via X-API-Key header or Authorization Bearer token
2. Keys are validated against hashed storage for security
3. Inactive or expired keys are rejected immediately
4. IP restrictions enforce additional access control when configured
5. Endpoint restrictions limit key access to specific API routes

### API Key Management
1. Users can create multiple API keys with different permissions
2. Keys support read, write, admin, and custom permissions
3. Keys can be activated, deactivated, or deleted at any time
4. Key expiration dates enable automatic lifecycle management
5. Usage tracking records every request for audit and analytics

## Rate Limiting Principles

### Rate Limit Configuration
1. Limits can be set per hour, minute, and day independently
2. Each API key has its own rate limit configuration
3. Burst limits allow temporary spikes in traffic
4. Rate limits are tier-based and upgrade with subscription
5. Limits reset on rolling window basis (not fixed intervals)

### Rate Limit Enforcement
1. Requests are tracked in-memory with timestamp precision
2. Old requests are cleaned automatically from tracking windows
3. Rate limit headers show remaining quota and reset time
4. Exceeded limits return 429 status with retry-after seconds
5. Rate limit state is isolated per API key for fairness

### Rate Limit Response
1. Response includes current limit, remaining quota, and reset time
2. Retry-after header guides client backoff strategy
3. Multiple window violations return the shortest retry period
4. Rate limit errors suggest upgrade paths and backoff strategies

## Error Handling Principles

### Error Classification
1. Errors are categorized by domain (API key, storage, validation, etc.)
2. Each error type maps to appropriate HTTP status code
3. Error responses include machine-readable error codes
4. Human-readable messages explain what went wrong
5. Recovery suggestions guide users toward resolution

### Error Recovery
1. Every error provides actionable recovery suggestions
2. Suggestions are context-aware based on error type
3. Rate limit errors suggest backoff and upgrade options
4. Permission errors guide users to access request flows
5. Validation errors reference documentation for correct format

### Error Logging
1. All errors are logged with full context for debugging
2. Error logs include user ID, API key ID, and endpoint
3. Internal errors are logged at Error level for alerting
4. Client errors are logged at Warn level for monitoring
5. Error patterns enable proactive issue detection
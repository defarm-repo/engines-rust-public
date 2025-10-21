# Claude Instructions

## 📋 Important Documentation References

**Production Deployment Plan**: See [PRODUCTION_DEPLOYMENT_PLAN.md](./PRODUCTION_DEPLOYMENT_PLAN.md) for complete production deployment strategy including PostgreSQL implementation, infrastructure setup, and deployment checklist.

**Production Deployment Guide**: See [PRODUCTION_DEPLOYMENT.md](./PRODUCTION_DEPLOYMENT.md) for comprehensive step-by-step deployment instructions, SSL/TLS setup, monitoring, and troubleshooting.

**Railway Deployment**: See [RAILWAY_DEPLOYMENT.md](./RAILWAY_DEPLOYMENT.md) for cloud deployment guide using Railway.app with CLI and GitHub integration options.

**Railway Dashboard Setup**: See [RAILWAY_DASHBOARD_SETUP.md](./RAILWAY_DASHBOARD_SETUP.md) for step-by-step Railway deployment using the web dashboard (recommended for initial setup).

## 🔐 Demo Accounts - Testing Credentials

Production Railway API includes pre-configured demo accounts for all user tiers. These accounts are used for testing, documentation examples, and client demonstrations.

| Username | Password | Tier | User ID | Purpose |
|----------|----------|------|---------|---------|
| hen | demo123 | Admin | hen-admin-001 | Admin operations and system management |
| chick | Demo123! | Basic | 97b51073-0ec5-40f9-822a-ea93ed1ec008 | Basic tier feature testing |
| pullet | demo123 | Professional | pullet-user-001 | Professional tier feature testing |
| cock | demo123 | Enterprise | cock-user-001 | Enterprise tier feature testing |
| gerbov | Gerbov2024!Test | Professional | user-2da9af70-c4c3-4b13-9180-dc1c7094b27c | Client demo and documentation |

### Notes
1. All accounts are verified working on production Railway API
2. Password requirements: 8+ characters, uppercase letter, special character
3. Gerbov account has pre-configured working circuit: 002ea6db-6b7b-4a69-8780-1f01ae074265
4. These credentials should NEVER be used in production client applications
5. For documentation updates, see: docs/development/GERBOV_UPDATED_DOC.md and docs/development/GERBOV_TEST_CREDENTIALS.md

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

### Local Item Creation
1. Items can be created locally with a LID (Local ID) without immediate DFID assignment
2. Local items use temporary DFID format "LID-{uuid}" for storage
3. Local items remain workspace-private until pushed to a circuit
4. Local items support both legacy identifiers and enhanced identifiers
5. Local items enable offline data collection before circuit tokenization

### LID-DFID Mapping
1. LID (Local ID) is a UUID generated when item is created locally
2. DFID is only assigned when item is first pushed to a circuit
3. LID-DFID mappings are stored for item tracking across lifecycle
4. Items can be queried by either LID or DFID after tokenization
5. Temporary DFID format enables local-only items to work with existing systems

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

### Event Security
1. Event source field is auto-populated from authenticated context (never user-provided)
2. Source is extracted from JWT token (user_id) or API key (user_id) automatically
3. Event creation requires authentication (JWT or API key) to prevent anonymous events
4. This prevents audit trail tampering by malicious actors
5. All events include cryptographically verified source attribution

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

## Circuit Tokenization Architecture

### Core Tokenization Principles
1. Workspaces maintain Local IDs (LIDs) as UUIDs generated locally
2. DFIDs are ONLY generated when items are first pushed to circuits (tokenization)
3. Circuits are the authority for deduplication and identity resolution
4. Same real-world entity gets same DFID across all circuits in ecosystem
5. Legacy items with workspace-generated DFIDs continue working (backward compatibility)

### Identifier Types

#### Canonical Identifiers
1. Globally unique within their registry (SISBOV, CPF, CAR, RFID)
2. Used for cross-user deduplication without fingerprints
3. Format validated by system against registry rules
4. Examples: bovino:sisbov:BR12345678901234, pessoa:cpf:12345678901

#### Contextual Identifiers
1. Locally unique within user/organization context only
2. Require fingerprint for deduplication across users
3. Examples: soja:lote:123, aves:granja:G5, milho:talhao:A3

### Namespace System
1. Prevents collision between value chains (bovino vs aves vs soja)
2. Standard namespaces: bovino, aves, suino, soja, milho, algodao, cafe, leite, generic
3. Circuit defines default_namespace for auto-application
4. Format namespace:key:value creates globally unique identifier key

### Circuit Alias Configuration
1. Circuits specify required canonical identifiers (e.g., ["sisbov", "cpf"])
2. Circuits specify required contextual identifiers (e.g., ["lote", "safra"])
3. Circuits can restrict allowed namespaces for data integrity
4. Auto-apply namespace if missing when auto_apply_namespace is true
5. Use fingerprint deduplication when use_fingerprint is true

### Tokenization Flow
1. Workspace creates item with LID (Local ID) via POST /api/items/local
2. Push to circuit includes LID + enhanced identifiers via POST /api/circuits/{id}/push-local
3. Circuit validates against alias requirements
4. Circuit checks for existing DFID via canonical identifier or fingerprint
5. Circuit creates new DFID or enriches existing item
6. DFID returned to workspace with LID mapping stored

### Tokenization API Endpoints
1. POST /api/items/local - Create local item with LID (no DFID yet)
2. POST /api/circuits/{id}/push-local - Push local item to circuit for tokenization
3. GET /api/items/mapping/{local_id} - Query LID-DFID mapping and status
4. GET /api/circuits/{id}/adapter - Retrieve circuit adapter configuration
5. PUT /api/circuits/{id}/adapter - Configure circuit adapter settings (owner/admin only)

### Fingerprint Generation
1. Used when no canonical identifier available
2. Format: BLAKE3(user:id|lid:uuid|time:nanoseconds|ids:sorted)
3. Scoped per circuit to prevent cross-contamination
4. Includes timestamp to prevent collisions
5. Deterministic within same user and identifier set

### External Aliases
1. Items track aliases from multiple sources (certifiers, ERPs, users)
2. Each alias includes scheme, value, issuer, timestamp, and evidence hash
3. Aliases enable cross-referencing across different systems
4. Conflicts between aliases tracked with issuer information

### Deduplication Strategy
1. Priority 1: Match by canonical identifier (SISBOV, CPF, etc.)
2. Priority 2: Match by fingerprint if circuit configured
3. Priority 3: Create new DFID if no match found
4. All matches enrich existing item with new data
5. Multiple pushes from same entity accumulate data

## Circuit Post-Action Webhook System

### Core Webhook Principles
1. Webhook system is completely optional for circuit owners/managers
2. Post-action webhooks trigger after successful push operations
3. Circuit owners configure which events trigger webhooks
4. Multiple webhooks can be configured per circuit
5. Webhooks include storage details and item metadata based on configuration

### Webhook Configuration
1. Circuit owners enable/disable webhook system per circuit
2. Configure trigger events: ItemPushed, ItemApproved, ItemTokenized, ItemPublished
3. Control what data is included: storage details, item metadata
4. Each webhook has individual enable/disable flag
5. Webhooks support authentication: None, BearerToken, ApiKey, BasicAuth, CustomHeader

### Webhook Delivery
1. Webhooks fire asynchronously after successful push operations
2. Automatic retry with exponential backoff for failed deliveries
3. Configurable retry parameters: max_retries, initial_delay, max_delay, backoff_multiplier
4. Default: 3 retries with 1s initial delay, 30s max delay, 2x multiplier
5. Delivery history tracked with status, response codes, and timestamps

### Webhook Security
1. URL validation prevents SSRF attacks (no localhost or private IPs)
2. Only HTTPS and HTTP protocols allowed
3. Authentication credentials encrypted at rest
4. Only circuit owner and admins can configure webhooks
5. Webhook test endpoint for validation before going live

### Webhook Payload Structure
1. event_type: Type of event that triggered webhook
2. circuit_id and circuit_name: Circuit context
3. timestamp: When event occurred
4. item: DFID, local_id, identifiers, pushed_by
5. storage: adapter_type, location, hash, CID, metadata (if enabled)
6. operation_id: Reference to circuit operation
7. status: Operation completion status

### Webhook API Endpoints
1. GET /api/circuits/:id/post-actions - Get post-action settings
2. PUT /api/circuits/:id/post-actions - Update post-action settings
3. POST /api/circuits/:id/post-actions/webhooks - Create webhook
4. GET /api/circuits/:id/post-actions/webhooks/:webhook_id - Get webhook details
5. PUT /api/circuits/:id/post-actions/webhooks/:webhook_id - Update webhook
6. DELETE /api/circuits/:id/post-actions/webhooks/:webhook_id - Delete webhook
7. POST /api/circuits/:id/post-actions/webhooks/:webhook_id/test - Test webhook
8. GET /api/circuits/:id/post-actions/deliveries - View delivery history
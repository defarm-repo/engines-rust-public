# Claude Instructions

## 🚂 Railway CLI Complete Guide

### Authentication Methods

#### IMPORTANTE: Como Autenticar no Railway
**Token Configurado para Claude (2025-11-13):**
- Token Name: `claude`
- Token Value: `fb76b340-b105-4172-b4bf-4dcb894225a8`
- Armazenado em: `/Users/gabrielrondon/rust/engines/.env`
- Variável: `RAILWAY_TOKEN`

**Autenticação via CLI:**
1. Com login interativo: `railway login` (já feito)
2. Com token em comandos: `RAILWAY_TOKEN=$RAILWAY_TOKEN railway [command]`
3. Após fazer link com projeto, comandos funcionam direto

**Projeto DeFarm Linkado:**
- Project ID: `2e6d7cdb-f993-4411-bcf4-1844f5b38011`
- Environment: `production`
- Service: `defarm-engines-api` (ID: `37705203-a155-44bc-95a1-62eba333d383`)

#### Token Types
1. **Project Token** (`RAILWAY_TOKEN`) - Scoped to specific project/environment
   - Deploy code: `railway up`
   - View logs: `railway logs`
   - Redeploy services
   - Cannot create projects or check account info

2. **API Token** (`RAILWAY_API_TOKEN`) - Account or team level access
   - Personal: Full access to all personal workspaces
   - Team: Access to team resources only
   - Can create projects, manage workspaces

3. **Token Generation**
   - Personal/Team tokens: https://railway.com/account/tokens
   - Project tokens: Project Settings → Tokens page
   - Only one token type active at a time (RAILWAY_TOKEN has precedence)

### Essential CLI Commands

#### Project & Service Management
```bash
railway init                        # Create new project
railway link                        # Link to existing project
railway link <projectId>            # Link to specific project
railway service                     # Link to a service
railway list                        # List all projects
railway status                      # Show current project info
railway open                        # Open project dashboard
```

#### Deployment & Logs
```bash
railway up                          # Deploy with build logs
railway up --detach                 # Deploy without waiting
railway logs                        # View latest deployment logs
railway logs -d                     # View deployment logs
railway logs -b                     # View build logs
railway logs <deployment_id>        # Logs from specific deployment
railway down                        # Remove latest deployment
railway redeploy                    # Redeploy latest deployment
```

#### Environment Variables
```bash
railway variables                   # Show all variables
railway run <command>               # Run command with Railway vars
railway shell                       # Open shell with Railway vars
railway environment                 # Switch/create environments
```

#### Service Operations
```bash
railway add                         # Add database service
railway ssh                         # SSH into running service
railway connect                     # Connect to database shell
railway volume                      # Manage volumes
railway scale                       # Scale service resources
```

### DeFarm Project Services

Expected services in DeFarm Railway project:
1. **defarm-engines-api** - Main API service
   - URL: https://defarm-engines-api-production.up.railway.app
   - Health: /health endpoint
   - Ports: 3000 (internal), 443 (external)

2. **ipcm-event-listener** - Blockchain event listener
   - Monitors Stellar blockchain events
   - Updates IPCM contract state

3. **postgres** - PostgreSQL database
   - Connection via DATABASE_URL
   - Persistent volume attached

4. **redis** - Redis cache (if configured)
   - Session and cache storage

### Como Acessar os Serviços DeFarm

#### Passo 1: Obter Token
1. Acesse: https://railway.com/account/tokens
2. Crie um Project Token para o projeto DeFarm
3. Copie o token gerado

#### Passo 2: Configurar Token
```bash
export RAILWAY_TOKEN="seu-token-aqui"
```

#### Passo 3: Comandos Essenciais
```bash
# Ver logs da API
RAILWAY_TOKEN=$RAILWAY_TOKEN railway logs --service defarm-engines-api

# Ver status do projeto
RAILWAY_TOKEN=$RAILWAY_TOKEN railway status

# Listar variáveis
RAILWAY_TOKEN=$RAILWAY_TOKEN railway variables --service defarm-engines-api

# SSH na API
RAILWAY_TOKEN=$RAILWAY_TOKEN railway ssh --service defarm-engines-api

# Ver logs do event listener
RAILWAY_TOKEN=$RAILWAY_TOKEN railway logs --service ipcm-event-listener

# Conectar ao PostgreSQL
RAILWAY_TOKEN=$RAILWAY_TOKEN railway connect --service postgres
```

### Non-Interactive Usage Examples

```bash
# Deploy with token
RAILWAY_TOKEN=xxx railway up --detach

# View logs for specific service
RAILWAY_TOKEN=xxx railway logs --service defarm-engines-api-production

# Check deployment status
RAILWAY_TOKEN=xxx railway status

# View environment variables
RAILWAY_TOKEN=xxx railway variables

# Run migrations
RAILWAY_TOKEN=xxx railway run npm run migrate

# SSH into service for debugging
RAILWAY_TOKEN=xxx railway ssh --service defarm-engines-api
```

### Debugging Railway Deployments

#### Network Connectivity
1. Check DNS: `dig +short {app}.up.railway.app`
2. Test TCP: `nc -zv -w 3 {app}.up.railway.app 443`
3. Test HTTPS: `curl -v -s -m 60 https://{app}.up.railway.app/health`
4. Check TLS: `openssl s_client -connect {app}.up.railway.app:443`

#### Service Status Indicators
1. **Hibernated**: TLS handshake OK but HTTP timeout (free tier sleep)
2. **Crashed**: TCP connection refused
3. **Building**: Returns 503 Service Unavailable
4. **Healthy**: Returns 200 OK on health endpoint

### Troubleshooting Common Issues

1. **"Cannot login in non-interactive mode"**
   - Solution: Use RAILWAY_TOKEN or RAILWAY_API_TOKEN environment variable

2. **Service hibernation on free tier**
   - Solution: Upgrade to paid plan or setup external monitoring (UptimeRobot)

3. **"Unauthorized. Please login"**
   - Solution: Token not set or expired, regenerate from dashboard

4. **Deployment fails silently**
   - Check build logs: `railway logs -b`
   - Verify Dockerfile/buildpack configuration
   - Check resource limits and quotas

5. **API Congelada/Timeout (RESOLVIDO)**
   - **Diagnóstico Completo (2025-11-13)**:
     - Serviço está configurado com `"sleepApplication": true`
     - Binário `/app/defarm-api` existe no container
     - PostgreSQL e Redis funcionando normalmente
     - Porta 8080 em uso internamente (processo travado)
   - **Como Verificar Status**:
     ```bash
     railway status --json | jq '.services.edges[] | select(.node.name=="defarm-engines-api")'
     railway ssh -- ls -la /app
     railway logs --service defarm-engines-api
     ```
   - **Serviços Confirmados no Projeto**:
     - defarm-engines-api (API principal) - HIBERNADO
     - ipcm-event-listener (Blockchain listener) - SUCCESS
     - Postgres (Database) - SUCCESS
     - Redis (2 instâncias) - SUCCESS
     - defarm-mvp (Worker) - SUCCESS

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

## Concurrency Model Principles

### Uniform Concurrency Standard (2025-01-15)
1. **Storage backends use `Arc<std::sync::Mutex<T>>`** - All StorageBackend implementations and internal synchronous state
2. **Async engine wrappers use `Arc<tokio::sync::RwLock<T>>`** - For engines in AppState that need async access (CircuitsEngine, ItemsEngine, EventsEngine, ActivityEngine, NotificationEngine)
3. **PostgreSQL persistence uses `Arc<tokio::sync::RwLock<Option<PostgresPersistence>>>`** - Shared async-mutable state
4. **NEVER hold `std::sync::Mutex` guard across `.await`** - Drop guard before await or use `tokio::task::block_in_place` + `Handle::block_on`
5. **Access pattern**: Storage uses `.lock().unwrap()`, async wrappers use `.read().await` or `.write().await`
6. **Validation script** - Run `./scripts/check_concurrency.sh` to verify uniformity before commits
7. **Zero tolerance** - No `Arc<std::sync::RwLock>` in storage layer, no mixing patterns
8. **Architecture Decision Record** - See `docs/adr/001-concurrency-model.md` for complete rationale

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

### Event Deduplication (2025-11-28)
1. Events are deduplicated using BLAKE3 content hash of (dfid, event_type, source, metadata)
2. Content hash excludes timestamp to enable proper deduplication across time
3. Duplicate events return the existing event instead of creating a new one
4. EventCreationResult includes was_deduplicated and original_event_id fields
5. get_event_by_content_hash() enables O(1) deduplication lookup

### Local Event Storage
1. Events can be created locally without a DFID via POST /api/events/local
2. Local events use temporary DFID format "LOCAL-EVENT-{uuid}" for storage
3. Local events have is_local=true and a unique local_event_id (UUID)
4. Local events remain workspace-private until pushed to a circuit
5. GET /api/events/local/:local_event_id retrieves a local event by its local_event_id

### Event Push to Circuit
1. POST /api/circuits/:id/push-events pushes local events to a circuit
2. Push operation assigns a real DFID to the event, replacing LOCAL-EVENT-* prefix
3. Events are deduplicated on push using content hash
4. Auto-merge: When duplicate detected, metadata is non-destructively merged (only adds new keys)
5. Response includes events_pushed, events_deduplicated, and merged_keys for each event

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

## Authentication and Authorization Principles

### Dual Authentication Support
1. All API endpoints support both JWT token and API key authentication
2. Handlers accept both `Option<Extension<Claims>>` and `Option<Extension<ApiKeyContext>>`
3. Authentication checked in order: JWT claims first, then API key context
4. HTTP 401 returned when neither authentication method provided
5. User ID extracted consistently from either JWT (user_id: String) or API key (user_id: Uuid)

### Authentication Methods
1. JWT tokens passed via Authorization header as "Bearer {token}"
2. API keys passed via X-API-Key header or Authorization header
3. JWT tokens contain user_id, workspace_id, and expiration claims
4. API keys provide user_id via ApiKeyContext after validation
5. WebSocket connections use custom JWT verification from query parameters

### Handler Authentication Pattern
1. Standard pattern: extract user_id from claims or api_key_ctx or return 401
2. User ID conversion: JWT provides String, API key provides Uuid.to_string()
3. All handlers use identical authentication extraction code for consistency
4. Authentication happens before any business logic execution
5. Extracted user_id passed to engine methods for authorization checks

### Authorization and Admin Verification
1. Admin endpoints use verify_admin helper function with user_id parameter
2. Admin status checked against user account in storage backend
3. HTTP 403 returned when admin privileges required but not present
4. Admin verification separate from authentication (auth first, then authz)
5. Circuit ownership and permissions checked per operation independently

### API Coverage
1. Items API: 27 handlers with dual authentication support
2. Circuits API: 40 handlers using AuthenticatedUser extractor or dual auth
3. Events API: All event creation and retrieval handlers support dual auth
4. Notifications API: 5 REST handlers with dual auth (WebSocket uses custom JWT)
5. Admin API: 17 handlers with dual auth and admin privilege verification
6. Adapters API: 2 handlers with dual authentication support

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

## Merkle State Tree Engine (2025-12-05)

### Core Architecture
1. Three-level Merkle tree hierarchy: Circuit Root → Item Roots → Event Leaves
2. BLAKE3 cryptographic hashing throughout for consistency with rest of system
3. Trees computed on-demand from events/items data
4. Proof generation enables cryptographic verification of data inclusion

### Merkle Tree Structure
1. **Event Level**: Each event is a leaf node with hash of (event_id, dfid, event_type, source, timestamp, metadata)
2. **Item Level**: Item root is Merkle root of all its events, proving complete event history
3. **Circuit Level**: Circuit root is Merkle root of all item roots, proving complete circuit state

### Proof Types
1. **Event Proof**: Proves specific event exists in item's event history
2. **Item Proof**: Proves specific item exists in circuit's item set
3. **Verification**: Any proof can be independently verified using only the proof data and root hash

### API Endpoints (Authenticated)
1. GET /api/merkle/items/:dfid/merkle-root - Get item's Merkle root (event count included)
2. GET /api/merkle/items/:dfid/merkle-proof/:event_id - Generate proof event exists in item
3. GET /api/merkle/circuits/:circuit_id/merkle-root - Get circuit's Merkle root (item count included)
4. GET /api/merkle/circuits/:circuit_id/merkle-proof/:dfid - Generate proof item exists in circuit
5. POST /api/merkle/verify-proof - Verify any Merkle proof client-side

### Public API Endpoints (No Auth Required)
1. GET /api/public/merkle/items/:dfid/merkle-root - Public item Merkle root
2. GET /api/public/merkle/items/:dfid/merkle-proof/:event_id - Public event proof
3. GET /api/public/merkle/circuits/:circuit_id/merkle-root - Public circuit Merkle root
4. GET /api/public/merkle/circuits/:circuit_id/merkle-proof/:dfid - Public item proof
5. POST /api/public/merkle/verify-proof - Public proof verification
6. Public endpoints only work for items/circuits with allow_public_visibility=true

### Implementation Files
1. `src/merkle_engine.rs` - Core Merkle tree computation and proof generation
2. `src/api/merkle.rs` - REST API handlers for authenticated and public endpoints

### Current Status: DEPLOYED ✅
- All 10 endpoints live on production (https://connect.defarm.net)
- Authenticated endpoints at /api/merkle/*
- Public endpoints at /api/public/merkle/*
- Proof verification working correctly

### TODO: Future Enhancements

#### Priority 1: Blockchain Anchoring
1. Periodically commit circuit Merkle roots to Stellar IPCM contract
2. Creates immutable timestamp proving circuit state at specific moment
3. Enables third-party verification against on-chain anchors
4. Suggested interval: configurable per circuit (hourly/daily/on-demand)

#### Priority 2: Root Caching
1. Store computed Merkle roots in PostgreSQL
2. Invalidate cache when tree changes (new event/item added)
3. Reduces computation overhead for frequently accessed circuits
4. Track last_computed_at timestamp for cache freshness

#### Priority 3: Anchor Events
1. Create "MerkleRootAnchored" event type when roots committed to blockchain
2. Event includes: circuit_id, merkle_root, stellar_tx_hash, anchor_timestamp
3. Links on-chain and off-chain state for complete audit trail
4. Enables historical proof verification against anchored roots

#### Priority 4: Incremental Updates
1. Update only affected branches when events added (not rebuild entire tree)
2. Store intermediate node hashes for faster recomputation
3. Significantly improves performance for large circuits
4. Maintain backward compatibility with full tree rebuild
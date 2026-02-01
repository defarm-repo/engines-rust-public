# OpenAPI Documentation Architecture

## Recomendação: Estrutura de Documentação por Layers

### Opção 1: Arquivo Único Consolidado (RECOMENDADO) ✅

**Vantagens:**
- Frontend consome um único endpoint de documentação
- Mais fácil de manter sincronizado
- Swagger UI mostra todos os endpoints juntos
- Ideal para equipes pequenas/médias

**Estrutura:**
```
docs/api/
├── openapi.yaml          # Spec completo consolidado (TODAS as layers)
├── swagger-ui.html       # UI para visualizar a spec
└── index.html            # Landing page com links
```

**Serviços disponíveis em:**
- GET /swagger-ui → Engines API (Layer 2)
- GET http://dfid-service/swagger-ui → DFID Service (Layer 1)
- GET http://index-service/swagger-ui → Index Service (Layer 4) (futuro)

---

### Opção 2: Arquivos Separados por Layer

**Vantagens:**
- Documentação acoplada ao serviço
- Deploy independente de cada layer
- Ideal para microserviços em produção
- Cada equipe mantém sua própria spec

**Estrutura:**
```
dfid-service/
└── openapi-dfid.yaml        # Layer 1: DFID Service

engines/ (Item Registry)
├── docs/api/
│   ├── openapi.yaml         # Layer 2: Item Registry + Adapters
│   └── openapi-consolidated.yaml  # Todas as layers juntas

index-service/ (futuro)
└── openapi-index.yaml       # Layer 4: Index Service
```

---

## Arquitetura Atual de Endpoints

### Layer 1: DFID Service
**Base URL:** `https://dfid-service-production.up.railway.app`

Endpoints:
- `POST /dfid/generate` - Gera um ou mais DFIDs
- `POST /dfid/batch` - Gera lote de DFIDs
- `GET /dfid/{id}/validate` - Valida formato e checksum
- `GET /health` - Health check
- `GET /metrics` - Prometheus metrics
- `GET /swagger-ui` - Swagger UI (já configurado via utoipa)

### Layer 2: Item Registry (Engines API)
**Base URL:** `https://connect.defarm.net/api`

**Tags principais:**
- Authentication (POST /auth/login, /auth/register, etc.)
- Items (POST /items/local, GET /items/{dfid}, etc.)
- Circuits (POST /circuits, POST /circuits/{id}/push-local, etc.)
- **NOVO:** Batch Operations (POST /circuits/{id}/batch-push-local)
- Events (POST /events, GET /events, etc.)
- Storage History (GET /storage-history/{dfid}, etc.)
- Timeline (GET /items/{dfid}/timeline, etc.)
- Adapters (GET /adapters/configs, POST /adapters/test, etc.)
- Notifications (WebSocket /notifications/ws, etc.)
- Workspaces (GET /workspaces, POST /workspaces, etc.)
- API Keys (POST /api-keys, GET /api-keys, etc.)
- Admin (GET /admin/users, POST /admin/workspaces, etc.)
- **NOVO:** Metrics (GET /metrics) - Prometheus

### Layer 3: Adapter Layer
**Integrado no Engines API** (não tem endpoints próprios)

Adapters configuráveis:
- IpfsIpfs
- StellarTestnetIpfs
- StellarMainnetIpfs
- (Custom adapters via trait)

### Layer 4: Index Service (FUTURO)
**Base URL:** `https://index-service-production.up.railway.app` (quando implementado)

Endpoints planejados:
- `GET /index/{dfid}/locations` - Busca localizações de um DFID
- `POST /index/register` - Registra nova localização
- `GET /index/search` - Busca por identifier
- `GET /health` - Health check
- `GET /metrics` - Prometheus metrics

---

## Recomendação Final: OPÇÃO 1 (Arquivo Único) ✅

### Estrutura Implementada:

**1 arquivo OpenAPI consolidado:**
```yaml
docs/api/openapi.yaml
```

**Sections:**
```yaml
tags:
  # Layer 2: Item Registry (Main)
  - Authentication
  - Items
  - Circuits
  - Batch Operations (NOVO)
  - Events
  - Storage History
  - Timeline
  - Adapters
  - Notifications
  - Workspaces
  - API Keys
  - Admin
  - Metrics (NOVO)

  # Layer 1: DFID Service (External Reference)
  - External Services
```

**Servir documentação:**
- Engines API: `GET /swagger-ui` (aponta para openapi.yaml)
- DFID Service: `GET /swagger-ui` (próprio utoipa)

**Frontend consome:**
- Engines API OpenAPI: `https://connect.defarm.net/api/openapi.yaml`
- DFID Service OpenAPI: `https://dfid-service-production.up.railway.app/api-docs/openapi.json`

---

## Endpoints NOVOS a Documentar

### 1. Metrics Endpoint (Layer 2)
```yaml
/metrics:
  get:
    tags:
      - Metrics
    summary: Prometheus metrics
    description: Returns Prometheus-compatible metrics for monitoring
    security: []  # Public endpoint
    responses:
      '200':
        description: Metrics in Prometheus text format
        content:
          text/plain:
            schema:
              type: string
              example: |
                # HELP items_created_total Total number of items created
                # TYPE items_created_total counter
                items_created_total 1234
                ...
```

### 2. Batch Push Local Items (Layer 2)
```yaml
/circuits/{id}/batch-push-local:
  post:
    tags:
      - Circuits
      - Batch Operations
    summary: Batch push multiple local items to circuit
    description: |
      Efficiently push multiple local items to a circuit in a single operation.
      Returns detailed results for each item (success or failure).
    parameters:
      - name: id
        in: path
        required: true
        schema:
          type: string
          format: uuid
    requestBody:
      required: true
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/BatchPushLocalItemRequest'
    responses:
      '200':
        description: Batch push results
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/BatchPushLocalItemResponse'
      '400':
        description: Invalid request
      '403':
        description: Permission denied
      '404':
        description: Circuit not found
```

**Schemas:**
```yaml
BatchPushLocalItemRequest:
  type: object
  required:
    - items
  properties:
    items:
      type: array
      items:
        $ref: '#/components/schemas/BatchLocalPushItem'

BatchLocalPushItem:
  type: object
  required:
    - local_id
  properties:
    local_id:
      type: string
      format: uuid
    identifiers:
      type: array
      items:
        $ref: '#/components/schemas/IdentifierRequest'
    enriched_data:
      type: object
      additionalProperties: true

BatchPushLocalItemResponse:
  type: object
  properties:
    results:
      type: array
      items:
        $ref: '#/components/schemas/BatchLocalPushResult'
    total:
      type: integer
      example: 10
    succeeded:
      type: integer
      example: 8
    failed:
      type: integer
      example: 2

BatchLocalPushResult:
  type: object
  properties:
    local_id:
      type: string
      format: uuid
    success:
      type: boolean
    dfid:
      type: string
      nullable: true
      example: "DFID-20250201-000042-A7B2C3"
    status:
      type: string
      nullable: true
      enum:
        - NewItemCreated
        - ExistingItemEnriched
        - ConflictDetected
    error:
      type: string
      nullable: true
```

---

## Próximos Passos

1. ✅ Adicionar tag "Metrics" ao openapi.yaml
2. ✅ Adicionar endpoint /metrics
3. ✅ Adicionar endpoint /circuits/{id}/batch-push-local
4. ✅ Adicionar schemas: BatchPushLocalItemRequest, BatchLocalPushItem, BatchPushLocalItemResponse, BatchLocalPushResult
5. ⏸️  (Futuro) Criar openapi-dfid.yaml para DFID Service separado
6. ⏸️  (Futuro) Criar openapi-index.yaml quando Index Service for implementado

---

## Como Frontends Devem Consumir

### Desenvolvimento Local:
```bash
# Engines API
curl http://localhost:3000/api/openapi.yaml

# DFID Service (se rodando local)
curl http://localhost:3001/api-docs/openapi.json
```

### Produção:
```bash
# Engines API (completo)
curl https://connect.defarm.net/api/openapi.yaml

# DFID Service (separado)
curl https://dfid-service-production.up.railway.app/api-docs/openapi.json
```

### Ferramentas de Codegen:
```bash
# Gerar client TypeScript
npx openapi-typescript https://connect.defarm.net/api/openapi.yaml -o src/api/schema.ts

# Gerar client Python
openapi-generator generate -i https://connect.defarm.net/api/openapi.yaml -g python -o python-client
```

---

## Conclusão

**Recomendação: Arquivo único `openapi.yaml` consolidado** ✅

Isso mantém a simplicidade, facilita o consumo pelo frontend, e ainda permite que cada serviço (DFID, Index) tenha seu próprio Swagger UI dinâmico gerado pelo framework (utoipa no caso do DFID Service).

O arquivo consolidado serve como "fonte da verdade" para ferramentas de codegen e documentação, enquanto cada serviço pode manter sua própria UI Swagger para desenvolvimento/debug.

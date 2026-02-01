# DeFarm Services - Endpoints Completos

## Visão Geral da Arquitetura

```
┌─────────────────────────────────────────────────────────┐
│  Frontend (Web/Mobile)                                   │
│  Consome: openapi.yaml consolidado                      │
└─────────────────────────────────────────────────────────┘
                          │
        ┌─────────────────┼─────────────────┐
        │                 │                 │
        ▼                 ▼                 ▼
┌──────────────┐  ┌──────────────┐  ┌──────────────┐
│  Layer 1:    │  │  Layer 2:    │  │  Layer 4:    │
│  DFID        │  │  Item        │  │  Index       │
│  Service     │  │  Registry    │  │  Service     │
│              │  │  (Engines)   │  │  (Futuro)    │
└──────────────┘  └──────────────┘  └──────────────┘
```

---

## Layer 1: DFID Service 🆔

**URL Base:** `https://dfid-service-production.up.railway.app`
**Documentação:** `GET /swagger-ui` (Swagger UI via utoipa)
**OpenAPI Spec:** `GET /api-docs/openapi.json`

### Endpoints:

| Método | Endpoint | Descrição | Auth |
|--------|----------|-----------|------|
| POST | `/dfid/generate` | Gera um ou mais DFIDs | ❌ |
| POST | `/dfid/batch` | Gera lote de DFIDs (até 10k) | ❌ |
| GET | `/dfid/{id}/validate` | Valida formato e checksum | ❌ |
| GET | `/health` | Health check do serviço | ❌ |
| GET | `/metrics` | Prometheus metrics | ❌ |

**Características:**
- ✅ Stateless (exceto sequence counter)
- ✅ Redis persistence para sequence
- ✅ Per-day sequence reset automático
- ✅ BLAKE3 checksum (24-bit)
- ✅ Rate limiting: 2 req/s (burst: 10)
- ✅ Retry automático (3 tentativas)

---

## Layer 2: Item Registry (Engines API) 📦

**URL Base:** `https://connect.defarm.net/api`
**Documentação:** `GET /swagger-ui` (Swagger UI)
**OpenAPI Spec:** `GET /openapi.yaml`

### 1. Authentication 🔐

| Método | Endpoint | Descrição |
|--------|----------|-----------|
| POST | `/auth/register` | Registrar novo usuário |
| POST | `/auth/login` | Login (retorna JWT) |
| POST | `/auth/logout` | Logout (invalida token) |
| GET | `/auth/verify` | Verifica JWT token |
| POST | `/auth/refresh` | Refresh JWT token |

### 2. Items (Local & DFID) 📝

| Método | Endpoint | Descrição |
|--------|----------|-----------|
| POST | `/items/local` | Criar item local (gera LID) |
| GET | `/items/{dfid}` | Buscar item por DFID |
| GET | `/items/mapping/{local_id}` | Buscar LID→DFID mapping |
| PUT | `/items/{dfid}` | Atualizar item |
| DELETE | `/items/{dfid}` | Deletar item |
| POST | `/items/{dfid}/merge` | Merge de items duplicados |
| POST | `/items/{dfid}/enrich` | Enriquecer item com dados |

### 3. Circuits (Tokenization & Sharing) 🔄

| Método | Endpoint | Descrição |
|--------|----------|-----------|
| POST | `/circuits` | Criar novo circuit |
| GET | `/circuits` | Listar circuits do usuário |
| GET | `/circuits/{id}` | Detalhes do circuit |
| PUT | `/circuits/{id}` | Atualizar circuit |
| DELETE | `/circuits/{id}` | Deletar circuit |
| POST | `/circuits/{id}/members` | Adicionar membro |
| POST | `/circuits/{id}/push/{dfid}` | Push item (DFID existente) |
| POST | `/circuits/{id}/push-local` | Push item local (tokenização) |
| **POST** | **`/circuits/{id}/batch-push-local`** | **✨ NOVO: Batch push** |
| POST | `/circuits/{id}/push-events` | Push eventos locais |
| POST | `/circuits/{id}/pull/{dfid}` | Pull item do circuit |
| GET | `/circuits/{id}/items` | Listar items do circuit |
| GET | `/circuits/{id}/operations` | Operações do circuit |
| POST | `/circuits/operations/{op_id}/approve` | Aprovar operação |

### 4. Events (Audit Trail) 📅

| Método | Endpoint | Descrição |
|--------|----------|-----------|
| POST | `/events` | Criar evento |
| POST | `/events/local` | Criar evento local |
| GET | `/events/{event_id}` | Buscar evento por ID |
| GET | `/events/local/{local_id}` | Buscar evento local |
| GET | `/events` | Listar eventos (filtros) |
| GET | `/items/{dfid}/events` | Eventos de um item |

### 5. Storage History & Timeline 📜

| Método | Endpoint | Descrição |
|--------|----------|-----------|
| GET | `/storage-history/{dfid}` | Histórico de storage |
| GET | `/items/{dfid}/timeline` | Timeline de CIDs (blockchain) |
| GET | `/public/storage-history/{dfid}` | Storage público |
| GET | `/public/snapshots/{dfid}` | Snapshots públicos |

### 6. Adapters (Blockchain/IPFS) ⚙️

| Método | Endpoint | Descrição |
|--------|----------|-----------|
| GET | `/adapters/configs` | Listar adapters disponíveis |
| POST | `/adapters/test` | Testar adapter |
| GET | `/circuits/{id}/adapter` | Config adapter do circuit |
| PUT | `/circuits/{id}/adapter` | Configurar adapter |

### 7. Workspaces & API Keys 🏢

| Método | Endpoint | Descrição |
|--------|----------|-----------|
| GET | `/workspaces` | Listar workspaces |
| POST | `/workspaces` | Criar workspace |
| GET | `/api-keys` | Listar API keys |
| POST | `/api-keys` | Criar API key |
| DELETE | `/api-keys/{id}` | Deletar API key |

### 8. Notifications 🔔

| Método | Endpoint | Descrição |
|--------|----------|-----------|
| WS | `/notifications/ws` | WebSocket de notificações |
| GET | `/notifications` | Listar notificações |
| PUT | `/notifications/{id}/read` | Marcar como lida |
| DELETE | `/notifications/{id}` | Deletar notificação |

### 9. Admin 👑

| Método | Endpoint | Descrição |
|--------|----------|-----------|
| GET | `/admin/users` | Listar usuários |
| POST | `/admin/workspaces` | Criar workspace (admin) |
| PUT | `/admin/users/{id}/tier` | Alterar tier de usuário |
| GET | `/admin/stats` | Estatísticas do sistema |

### 10. Metrics (Prometheus) 📊

| Método | Endpoint | Descrição |
|--------|----------|-----------|
| **GET** | **`/metrics`** | **✨ NOVO: Prometheus metrics** |

**Métricas disponíveis:**
- Items: `items_created_total`, `items_enriched_total`, `items_merged_total`, `items_active`
- Circuits: `circuits_created_total`, `circuit_pushes_total`, `circuit_pushes_failed`
- Adapters: `adapter_uploads_total`, `adapter_uploads_success`, `adapter_uploads_failed`
- Cache: `cache_hits_total`, `cache_misses_total`, `cache_writes_total`
- External: `dfid_service_calls_total`, `index_service_calls_total`, `index_retry_queue_size`

---

## Layer 3: Adapter Layer 🔌

**Integrado no Engines API** (sem endpoints próprios)

### Adapters Disponíveis:

1. **IpfsIpfs** - IPFS puro
2. **StellarTestnetIpfs** - Stellar Testnet + IPFS + NFT
3. **StellarMainnetIpfs** - Stellar Mainnet + IPFS + NFT
4. **Custom** - Via trait `StorageAdapter`

**Configuração:**
- Circuit define `adapter_type` e `sponsor_adapter_access`
- Usuário precisa ter adapter disponível (tier ou custom)
- Push automático registra em blockchain/IPFS

---

## Layer 4: Index Service 🔍 (FUTURO)

**URL Base:** `https://index-service-production.up.railway.app` (quando implementado)

### Endpoints Planejados:

| Método | Endpoint | Descrição |
|--------|----------|-----------|
| GET | `/index/{dfid}/locations` | Busca localizações de DFID |
| POST | `/index/register` | Registra nova localização |
| GET | `/index/search` | Busca por identifier |
| GET | `/health` | Health check |
| GET | `/metrics` | Prometheus metrics |

**Características (planejadas):**
- Índice centralizado mas replicável
- Descobre onde DFIDs existem (circuits, blockchains, registries)
- Retry queue para registros falhados
- Verificação de provas criptográficas

---

## Resumo: Novos Endpoints Documentados ✨

### 1. Batch Push Local Items
```http
POST /api/circuits/{id}/batch-push-local
Authorization: Bearer <jwt-token>

{
  "items": [
    {
      "local_id": "uuid",
      "identifiers": [...],
      "enriched_data": {...}
    },
    ...
  ]
}

Response 200 OK:
{
  "results": [
    {
      "local_id": "uuid",
      "success": true,
      "dfid": "DFID-20250201-000042-A7B2C3",
      "status": "NewItemCreated"
    },
    ...
  ],
  "total": 10,
  "succeeded": 8,
  "failed": 2
}
```

**Benefícios:**
- ✅ Mais eficiente que requests individuais
- ✅ Validação de circuit uma única vez
- ✅ Resultados detalhados por item
- ✅ Continua processando mesmo com falhas parciais

### 2. Prometheus Metrics
```http
GET /api/metrics

Response 200 OK (text/plain):
# HELP items_created_total Total number of items created
# TYPE items_created_total counter
items_created_total 1234

# HELP circuit_pushes_total Total circuit push operations
# TYPE circuit_pushes_total counter
circuit_pushes_total 567
...
```

**Benefícios:**
- ✅ Monitoramento em tempo real
- ✅ Compatível com Grafana/Prometheus
- ✅ Métricas de cache, adapters, serviços externos
- ✅ Endpoint público (sem auth)

---

## Como Consumir (Frontend)

### 1. Desenvolvimento Local

```bash
# OpenAPI spec completo
curl http://localhost:3000/api/openapi.yaml

# Swagger UI interativo
open http://localhost:3000/swagger-ui
```

### 2. Produção

```bash
# OpenAPI spec completo (Engines API)
curl https://connect.defarm.net/api/openapi.yaml

# DFID Service OpenAPI
curl https://dfid-service-production.up.railway.app/api-docs/openapi.json

# Swagger UI interativo
open https://connect.defarm.net/swagger-ui
```

### 3. Gerar Client TypeScript

```bash
npx openapi-typescript https://connect.defarm.net/api/openapi.yaml \
  -o src/api/schema.ts

# Uso no código:
import type { paths } from './api/schema'

type BatchPushRequest = paths['/circuits/{id}/batch-push-local']['post']['requestBody']['content']['application/json']
```

### 4. Gerar Client Python

```bash
openapi-generator generate \
  -i https://connect.defarm.net/api/openapi.yaml \
  -g python \
  -o python-client

# Uso no código:
from defarm_client import CircuitsApi
circuits_api = CircuitsApi()
result = circuits_api.batch_push_local_items(circuit_id, request)
```

---

## Arquivos de Documentação

```
docs/api/
├── openapi.yaml                    # ✅ Spec consolidada (4766 linhas)
├── OPENAPI_ARCHITECTURE.md         # ✅ Decisões de arquitetura
├── SERVICES_ENDPOINTS.md           # ✅ Este arquivo (resumo)
├── swagger-ui.html                 # Swagger UI local
├── index.html                      # Landing page
├── API_GUIDE.md                    # Guia de uso
└── COMPLETE_DEVELOPER_GUIDE.md     # Guia completo

dfid-service/
└── (Swagger UI dinâmico via utoipa)
```

---

## Conclusão

**Status da Documentação OpenAPI:** ✅ COMPLETO

- ✅ Todos os endpoints da Layer 2 documentados
- ✅ Novos endpoints adicionados (batch-push-local, /metrics)
- ✅ Schemas completos com exemplos
- ✅ Referências para serviços externos (DFID Service)
- ✅ Tags organizadas por funcionalidade
- ✅ Exemplos de request/response
- ✅ Códigos de erro documentados

**Pronto para consumo por qualquer frontend!** 🚀

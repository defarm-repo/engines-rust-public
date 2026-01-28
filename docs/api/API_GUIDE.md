# DeFarm Engines API Guide | Guia da API DeFarm Engines

## 🌐 Language | Idioma
- [English](#english-documentation)
- [Português](#documentação-em-português)

---

# English Documentation

## 📋 Table of Contents
1. [Overview](#overview)
2. [Base URL](#base-url)
3. [Authentication](#authentication)
4. [API Keys](#api-keys)
5. [Core Concepts](#core-concepts)
6. [Common Workflows](#common-workflows)
7. [Error Handling](#error-handling)
8. [Rate Limiting](#rate-limiting)
9. [Best Practices](#best-practices)

## Overview

DeFarm Engines API provides a comprehensive system for data reception, tokenization, and circuit-based sharing with blockchain storage capabilities.

**Key Features:**
- 🔐 Dual authentication (JWT + API Keys)
- 🎯 Item tokenization with DFIDs
- 🔄 Circuit-based data sharing
- 📦 Multi-adapter blockchain storage
- 📊 Complete audit trail and event tracking
- 🔔 Real-time notifications

## Base URL

**Production:**
```
https://connect.defarm.net
```

All API endpoints are prefixed with `/api` unless specified otherwise.

## Authentication

### Method 1: JWT Token (Web Applications)

JWT tokens are ideal for web applications with user sessions.

**Step 1: Login**
```bash
curl -X POST "https://connect.defarm.net/api/auth/login" \
  -H "Content-Type: application/json" \
  -d '{
    "username": "your_username",
    "password": "your_password"
  }'
```

**Response:**
```json
{
  "success": true,
  "data": {
    "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "user_id": "user-123",
    "workspace_id": "workspace-456",
    "expires_in": 86400
  }
}
```

**Step 2: Use Token**
```bash
curl -X GET "https://connect.defarm.net/api/circuits" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN"
```

**JWT Token Characteristics:**
- ✅ Short-lived (24 hours by default)
- ✅ Tied to user session
- ✅ Includes user and workspace context
- ✅ Ideal for frontend applications
- ⚠️ Must be refreshed periodically

### Method 2: API Keys (Server-to-Server Integration)

API keys are ideal for server-to-server integrations, IoT devices, and third-party applications.

**Creating an API Key:**
```bash
# First, authenticate with JWT
TOKEN="your_jwt_token"

# Create API key
curl -X POST "https://connect.defarm.net/api/api-keys" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Production Integration Key",
    "permissions": {
      "read": true,
      "write": true,
      "admin": false,
      "custom": {}
    },
    "rate_limit_per_hour": 1000,
    "expires_in_days": 365,
    "notes": "Main integration key for production system"
  }'
```

**Response:**
```json
{
  "success": true,
  "data": {
    "api_key": "dfm_a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6",
    "metadata": {
      "id": "key-uuid-123",
      "name": "Production Integration Key",
      "key_prefix": "dfm_a1b2",
      "organization_type": "Producer",
      "permissions": {
        "read": true,
        "write": true,
        "admin": false,
        "custom": {}
      },
      "is_active": true,
      "rate_limit_per_hour": 1000,
      "created_at": "2025-01-15T10:30:00Z",
      "expires_at": "2026-01-15T10:30:00Z"
    }
  },
  "message": "⚠️  SAVE THIS KEY NOW - IT WON'T BE SHOWN AGAIN"
}
```

**⚠️ IMPORTANT:** The full API key is shown **only once** at creation. Save it securely!

**Using API Keys:**

Option 1: X-API-Key Header (Recommended)
```bash
curl -X GET "https://connect.defarm.net/api/circuits" \
  -H "X-API-Key: dfm_a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6"
```

Option 2: Authorization Bearer Header
```bash
curl -X GET "https://connect.defarm.net/api/circuits" \
  -H "Authorization: Bearer dfm_a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6"
```

**API Key Characteristics:**
- ✅ Long-lived (configurable expiration)
- ✅ No session required
- ✅ Inherits all user permissions and tier limits
- ✅ Can be restricted to specific endpoints
- ✅ Can be restricted to specific IP addresses
- ✅ Individual rate limiting per key
- ✅ Ideal for integrations, apps, IoT devices

## API Keys

### Creating API Keys

**Endpoint:** `POST /api/api-keys`

**Request Body:**
```json
{
  "name": "Key Name",
  "permissions": {
    "read": true,
    "write": true,
    "admin": false,
    "custom": {
      "special_feature": true
    }
  },
  "allowed_endpoints": [],
  "allowed_ips": [],
  "rate_limit_per_hour": 1000,
  "expires_in_days": 365,
  "notes": "Optional description"
}
```

**Field Descriptions:**
- `name` (required): Human-readable name for the key
- `permissions` (optional): Permissions granted to this key
  - `read`: Allow read operations (default: true)
  - `write`: Allow write operations (default: false)
  - `admin`: Allow admin operations (default: false, requires admin user)
  - `custom`: Custom permissions map
- `allowed_endpoints` (optional): Empty array = all endpoints allowed. Otherwise, specify allowed endpoints like `["/api/circuits", "/api/items"]`
- `allowed_ips` (optional): Empty array = all IPs allowed. Otherwise, restrict to specific IPs
- `rate_limit_per_hour` (optional): Requests per hour (default: 100)
- `expires_in_days` (optional): Days until expiration (omit for no expiration)
- `notes` (optional): Internal notes about this key

### Listing API Keys

**Endpoint:** `GET /api/api-keys`

```bash
curl -X GET "https://connect.defarm.net/api/api-keys" \
  -H "Authorization: Bearer $TOKEN"
```

**Response:**
```json
{
  "success": true,
  "data": {
    "api_keys": [
      {
        "id": "key-uuid-123",
        "name": "Production Key",
        "key_prefix": "dfm_a1b2",
        "organization_type": "Producer",
        "permissions": { "read": true, "write": true, "admin": false },
        "is_active": true,
        "last_used_at": "2025-01-15T14:20:00Z",
        "usage_count": 1523,
        "rate_limit_per_hour": 1000,
        "created_at": "2025-01-01T10:00:00Z",
        "expires_at": "2026-01-01T10:00:00Z"
      }
    ]
  }
}
```

### Getting API Key Details

**Endpoint:** `GET /api/api-keys/{key_id}`

```bash
curl -X GET "https://connect.defarm.net/api/api-keys/key-uuid-123" \
  -H "Authorization: Bearer $TOKEN"
```

### Revoking API Keys

**Endpoint:** `PATCH /api/api-keys/{key_id}`

```bash
# Deactivate key
curl -X PATCH "https://connect.defarm.net/api/api-keys/key-uuid-123" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "is_active": false
  }'

# Reactivate key
curl -X PATCH "https://connect.defarm.net/api/api-keys/key-uuid-123" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "is_active": true
  }'
```

### Deleting API Keys

**Endpoint:** `DELETE /api/api-keys/{key_id}`

```bash
curl -X DELETE "https://connect.defarm.net/api/api-keys/key-uuid-123" \
  -H "Authorization: Bearer $TOKEN"
```

⚠️ **Warning:** Deletion is permanent and cannot be undone.

## Core Concepts

### Items and DFIDs

**LID (Local ID):** UUID generated when item is created locally
**DFID (DeFarm ID):** Globally unique ID assigned when item is tokenized in a circuit

**Item Lifecycle:**
1. Create local item → Gets LID (workspace-private)
2. Push to circuit → Gets DFID (circuit-global identity)
3. Item is now globally identifiable across the ecosystem

#### LID Persistence After Tokenization

**Important:** The LID does **not** disappear after tokenization. Here's what happens:

**During Local Creation:**
- User creates item locally → receives a **LID** (UUID)
- Item stored with temporary DFID format `"LID-{uuid}"`
- Item remains **workspace-private** at this stage

**After Push to Circuit (Tokenization):**
- Item pushed to circuit → assigned a **DFID**
- **LID-DFID mapping permanently stored** in the system
- Item can be queried by **either LID or DFID**
- Use `GET /api/items/mapping/{local_id}` to retrieve the mapping

**Visibility Rules:**

| Viewer | Can See LID? | Can See DFID? | Use Case |
|--------|--------------|---------------|----------|
| **Original Creator** | ✅ Yes | ✅ Yes | Both work for queries and references |
| **Other Circuit Members** | ❌ No | ✅ Yes | Only see global circuit identity |
| **In Circuit Operations** | ❌ No | ✅ Yes | All circuit views show DFID only |

**Summary:**
- **LID = Internal reference** for the creating workspace (persists forever with mapping)
- **DFID = Global identity** across all circuits (assigned at tokenization)
- Same real-world entity gets the **same DFID** across all circuits in the ecosystem

### Identifiers

**Canonical Identifiers:** Globally unique (SISBOV, CPF, CAR)
```json
{
  "namespace": "bovino",
  "key": "sisbov",
  "value": "BR12345678901234",
  "id_type": "Canonical",
  "verified": false
}
```

**Contextual Identifiers:** Locally unique (batch number, farm ID)
```json
{
  "namespace": "soja",
  "key": "lote",
  "value": "123",
  "id_type": "Contextual",
  "verified": false
}
```

### Circuits

Circuits are permission-controlled repositories for sharing items with blockchain storage.

**Circuit Roles:**
- **Owner:** Full control
- **Admin:** Can manage members and approve operations
- **Member:** Can push/pull based on circuit settings
- **Viewer:** Read-only access

## Common Workflows

### Workflow 1: Create and Tokenize an Item

```bash
TOKEN="your_jwt_token"

# Step 1: Create local item
LOCAL_RESPONSE=$(curl -s -X POST "https://connect.defarm.net/api/items/local" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "identifiers": [
      {
        "namespace": "bovino",
        "key": "sisbov",
        "value": "BR12345678901234",
        "id_type": "Canonical",
        "verified": false
      }
    ],
    "enriched_data": {
      "weight": "500kg",
      "breed": "Angus",
      "birth_date": "2024-01-15"
    }
  }')

LOCAL_ID=$(echo "$LOCAL_RESPONSE" | jq -r '.data.local_id')
echo "Local ID: $LOCAL_ID"

# Step 2: Push to circuit (tokenization)
CIRCUIT_ID="your-circuit-uuid"

PUSH_RESPONSE=$(curl -s -X POST "https://connect.defarm.net/api/circuits/$CIRCUIT_ID/push-local" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"local_id\": \"$LOCAL_ID\"
  }")

DFID=$(echo "$PUSH_RESPONSE" | jq -r '.data.dfid')
echo "DFID: $DFID"

# Step 3: Check storage history
curl -s "https://connect.defarm.net/api/items/$DFID/storage-history" \
  -H "Authorization: Bearer $TOKEN" | jq '.'
```

### Workflow 2: Using API Keys for Integration

```bash
# Get your API key (do this once, save the key securely)
TOKEN="your_jwt_token"

API_KEY_RESPONSE=$(curl -s -X POST "https://connect.defarm.net/api/api-keys" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Integration Key",
    "permissions": {
      "read": true,
      "write": true,
      "admin": false
    },
    "rate_limit_per_hour": 1000,
    "expires_in_days": 365
  }')

API_KEY=$(echo "$API_KEY_RESPONSE" | jq -r '.data.api_key')
echo "Save this API key: $API_KEY"

# Use API key for all subsequent requests
curl -X GET "https://connect.defarm.net/api/circuits" \
  -H "X-API-Key: $API_KEY"

curl -X GET "https://connect.defarm.net/api/items" \
  -H "X-API-Key: $API_KEY"
```

## Error Handling

All errors follow this format:

```json
{
  "error": "ERROR_CODE",
  "message": "Human-readable error message",
  "details": {
    "field": "Additional context"
  },
  "suggestions": [
    "Try this to fix the issue",
    "Or try this alternative"
  ]
}
```

**Common Error Codes:**

| Code | Status | Description | Solution |
|------|--------|-------------|----------|
| AUTHENTICATION_FAILED | 401 | Invalid credentials or token | Check username/password or refresh token |
| API_KEY_NOT_FOUND | 401 | API key not found | Verify API key is correct and active |
| API_KEY_EXPIRED | 401 | API key has expired | Create a new API key |
| ENDPOINT_NOT_ALLOWED | 403 | API key cannot access this endpoint | Update API key allowed_endpoints |
| IP_NOT_ALLOWED | 403 | Request from unauthorized IP | Update API key allowed_ips |
| PERMISSION_DENIED | 403 | Insufficient permissions | Contact admin to upgrade permissions |
| RATE_LIMIT_EXCEEDED | 429 | Too many requests | Wait and retry, or upgrade rate limit |
| ITEM_NOT_FOUND | 404 | Item doesn't exist | Check DFID is correct |
| CIRCUIT_NOT_FOUND | 404 | Circuit doesn't exist | Check circuit ID |

## Rate Limiting

**Response Headers:**
```
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 847
X-RateLimit-Reset: 1705324800
Retry-After: 3600
```

**When Rate Limited (429):**
```json
{
  "error": "RATE_LIMIT_EXCEEDED",
  "message": "Rate limit exceeded for this API key",
  "details": {
    "limit": 1000,
    "window": "hour",
    "retry_after_seconds": 3600
  },
  "suggestions": [
    "Wait 3600 seconds before retrying",
    "Consider upgrading your rate limit",
    "Implement exponential backoff"
  ]
}
```

## Best Practices

### Security
1. ✅ Never commit API keys to version control
2. ✅ Use environment variables for API keys
3. ✅ Rotate API keys regularly
4. ✅ Use specific endpoint restrictions when possible
5. ✅ Set expiration dates on API keys
6. ✅ Use HTTPS only (http will be upgraded automatically)

### Performance
1. ✅ Implement exponential backoff for retries
2. ✅ Cache responses when appropriate
3. ✅ Use connection pooling
4. ✅ Monitor rate limit headers
5. ✅ Implement request queuing to respect rate limits

### Error Handling
1. ✅ Always check HTTP status codes
2. ✅ Parse error messages for user feedback
3. ✅ Log errors for debugging
4. ✅ Implement retry logic for 5xx errors
5. ✅ Don't retry 4xx errors (client errors)

### API Keys
1. ✅ Use descriptive names for keys
2. ✅ Create separate keys for different environments (dev/staging/prod)
3. ✅ Create separate keys for different services
4. ✅ Monitor usage_count to detect issues
5. ✅ Deactivate unused keys
6. ✅ Delete compromised keys immediately

---

# Documentação em Português

## 📋 Índice
1. [Visão Geral](#visão-geral)
2. [URL Base](#url-base)
3. [Autenticação](#autenticação-1)
4. [Chaves de API](#chaves-de-api)
5. [Conceitos Principais](#conceitos-principais)
6. [Fluxos Comuns](#fluxos-comuns)
7. [Tratamento de Erros](#tratamento-de-erros)
8. [Limite de Taxa](#limite-de-taxa)
9. [Melhores Práticas](#melhores-práticas)

## Visão Geral

A API DeFarm Engines fornece um sistema completo para recepção de dados, tokenização e compartilhamento baseado em circuitos com capacidades de armazenamento blockchain.

**Recursos Principais:**
- 🔐 Autenticação dupla (JWT + Chaves de API)
- 🎯 Tokenização de itens com DFIDs
- 🔄 Compartilhamento de dados baseado em circuitos
- 📦 Armazenamento blockchain multi-adaptador
- 📊 Trilha de auditoria e rastreamento de eventos completos
- 🔔 Notificações em tempo real

## URL Base

**Produção:**
```
https://connect.defarm.net
```

Todos os endpoints da API são prefixados com `/api`, a menos que especificado de outra forma.

## Autenticação

### Método 1: Token JWT (Aplicações Web)

Tokens JWT são ideais para aplicações web com sessões de usuário.

**Passo 1: Login**
```bash
curl -X POST "https://connect.defarm.net/api/auth/login" \
  -H "Content-Type: application/json" \
  -d '{
    "username": "seu_usuario",
    "password": "sua_senha"
  }'
```

**Resposta:**
```json
{
  "success": true,
  "data": {
    "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "user_id": "user-123",
    "workspace_id": "workspace-456",
    "expires_in": 86400
  }
}
```

**Passo 2: Usar Token**
```bash
curl -X GET "https://connect.defarm.net/api/circuits" \
  -H "Authorization: Bearer SEU_TOKEN_JWT"
```

**Características do Token JWT:**
- ✅ Curta duração (24 horas por padrão)
- ✅ Vinculado à sessão do usuário
- ✅ Inclui contexto de usuário e workspace
- ✅ Ideal para aplicações frontend
- ⚠️ Deve ser atualizado periodicamente

### Método 2: Chaves de API (Integração Servidor-a-Servidor)

Chaves de API são ideais para integrações servidor-a-servidor, dispositivos IoT e aplicações de terceiros.

**Criando uma Chave de API:**
```bash
# Primeiro, autentique com JWT
TOKEN="seu_token_jwt"

# Criar chave de API
curl -X POST "https://connect.defarm.net/api/api-keys" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Chave de Integração Produção",
    "permissions": {
      "read": true,
      "write": true,
      "admin": false,
      "custom": {}
    },
    "rate_limit_per_hour": 1000,
    "expires_in_days": 365,
    "notes": "Chave principal de integração para sistema de produção"
  }'
```

**Resposta:**
```json
{
  "success": true,
  "data": {
    "api_key": "dfm_a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6",
    "metadata": {
      "id": "key-uuid-123",
      "name": "Chave de Integração Produção",
      "key_prefix": "dfm_a1b2",
      "organization_type": "Producer",
      "permissions": {
        "read": true,
        "write": true,
        "admin": false,
        "custom": {}
      },
      "is_active": true,
      "rate_limit_per_hour": 1000,
      "created_at": "2025-01-15T10:30:00Z",
      "expires_at": "2026-01-15T10:30:00Z"
    }
  },
  "message": "⚠️  SALVE ESTA CHAVE AGORA - ELA NÃO SERÁ MOSTRADA NOVAMENTE"
}
```

**⚠️ IMPORTANTE:** A chave de API completa é mostrada **apenas uma vez** na criação. Salve-a com segurança!

**Usando Chaves de API:**

Opção 1: Header X-API-Key (Recomendado)
```bash
curl -X GET "https://connect.defarm.net/api/circuits" \
  -H "X-API-Key: dfm_a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6"
```

Opção 2: Header Authorization Bearer
```bash
curl -X GET "https://connect.defarm.net/api/circuits" \
  -H "Authorization: Bearer dfm_a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6"
```

**Características da Chave de API:**
- ✅ Longa duração (expiração configurável)
- ✅ Não requer sessão
- ✅ Herda todas as permissões do usuário e limites de tier
- ✅ Pode ser restrita a endpoints específicos
- ✅ Pode ser restrita a endereços IP específicos
- ✅ Limitação de taxa individual por chave
- ✅ Ideal para integrações, apps, dispositivos IoT

## Chaves de API

### Criando Chaves de API

**Endpoint:** `POST /api/api-keys`

**Corpo da Requisição:**
```json
{
  "name": "Nome da Chave",
  "permissions": {
    "read": true,
    "write": true,
    "admin": false,
    "custom": {
      "recurso_especial": true
    }
  },
  "allowed_endpoints": [],
  "allowed_ips": [],
  "rate_limit_per_hour": 1000,
  "expires_in_days": 365,
  "notes": "Descrição opcional"
}
```

**Descrição dos Campos:**
- `name` (obrigatório): Nome legível para a chave
- `permissions` (opcional): Permissões concedidas a esta chave
  - `read`: Permitir operações de leitura (padrão: true)
  - `write`: Permitir operações de escrita (padrão: false)
  - `admin`: Permitir operações administrativas (padrão: false, requer usuário admin)
  - `custom`: Mapa de permissões personalizadas
- `allowed_endpoints` (opcional): Array vazio = todos endpoints permitidos. Caso contrário, especifique endpoints como `["/api/circuits", "/api/items"]`
- `allowed_ips` (opcional): Array vazio = todos IPs permitidos. Caso contrário, restringir a IPs específicos
- `rate_limit_per_hour` (opcional): Requisições por hora (padrão: 100)
- `expires_in_days` (opcional): Dias até expiração (omitir para sem expiração)
- `notes` (opcional): Notas internas sobre esta chave

### Listando Chaves de API

**Endpoint:** `GET /api/api-keys`

```bash
curl -X GET "https://connect.defarm.net/api/api-keys" \
  -H "Authorization: Bearer $TOKEN"
```

**Resposta:**
```json
{
  "success": true,
  "data": {
    "api_keys": [
      {
        "id": "key-uuid-123",
        "name": "Chave de Produção",
        "key_prefix": "dfm_a1b2",
        "organization_type": "Producer",
        "permissions": { "read": true, "write": true, "admin": false },
        "is_active": true,
        "last_used_at": "2025-01-15T14:20:00Z",
        "usage_count": 1523,
        "rate_limit_per_hour": 1000,
        "created_at": "2025-01-01T10:00:00Z",
        "expires_at": "2026-01-01T10:00:00Z"
      }
    ]
  }
}
```

### Obtendo Detalhes da Chave de API

**Endpoint:** `GET /api/api-keys/{key_id}`

```bash
curl -X GET "https://connect.defarm.net/api/api-keys/key-uuid-123" \
  -H "Authorization: Bearer $TOKEN"
```

### Revogando Chaves de API

**Endpoint:** `PATCH /api/api-keys/{key_id}`

```bash
# Desativar chave
curl -X PATCH "https://connect.defarm.net/api/api-keys/key-uuid-123" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "is_active": false
  }'

# Reativar chave
curl -X PATCH "https://connect.defarm.net/api/api-keys/key-uuid-123" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "is_active": true
  }'
```

### Excluindo Chaves de API

**Endpoint:** `DELETE /api/api-keys/{key_id}`

```bash
curl -X DELETE "https://connect.defarm.net/api/api-keys/key-uuid-123" \
  -H "Authorization: Bearer $TOKEN"
```

⚠️ **Aviso:** A exclusão é permanente e não pode ser desfeita.

## Conceitos Principais

### Itens e DFIDs

**LID (Local ID):** UUID gerado quando o item é criado localmente
**DFID (DeFarm ID):** ID globalmente único atribuído quando o item é tokenizado em um circuito

**Ciclo de Vida do Item:**
1. Criar item local → Obtém LID (privado no workspace)
2. Push para circuito → Obtém DFID (identidade global no circuito)
3. Item agora é globalmente identificável em todo o ecossistema

#### Persistência do LID Após Tokenização

**Importante:** O LID **não** desaparece após a tokenização. Veja o que acontece:

**Durante Criação Local:**
- Usuário cria item localmente → recebe um **LID** (UUID)
- Item armazenado com formato temporário de DFID `"LID-{uuid}"`
- Item permanece **privado no workspace** nesta fase

**Após Push para Circuito (Tokenização):**
- Item enviado para circuito → recebe um **DFID**
- **Mapeamento LID-DFID armazenado permanentemente** no sistema
- Item pode ser consultado por **LID ou DFID**
- Use `GET /api/items/mapping/{local_id}` para recuperar o mapeamento

**Regras de Visibilidade:**

| Visualizador | Vê o LID? | Vê o DFID? | Caso de Uso |
|--------------|-----------|------------|-------------|
| **Criador Original** | ✅ Sim | ✅ Sim | Ambos funcionam para consultas e referências |
| **Outros Membros do Circuito** | ❌ Não | ✅ Sim | Veem apenas a identidade global do circuito |
| **Em Operações do Circuito** | ❌ Não | ✅ Sim | Todas visualizações do circuito mostram apenas DFID |

**Resumo:**
- **LID = Referência interna** do workspace criador (persiste para sempre com mapeamento)
- **DFID = Identidade global** em todos os circuitos (atribuído na tokenização)
- Mesma entidade do mundo real recebe o **mesmo DFID** em todos os circuitos do ecossistema

### Identificadores

**Identificadores Canônicos:** Globalmente únicos (SISBOV, CPF, CAR)
```json
{
  "namespace": "bovino",
  "key": "sisbov",
  "value": "BR12345678901234",
  "id_type": "Canonical",
  "verified": false
}
```

**Identificadores Contextuais:** Localmente únicos (número de lote, ID de fazenda)
```json
{
  "namespace": "soja",
  "key": "lote",
  "value": "123",
  "id_type": "Contextual",
  "verified": false
}
```

### Circuitos

Circuitos são repositórios controlados por permissão para compartilhar itens com armazenamento blockchain.

**Papéis no Circuito:**
- **Owner:** Controle total
- **Admin:** Pode gerenciar membros e aprovar operações
- **Member:** Pode fazer push/pull baseado nas configurações do circuito
- **Viewer:** Acesso somente leitura

## Fluxos Comuns

### Fluxo 1: Criar e Tokenizar um Item

```bash
TOKEN="seu_token_jwt"

# Passo 1: Criar item local
LOCAL_RESPONSE=$(curl -s -X POST "https://connect.defarm.net/api/items/local" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "identifiers": [
      {
        "namespace": "bovino",
        "key": "sisbov",
        "value": "BR12345678901234",
        "id_type": "Canonical",
        "verified": false
      }
    ],
    "enriched_data": {
      "weight": "500kg",
      "breed": "Angus",
      "birth_date": "2024-01-15"
    }
  }')

LOCAL_ID=$(echo "$LOCAL_RESPONSE" | jq -r '.data.local_id')
echo "Local ID: $LOCAL_ID"

# Passo 2: Push para circuito (tokenização)
CIRCUIT_ID="seu-circuit-uuid"

PUSH_RESPONSE=$(curl -s -X POST "https://connect.defarm.net/api/circuits/$CIRCUIT_ID/push-local" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"local_id\": \"$LOCAL_ID\"
  }")

DFID=$(echo "$PUSH_RESPONSE" | jq -r '.data.dfid')
echo "DFID: $DFID"

# Passo 3: Verificar histórico de armazenamento
curl -s "https://connect.defarm.net/api/items/$DFID/storage-history" \
  -H "Authorization: Bearer $TOKEN" | jq '.'
```

### Fluxo 2: Usando Chaves de API para Integração

```bash
# Obter sua chave de API (faça isso uma vez, salve a chave com segurança)
TOKEN="seu_token_jwt"

API_KEY_RESPONSE=$(curl -s -X POST "https://connect.defarm.net/api/api-keys" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Chave de Integração",
    "permissions": {
      "read": true,
      "write": true,
      "admin": false
    },
    "rate_limit_per_hour": 1000,
    "expires_in_days": 365
  }')

API_KEY=$(echo "$API_KEY_RESPONSE" | jq -r '.data.api_key')
echo "Salve esta chave de API: $API_KEY"

# Use a chave de API para todas as requisições subsequentes
curl -X GET "https://connect.defarm.net/api/circuits" \
  -H "X-API-Key: $API_KEY"

curl -X GET "https://connect.defarm.net/api/items" \
  -H "X-API-Key: $API_KEY"
```

## Tratamento de Erros

Todos os erros seguem este formato:

```json
{
  "error": "CODIGO_ERRO",
  "message": "Mensagem de erro legível",
  "details": {
    "field": "Contexto adicional"
  },
  "suggestions": [
    "Tente isto para corrigir o problema",
    "Ou tente esta alternativa"
  ]
}
```

**Códigos de Erro Comuns:**

| Código | Status | Descrição | Solução |
|--------|--------|-----------|---------|
| AUTHENTICATION_FAILED | 401 | Credenciais ou token inválidos | Verifique usuário/senha ou atualize token |
| API_KEY_NOT_FOUND | 401 | Chave de API não encontrada | Verifique se a chave está correta e ativa |
| API_KEY_EXPIRED | 401 | Chave de API expirou | Crie uma nova chave de API |
| ENDPOINT_NOT_ALLOWED | 403 | Chave de API não pode acessar este endpoint | Atualize allowed_endpoints da chave |
| IP_NOT_ALLOWED | 403 | Requisição de IP não autorizado | Atualize allowed_ips da chave |
| PERMISSION_DENIED | 403 | Permissões insuficientes | Contate admin para atualizar permissões |
| RATE_LIMIT_EXCEEDED | 429 | Muitas requisições | Aguarde e tente novamente, ou atualize limite |
| ITEM_NOT_FOUND | 404 | Item não existe | Verifique se o DFID está correto |
| CIRCUIT_NOT_FOUND | 404 | Circuito não existe | Verifique o ID do circuito |

## Limite de Taxa

**Headers de Resposta:**
```
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 847
X-RateLimit-Reset: 1705324800
Retry-After: 3600
```

**Quando Limitado (429):**
```json
{
  "error": "RATE_LIMIT_EXCEEDED",
  "message": "Limite de taxa excedido para esta chave de API",
  "details": {
    "limit": 1000,
    "window": "hour",
    "retry_after_seconds": 3600
  },
  "suggestions": [
    "Aguarde 3600 segundos antes de tentar novamente",
    "Considere atualizar seu limite de taxa",
    "Implemente backoff exponencial"
  ]
}
```

## Melhores Práticas

### Segurança
1. ✅ Nunca commite chaves de API no controle de versão
2. ✅ Use variáveis de ambiente para chaves de API
3. ✅ Rotacione chaves de API regularmente
4. ✅ Use restrições de endpoint específicas quando possível
5. ✅ Defina datas de expiração nas chaves de API
6. ✅ Use apenas HTTPS (http será atualizado automaticamente)

### Performance
1. ✅ Implemente backoff exponencial para tentativas
2. ✅ Faça cache de respostas quando apropriado
3. ✅ Use pool de conexões
4. ✅ Monitore headers de limite de taxa
5. ✅ Implemente fila de requisições para respeitar limites

### Tratamento de Erros
1. ✅ Sempre verifique códigos de status HTTP
2. ✅ Parse mensagens de erro para feedback ao usuário
3. ✅ Registre erros para debugging
4. ✅ Implemente lógica de retry para erros 5xx
5. ✅ Não tente novamente erros 4xx (erros do cliente)

### Chaves de API
1. ✅ Use nomes descritivos para chaves
2. ✅ Crie chaves separadas para diferentes ambientes (dev/staging/prod)
3. ✅ Crie chaves separadas para diferentes serviços
4. ✅ Monitore usage_count para detectar problemas
5. ✅ Desative chaves não utilizadas
6. ✅ Delete imediatamente chaves comprometidas

---

## 📞 Support | Suporte

**English:**
- Documentation: https://connect.defarm.net/docs
- Support: support@defarm.net
- Status Page: https://status.defarm.net

**Português:**
- Documentação: https://connect.defarm.net/docs
- Suporte: suporte@defarm.net
- Página de Status: https://status.defarm.net

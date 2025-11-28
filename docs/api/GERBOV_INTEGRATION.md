# DeFarm API - Gerbov Integration Guide

**Última atualização:** 27 de Novembro de 2025
**Versão da API:** 1.0
**URL de Produção:** `https://connect.defarm.net`

---

## Índice

1. [Credenciais de Acesso](#credenciais-de-acesso)
2. [Autenticação](#autenticação)
3. [Chaves de API](#chaves-de-api)
4. [Fluxo Completo de Rastreabilidade](#fluxo-completo-de-rastreabilidade)
5. [Endpoints Principais](#endpoints-principais)
6. [Códigos de Erro](#códigos-de-erro)
7. [Exemplos em Bash](#exemplos-em-bash)
8. [Troubleshooting](#troubleshooting)

---

## Credenciais de Acesso

```
Nome de Usuário: gerbov
Senha: Gerbov2024!Test
ID do Usuário: user-2da9af70-c4c3-4b13-9180-dc1c7094b27c
Plano: Professional
Email: gerbov@testclient.com
```

> **IMPORTANTE:** Estas credenciais são exclusivas para ambiente de testes e desenvolvimento. Para produção real, você receberá credenciais dedicadas.

---

## Autenticação

A API suporta dois métodos de autenticação:

### 1. JWT Token (Recomendado para sessões interativas)

```bash
# Fazer login e obter token
curl -X POST "https://connect.defarm.net/api/auth/login" \
  -H "Content-Type: application/json" \
  -d '{"username":"gerbov","password":"Gerbov2024!Test"}'
```

**Resposta:**
```json
{
  "token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "user_id": "user-2da9af70-c4c3-4b13-9180-dc1c7094b27c",
  "workspace_id": "Gerbov Workspace-workspace",
  "expires_at": 1764339762
}
```

**Usar o token em requisições:**
```bash
curl -X GET "https://connect.defarm.net/api/circuits" \
  -H "Authorization: Bearer SEU_TOKEN_AQUI"
```

- Token válido por **24 horas**
- Use o endpoint `/api/auth/refresh` para renovar

### 2. API Key (Recomendado para integrações automatizadas)

```bash
# Usar API Key via header X-API-Key
curl -X GET "https://connect.defarm.net/api/circuits" \
  -H "X-API-Key: dfm_sua_chave_aqui"

# OU via Authorization header
curl -X GET "https://connect.defarm.net/api/circuits" \
  -H "Authorization: Bearer dfm_sua_chave_aqui"
```

---

## Chaves de API

### Criar uma Chave de API

Primeiro, faça login para obter um JWT token. Depois, crie a chave:

```bash
TOKEN="seu_jwt_token"

curl -X POST "https://connect.defarm.net/api/api-keys" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Integração ERP Gerbov",
    "organization_type": "Producer",
    "permissions": {
      "read": true,
      "write": true,
      "admin": false,
      "custom": {}
    },
    "rate_limit_per_hour": 1000,
    "expires_in_days": 90
  }'
```

**Resposta:**
```json
{
  "api_key": "dfm_hvvlLiRyyBfbnWfdk5IHWfS0KrmwiyZc",
  "metadata": {
    "id": "d9846ddc-150c-4672-a77e-ecc0fd03050c",
    "name": "Integração ERP Gerbov",
    "key_prefix": "dfm_9VvJ",
    "organization_type": "Producer",
    "is_active": true,
    "rate_limit_per_hour": 1000,
    "expires_at": "2026-02-25T14:25:29Z"
  },
  "warning": "Save this API key securely. You won't be able to see it again."
}
```

> **ATENÇÃO:** A chave completa (`api_key`) é mostrada **APENAS UMA VEZ**. Salve em local seguro!

### Gerenciar Chaves

```bash
# Listar suas chaves
curl -X GET "https://connect.defarm.net/api/api-keys" \
  -H "Authorization: Bearer $TOKEN"

# Revogar uma chave
curl -X POST "https://connect.defarm.net/api/api-keys/{key_id}/revoke" \
  -H "Authorization: Bearer $TOKEN"

# Deletar uma chave
curl -X DELETE "https://connect.defarm.net/api/api-keys/{key_id}" \
  -H "Authorization: Bearer $TOKEN"
```

---

## Fluxo Completo de Rastreabilidade

### Passo 1: Criar um Circuito

Circuitos são repositórios onde itens são tokenizados e compartilhados.

```bash
curl -X POST "https://connect.defarm.net/api/circuits" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Rastreabilidade Gado Gerbov 2025",
    "description": "Circuito para rastreamento de bovinos"
  }'
```

**Resposta:**
```json
{
  "circuit_id": "a96387f1-1cc8-4f95-ac59-2fa6649f954f",
  "name": "Rastreabilidade Gado Gerbov 2025",
  "owner_id": "user-2da9af70-c4c3-4b13-9180-dc1c7094b27c",
  "status": "Active"
}
```

> **Nota:** Para usar blockchain (Stellar + IPFS), o tier Professional não tem acesso ao StellarMainnetIpfs. Circuitos simples funcionam sem adaptador.

### Passo 2: Criar Item Local

Crie um item localmente. Ele recebe um LID (Local ID) mas ainda não tem DFID.

```bash
CIRCUIT_ID="a96387f1-1cc8-4f95-ac59-2fa6649f954f"

curl -X POST "https://connect.defarm.net/api/items/local" \
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
      },
      {
        "namespace": "bovino",
        "key": "brinco",
        "value": "GERBOV-2024-001",
        "id_type": "Contextual",
        "verified": true
      }
    ],
    "enriched_data": {
      "peso_kg": "520",
      "raca": "Angus",
      "data_nascimento": "2023-05-15",
      "fazenda": "Fazenda Gerbov"
    }
  }'
```

**Resposta:**
```json
{
  "success": true,
  "data": {
    "local_id": "bb257aa2-52a9-45de-9531-4bf1eb73dc03",
    "status": "LocalOnly"
  }
}
```

### Passo 3: Tokenizar (Push para Circuito)

Envie o item para um circuito para gerar o DFID permanente:

```bash
LOCAL_ID="bb257aa2-52a9-45de-9531-4bf1eb73dc03"

curl -X POST "https://connect.defarm.net/api/circuits/$CIRCUIT_ID/push-local" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"local_id\":\"$LOCAL_ID\"}"
```

**Resposta (sucesso):**
```json
{
  "success": true,
  "data": {
    "dfid": "DFID-20251127-000001-A3F2",
    "local_id": "bb257aa2-52a9-45de-9531-4bf1eb73dc03",
    "status": "NewItemCreated",
    "operation_id": "eb4e450a-f9fa-4a56-80d8-851690d64f46"
  }
}
```

### Passo 4: Consultar Item

```bash
DFID="DFID-20251127-000001-A3F2"

# Obter detalhes do item
curl -X GET "https://connect.defarm.net/api/items/$DFID" \
  -H "Authorization: Bearer $TOKEN"

# Obter histórico de armazenamento
curl -X GET "https://connect.defarm.net/api/items/$DFID/storage-history" \
  -H "Authorization: Bearer $TOKEN"

# Obter timeline
curl -X GET "https://connect.defarm.net/api/items/$DFID/timeline" \
  -H "Authorization: Bearer $TOKEN"
```

---

## Endpoints Principais

| Endpoint | Método | Descrição |
|----------|--------|-----------|
| `/api/auth/login` | POST | Autenticação e obtenção de JWT |
| `/api/auth/profile` | GET | Perfil do usuário autenticado |
| `/api/circuits` | GET | Listar circuitos |
| `/api/circuits` | POST | Criar circuito |
| `/api/circuits/{id}` | GET | Detalhes do circuito |
| `/api/items/local` | POST | Criar item local |
| `/api/circuits/{id}/push-local` | POST | Tokenizar item (push) |
| `/api/items/{dfid}` | GET | Obter item por DFID |
| `/api/items/{dfid}/storage-history` | GET | Histórico de armazenamento |
| `/api/items/{dfid}/timeline` | GET | Timeline do item |
| `/api/api-keys` | GET/POST | Gerenciar chaves de API |
| `/api/notifications` | GET | Listar notificações |
| `/health` | GET | Status da API |
| `/health/db` | GET | Status do banco de dados |

---

## Códigos de Erro

| Código | Erro | Significado | Solução |
|--------|------|-------------|---------|
| 400 | BAD_REQUEST | Dados inválidos | Verifique o JSON enviado |
| 401 | UNAUTHORIZED | Token inválido/expirado | Faça login novamente |
| 403 | FORBIDDEN | Sem permissão | Verifique suas permissões ou tier |
| 404 | NOT_FOUND | Recurso não encontrado | Verifique o ID |
| 409 | CONFLICT | Conflito (duplicado) | Use identificador diferente |
| 429 | RATE_LIMIT | Muitas requisições | Aguarde e tente novamente |
| 500 | INTERNAL_ERROR | Erro interno | Contate o suporte |

**Formato de erro:**
```json
{
  "error": "UNAUTHORIZED",
  "message": "Token JWT expirado",
  "suggestions": [
    "Faça login novamente para obter um novo token"
  ]
}
```

---

## Exemplos em Bash

### Script Completo de Teste

```bash
#!/bin/bash
set -e

API="https://connect.defarm.net/api"

echo "=== 1. Login ==="
TOKEN=$(curl -s -X POST "$API/auth/login" \
  -H "Content-Type: application/json" \
  -d '{"username":"gerbov","password":"Gerbov2024!Test"}' | jq -r '.token')
echo "Token: ${TOKEN:0:30}..."

echo ""
echo "=== 2. Criar Circuito ==="
CIRCUIT_RESPONSE=$(curl -s -X POST "$API/circuits" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"Teste Gerbov '$(date +%s)'","description":"Circuito de teste"}')
CIRCUIT_ID=$(echo "$CIRCUIT_RESPONSE" | jq -r '.circuit_id')
echo "Circuit ID: $CIRCUIT_ID"

echo ""
echo "=== 3. Criar Item Local ==="
LOCAL_RESPONSE=$(curl -s -X POST "$API/items/local" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "identifiers": [{
      "namespace": "bovino",
      "key": "sisbov",
      "value": "BR'$(date +%s)'",
      "id_type": "Canonical",
      "verified": false
    }],
    "enriched_data": {"peso_kg": "500", "raca": "Nelore"}
  }')
LOCAL_ID=$(echo "$LOCAL_RESPONSE" | jq -r '.data.local_id')
echo "Local ID: $LOCAL_ID"

echo ""
echo "=== 4. Criar Chave de API ==="
API_KEY_RESPONSE=$(curl -s -X POST "$API/api-keys" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name":"Teste '$(date +%s)'",
    "organization_type":"Producer",
    "permissions":{"read":true,"write":true,"admin":false,"custom":{}},
    "rate_limit_per_hour":100,
    "expires_in_days":1
  }')
API_KEY=$(echo "$API_KEY_RESPONSE" | jq -r '.api_key')
echo "API Key: $API_KEY"

echo ""
echo "=== 5. Testar API Key ==="
curl -s -X GET "$API/circuits" -H "X-API-Key: $API_KEY" | jq '.'

echo ""
echo "=== Teste Completo! ==="
```

### Script de Verificação Rápida

```bash
#!/bin/bash
API="https://connect.defarm.net"

echo "Verificando API..."
curl -s "$API/health" | jq '.'

echo ""
echo "Verificando banco de dados..."
curl -s "$API/health/db" | jq '.'
```

---

## Troubleshooting

### Login falha
- Verifique username/senha exatos (case-sensitive)
- Senha contém caractere especial `!` - use aspas simples no JSON

### Push falha com erro de storage
- Verificar se o circuito existe e está ativo
- Se erro "db error", pode ser problema temporário - aguarde alguns minutos

### Token expirado
- Tokens JWT duram 24 horas
- Use `/api/auth/refresh` ou faça novo login

### Rate limit
- Limite padrão: 100 requests/hora
- Crie API key com limite maior se necessário
- Aguarde o período indicado em `retry_after`

### Tier não tem acesso ao adaptador
- Professional tier não tem acesso ao StellarMainnetIpfs
- Crie circuitos sem adapter_config ou use StellarTestnetIpfs (quando disponível)

---

## Suporte

- **Email:** suporte@defarm.net
- **Documentação completa:** https://docs.defarm.net
- **Status da API:** https://connect.defarm.net/health

---

**Versão do documento:** 3.0
**Última verificação em produção:** 27 de Novembro de 2025

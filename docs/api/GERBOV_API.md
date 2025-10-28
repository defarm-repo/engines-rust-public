# 🔷 Documentação API DeFarm - Gerbov

**Última atualização:** 28 de Outubro de 2025
**Status:** ✅ Todos os endpoints testados e funcionando
**URL da API:** `https://connect.defarm.net`

---

## 📋 Índice

1. [Credenciais de Acesso](#-credenciais-de-acesso)
2. [Seu Circuito Ativo](#-seu-circuito-ativo)
3. [Autenticação](#-autenticação)
4. [Chaves de API](#-chaves-de-api)
5. [Gerenciamento de Itens](#-gerenciamento-de-itens)
6. [Operações de Circuito](#-operações-de-circuito)
7. [Histórico e Rastreabilidade](#-histórico-e-rastreabilidade)
8. [Códigos de Erro](#-códigos-de-erro)
9. [Exemplos Completos](#-exemplos-completos)

---

## 🔐 Credenciais de Acesso

```
Nome de Usuário: gerbov
Senha: Gerbov2024!Test
ID do Usuário: user-2da9af70-c4c3-4b13-9180-dc1c7094b27c
Plano: Professional
```

**⚠️ IMPORTANTE:** Estas credenciais são para ambiente de testes. Em produção, você receberá credenciais específicas.

---

## 🔷 Seu Circuito Ativo

```
ID do Circuito: 002ea6db-6b7b-4a69-8780-1f01ae074265
Nome: Gerbov Test Circuit
Proprietário: gerbov (você)
Status: ✅ Ativo e Funcionando
Adaptador: StellarMainnetIpfs
```

---

## 🔑 Autenticação

### 1. Fazer Login

Obtenha um token JWT para autenticar suas requisições:

```bash
curl -X POST "https://connect.defarm.net/api/auth/login" \
  -H "Content-Type: application/json" \
  -d '{
    "username": "gerbov",
    "password": "Gerbov2024!Test"
  }'
```

**Resposta de Sucesso:**
```json
{
  "token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "user_id": "user-2da9af70-c4c3-4b13-9180-dc1c7094b27c",
  "expires_in": 86400
}
```

**O que fazer com o token:**
- Salve o campo `token`
- Use em todas as requisições seguintes: `Authorization: Bearer SEU_TOKEN`
- Token válido por 24 horas

---

## 🔐 Chaves de API

### O que são Chaves de API?

Chaves de API são tokens de longa duração para integrações automatizadas, scripts ou aplicações que não podem fazer login interativo.

### Criar uma Chave de API

```bash
TOKEN="seu_jwt_token_aqui"

curl -X POST "https://connect.defarm.net/api/api-keys" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Integração Produção Gerbov",
    "organization_type": "Producer",
    "permissions": {
      "read": true,
      "write": true,
      "admin": false,
      "custom": {}
    },
    "rate_limit_per_hour": 1000,
    "expires_in_days": 90,
    "notes": "Chave para sistema de rastreamento"
  }'
```

**Resposta:**
```json
{
  "api_key": "dfm_a1b2c3d4e5f6g7h8...",
  "metadata": {
    "id": "uuid-da-chave",
    "name": "Integração Produção Gerbov",
    "created_at": "2025-10-28T10:00:00Z",
    "expires_at": "2026-01-26T10:00:00Z"
  },
  "warning": "Salve esta chave em local seguro. Você não poderá visualizá-la novamente."
}
```

**⚠️ IMPORTANTE:**
- A chave completa é mostrada APENAS UMA VEZ
- Salve em local seguro (variável de ambiente, cofre de senhas)
- Nunca commit em código ou repositórios Git

### Usar Chave de API

```bash
API_KEY="dfm_a1b2c3d4e5f6g7h8..."

curl -X GET "https://connect.defarm.net/api/circuits" \
  -H "X-API-Key: $API_KEY"
```

### Listar Suas Chaves

```bash
curl -X GET "https://connect.defarm.net/api/api-keys" \
  -H "Authorization: Bearer $TOKEN"
```

### Revogar uma Chave

```bash
KEY_ID="uuid-da-chave"

curl -X POST "https://connect.defarm.net/api/api-keys/$KEY_ID/revoke" \
  -H "Authorization: Bearer $TOKEN"
```

### Deletar uma Chave

```bash
curl -X DELETE "https://connect.defarm.net/api/api-keys/$KEY_ID" \
  -H "Authorization: Bearer $TOKEN"
```

---

## 📦 Gerenciamento de Itens

### 1. Criar Item Local (sem DFID ainda)

Crie um item localmente. Ele recebe um LID (Local ID) mas ainda não tem DFID.

```bash
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
    "local_id": "f8e4d20e-922a-4f20-b190-1477fbff1274",
    "status": "local",
    "identifiers": [
      {
        "namespace": "bovino",
        "key": "sisbov",
        "value": "BR12345678901234"
      }
    ],
    "created_at": "2025-10-28T10:15:00Z"
  },
  "message": "Item criado localmente. Use push-local para tokenizar."
}
```

### 2. Enviar para Circuito (Tokenização)

Agora envie o item para um circuito. Isto gera o DFID permanente.

```bash
LOCAL_ID="f8e4d20e-922a-4f20-b190-1477fbff1274"
CIRCUIT_ID="002ea6db-6b7b-4a69-8780-1f01ae074265"

curl -X POST "https://connect.defarm.net/api/circuits/$CIRCUIT_ID/push-local" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "local_id": "'$LOCAL_ID'",
    "requester_id": "gerbov"
  }'
```

**Resposta:**
```json
{
  "success": true,
  "data": {
    "dfid": "DFID-20251028-000042-A3F2",
    "local_id": "f8e4d20e-922a-4f20-b190-1477fbff1274",
    "circuit_id": "002ea6db-6b7b-4a69-8780-1f01ae074265",
    "blockchain_location": {
      "adapter_type": "StellarMainnetIpfs",
      "ipfs_cid": "QmX7Yh3K9...",
      "stellar_tx": "a1b2c3d4...",
      "timestamp": "2025-10-28T10:16:30Z"
    },
    "status": "tokenized"
  },
  "message": "Item tokenizado com sucesso"
}
```

**O que aconteceu:**
1. ✅ DFID permanente foi gerado
2. ✅ Dados foram enviados para IPFS
3. ✅ Transação foi registrada na blockchain Stellar
4. ✅ Item agora é rastreável globalmente

### 3. Consultar Item por DFID

```bash
DFID="DFID-20251028-000042-A3F2"

curl -X GET "https://connect.defarm.net/api/items/$DFID" \
  -H "Authorization: Bearer $TOKEN"
```

**Resposta:**
```json
{
  "dfid": "DFID-20251028-000042-A3F2",
  "identifiers": [
    {
      "namespace": "bovino",
      "key": "sisbov",
      "value": "BR12345678901234"
    },
    {
      "namespace": "bovino",
      "key": "brinco",
      "value": "GERBOV-2024-001"
    }
  ],
  "enriched_data": {
    "peso_kg": "520",
    "raca": "Angus"
  },
  "created_at": "2025-10-28T10:15:00Z",
  "lifecycle": "Active"
}
```

### 4. Consultar Histórico de Armazenamento

Veja todos os locais onde o item foi armazenado (IPFS, blockchains, etc):

```bash
curl -X GET "https://connect.defarm.net/api/items/$DFID/storage-history" \
  -H "Authorization: Bearer $TOKEN"
```

**Resposta:**
```json
{
  "dfid": "DFID-20251028-000042-A3F2",
  "storage_locations": [
    {
      "adapter_type": "StellarMainnetIpfs",
      "ipfs_cid": "QmX7Yh3K9abc123...",
      "ipfs_gateway_url": "https://ipfs.io/ipfs/QmX7Yh3K9abc123...",
      "stellar_network": "mainnet",
      "stellar_tx_hash": "a1b2c3d4e5f6...",
      "stellar_explorer_url": "https://stellarchain.io/tx/a1b2c3d4e5f6...",
      "stored_at": "2025-10-28T10:16:30Z",
      "status": "confirmed"
    }
  ],
  "total_locations": 1
}
```

---

## 🔷 Operações de Circuito

### Listar Seus Circuitos

```bash
curl -X GET "https://connect.defarm.net/api/circuits" \
  -H "Authorization: Bearer $TOKEN"
```

### Detalhes de um Circuito

```bash
CIRCUIT_ID="002ea6db-6b7b-4a69-8780-1f01ae074265"

curl -X GET "https://connect.defarm.net/api/circuits/$CIRCUIT_ID" \
  -H "Authorization: Bearer $TOKEN"
```

### Criar Novo Circuito

```bash
curl -X POST "https://connect.defarm.net/api/circuits" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Rastreabilidade Gado Gerbov 2025",
    "description": "Circuito para rastrear gado da Fazenda Gerbov",
    "adapter_config": {
      "adapter_type": "StellarMainnetIpfs",
      "requires_approval": false,
      "auto_migrate_existing": false,
      "sponsor_adapter_access": true
    }
  }'
```

### Listar Itens do Circuito

```bash
curl -X GET "https://connect.defarm.net/api/circuits/$CIRCUIT_ID/items" \
  -H "Authorization: Bearer $TOKEN"
```

---

## 📊 Histórico e Rastreabilidade

### Eventos de um Item

Veja todo o histórico de um item:

```bash
curl -X GET "https://connect.defarm.net/api/items/$DFID/events" \
  -H "Authorization: Bearer $TOKEN"
```

**Resposta:**
```json
{
  "dfid": "DFID-20251028-000042-A3F2",
  "events": [
    {
      "event_id": "evt-001",
      "event_type": "Created",
      "timestamp": "2025-10-28T10:15:00Z",
      "source": "gerbov",
      "description": "Item criado localmente"
    },
    {
      "event_id": "evt-002",
      "event_type": "Tokenized",
      "timestamp": "2025-10-28T10:16:30Z",
      "source": "gerbov",
      "circuit_id": "002ea6db-6b7b-4a69-8780-1f01ae074265",
      "description": "Item tokenizado e enviado para blockchain"
    }
  ],
  "total_events": 2
}
```

### Adicionar Evento Customizado

```bash
curl -X POST "https://connect.defarm.net/api/events" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "dfid": "'$DFID'",
    "event_type": "Custom",
    "description": "Animal vacinado contra febre aftosa",
    "metadata": {
      "vacina": "Aftosa Premium",
      "veterinario": "Dr. Silva",
      "data_aplicacao": "2025-10-28"
    },
    "visibility": "Public"
  }'
```

---

## ❌ Códigos de Erro

| Código HTTP | Erro | Significado | Solução |
|-------------|------|-------------|---------|
| 400 | `BAD_REQUEST` | Dados inválidos na requisição | Verifique o JSON enviado |
| 401 | `UNAUTHORIZED` | Token inválido ou expirado | Faça login novamente |
| 403 | `FORBIDDEN` | Sem permissão para esta operação | Verifique suas permissões |
| 404 | `NOT_FOUND` | Recurso não encontrado | Verifique o ID do item/circuito |
| 409 | `CONFLICT` | Item já existe | Use outro identificador |
| 429 | `RATE_LIMIT_EXCEEDED` | Muitas requisições | Aguarde e tente novamente |
| 500 | `INTERNAL_ERROR` | Erro no servidor | Contate o suporte |
| 503 | `SERVICE_UNAVAILABLE` | Serviço temporariamente indisponível | Tente novamente em alguns segundos |

**Formato de Resposta de Erro:**
```json
{
  "error": "UNAUTHORIZED",
  "message": "Token JWT expirado",
  "suggestions": [
    "Faça login novamente para obter um novo token",
    "Verifique se o token está sendo enviado corretamente"
  ]
}
```

---

## 💡 Exemplos Completos

### Fluxo Completo: Da Criação à Rastreabilidade

```bash
#!/bin/bash

# 1. Login
echo "1. Fazendo login..."
TOKEN=$(curl -s -X POST "https://connect.defarm.net/api/auth/login" \
  -H "Content-Type: application/json" \
  -d '{"username":"gerbov","password":"Gerbov2024!Test"}' | jq -r '.token')

echo "✅ Token obtido"

# 2. Criar item local
echo "2. Criando item local..."
LOCAL_RESPONSE=$(curl -s -X POST "https://connect.defarm.net/api/items/local" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "identifiers": [
      {
        "namespace": "bovino",
        "key": "sisbov",
        "value": "BR98765432109876",
        "id_type": "Canonical",
        "verified": false
      }
    ],
    "enriched_data": {
      "peso_kg": "550",
      "raca": "Nelore"
    }
  }')

LOCAL_ID=$(echo "$LOCAL_RESPONSE" | jq -r '.data.local_id')
echo "✅ Item criado: $LOCAL_ID"

# 3. Tokenizar (enviar para circuito)
echo "3. Tokenizando item..."
CIRCUIT_ID="002ea6db-6b7b-4a69-8780-1f01ae074265"

PUSH_RESPONSE=$(curl -s -X POST "https://connect.defarm.net/api/circuits/$CIRCUIT_ID/push-local" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"local_id\":\"$LOCAL_ID\",\"requester_id\":\"gerbov\"}")

DFID=$(echo "$PUSH_RESPONSE" | jq -r '.data.dfid')
echo "✅ Item tokenizado: $DFID"

# 4. Consultar histórico de armazenamento
echo "4. Consultando histórico de armazenamento..."
curl -s -X GET "https://connect.defarm.net/api/items/$DFID/storage-history" \
  -H "Authorization: Bearer $TOKEN" | jq '.'

echo "✅ Fluxo completo executado com sucesso!"
```

### Criar Chave de API para Integração

```bash
#!/bin/bash

# Login
TOKEN=$(curl -s -X POST "https://connect.defarm.net/api/auth/login" \
  -H "Content-Type: application/json" \
  -d '{"username":"gerbov","password":"Gerbov2024!Test"}' | jq -r '.token')

# Criar chave de API
API_KEY_RESPONSE=$(curl -s -X POST "https://connect.defarm.net/api/api-keys" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Sistema ERP Gerbov",
    "organization_type": "Producer",
    "permissions": {
      "read": true,
      "write": true,
      "admin": false,
      "custom": {}
    },
    "rate_limit_per_hour": 5000,
    "expires_in_days": 365
  }')

API_KEY=$(echo "$API_KEY_RESPONSE" | jq -r '.api_key')

echo "============================================"
echo "🔑 SUA CHAVE DE API (SALVE EM LOCAL SEGURO):"
echo "$API_KEY"
echo "============================================"
echo ""
echo "⚠️  IMPORTANTE: Esta chave não será mostrada novamente!"
echo "Salve em arquivo .env ou cofre de senhas."
echo ""
echo "Exemplo de uso:"
echo "export DEFARM_API_KEY=\"$API_KEY\""
```

---

## 📞 Suporte

**Dúvidas ou problemas?**

- 📧 Email: suporte@defarm.net
- 📱 WhatsApp: +55 11 9XXXX-XXXX
- 🌐 Documentação completa: https://docs.defarm.net

**Horário de atendimento:**
- Segunda a Sexta: 9h às 18h (horário de Brasília)
- Sábados: 9h às 13h
- Emergências 24/7: suporte-urgente@defarm.net

---

**Última atualização:** 28 de Outubro de 2025
**Versão da API:** 1.0
**Status:** ✅ Produção

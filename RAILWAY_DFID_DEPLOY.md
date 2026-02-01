# Deploy DFID Service no Railway - Passo a Passo

## Pré-requisitos

✅ Você já tem:
- Railway account
- Railway CLI instalado
- Token Railway configurado: `fb76b340-b105-4172-b4bf-4dcb894225a8`
- Projeto DeFarm linkado (ID: `2e6d7cdb-f993-4411-bcf4-1844f5b38011`)

## Passo 1: Commit e Push das Mudanças

```bash
# No diretório /Users/gabrielrondon/rust/engines

# Ver o que foi modificado
git status

# Adicionar novos arquivos
git add dfid-service/
git add src/dfid_client.rs
git add src/items_engine.rs
git add src/circuits_engine.rs
git add src/api/shared_state.rs
git add src/bin/api.rs
git add docker-compose.yml
git add *.md
git add scripts/

# Commit
git commit -m "feat: separate DFID generation into external service

- Create dfid-service microservice with BLAKE3 checksums
- Add DfidClient for HTTP communication
- Update ItemsEngine and CircuitsEngine for hybrid mode
- Add Docker Compose for local development
- Include comprehensive documentation and migration guide"

# Push
git push origin main
```

## Passo 2: Criar Serviço DFID no Railway

### Opção A: Via Dashboard (Recomendado)

1. Acesse: https://railway.app/project/2e6d7cdb-f993-4411-bcf4-1844f5b38011

2. Clique em **"+ New Service"**

3. Selecione **"Deploy from GitHub repo"**

4. Escolha o repositório **defarm-engines**

5. Configure:
   - **Service Name**: `defarm-dfid-service`
   - **Root Directory**: `/dfid-service`
   - **Build Command**: Auto-detected (Dockerfile)
   - **Start Command**: Auto-detected

6. Clique em **"Deploy"**

### Opção B: Via CLI

```bash
# Autenticar (se necessário)
export RAILWAY_TOKEN=fb76b340-b105-4172-b4bf-4dcb894225a8

# Criar novo serviço
railway service create defarm-dfid-service

# Deploy
railway up --detach
```

## Passo 3: Configurar Variáveis de Ambiente

Via Dashboard ou CLI:

```bash
# Via CLI
railway service defarm-dfid-service

# Configurar variáveis
railway variables set PORT=3001
railway variables set REDIS_URL='${{Redis.REDIS_URL}}'
railway variables set RUST_LOG=info

# Verificar
railway variables
```

Via Dashboard:
1. Vá em **Variables**
2. Adicione:
   - `PORT` = `3001`
   - `REDIS_URL` = `${{Redis.REDIS_URL}}` (referência ao serviço Redis)
   - `RUST_LOG` = `info`

## Passo 4: Aguardar Deploy e Testar

```bash
# Ver logs do build
railway logs --service defarm-dfid-service

# Aguardar até ver:
# "DFID Service listening on 0.0.0.0:3001"

# Obter URL do serviço
railway status --service defarm-dfid-service

# Testar health check
curl https://defarm-dfid-service-production.up.railway.app/health

# Deve retornar:
# {"status":"healthy","current_sequence":1}
```

## Passo 5: Testar DFID Generation

```bash
# Gerar DFID
curl -X POST https://defarm-dfid-service-production.up.railway.app/dfid/generate \
  -H "Content-Type: application/json" \
  -d '{"count": 1}'

# Resposta esperada:
{
  "dfids": ["DFID-20250131-000001-A7B2C3"],
  "format_version": "1.0",
  "generated_at": "2025-01-31T..."
}

# Validar DFID
curl https://defarm-dfid-service-production.up.railway.app/dfid/DFID-20250131-000001-A7B2C3/validate

# Resposta esperada:
{"valid":true}
```

## Passo 6: Conectar API ao DFID Service

```bash
# Adicionar variável ao serviço da API
railway service defarm-engines-api

railway variables set DFID_SERVICE_URL=https://defarm-dfid-service-production.up.railway.app

# Redeploy da API
railway redeploy
```

## Passo 7: Verificar Integração

```bash
# Ver logs da API
railway logs --service defarm-engines-api | grep DFID

# Deve ver:
# "✨ DFID Service client enabled - using remote DFID generation"

# Testar criação de item (precisa de token JWT)
# Isso vai usar o DFID Service remotamente
```

## Troubleshooting

### Erro: "Service not found"

```bash
# Listar todos os serviços
railway list

# Verificar se está no projeto correto
railway status
```

### Erro: "Build failed"

```bash
# Ver logs detalhados
railway logs -b --service defarm-dfid-service

# Verificar Dockerfile
cat dfid-service/Dockerfile
```

### Erro: "Redis connection failed"

```bash
# Verificar se Redis está rodando
railway status --service redis

# Verificar variável
railway variables get REDIS_URL --service defarm-dfid-service
```

### API não está usando DFID Service

```bash
# Verificar variável
railway variables get DFID_SERVICE_URL --service defarm-engines-api

# Deve retornar a URL do DFID Service

# Se não, adicionar:
railway variables set DFID_SERVICE_URL=https://defarm-dfid-service-production.up.railway.app
```

## Rollback

Se algo der errado:

```bash
# Remover DFID_SERVICE_URL da API
railway service defarm-engines-api
railway variables unset DFID_SERVICE_URL

# Redeploy
railway redeploy

# API volta a usar geração local
# Sem impacto nos usuários!
```

## Próximos Passos

Após deploy bem-sucedido:

1. ✅ Monitorar logs por 24h
2. ✅ Verificar métricas (latência, erros)
3. ✅ Testar criação de items via API
4. ✅ Verificar formato dos DFIDs (6 chars checksum)
5. ✅ Documentar URL de produção

## URLs Importantes

- **Dashboard Railway**: https://railway.app/project/2e6d7cdb-f993-4411-bcf4-1844f5b38011
- **DFID Service** (após deploy): https://defarm-dfid-service-production.up.railway.app
- **API Atual**: https://connect.defarm.net

## Comandos Úteis

```bash
# Ver todos os serviços
railway list

# Logs em tempo real
railway logs -f --service defarm-dfid-service

# Restart serviço
railway restart --service defarm-dfid-service

# Ver configuração
railway status --service defarm-dfid-service --json | jq

# SSH (se necessário debugar)
railway ssh --service defarm-dfid-service
```

## Checklist Final

- [ ] Código commitado e pushed
- [ ] Serviço criado no Railway
- [ ] Variáveis configuradas (PORT, REDIS_URL, RUST_LOG)
- [ ] Deploy concluído com sucesso
- [ ] Health check retorna 200
- [ ] DFID generation funciona
- [ ] Validation funciona
- [ ] DFID_SERVICE_URL configurado na API
- [ ] API logs mostram "DFID Service client enabled"
- [ ] Teste de criação de item funciona

**Tempo estimado**: 15-30 minutos

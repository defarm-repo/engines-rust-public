#!/bin/bash
# Script para deploy do DFID Service no Railway

set -e

echo "🚀 Deploy DFID Service no Railway"
echo "=================================="
echo ""

# Verificar se railway CLI está instalado
if ! command -v railway &> /dev/null; then
    echo "❌ Railway CLI não instalado"
    echo "   Instale com: npm install -g @railway/cli"
    exit 1
fi

echo "✅ Railway CLI detectado"

# Verificar token
if [ -z "$RAILWAY_TOKEN" ]; then
    echo "❌ RAILWAY_TOKEN não configurado"
    echo "   Export o token: export RAILWAY_TOKEN=fb76b340-b105-4172-b4bf-4dcb894225a8"
    exit 1
fi

echo "✅ Token Railway configurado"
echo ""

# Verificar se há mudanças não commitadas
if ! git diff-index --quiet HEAD --; then
    echo "⚠️  Há mudanças não commitadas"
    echo ""
    echo "Deseja commitar agora? (y/n)"
    read -r response
    if [ "$response" = "y" ]; then
        echo ""
        echo "📝 Adicionando arquivos..."
        git add dfid-service/
        git add src/dfid_client.rs
        git add src/items_engine.rs
        git add src/circuits_engine.rs
        git add src/api/shared_state.rs
        git add src/bin/api.rs
        git add src/lib.rs
        git add docker-compose.yml
        git add *.md
        git add scripts/

        echo "💾 Commitando..."
        git commit -m "feat: separate DFID generation into external service

- Create dfid-service microservice with BLAKE3 checksums
- Add DfidClient for HTTP communication
- Update ItemsEngine and CircuitsEngine for hybrid mode
- Add Docker Compose for local development
- Include comprehensive documentation and migration guide"

        echo "📤 Pushing para GitHub..."
        git push origin main

        echo "✅ Código commitado e pushed"
    else
        echo "⏭️  Pulando commit"
    fi
fi

echo ""
echo "🏗️  Preparando deploy..."
echo ""

# Perguntar se deve criar novo serviço ou usar existente
echo "Deseja criar novo serviço ou usar existente?"
echo "1) Criar novo serviço 'defarm-dfid-service'"
echo "2) Usar serviço existente"
echo ""
read -p "Escolha (1 ou 2): " choice

if [ "$choice" = "1" ]; then
    echo ""
    echo "📦 Criando novo serviço..."
    railway service create defarm-dfid-service || true
    echo "✅ Serviço criado (ou já existe)"
fi

echo ""
echo "🔧 Configurando variáveis de ambiente..."

# Configurar variáveis
railway variables set PORT=3001 || true
railway variables set REDIS_URL='${{Redis.REDIS_URL}}' || true
railway variables set RUST_LOG=info || true

echo "✅ Variáveis configuradas"
echo ""

# Deploy
echo "🚢 Iniciando deploy..."
cd dfid-service
railway up --detach

echo ""
echo "✅ Deploy iniciado!"
echo ""
echo "📊 Ver logs:"
echo "   railway logs -f --service defarm-dfid-service"
echo ""
echo "🔍 Verificar status:"
echo "   railway status --service defarm-dfid-service"
echo ""
echo "🏥 Testar health (após deploy):"
echo "   railway status --service defarm-dfid-service --json | jq -r '.services.edges[0].node.url'"
echo "   curl \$(railway status --service defarm-dfid-service --json | jq -r '.services.edges[0].node.url')/health"
echo ""
echo "🎉 Deploy concluído!"
echo ""
echo "Próximos passos:"
echo "1. Aguarde o deploy terminar (railway logs)"
echo "2. Teste o health endpoint"
echo "3. Configure DFID_SERVICE_URL na API"
echo "4. Redeploy a API"
echo ""

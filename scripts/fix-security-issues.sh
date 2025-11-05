#!/bin/bash

# Script automático para corrigir issues de segurança detectados pelo security-scan.sh
# Remove arquivos sensíveis do tracking do Git e atualiza .gitignore

set -e

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🔧 Fix Security Issues"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Verificar se estamos em um repositório git
if ! git rev-parse --git-dir > /dev/null 2>&1; then
    echo "❌ Erro: Não está em um repositório Git"
    exit 1
fi

echo "Etapa 1: Atualizando .gitignore com padrões sensíveis..."
echo ""

# Backup do .gitignore atual
if [ -f .gitignore ]; then
    cp .gitignore .gitignore.backup
    echo "✅ Backup criado: .gitignore.backup"
fi

# Adicionar padrões sensíveis ao .gitignore se não existirem
PATTERNS_TO_ADD=(
    "# Sensitive files - Added by security fix"
    "*.key"
    "*.pem"
    "*.p12"
    "*.pfx"
    "credentials.json"
    "service-account.json"
    "secrets.toml"
    "secrets.json"
    ""
    "# SSL certificates"
    "config/nginx/ssl/*.pem"
    "config/nginx/ssl/*.key"
)

for pattern in "${PATTERNS_TO_ADD[@]}"; do
    if [ ! -z "$pattern" ] && [ "$pattern" != "# Sensitive files - Added by security fix" ] && [ "$pattern" != "# SSL certificates" ]; then
        # Verificar se já existe no .gitignore
        if ! grep -qF "$pattern" .gitignore 2>/dev/null; then
            echo "$pattern" >> .gitignore
            echo "  ➕ Adicionado ao .gitignore: $pattern"
        else
            echo "  ✓ Já existe no .gitignore: $pattern"
        fi
    elif [ ! -z "$pattern" ]; then
        # Adicionar comentários
        if ! grep -qF "$pattern" .gitignore 2>/dev/null; then
            echo "" >> .gitignore
            echo "$pattern" >> .gitignore
        fi
    fi
done

echo ""
echo "Etapa 2: Removendo arquivos sensíveis do tracking do Git..."
echo ""

# Lista de arquivos para remover do git (mas manter localmente)
SENSITIVE_FILES=(
    "config/nginx/ssl/privkey.pem"
    "config/nginx/ssl/fullchain.pem"
)

for file in "${SENSITIVE_FILES[@]}"; do
    if git ls-files --error-unmatch "$file" > /dev/null 2>&1; then
        echo "  🗑️  Removendo do Git: $file"
        git rm --cached "$file" 2>/dev/null || true
        echo "     ✅ Arquivo removido do tracking (mantido localmente)"
    else
        echo "  ✓ Arquivo já não está sendo trackeado: $file"
    fi
done

echo ""
echo "Etapa 3: Verificando .env..."
echo ""

# Verificar se .env está sendo trackeado
if git ls-files --error-unmatch ".env" > /dev/null 2>&1; then
    echo "  ⚠️  .env está sendo trackeado no Git"
    echo "  🗑️  Removendo .env do tracking..."
    git rm --cached .env 2>/dev/null || true
    echo "     ✅ .env removido do tracking (mantido localmente)"
else
    echo "  ✅ .env não está sendo trackeado"
fi

echo ""
echo "Etapa 4: Criando exemplo de .env (se necessário)..."
echo ""

if [ -f .env ] && [ ! -f .env.example ]; then
    echo "  📝 Criando .env.example..."
    # Criar .env.example com valores de placeholder
    sed 's/=.*/=YOUR_VALUE_HERE/' .env > .env.example
    git add .env.example 2>/dev/null || true
    echo "     ✅ .env.example criado"
elif [ -f .env.example ]; then
    echo "  ✅ .env.example já existe"
else
    echo "  ℹ️  Nenhum .env encontrado"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📊 Resumo das Alterações"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Mostrar status do git
echo "Arquivos modificados:"
git status --short | grep -E '(\.gitignore|\.env)' || echo "  Nenhum"

echo ""
echo "Arquivos removidos do tracking:"
git status --short | grep "^ D" || echo "  Nenhum"

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Correções Aplicadas!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Próximos passos:"
echo "1. Revise as mudanças: git status"
echo "2. Execute o security scan novamente: ./scripts/security-scan.sh"
echo "3. Se passar, faça commit: git commit -m 'security: remove sensitive files from tracking'"
echo ""

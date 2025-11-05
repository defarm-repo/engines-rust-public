#!/bin/bash

# Script de configuração rápida do remote público
# Este script adiciona o remote público ao repositório local

set -e

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🔧 Setup: Dual Remote Configuration"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

PUBLIC_REPO_URL="git@github.com:defarm-repo/engines-rust-public.git"
REMOTE_NAME="public"

# Verificar se já existe remote público
if git remote | grep -q "^${REMOTE_NAME}$"; then
    echo "⚠️  Remote '${REMOTE_NAME}' já existe!"
    echo ""
    echo "Remotes atuais:"
    git remote -v
    echo ""
    echo "Para reconfigurar, primeiro remova o remote existente:"
    echo "  git remote remove ${REMOTE_NAME}"
    echo ""
    exit 1
fi

# Adicionar remote público
echo "➕ Adicionando remote público..."
git remote add "${REMOTE_NAME}" "${PUBLIC_REPO_URL}"

echo "✅ Remote público adicionado com sucesso!"
echo ""

# Mostrar configuração atual
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📋 Configuração Atual de Remotes:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
git remote -v
echo ""

# Informações importantes
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "ℹ️  Informações Importantes:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "✅ Remote público configurado localmente"
echo "✅ Você pode fazer 'git fetch public' para buscar mudanças"
echo ""
echo "⚠️  IMPORTANTE:"
echo "   • Você NÃO precisa fazer push manual para o remote público"
echo "   • O GitHub Actions sincroniza automaticamente após push para origin"
echo "   • Continue usando: git push origin main"
echo ""
echo "📚 Para mais informações, veja: DUAL_REMOTE_SETUP.md"
echo ""

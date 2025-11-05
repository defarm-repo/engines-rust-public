#!/bin/bash

# Script de teste para verificar configuração de dual remote
# Verifica se o workflow e configuração estão corretos antes do primeiro push

set -e

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🧪 Test: Dual Remote Setup Verification"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

PASS_COUNT=0
FAIL_COUNT=0
WARN_COUNT=0

# Função para marcar testes
check_pass() {
    echo "✅ PASS: $1"
    PASS_COUNT=$((PASS_COUNT + 1))
}

check_fail() {
    echo "❌ FAIL: $1"
    FAIL_COUNT=$((FAIL_COUNT + 1))
}

check_warn() {
    echo "⚠️  WARN: $1"
    WARN_COUNT=$((WARN_COUNT + 1))
}

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📋 Verificando Estrutura Local"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Verificar se é um repositório Git
if git rev-parse --git-dir > /dev/null 2>&1; then
    check_pass "Diretório é um repositório Git válido"
else
    check_fail "Não é um repositório Git"
    exit 1
fi

# Verificar branch atual
CURRENT_BRANCH=$(git branch --show-current)
if [ "$CURRENT_BRANCH" = "main" ]; then
    check_pass "Branch atual é 'main'"
else
    check_warn "Branch atual é '$CURRENT_BRANCH' (esperado: main)"
fi

# Verificar remote origin
if git remote | grep -q "^origin$"; then
    check_pass "Remote 'origin' está configurado"
    echo "   URL: $(git remote get-url origin)"
else
    check_fail "Remote 'origin' não encontrado"
fi

# Verificar remote public (opcional)
if git remote | grep -q "^public$"; then
    check_pass "Remote 'public' está configurado (opcional)"
    echo "   URL: $(git remote get-url public)"
else
    check_warn "Remote 'public' não configurado localmente (opcional)"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📁 Verificando Arquivos do Workflow"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Verificar workflow sync-to-public.yml
WORKFLOW_FILE=".github/workflows/sync-to-public.yml"
if [ -f "$WORKFLOW_FILE" ]; then
    check_pass "Workflow file existe: $WORKFLOW_FILE"

    # Verificar conteúdo do workflow
    if grep -q "PUBLIC_REPO_TOKEN" "$WORKFLOW_FILE"; then
        check_pass "Workflow referencia PUBLIC_REPO_TOKEN"
    else
        check_fail "Workflow não referencia PUBLIC_REPO_TOKEN"
    fi

    if grep -q "PUBLIC_REPO_URL" "$WORKFLOW_FILE"; then
        check_pass "Workflow referencia PUBLIC_REPO_URL"
    else
        check_fail "Workflow não referencia PUBLIC_REPO_URL"
    fi

    if grep -q "git-filter-repo" "$WORKFLOW_FILE"; then
        check_pass "Workflow usa git-filter-repo"
    else
        check_fail "Workflow não usa git-filter-repo"
    fi
else
    check_fail "Workflow file não encontrado: $WORKFLOW_FILE"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📄 Verificando Arquivos que Serão Filtrados"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Contar arquivos .md
MD_COUNT=$(find . -type f -name "*.md" -not -path "./.git/*" | wc -l | tr -d ' ')
if [ "$MD_COUNT" -gt 0 ]; then
    check_pass "Encontrados $MD_COUNT arquivos .md para filtrar"
    echo "   Exemplos:"
    find . -type f -name "*.md" -not -path "./.git/*" | head -5 | sed 's/^/   - /'
else
    check_warn "Nenhum arquivo .md encontrado"
fi

# Verificar diretório tests/
if [ -d "tests" ]; then
    TEST_COUNT=$(find tests -type f | wc -l | tr -d ' ')
    check_pass "Diretório tests/ existe com $TEST_COUNT arquivos"
else
    check_warn "Diretório tests/ não existe"
fi

# Verificar diretório docs/
if [ -d "docs" ]; then
    DOCS_COUNT=$(find docs -type f | wc -l | tr -d ' ')
    check_pass "Diretório docs/ existe com $DOCS_COUNT arquivos"
else
    check_warn "Diretório docs/ não existe"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🔑 Verificando Configuração do GitHub"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Extrair info do remote origin
if git remote get-url origin > /dev/null 2>&1; then
    ORIGIN_URL=$(git remote get-url origin)

    # Tentar extrair usuário e repo do URL
    if [[ $ORIGIN_URL =~ github\.com[:/]([^/]+)/([^/\.]+) ]]; then
        GH_USER="${BASH_REMATCH[1]}"
        GH_REPO="${BASH_REMATCH[2]}"

        echo "🔍 Repositório GitHub detectado:"
        echo "   Usuário: $GH_USER"
        echo "   Repo: $GH_REPO"
        echo ""

        check_pass "URL do GitHub parseado com sucesso"

        echo "⚠️  ATENÇÃO: Não é possível verificar GitHub Secrets via script local"
        echo ""
        echo "   Você precisa configurar manualmente no GitHub:"
        echo "   https://github.com/$GH_USER/$GH_REPO/settings/secrets/actions"
        echo ""
        echo "   Secrets necessários:"
        echo "   1. PUBLIC_REPO_TOKEN = seu Personal Access Token"
        echo "   2. PUBLIC_REPO_URL = github.com/defarm-repo/engines-rust-public.git"
        echo ""
    else
        check_warn "Não foi possível extrair informações do GitHub do remote origin"
    fi
else
    check_fail "Não foi possível obter URL do remote origin"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📊 Resumo do Teste"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "✅ Passou: $PASS_COUNT"
echo "❌ Falhou: $FAIL_COUNT"
echo "⚠️  Avisos: $WARN_COUNT"
echo ""

if [ $FAIL_COUNT -eq 0 ]; then
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "🎉 SUCCESS: Setup está pronto para uso!"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    echo "Próximos passos:"
    echo "1. Configure os GitHub Secrets (se ainda não configurou)"
    echo "2. Faça commit e push: git push origin main"
    echo "3. Monitore o workflow em: https://github.com/$GH_USER/$GH_REPO/actions"
    echo ""
    exit 0
else
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "❌ FAILED: Corrija os erros antes de continuar"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    echo "Consulte: DUAL_REMOTE_SETUP.md para instruções detalhadas"
    echo ""
    exit 1
fi

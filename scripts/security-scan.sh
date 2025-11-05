#!/bin/bash

# Script de segurança para detectar informações sensíveis antes do push público
# Este script procura por private keys, senhas, tokens, e outros dados sensíveis

set -e

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🔒 Security Scan: Sensitive Data Detection"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "ℹ️  Scanning only Git-tracked files (what will be pushed to repo)"
echo ""

ISSUES_FOUND=0
WARNINGS_FOUND=0

# Criar lista de arquivos trackeados pelo git
TRACKED_FILES=$(git ls-files 2>/dev/null || find . -type f -not -path "./.git/*")

# Cores para output
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

# Função para reportar issue crítico
report_critical() {
    echo -e "${RED}❌ CRITICAL: $1${NC}"
    echo "   File: $2"
    echo "   Line: $3"
    echo ""
    ISSUES_FOUND=$((ISSUES_FOUND + 1))
}

# Função para reportar warning
report_warning() {
    echo -e "${YELLOW}⚠️  WARNING: $1${NC}"
    echo "   File: $2"
    echo "   Context: $3"
    echo ""
    WARNINGS_FOUND=$((WARNINGS_FOUND + 1))
}

# Função para reportar sucesso
report_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🔍 Scanning for Private Keys (Git-tracked files only)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Procurar por private keys apenas em arquivos trackeados pelo git
PRIVATE_KEY_PATTERNS=(
    "BEGIN RSA PRIVATE KEY"
    "BEGIN DSA PRIVATE KEY"
    "BEGIN EC PRIVATE KEY"
    "BEGIN OPENSSH PRIVATE KEY"
    "BEGIN PRIVATE KEY"
    "BEGIN PGP PRIVATE KEY"
)

for pattern in "${PRIVATE_KEY_PATTERNS[@]}"; do
    # Buscar apenas em arquivos trackeados
    for file in $TRACKED_FILES; do
        if [ -f "$file" ] && grep -q "$pattern" "$file" 2>/dev/null; then
            content=$(grep "$pattern" "$file" | head -1)
            report_critical "Private key found: $pattern" "$file" "$content"
        fi
    done
done

if [ $ISSUES_FOUND -eq 0 ]; then
    report_success "No private keys found in tracked files"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🔑 Scanning for API Keys and Tokens"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Padrões de API keys conhecidas
API_KEY_PATTERNS=(
    'ghp_[a-zA-Z0-9]{36}'              # GitHub Personal Access Token
    'github_pat_[a-zA-Z0-9_]{82}'      # GitHub Fine-grained PAT
    'gho_[a-zA-Z0-9]{36}'              # GitHub OAuth Token
    'ghs_[a-zA-Z0-9]{36}'              # GitHub App Token
    'AKIA[0-9A-Z]{16}'                 # AWS Access Key
    'ya29\.[a-zA-Z0-9_-]{100,}'        # Google OAuth Token
    'sk_live_[a-zA-Z0-9]{24,}'         # Stripe Live Key
    'sk_test_[a-zA-Z0-9]{24,}'         # Stripe Test Key
    'AIza[a-zA-Z0-9_-]{35}'            # Google API Key
    'xox[baprs]-[a-zA-Z0-9-]+'         # Slack Token
    'SG\.[a-zA-Z0-9_-]{22}\.[a-zA-Z0-9_-]{43}' # SendGrid API Key
)

for pattern in "${API_KEY_PATTERNS[@]}"; do
    results=$(grep -rE "$pattern" --include="*.rs" --include="*.toml" --include="*.sh" --include="*.txt" . 2>/dev/null | grep -v ".git/" | grep -v "security-scan.sh" || true)
    if [ ! -z "$results" ]; then
        while IFS= read -r line; do
            file=$(echo "$line" | cut -d: -f1)
            content=$(echo "$line" | cut -d: -f2-)
            report_critical "Potential API key/token found" "$file" "$content"
        done <<< "$results"
    fi
done

if [ $ISSUES_FOUND -eq 0 ]; then
    report_success "No API keys or tokens found"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🔐 Scanning for Hardcoded Passwords"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Padrões de senhas hardcoded
PASSWORD_PATTERNS=(
    'password\s*=\s*["\x27][^"\x27]{4,}["\x27]'
    'PASSWORD\s*=\s*["\x27][^"\x27]{4,}["\x27]'
    'pwd\s*=\s*["\x27][^"\x27]{4,}["\x27]'
    'secret\s*=\s*["\x27][^"\x27]{8,}["\x27]'
    'SECRET\s*=\s*["\x27][^"\x27]{8,}["\x27]'
)

for pattern in "${PASSWORD_PATTERNS[@]}"; do
    results=$(grep -rE -i "$pattern" --include="*.rs" --include="*.toml" --include="*.sh" . 2>/dev/null | grep -v ".git/" | grep -v "test" | grep -v "demo" | grep -v "example" | grep -v "security-scan.sh" || true)
    if [ ! -z "$results" ]; then
        while IFS= read -r line; do
            file=$(echo "$line" | cut -d: -f1)
            content=$(echo "$line" | cut -d: -f2-)
            # Excluir casos óbvios de teste/demo
            if [[ ! "$content" =~ (demo|test|example|placeholder|your_password|changeme|12345) ]]; then
                report_warning "Potential hardcoded password" "$file" "$content"
            fi
        done <<< "$results"
    fi
done

if [ $WARNINGS_FOUND -eq 0 ]; then
    report_success "No suspicious hardcoded passwords found"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🗄️  Scanning for Database Connection Strings"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Padrões de connection strings
CONNECTION_PATTERNS=(
    'postgres://[^:]+:[^@]+@[^/]+'
    'postgresql://[^:]+:[^@]+@[^/]+'
    'mysql://[^:]+:[^@]+@[^/]+'
    'mongodb://[^:]+:[^@]+@[^/]+'
    'redis://[^:]+:[^@]+@[^/]+'
)

for pattern in "${CONNECTION_PATTERNS[@]}"; do
    results=$(grep -rE "$pattern" --include="*.rs" --include="*.toml" --include="*.sh" . 2>/dev/null | grep -v ".git/" | grep -v "security-scan.sh" || true)
    if [ ! -z "$results" ]; then
        while IFS= read -r line; do
            file=$(echo "$line" | cut -d: -f1)
            content=$(echo "$line" | cut -d: -f2-)
            # Excluir exemplos óbvios
            if [[ ! "$content" =~ (localhost|127\.0\.0\.1|example\.com|username|password|changeme) ]]; then
                report_critical "Database connection string with credentials" "$file" "$content"
            fi
        done <<< "$results"
    fi
done

if [ $ISSUES_FOUND -eq 0 ]; then
    report_success "No database connection strings with credentials found"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📝 Scanning for Suspicious Environment Variables"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Procurar por variáveis de ambiente com valores reais (não exemplos)
ENV_VAR_PATTERNS=(
    'JWT_SECRET\s*=\s*["\x27][a-zA-Z0-9]{16,}["\x27]'
    'ENCRYPTION_KEY\s*=\s*["\x27][a-zA-Z0-9]{16,}["\x27]'
    'PRIVATE_KEY\s*=\s*["\x27][a-zA-Z0-9]{16,}["\x27]'
    'DATABASE_PASSWORD\s*=\s*["\x27][^"\x27]{4,}["\x27]'
)

for pattern in "${ENV_VAR_PATTERNS[@]}"; do
    results=$(grep -rE "$pattern" --include="*.rs" --include="*.sh" --include="*.toml" . 2>/dev/null | grep -v ".git/" | grep -v "security-scan.sh" || true)
    if [ ! -z "$results" ]; then
        while IFS= read -r line; do
            file=$(echo "$line" | cut -d: -f1)
            content=$(echo "$line" | cut -d: -f2-)
            report_warning "Environment variable with potential real value" "$file" "$content"
        done <<< "$results"
    fi
done

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🐛 Scanning for Debug/Console Logs with Sensitive Data"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Procurar por logs suspeitos
LOG_PATTERNS=(
    'println!.*password'
    'println!.*secret'
    'println!.*token'
    'println!.*key'
    'dbg!.*password'
    'dbg!.*secret'
    'dbg!.*token'
    'eprintln!.*password'
    'eprintln!.*secret'
)

for pattern in "${LOG_PATTERNS[@]}"; do
    results=$(grep -rE -i "$pattern" --include="*.rs" . 2>/dev/null | grep -v ".git/" | grep -v "test" | grep -v "security-scan.sh" || true)
    if [ ! -z "$results" ]; then
        while IFS= read -r line; do
            file=$(echo "$line" | cut -d: -f1)
            content=$(echo "$line" | cut -d: -f2-)
            report_warning "Debug/console log with potentially sensitive data" "$file" "$content"
        done <<< "$results"
    fi
done

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📂 Checking for Sensitive Files (Git-tracked only)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Verificar arquivos sensíveis que estão sendo trackeados pelo git
# Padrões mais específicos para evitar falsos positivos
SENSITIVE_PATTERNS=(
    "^\.env$"                    # .env exato (não .env.example)
    "^\.env\.local$"             # .env.local exato
    "^\.env\.production$"        # .env.production exato
    "credentials\.json$"         # qualquer credentials.json
    "service-account\.json$"     # qualquer service-account.json
    "private-key\.pem$"          # private-key.pem
    "id_rsa$"                    # SSH private key
    "id_dsa$"                    # SSH private key
    "secrets\.toml$"             # secrets.toml
    "secrets\.json$"             # secrets.json
)

# Exceções: arquivos que podem conter "env" ou "secret" mas são seguros
SAFE_PATTERNS=(
    "\.env\.example$"            # Template sem valores reais
    ".*-env\.sh$"                # Scripts de setup de env (não contém valores)
    "environment"                # Diretórios ou conceitos
)

initial_issues=$ISSUES_FOUND
for pattern in "${SENSITIVE_PATTERNS[@]}"; do
    # Verificar se algum arquivo trackeado pelo git corresponde ao padrão
    matched_files=$(echo "$TRACKED_FILES" | grep -E "$pattern" || true)
    if [ ! -z "$matched_files" ]; then
        while IFS= read -r file; do
            if [ ! -z "$file" ]; then
                # Verificar se não é uma exceção segura
                is_safe=false
                for safe_pattern in "${SAFE_PATTERNS[@]}"; do
                    if echo "$file" | grep -qE "$safe_pattern"; then
                        is_safe=true
                        break
                    fi
                done

                if [ "$is_safe" = false ]; then
                    report_critical "Sensitive file is being tracked by Git" "$file" "This file should be in .gitignore and removed from Git"
                fi
            fi
        done <<< "$matched_files"
    fi
done

if [ $ISSUES_FOUND -eq $initial_issues ]; then
    report_success "No sensitive files being tracked by Git"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🔍 Checking .gitignore Coverage"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

if [ -f ".gitignore" ]; then
    report_success ".gitignore file exists"

    # Verificar se padrões comuns estão no .gitignore
    REQUIRED_PATTERNS=(
        ".env"
        "*.key"
        "*.pem"
        "credentials"
        "secrets"
    )

    missing_patterns=()
    for pattern in "${REQUIRED_PATTERNS[@]}"; do
        if ! grep -q "$pattern" .gitignore; then
            missing_patterns+=("$pattern")
        fi
    done

    if [ ${#missing_patterns[@]} -gt 0 ]; then
        report_warning ".gitignore missing common sensitive patterns" ".gitignore" "Missing: ${missing_patterns[*]}"
    else
        report_success ".gitignore covers common sensitive file patterns"
    fi
else
    report_warning ".gitignore file not found" "." "Consider creating one"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📊 Scan Summary"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Critical Issues: $ISSUES_FOUND"
echo "Warnings: $WARNINGS_FOUND"
echo ""

if [ $ISSUES_FOUND -gt 0 ]; then
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo -e "${RED}❌ SECURITY SCAN FAILED${NC}"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    echo "CRITICAL issues found that MUST be fixed before pushing to public repo!"
    echo ""
    echo "Actions required:"
    echo "1. Review all critical issues listed above"
    echo "2. Remove or encrypt sensitive data"
    echo "3. Add sensitive files to .gitignore"
    echo "4. Re-run this scan until it passes"
    echo ""
    exit 1
elif [ $WARNINGS_FOUND -gt 0 ]; then
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo -e "${YELLOW}⚠️  SECURITY SCAN PASSED WITH WARNINGS${NC}"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    echo "No critical issues found, but there are warnings to review."
    echo ""
    echo "Recommended actions:"
    echo "1. Review warnings listed above"
    echo "2. Verify they are not actual sensitive data"
    echo "3. Consider refactoring to remove warnings"
    echo ""
    echo "It's safe to proceed, but review warnings first."
    echo ""
    exit 0
else
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo -e "${GREEN}✅ SECURITY SCAN PASSED${NC}"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    echo "No sensitive data detected. Safe to push to public repository!"
    echo ""
    exit 0
fi

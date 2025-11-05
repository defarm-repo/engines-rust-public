# Dual Remote - Guia Rápido

> **TL;DR**: Configure dois repositórios Git onde um único `git push` atualiza ambos automaticamente, com o repositório público recebendo uma versão filtrada (sem .md, tests/, docs/).

## 🚀 Setup em 5 Minutos

### 1️⃣ Criar Personal Access Token

```bash
# Acesse e crie token com scope 'repo':
open https://github.com/settings/tokens
```

**Copie o token** (formato: `ghp_...`) - você vai usar no próximo passo!

### 2️⃣ Configurar GitHub Secrets

```bash
# Acesse configurações do repositório privado:
open https://github.com/defarm-repo/engines/settings/secrets/actions
```

**Adicione 2 secrets**:
- `PUBLIC_REPO_TOKEN` = token copiado acima
- `PUBLIC_REPO_URL` = `github.com/defarm-repo/engines-rust-public.git`

### 3️⃣ Verificar Setup

```bash
# Rodar script de teste
./scripts/test-dual-remote.sh
```

### 4️⃣ Fazer Push de Teste

```bash
# Commit e push normalmente
git add .
git commit -m "feat: setup dual remote sync"
git push origin main

# Aguarde ~30 segundos e verifique:
open https://github.com/defarm-repo/engines/actions
open https://github.com/defarm-repo/engines-rust-public
```

✅ **Pronto!** Agora todo `git push origin main` sincroniza automaticamente.

---

## 📁 Arquivos Criados

| Arquivo | Descrição |
|---------|-----------|
| `.github/workflows/sync-to-public.yml` | Workflow do GitHub Actions |
| `DUAL_REMOTE_SETUP.md` | Guia completo passo-a-passo |
| `GITHUB_SECRETS_REFERENCE.md` | Referência de configuração de secrets |
| `scripts/setup-public-remote.sh` | Script para config local (opcional) |
| `scripts/test-dual-remote.sh` | Script de verificação |
| `DUAL_REMOTE_QUICKSTART.md` | Este arquivo |

---

## 🎯 Como Funciona

```
Você faz:                 Sistema faz:
┌──────────────────┐      ┌──────────────────┐
│  git add .       │      │                  │
│  git commit      │      │                  │
│  git push origin │  →   │  GitHub Actions  │
└──────────────────┘      │  • Remove .md    │
                          │  • Remove tests/ │
                          │  • Remove docs/  │
                          │  • Push público  │
                          └──────────────────┘
```

---

## 📊 O Que é Filtrado

### ❌ Excluído do Repositório Público

- Todos arquivos `.md` (documentação)
- Diretório `tests/` completo
- Diretório `docs/` completo
- Diretório `.github/` (workflows)

### ✅ Mantido no Repositório Público

- Código-fonte `/src`
- Dependências `Cargo.toml`, `Cargo.lock`
- Scripts `/scripts`
- Configurações `/config`
- Arquivos públicos `/public`

---

## 💻 Comandos Úteis

### Verificar Configuração

```bash
# Ver remotes configurados
git remote -v

# Testar setup
./scripts/test-dual-remote.sh

# Ver último workflow
gh run list --workflow=sync-to-public.yml --limit=1
```

### Adicionar Remote Local (Opcional)

```bash
# Adicionar remote público localmente
./scripts/setup-public-remote.sh

# Ou manualmente:
git remote add public git@github.com:defarm-repo/engines-rust-public.git
```

### Monitorar Sincronização

```bash
# Ver workflows em execução
gh run list --workflow=sync-to-public.yml

# Ver logs do último workflow
gh run view --log

# Abrir Actions no navegador
gh browse --settings
```

---

## 🐛 Solução de Problemas

### Workflow não executa

```bash
# Verificar se workflow existe
cat .github/workflows/sync-to-public.yml

# Verificar se está na branch main
git branch --show-current

# Forçar trigger
git commit --allow-empty -m "trigger: sync workflow"
git push origin main
```

### Erro de autenticação

```bash
# Verificar se secrets estão configurados
gh secret list

# Recriar token em:
open https://github.com/settings/tokens

# Atualizar secret:
gh secret set PUBLIC_REPO_TOKEN
```

### Arquivos não são filtrados

```bash
# Verificar conteúdo do workflow
grep -A 10 "Create filtered branch" .github/workflows/sync-to-public.yml

# Ver logs do último workflow
gh run view --log
```

---

## 📚 Documentação Completa

Para informações detalhadas, consulte:

- **Setup Completo**: [DUAL_REMOTE_SETUP.md](./DUAL_REMOTE_SETUP.md)
- **Secrets Reference**: [GITHUB_SECRETS_REFERENCE.md](./GITHUB_SECRETS_REFERENCE.md)
- **GitHub Actions**: https://docs.github.com/actions

---

## ✅ Checklist de Uso

### Configuração Inicial (uma vez)

- [ ] Personal Access Token criado
- [ ] Secret `PUBLIC_REPO_TOKEN` configurado
- [ ] Secret `PUBLIC_REPO_URL` configurado
- [ ] Repositório público criado
- [ ] Teste executado com sucesso
- [ ] Primeiro push sincronizado

### Uso Diário (sempre)

- [ ] Desenvolver código normalmente
- [ ] Commit com mensagem descritiva
- [ ] Push para `origin main`
- [ ] *(Automático)* Verificar Actions se necessário

---

## 🎉 Pronto para Produção

Seu setup está completo quando:

✅ Script de teste passa sem erros
✅ Primeiro push sincroniza com sucesso
✅ Repositório público não contém .md files
✅ Repositório público não contém tests/
✅ Repositório público não contém docs/
✅ GitHub Actions executa em ~30 segundos

---

## 💡 Dicas

### Pull Requests

O workflow também funciona com PRs. Configure em `.github/workflows/sync-to-public.yml`:

```yaml
on:
  push:
    branches: [ main ]
  pull_request:  # Adicione esta linha
    branches: [ main ]
```

### Customizar Filtragem

Edite `.github/workflows/sync-to-public.yml` seção "Create filtered branch":

```bash
# Manter README.md mas remover outros .md
find . -type f -name "*.md" -not -name "README.md" -delete

# Remover diretórios adicionais
rm -rf internal/ private/ .env.example
```

### Notificações

Configure notificações do GitHub Actions:

1. Settings → Notifications
2. Actions → Check "Send notifications for failed workflows only"

---

## 🔗 Links Úteis

- **Repositório Privado**: https://github.com/defarm-repo/engines
- **Repositório Público**: https://github.com/defarm-repo/engines-rust-public
- **Actions Dashboard**: https://github.com/defarm-repo/engines/actions
- **Secrets Config**: https://github.com/defarm-repo/engines/settings/secrets/actions
- **Token Management**: https://github.com/settings/tokens

---

**Última atualização**: 2025-11-05
**Versão**: 1.0

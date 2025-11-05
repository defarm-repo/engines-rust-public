# Configuração de Dual Remote com Filtragem Automática

Este guia explica como configurar dois repositórios Git onde um único `git push` atualiza ambos:
- **Repositório Privado**: Versão completa com todos os arquivos
- **Repositório Público**: Versão filtrada sem documentação e testes

## 📋 Pré-requisitos

- Repositório privado atual já configurado no GitHub
- Conta GitHub com permissão para criar novos repositórios
- Git configurado localmente
- Acesso às configurações do repositório no GitHub

## 🚀 Passo a Passo

### Etapa 1: Criar Repositório Público no GitHub

✅ **Repositório público já criado**: `git@github.com:defarm-repo/engines-rust-public.git`

Se você ainda não criou o repositório, siga estes passos:

1. Acesse https://github.com/new
2. Configure o novo repositório:
   - **Nome**: `engines-rust-public`
   - **Visibilidade**: Public
   - **Inicialização**: ❌ NÃO inicialize com README, .gitignore ou license
   - **Descrição**: "Public version of defarm engines (filtered)"

### Etapa 2: Criar Personal Access Token (PAT)

O GitHub Actions precisa de um token para fazer push no repositório público.

1. Acesse: https://github.com/settings/tokens
2. Clique em "Generate new token" → "Generate new token (classic)"
3. Configure o token:
   - **Nome**: `Dual Remote Sync Token`
   - **Expiração**: 90 days (ou conforme preferência)
   - **Scopes**: Marque apenas `repo` (acesso completo a repositórios)

4. Clique em "Generate token"
5. **⚠️ IMPORTANTE**: Copie o token imediatamente (você não verá novamente!)
   - Formato: `ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx`

### Etapa 3: Configurar GitHub Secrets

1. Acesse o repositório **PRIVADO** no GitHub
2. Vá em **Settings** → **Secrets and variables** → **Actions**
3. Clique em **New repository secret**

**Secret 1: PUBLIC_REPO_TOKEN**
- Nome: `PUBLIC_REPO_TOKEN`
- Valor: Cole o Personal Access Token criado na Etapa 2

**Secret 2: PUBLIC_REPO_URL**
- Nome: `PUBLIC_REPO_URL`
- Valor: `github.com/defarm-repo/engines-rust-public.git`
  - ⚠️ **Não inclua** `https://` no início
  - ✅ Formato correto: `github.com/usuario/repo.git`
  - ❌ Formato incorreto: `https://github.com/usuario/repo.git`

### Etapa 4: Verificar Workflow

O workflow já foi criado em `.github/workflows/sync-to-public.yml`.

Verifique se o arquivo existe:
```bash
ls -la .github/workflows/sync-to-public.yml
```

### Etapa 5: Fazer Push Inicial

1. Commit do workflow (se ainda não foi feito):
```bash
git add .github/workflows/sync-to-public.yml
git commit -m "feat: add GitHub Actions workflow for dual remote sync"
```

2. Push para o repositório privado:
```bash
git push origin main
```

3. Acompanhe a execução:
   - Acesse: `https://github.com/SEU-USUARIO/SEU-REPO/actions`
   - Clique no workflow "Sync to Public Repository"
   - Veja os logs em tempo real

### Etapa 6: Verificar Sincronização

1. Acesse o repositório público no GitHub
2. Verifique que **NÃO** contém:
   - ❌ Arquivos .md (README.md, CLAUDE.md, etc.)
   - ❌ Diretório `tests/`
   - ❌ Diretório `docs/`
   - ❌ Diretório `.github/`

3. Verifique que **CONTÉM**:
   - ✅ Diretório `src/`
   - ✅ Arquivos Cargo.toml e Cargo.lock
   - ✅ Diretórios `config/`, `scripts/`, `public/`
   - ✅ Todos os arquivos de código-fonte

## 🔧 Configuração Local do Remote (Opcional)

Se quiser ter o remote público configurado localmente também:

```bash
# Adicionar remote público
git remote add public git@github.com:defarm-repo/engines-rust-public.git

# Verificar remotes configurados
git remote -v
```

**Resultado esperado:**
```
origin    git@github.com:defarm-repo/engines.git (fetch)
origin    git@github.com:defarm-repo/engines.git (push)
public    git@github.com:defarm-repo/engines-rust-public.git (fetch)
public    git@github.com:defarm-repo/engines-rust-public.git (push)
```

⚠️ **Nota**: Você **NÃO precisa** fazer push manual para o remote público. O GitHub Actions faz isso automaticamente!

## 📊 Como Funciona

```
┌─────────────────────────┐
│  git push origin main   │ ← Você faz apenas isso
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│  Repositório Privado    │
│  (origin)               │
│  • Código completo      │
│  • Todos .md            │
│  • tests/               │
│  • docs/                │
└───────────┬─────────────┘
            │
            │ GitHub Actions detecta push
            ▼
┌─────────────────────────┐
│  Workflow Automático    │
│  1. Checkout código     │
│  2. Remove .md          │
│  3. Remove tests/       │
│  4. Remove docs/        │
│  5. Remove .github/     │
│  6. Push para público   │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│  Repositório Público    │
│  • Código-fonte apenas  │
│  • Sem documentação     │
│  • Sem testes           │
│  • Sem workflows        │
└─────────────────────────┘
```

## ✅ Uso Diário

Depois da configuração inicial, seu workflow será:

```bash
# 1. Desenvolver normalmente
vim src/main.rs

# 2. Commit normalmente
git add .
git commit -m "feat: nova funcionalidade"

# 3. Push APENAS para origin
git push origin main

# 4. GitHub Actions sincroniza automaticamente para público
# (você não faz nada, acontece em ~30 segundos)
```

## 🔍 Monitoramento

Para acompanhar sincronizações:

1. **GitHub Actions**: https://github.com/SEU-USUARIO/SEU-REPO/actions
2. **Ver último sync**: Clique no workflow mais recente
3. **Verificar logs**: Veja detalhes de cada passo

## 🐛 Troubleshooting

### Erro: "PUBLIC_REPO_TOKEN not configured"

**Causa**: Secret não configurado corretamente no GitHub

**Solução**:
1. Verifique Settings → Secrets → Actions
2. Confirme que `PUBLIC_REPO_TOKEN` e `PUBLIC_REPO_URL` existem
3. Recrie o token se necessário

### Erro: "Permission denied" ao fazer push

**Causa**: Token sem permissões adequadas ou expirado

**Solução**:
1. Crie novo Personal Access Token com scope `repo`
2. Atualize o secret `PUBLIC_REPO_TOKEN`
3. Tente o push novamente

### Erro: "Repository not found"

**Causa**: URL do repositório público incorreta

**Solução**:
1. Verifique o secret `PUBLIC_REPO_URL`
2. Formato correto: `github.com/usuario/repo.git` (sem https://)
3. Atualize e tente novamente

### Workflow não executa

**Causa**: Workflow pode estar desabilitado ou branch incorreta

**Solução**:
1. Verifique se o workflow está em `.github/workflows/`
2. Confirme que está na branch `main`
3. Vá em Actions → Verifique se workflows estão habilitados

## 🔒 Segurança

### Boas Práticas

1. **Token Rotation**: Troque o PAT a cada 90 dias
2. **Scope Mínimo**: Use apenas scope `repo` necessário
3. **Secrets**: NUNCA commite tokens no código
4. **Auditoria**: Monitore GitHub Actions logs regularmente

### O Que é Seguro?

✅ Código-fonte no repositório público
✅ Scripts de configuração
✅ Arquivos Cargo.toml/Cargo.lock

### O Que é Privado?

🔒 Documentação interna (.md files)
🔒 Testes (tests/)
🔒 Documentação de estratégia (docs/)
🔒 Workflows do GitHub Actions

## 📝 Customização

### Alterar Arquivos Filtrados

Edite `.github/workflows/sync-to-public.yml` na seção "Create filtered branch":

```yaml
# Exemplo: manter README.md mas remover outros .md
find . -type f -name "*.md" -not -name "README.md" -not -path "./.git/*" -delete

# Exemplo: remover diretório adicional
rm -rf internal/

# Exemplo: remover arquivos de configuração sensíveis
rm -f .env config/secrets.toml
```

### Alterar Branch de Sincronização

Por padrão sincroniza apenas `main`. Para sincronizar outras branches:

```yaml
on:
  push:
    branches: [ main, develop, staging ]
```

## 📚 Recursos Adicionais

- **GitHub Actions Docs**: https://docs.github.com/actions
- **git-filter-repo**: https://github.com/newren/git-filter-repo
- **Personal Access Tokens**: https://docs.github.com/authentication/keeping-your-account-and-data-secure/creating-a-personal-access-token

## 🎯 Resumo

| Ação | Você Faz | GitHub Actions Faz |
|------|----------|-------------------|
| Desenvolvimento | ✅ Código, commits normais | - |
| Push | ✅ `git push origin main` | - |
| Filtragem | - | ✅ Remove .md, tests/, docs/ |
| Sync público | - | ✅ Push automático para público |
| Pull | ✅ `git pull origin main` | - |

**Resultado**: Workflow normal de desenvolvimento, sincronização automática!

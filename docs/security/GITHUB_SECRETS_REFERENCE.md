# GitHub Secrets - Referência Rápida

Este documento contém as informações necessárias para configurar os GitHub Secrets para sincronização automática do repositório público.

## 📋 Secrets Necessários

### 1. PUBLIC_REPO_TOKEN

**Tipo**: Personal Access Token (PAT)

**Valor**: Token gerado no GitHub com permissões de repositório

**Como obter**:
1. Acesse: https://github.com/settings/tokens
2. Clique em "Generate new token" → "Generate new token (classic)"
3. Configure:
   - **Nome**: `Dual Remote Sync Token`
   - **Expiração**: 90 days (ou sua preferência)
   - **Scopes**: ✅ Marque apenas `repo` (Full control of private repositories)
4. Clique em "Generate token"
5. **⚠️ COPIE IMEDIATAMENTE** - você não verá novamente!

**Formato esperado**: `ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx`

**Permissões necessárias**:
- ✅ `repo` - Full control of private repositories
  - `repo:status` - Access commit status
  - `repo_deployment` - Access deployment status
  - `public_repo` - Access public repositories
  - `repo:invite` - Access repository invitations
  - `security_events` - Read and write security events

---

### 2. PUBLIC_REPO_URL

**Tipo**: String (URL do repositório)

**Valor**: `github.com/defarm-repo/engines-rust-public.git`

⚠️ **IMPORTANTE**:
- ❌ NÃO inclua `https://` ou `git@` no início
- ❌ NÃO inclua protocolo
- ✅ Use apenas: `github.com/usuario/repositorio.git`

**Exemplos**:
```
✅ CORRETO: github.com/defarm-repo/engines-rust-public.git
❌ ERRADO:  https://github.com/defarm-repo/engines-rust-public.git
❌ ERRADO:  git@github.com:defarm-repo/engines-rust-public.git
```

---

## 🔧 Como Configurar no GitHub

### Passo a Passo

1. **Acesse o repositório privado no GitHub**:
   ```
   https://github.com/defarm-repo/engines
   ```

2. **Vá para Settings**:
   - Clique na aba "Settings" (ícone de engrenagem)
   - No menu lateral, selecione "Secrets and variables" → "Actions"

3. **Adicione o primeiro secret**:
   - Clique em "New repository secret"
   - **Name**: `PUBLIC_REPO_TOKEN`
   - **Secret**: Cole o Personal Access Token gerado
   - Clique em "Add secret"

4. **Adicione o segundo secret**:
   - Clique em "New repository secret" novamente
   - **Name**: `PUBLIC_REPO_URL`
   - **Secret**: `github.com/defarm-repo/engines-rust-public.git`
   - Clique em "Add secret"

5. **Verificação**:
   - Você deve ver dois secrets listados:
     ```
     PUBLIC_REPO_TOKEN    •••••••••••••••••  Updated X seconds ago
     PUBLIC_REPO_URL      •••••••••••••••••  Updated X seconds ago
     ```

---

## 🔒 Segurança

### Boas Práticas

1. **Token Rotation**:
   - Troque o PAT a cada 90 dias
   - Crie lembrete no calendário
   - GitHub pode notificar próximo à expiração

2. **Scope Mínimo**:
   - Use apenas `repo` scope
   - Não adicione permissões desnecessárias
   - Princípio do menor privilégio

3. **Secrets Management**:
   - NUNCA commite tokens no código
   - NUNCA compartilhe tokens via chat/email
   - Use GitHub Secrets exclusivamente

4. **Auditoria**:
   - Monitore GitHub Actions logs
   - Verifique acessos ao repositório
   - Revise tokens periodicamente em https://github.com/settings/tokens

### Revogar Token (Se Comprometido)

Se você suspeitar que o token foi comprometido:

1. Acesse: https://github.com/settings/tokens
2. Encontre o token "Dual Remote Sync Token"
3. Clique em "Delete" ou "Revoke"
4. Gere novo token seguindo passos acima
5. Atualize o secret `PUBLIC_REPO_TOKEN` no GitHub

---

## 🧪 Testar Configuração

Depois de configurar os secrets:

1. **Verifique workflow exists**:
   ```bash
   cat .github/workflows/sync-to-public.yml
   ```

2. **Faça um commit de teste**:
   ```bash
   echo "# Test" >> TEST.md
   git add TEST.md
   git commit -m "test: dual remote sync"
   git push origin main
   ```

3. **Monitore o workflow**:
   - Acesse: https://github.com/defarm-repo/engines/actions
   - Clique no workflow "Sync to Public Repository"
   - Veja os logs em tempo real

4. **Verifique repositório público**:
   - Acesse: https://github.com/defarm-repo/engines-rust-public
   - Confirme que o push foi sincronizado
   - Verifique que arquivos .md NÃO aparecem

---

## 🐛 Troubleshooting

### Erro: "PUBLIC_REPO_TOKEN not configured"

**Sintoma**: Workflow executa mas pula o push

**Causa**: Secret não foi configurado ou nome está errado

**Solução**:
1. Verifique Settings → Secrets → Actions
2. Confirme nome exato: `PUBLIC_REPO_TOKEN` (case-sensitive)
3. Recrie o secret se necessário

---

### Erro: "Authentication failed"

**Sintoma**: Workflow falha ao fazer push

**Causa**: Token inválido, expirado ou sem permissões

**Solução**:
1. Verifique expiração em: https://github.com/settings/tokens
2. Confirme que scope `repo` está marcado
3. Gere novo token e atualize secret

---

### Erro: "Repository not found"

**Sintoma**: Push falha com erro 404

**Causa**: URL do repositório incorreta ou inexistente

**Solução**:
1. Verifique que `engines-rust-public` existe
2. Confirme URL: `github.com/defarm-repo/engines-rust-public.git`
3. Verifique secret `PUBLIC_REPO_URL` (sem https://)

---

### Erro: "Permission denied"

**Sintoma**: Push falha com erro de permissão

**Causa**: Token não tem acesso ao repositório público

**Solução**:
1. Confirme que você é owner de ambos repositórios
2. Verifique permissões do token
3. Teste acesso manual: `git clone https://TOKEN@github.com/...`

---

## 📚 Recursos

- **GitHub Actions Secrets**: https://docs.github.com/en/actions/security-guides/encrypted-secrets
- **Personal Access Tokens**: https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/creating-a-personal-access-token
- **Token Permissions**: https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/scopes-for-oauth-apps

---

## ✅ Checklist de Configuração

Use este checklist para confirmar que tudo está configurado:

- [ ] Personal Access Token gerado com scope `repo`
- [ ] Secret `PUBLIC_REPO_TOKEN` adicionado no GitHub
- [ ] Secret `PUBLIC_REPO_URL` adicionado no GitHub
- [ ] Repositório público `engines-rust-public` existe
- [ ] Workflow `.github/workflows/sync-to-public.yml` commitado
- [ ] Teste realizado com commit dummy
- [ ] Workflow executou com sucesso
- [ ] Repositório público recebeu código filtrado
- [ ] Arquivos .md NÃO aparecem no público

---

**Data de criação**: 2025-11-05
**Repositório privado**: https://github.com/defarm-repo/engines
**Repositório público**: https://github.com/defarm-repo/engines-rust-public

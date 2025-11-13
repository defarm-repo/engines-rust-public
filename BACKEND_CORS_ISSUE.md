# 🚨 Backend Freeze Issue - CORS/Proxy Configuration

## Problema Observado

**Sintoma**: Frontend "congela" ao tentar fazer login ou qualquer operação que chame a API
**Erro no Browser**: `Failed to fetch` ao fazer requisições para `https://connect.defarm.net/api/*`

## Diagnóstico

### Request que Falha:
```
POST https://connect.defarm.net/api/auth/login
Origin: https://a08bbda8-1b2d-44e7-98e1-9445d1a34d52.lovableproject.com
Error: Failed to fetch
```

### O que "Failed to fetch" significa:
Este erro ocorre ANTES de qualquer resposta HTTP, indicando que o browser não conseguiu completar a request. Causas comuns:

## 🎯 Causas Prováveis (em ordem de probabilidade)

### 1. ❌ CORS Não Configurado para o Subdomínio
**Provável causa raiz**: O backend em `defarm-engines-api-production.up.railway.app` provavelmente tem CORS configurado, MAS o proxy/subdomain `connect.defarm.net` pode estar bloqueando os headers CORS ou não os repassando corretamente.

**Verificar**:
```bash
# Testar CORS do Railway direto:
curl -X OPTIONS https://defarm-engines-api-production.up.railway.app/api/auth/login \
  -H "Origin: https://a08bbda8-1b2d-44e7-98e1-9445d1a34d52.lovableproject.com" \
  -H "Access-Control-Request-Method: POST" \
  -v

# Testar CORS do subdomain:
curl -X OPTIONS https://connect.defarm.net/api/auth/login \
  -H "Origin: https://a08bbda8-1b2d-44e7-98e1-9445d1a34d52.lovableproject.com" \
  -H "Access-Control-Request-Method: POST" \
  -v
```

**Headers CORS necessários** que o backend deve retornar:
```
Access-Control-Allow-Origin: https://a08bbda8-1b2d-44e7-98e1-9445d1a34d52.lovableproject.com
Access-Control-Allow-Methods: GET, POST, PUT, PATCH, DELETE, OPTIONS
Access-Control-Allow-Headers: Content-Type, Authorization
Access-Control-Allow-Credentials: true
```

**Importante**: Para ambiente de desenvolvimento/preview, considerar:
```javascript
// Permitir todos os subdomains do Lovable
Access-Control-Allow-Origin: *.lovableproject.com (ou usar regex)
// OU configurar lista de origins permitidos incluindo o preview
```

### 2. 🌐 Proxy/DNS do Subdomain Mal Configurado
O subdomain `connect.defarm.net` pode não estar configurado corretamente para fazer proxy para `defarm-engines-api-production.up.railway.app`.

**Verificar**:
```bash
# 1. Testar se o subdomain responde:
curl -v https://connect.defarm.net/api/auth/login

# 2. Verificar DNS:
dig connect.defarm.net
nslookup connect.defarm.net

# 3. Testar se o proxy está funcionando:
curl -X POST https://connect.defarm.net/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"hen","password":"demo123"}' \
  -v
```

**Possíveis problemas de proxy**:
- Cloudflare/proxy não configurado para passar headers CORS
- Timeout muito curto no proxy
- SSL certificate do subdomain inválido ou não configurado
- Proxy não repassa corretamente os headers de request/response

### 3. 🔒 SSL/TLS Certificate Issue
Se o certificado SSL do `connect.defarm.net` estiver inválido ou não configurado, browsers modernos bloqueiam a request.

**Verificar**:
```bash
# Verificar certificado SSL:
openssl s_client -connect connect.defarm.net:443 -servername connect.defarm.net

# Ou usar online tools:
# https://www.ssllabs.com/ssltest/analyze.html?d=connect.defarm.net
```

## ✅ Soluções Recomendadas

### Solução 1: Configurar CORS no Backend para Origins de Preview
```javascript
// Node.js/Express example
const cors = require('cors');

const allowedOrigins = [
  'https://connect.defarm.net',
  /https:\/\/.*\.lovableproject\.com$/, // Permite todos os previews do Lovable
  'http://localhost:5173', // Dev local
];

app.use(cors({
  origin: function(origin, callback) {
    // Permite requests sem origin (mobile apps, Postman, etc)
    if (!origin) return callback(null, true);
    
    const isAllowed = allowedOrigins.some(allowed => {
      if (allowed instanceof RegExp) {
        return allowed.test(origin);
      }
      return allowed === origin;
    });
    
    if (isAllowed) {
      callback(null, true);
    } else {
      callback(new Error('Not allowed by CORS'));
    }
  },
  credentials: true,
  methods: ['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'OPTIONS'],
  allowedHeaders: ['Content-Type', 'Authorization'],
}));
```

### Solução 2: Configurar Proxy Corretamente (Cloudflare/Nginx/outro)

**Se usando Cloudflare**:
1. Verificar que "Proxy status" está ativo (orange cloud)
2. Verificar que SSL/TLS está em modo "Full" ou "Full (strict)"
3. Adicionar regra de Page Rule para passar headers CORS

**Se usando Nginx**:
```nginx
server {
    listen 443 ssl;
    server_name connect.defarm.net;
    
    # SSL config
    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;
    
    location / {
        # Proxy para Railway
        proxy_pass https://defarm-engines-api-production.up.railway.app;
        
        # Headers importantes
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # IMPORTANTE: Não sobrescrever headers CORS do backend
        proxy_hide_header Access-Control-Allow-Origin;
        proxy_hide_header Access-Control-Allow-Methods;
        proxy_hide_header Access-Control-Allow-Headers;
        
        # Timeout
        proxy_read_timeout 90s;
    }
}
```

### Solução 3: Alternativa Temporária - Usar Railway URL Diretamente
Se a configuração do subdomain está complexa, temporariamente usar a URL do Railway direto no frontend:

```typescript
// src/lib/api/config.ts
baseUrl: 'https://defarm-engines-api-production.up.railway.app'
```

E configurar CORS no backend para aceitar `*.lovableproject.com`.

## 🧪 Como Testar a Solução

1. **Teste de CORS com curl**:
```bash
curl -X OPTIONS https://connect.defarm.net/api/auth/login \
  -H "Origin: https://a08bbda8-1b2d-44e7-98e1-9445d1a34d52.lovableproject.com" \
  -H "Access-Control-Request-Method: POST" \
  -v
```
Deve retornar status 200 ou 204 com headers CORS.

2. **Teste de POST real**:
```bash
curl -X POST https://connect.defarm.net/api/auth/login \
  -H "Content-Type: application/json" \
  -H "Origin: https://a08bbda8-1b2d-44e7-98e1-9445d1a34d52.lovableproject.com" \
  -d '{"username":"hen","password":"demo123"}' \
  -v
```
Deve retornar 200 com token OU erro de credenciais (mas não "Failed to fetch").

3. **Teste no browser console**:
```javascript
fetch('https://connect.defarm.net/api/auth/login', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ username: 'hen', password: 'demo123' })
})
.then(r => r.json())
.then(console.log)
.catch(console.error);
```

## 📋 Checklist para Backend Team

- [ ] Verificar logs do Railway - há requests chegando?
- [ ] Testar CORS direto no Railway URL (sem subdomain)
- [ ] Testar CORS através do subdomain connect.defarm.net
- [ ] Verificar se o proxy/Cloudflare está configurado corretamente
- [ ] Confirmar que certificado SSL do subdomain é válido
- [ ] Adicionar `*.lovableproject.com` nos allowed origins do CORS
- [ ] Testar com curl (OPTIONS e POST)
- [ ] Testar no browser console
- [ ] Confirmar que headers CORS estão sendo retornados corretamente

## 🎯 Próximos Passos

1. **Backend team**: Execute os testes de CORS acima e compartilhe os resultados
2. **Frontend**: Aguardar correção do backend/proxy
3. **Alternativa**: Se urgente, podemos temporariamente usar a URL do Railway direto

---
**Status**: 🔴 BLOQUEADO - Aguardando configuração de CORS/Proxy no backend
**Data**: 2025-11-10
**Prioridade**: 🚨 CRÍTICA - Sistema completamente inutilizável sem isso

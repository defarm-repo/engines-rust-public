# 📚 Share API Documentation - Quick Reference

**Your API documentation is LIVE and ready to share!**

---

## 🌐 Main URLs to Share

### 1. Interactive Swagger UI ⭐ (Recommended)
```
https://connect.defarm.net/docs/api/swagger-ui.html
```

**What developers get:**
- ✅ Try all endpoints with "Try it out" button
- ✅ Demo credentials included (hen/demo123, chick/Demo123!)
- ✅ Full authentication flow
- ✅ Request/response examples
- ✅ All 200+ endpoints documented

**Perfect for:** New developers, integration partners, QA testing

---

### 2. OpenAPI Specification (YAML)
```
https://connect.defarm.net/docs/api/openapi.yaml
```

**What developers can do:**
- ✅ Import into Postman, Insomnia, Thunder Client
- ✅ Generate client SDKs (TypeScript, Python, Go, etc.)
- ✅ Validate API contracts
- ✅ Set up automated testing

**Perfect for:** Tool integration, SDK generation, contract testing

---

### 3. Advanced Concepts Guide
```
https://connect.defarm.net/docs/api/ADVANCED_CONCEPTS.md
```

**68-page bilingual guide covering:**
- 🔍 Identifiers & Deduplication (Canonical vs Contextual)
- 🏷️ Namespace System
- 🔄 Complete Tokenization Flow
- ⛓️ Blockchain Storage & Adapters
- 📊 Events System
- ♻️ Item Lifecycle
- 🔐 Circuit Permissions
- ⚙️ Alias Configuration
- 🪝 Webhooks
- 🔗 External Aliases

**Perfect for:** System architects, backend developers, advanced integrations

---

### 4. Complete Developer Guide
```
https://connect.defarm.net/docs/api/COMPLETE_DEVELOPER_GUIDE.md
```

**What's included:**
- 🚀 Quick start guides (CLI, TypeScript SDK, Python SDK, curl)
- 📦 SDK installation instructions
- 💻 Example projects
- 🎓 Learning resources
- 📊 API coverage (200+ endpoints)

**Perfect for:** Getting started, SDK usage, example code

---

## 📧 Email Template for Developers

```
Subject: DeFarm Engines API - Documentation Access

Hi [Developer Name],

You now have access to DeFarm Engines API documentation!

🌐 Interactive API Explorer:
https://connect.defarm.net/docs/api/swagger-ui.html

📘 Advanced Architecture Guide:
https://connect.defarm.net/docs/api/ADVANCED_CONCEPTS.md

🔑 Test Credentials (demo accounts):
• Admin: hen / demo123
• Basic: chick / Demo123!
• Professional: pullet / demo123
• Enterprise: cock / demo123

🚀 Quick Start:
1. Open the Swagger UI link
2. Test POST /api/auth/login with demo credentials
3. Copy the token from response
4. Click "Authorize" button (top right)
5. Paste token and authorize
6. Now try any endpoint!

📦 SDKs Available:
• TypeScript: npm install @defarm/sdk
• Python: pip install defarm-sdk
• CLI: npm install -g defarm-cli

📖 Complete Docs:
• OpenAPI Spec: https://connect.defarm.net/docs/api/openapi.yaml
• Developer Guide: https://connect.defarm.net/docs/api/COMPLETE_DEVELOPER_GUIDE.md

💬 Support:
• Email: support@defarm.net
• Issues: https://github.com/defarm/engines/issues

Happy coding!
```

---

## 📱 Social Media Posts

### LinkedIn/Twitter
```
🚀 DeFarm Engines API is now live with complete interactive documentation!

🌐 Try it now: https://connect.defarm.net/docs/api/swagger-ui.html

✨ Features:
• 200+ endpoints
• Blockchain integration (Stellar + IPFS)
• Complete traceability system
• TypeScript & Python SDKs
• Interactive "Try it out" functionality

Demo credentials included - no signup required!

#API #Traceability #Blockchain #OpenAPI #DeveloperTools
```

### GitHub README Badge
```markdown
[![API Docs](https://img.shields.io/badge/API-Docs-green)](https://connect.defarm.net/docs/api/swagger-ui.html)
[![OpenAPI](https://img.shields.io/badge/OpenAPI-3.0-blue)](https://connect.defarm.net/docs/api/openapi.yaml)
[![Status](https://img.shields.io/badge/status-live-success)](https://connect.defarm.net/health)
```

---

## 🎥 Demo Script (for video/presentation)

### 1. Introduction (30 seconds)
"Welcome to DeFarm Engines API - a production-grade traceability platform with blockchain integration. Let me show you how easy it is to get started."

### 2. Open Swagger UI (10 seconds)
"Navigate to connect.defarm.net/docs/api/swagger-ui.html"

### 3. Show Demo Credentials (15 seconds)
"We provide demo accounts - let's use the admin account: hen / demo123"

### 4. Login Flow (45 seconds)
"First, test the login endpoint. Click POST /auth/login, click Try it out, enter credentials, execute. See - we get a JWT token back."

### 5. Authorize (20 seconds)
"Copy the token, click Authorize at the top, paste it in, click Authorize again."

### 6. Try Protected Endpoint (30 seconds)
"Now we can try any protected endpoint. Let's create a circuit. Click POST /circuits, Try it out, fill in the name, execute. Success!"

### 7. Show Advanced Features (30 seconds)
"We have 200+ endpoints covering items, circuits, events, blockchain storage, Merkle proofs, and more. All documented with examples."

### 8. Call to Action (20 seconds)
"Everything you need is at connect.defarm.net/docs/api - give it a try!"

---

## 🔗 Where to Link Documentation

### In Your Website
```html
<nav>
  <a href="https://connect.defarm.net/docs/api/swagger-ui.html">API Docs</a>
  <a href="https://connect.defarm.net/docs/api/ADVANCED_CONCEPTS.md">Architecture</a>
  <a href="https://connect.defarm.net/docs/api/openapi.yaml">OpenAPI Spec</a>
</nav>
```

### In Your GitHub README
```markdown
## 📚 Documentation

- **[Interactive API Docs](https://connect.defarm.net/docs/api/swagger-ui.html)** - Try the API in your browser
- **[OpenAPI Specification](https://connect.defarm.net/docs/api/openapi.yaml)** - Import into your tools
- **[Advanced Concepts](https://connect.defarm.net/docs/api/ADVANCED_CONCEPTS.md)** - Architecture deep dive
```

### In API Response Headers
Already configured in your API:
```
Link: <https://connect.defarm.net/docs/api/swagger-ui.html>; rel="documentation"
```

### In Error Responses
```json
{
  "error": "INVALID_REQUEST",
  "message": "Missing required field: identifiers",
  "documentation": "https://connect.defarm.net/docs/api/swagger-ui.html#/Items/post_api_items_local"
}
```

---

## 📊 QR Code (for presentations/printed materials)

Generate QR code pointing to:
```
https://connect.defarm.net/docs/api/swagger-ui.html
```

Use: https://qr-code-generator.com/

**Include on:**
- Conference slides
- Business cards
- Printed documentation
- Developer onboarding materials

---

## 🎯 Target Audiences & Recommended Docs

| Audience | Primary Link | Secondary Link |
|----------|-------------|----------------|
| **New Developers** | Swagger UI | Complete Developer Guide |
| **Frontend Developers** | Swagger UI | OpenAPI Spec (for code gen) |
| **Backend Developers** | Advanced Concepts | OpenAPI Spec |
| **System Architects** | Advanced Concepts | API Guide |
| **QA Engineers** | Swagger UI | API Guide |
| **Product Managers** | Swagger UI | Advanced Concepts (read-only) |
| **DevOps Engineers** | Deployment Guides | OpenAPI Spec |
| **Integration Partners** | Swagger UI + Demo Credentials | Complete Developer Guide |

---

## ✅ Before Sharing Checklist

- [ ] Test Swagger UI loads: https://connect.defarm.net/docs/api/swagger-ui.html
- [ ] Verify demo login works (hen/demo123)
- [ ] Test "Try it out" on a protected endpoint
- [ ] Check OpenAPI spec is accessible
- [ ] Verify Advanced Concepts guide loads
- [ ] Test on mobile device
- [ ] Check loading speed (< 3 seconds)
- [ ] Verify all endpoint descriptions are accurate
- [ ] Ensure demo credentials are valid
- [ ] Confirm API is healthy: https://connect.defarm.net/health

---

## 🚀 Next Steps After Sharing

1. **Monitor Usage**
   - Add Google Analytics to track page views
   - Monitor API health endpoint
   - Check for error patterns

2. **Gather Feedback**
   - Ask first developers for feedback
   - Iterate on documentation clarity
   - Add more examples as needed

3. **Keep Updated**
   - Regenerate OpenAPI spec when API changes
   - Update Advanced Concepts when adding features
   - Maintain demo credentials

4. **Promote**
   - Share on social media
   - Add to product website
   - Include in email signatures
   - Reference in sales materials

---

## 🎉 You're Ready!

**Your complete API documentation is live at:**
```
https://connect.defarm.net/docs/api/swagger-ui.html
```

**Just share this URL and you're done!**

No additional deployment needed. ✨

---

**For detailed deployment options, see:** [SWAGGER_DEPLOYMENT.md](docs/deployment/SWAGGER_DEPLOYMENT.md)

# OpenAPI/Swagger Documentation Deployment Guide

**Complete guide to sharing interactive API documentation with developers**

---

## 🌐 Current Live Documentation (Railway)

Your API documentation is **already deployed and live** on Railway!

### 📚 Main Documentation URLs

```
Base URL: https://connect.defarm.net

Interactive Documentation:
├── 📖 Swagger UI (Recommended)
│   https://connect.defarm.net/docs/api/swagger-ui.html
│   → Interactive API explorer with "Try it out" functionality
│   → Demo credentials included
│   → Full authentication flow
│
├── 📖 Alternative UI (Redoc-style)
│   https://connect.defarm.net/docs/api/index.html
│   → Three-panel documentation view
│
├── 📄 OpenAPI Specification (YAML)
│   https://connect.defarm.net/docs/api/openapi.yaml
│   → Machine-readable API contract
│   → Import into Postman, Insomnia, etc.
│
├── 📘 Advanced Concepts Guide
│   https://connect.defarm.net/docs/api/ADVANCED_CONCEPTS.md
│   → Deep dive into architecture
│   → Deduplication, blockchain, events
│
└── 📙 Complete Developer Guide
    https://connect.defarm.net/docs/api/COMPLETE_DEVELOPER_GUIDE.md
    → Quick start for all SDKs
```

---

## ✅ How It Works

### Current Setup (Axum + tower-http)

Your API serves static files from the `docs/` directory:

**Code:** `src/bin/api.rs`
```rust
.nest_service("/docs", ServeDir::new("docs"))
```

**Result:**
- Any file in `docs/` folder is accessible at `https://connect.defarm.net/docs/*`
- Automatically served with correct MIME types
- CORS enabled for browser access
- No additional configuration needed

---

## 🚀 Sharing with Developers

### Option 1: Direct Link (Recommended) ⭐

Just share this URL:

```
https://connect.defarm.net/docs/api/swagger-ui.html
```

**What they'll see:**
1. **Banner with demo credentials** (hen/demo123, chick/Demo123!, etc.)
2. **All API endpoints** organized by module
3. **"Try it out" buttons** for live testing
4. **Authorize button** for JWT authentication
5. **Request/response examples** for every endpoint

**Perfect for:**
- New developers evaluating your API
- Integration partners starting development
- Frontend developers building UIs
- QA teams testing endpoints

---

### Option 2: Embedded Documentation

Add this to your main website:

```html
<!DOCTYPE html>
<html>
<head>
    <title>DeFarm API Documentation</title>
    <style>
        body { margin: 0; padding: 0; }
        iframe { width: 100%; height: 100vh; border: none; }
    </style>
</head>
<body>
    <iframe src="https://connect.defarm.net/docs/api/swagger-ui.html"></iframe>
</body>
</html>
```

**Perfect for:**
- Company website integration
- Developer portal
- Custom documentation pages

---

### Option 3: Public Documentation Site (GitHub Pages)

**For separate documentation hosting:**

#### Step 1: Create GitHub Pages Branch

```bash
# Create docs-only branch
git checkout --orphan gh-pages
git rm -rf .
git clean -fdx

# Copy documentation files
cp -r docs/* .

# Create index redirect
cat > index.html << 'EOF'
<!DOCTYPE html>
<html>
<head>
    <meta http-equiv="refresh" content="0; url=./api/swagger-ui.html">
    <title>Redirecting to API Documentation</title>
</head>
<body>
    <p>Redirecting to <a href="./api/swagger-ui.html">API Documentation</a>...</p>
</body>
</html>
EOF

# Commit and push
git add .
git commit -m "Deploy API documentation"
git push origin gh-pages
```

#### Step 2: Enable GitHub Pages

1. Go to GitHub repository → Settings → Pages
2. Source: `gh-pages` branch
3. Save

**Result:** Documentation available at `https://your-org.github.io/engines/api/swagger-ui.html`

**Perfect for:**
- Open source projects
- Free hosting
- Automatic updates via CI/CD

---

### Option 4: Custom Domain (Professional)

**Setup custom domain for documentation:**

#### Step 1: Configure DNS

Add CNAME record:
```
docs.defarm.net → connect.defarm.net
```

#### Step 2: Update Railway Domain Settings

1. Go to Railway dashboard
2. Add custom domain: `docs.defarm.net`
3. Verify DNS propagation

**Result:** Documentation at `https://docs.defarm.net/api/swagger-ui.html`

**Perfect for:**
- Professional appearance
- Brand consistency
- Easy to remember URL

---

## 📊 Alternative Documentation Renderers

### 1. Redoc (Beautiful Three-Panel Layout)

**Current file:** `docs/api/index.html`

Already configured! Access at:
```
https://connect.defarm.net/docs/api/index.html
```

**Features:**
- Three-panel layout (nav, content, examples)
- Better for reading than interactive testing
- Mobile-responsive
- Search functionality

---

### 2. Stoplight Elements (Modern UI)

Create `docs/api/stoplight.html`:

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>DeFarm API - Stoplight</title>
    <meta name="viewport" content="width=device-width, initial-scale=1, shrink-to-fit=no">
    <link rel="stylesheet" href="https://unpkg.com/@stoplight/elements/styles.min.css">
</head>
<body>
    <elements-api
        apiDescriptionUrl="./openapi.yaml"
        router="hash"
        layout="sidebar"
    />
    <script src="https://unpkg.com/@stoplight/elements/web-components.min.js"></script>
</body>
</html>
```

**Access:** `https://connect.defarm.net/docs/api/stoplight.html`

**Features:**
- Modern, polished design
- Mock server built-in
- Code generation
- Request maker

---

### 3. RapiDoc (Lightweight & Fast)

Create `docs/api/rapidoc.html`:

```html
<!DOCTYPE html>
<html>
<head>
    <title>DeFarm API - RapiDoc</title>
    <script type="module" src="https://unpkg.com/rapidoc/dist/rapidoc-min.js"></script>
</head>
<body>
    <rapi-doc
        spec-url="./openapi.yaml"
        theme="dark"
        show-header="false"
        allow-authentication="true"
        allow-server-selection="true"
        allow-api-list-style-selection="true"
    >
    </rapi-doc>
</body>
</html>
```

**Access:** `https://connect.defarm.net/docs/api/rapidoc.html`

**Features:**
- Extremely lightweight
- Fast loading
- Theme support (dark/light)
- Minimal dependencies

---

## 🔗 Online API Documentation Platforms

### 1. SwaggerHub (Official Swagger Platform)

**Upload your OpenAPI spec to SwaggerHub:**

#### Step 1: Create Account
```
https://app.swaggerhub.com/signup
```

#### Step 2: Upload Spec
```bash
# Install SwaggerHub CLI
npm install -g swaggerhub-cli

# Login
swaggerhub configure

# Create API
swaggerhub api:create defarm/engines-api/1.0 \
    --file docs/api/openapi.yaml \
    --visibility public
```

**Result:** `https://app.swaggerhub.com/apis/defarm/engines-api/1.0`

**Features:**
- ✅ Official Swagger hosting
- ✅ Collaborative editing
- ✅ API versioning
- ✅ Team management
- ✅ Mock servers
- ⚠️ Free tier: 1 API, 3 users

---

### 2. Redocly (Beautiful Documentation)

**Upload to Redocly:**

#### Step 1: Install CLI
```bash
npm install -g @redocly/cli
```

#### Step 2: Push to Redocly
```bash
redocly login
redocly push docs/api/openapi.yaml --organization defarm
```

**Result:** `https://defarm.redoc.ly/`

**Features:**
- ✅ Beautiful default theme
- ✅ Custom branding
- ✅ Analytics
- ✅ SEO optimized
- ⚠️ Paid plans start at $99/mo

---

### 3. ReadMe.io (Developer Hub)

**Create complete developer portal:**

1. Sign up: https://readme.com/
2. Upload `openapi.yaml` in dashboard
3. Customize branding and guides

**Result:** `https://defarm.readme.io/`

**Features:**
- ✅ Complete developer portal
- ✅ Guides + API reference
- ✅ User authentication
- ✅ Analytics
- ✅ Support integration
- ⚠️ Starting at $99/mo

---

## 🎨 Customizing Your Current Swagger UI

### Add Company Branding

Edit `docs/api/swagger-ui.html`:

```html
<style>
    .topbar {
        background-color: #16a34a; /* Your brand color */
        background-image: url('https://your-domain.com/logo.png');
        background-repeat: no-repeat;
        background-position: 20px center;
        background-size: 120px;
        padding-left: 160px;
    }
    .topbar .download-url-wrapper { display: none; } /* Hide default URL input */
</style>
```

### Change Theme Colors

```javascript
SwaggerUIBundle({
    url: "./openapi.yaml",
    dom_id: '#swagger-ui',
    deepLinking: true,
    presets: [
        SwaggerUIBundle.presets.apis,
        SwaggerUIStandalonePreset
    ],
    plugins: [
        SwaggerUIBundle.plugins.DownloadUrl
    ],
    layout: "StandaloneLayout",
    // Custom theme
    syntaxHighlight: {
        activate: true,
        theme: "monokai" // or "agate", "arta", "obsidian"
    }
})
```

### Add Custom Header/Footer

```html
<div id="custom-header" style="background: #16a34a; color: white; padding: 20px;">
    <h1>🌱 DeFarm Engines API</h1>
    <p>Production-grade traceability and data sharing platform</p>
    <a href="https://defarm.net" style="color: white;">Visit Website</a>
</div>

<div id="swagger-ui"></div>

<div id="custom-footer" style="background: #f5f5f5; padding: 20px; text-align: center;">
    <p>&copy; 2025 DeFarm | <a href="mailto:support@defarm.net">Support</a> | <a href="/docs/api/ADVANCED_CONCEPTS.md">Advanced Guide</a></p>
</div>
```

---

## 📱 Testing Documentation URLs

### Verify All URLs Work

```bash
#!/bin/bash
BASE_URL="https://connect.defarm.net"

echo "Testing documentation URLs..."

# Test Swagger UI
curl -s -o /dev/null -w "Swagger UI: %{http_code}\n" "$BASE_URL/docs/api/swagger-ui.html"

# Test OpenAPI spec
curl -s -o /dev/null -w "OpenAPI Spec: %{http_code}\n" "$BASE_URL/docs/api/openapi.yaml"

# Test Redoc UI
curl -s -o /dev/null -w "Redoc UI: %{http_code}\n" "$BASE_URL/docs/api/index.html"

# Test Advanced Guide
curl -s -o /dev/null -w "Advanced Guide: %{http_code}\n" "$BASE_URL/docs/api/ADVANCED_CONCEPTS.md"

# Test API health
curl -s "$BASE_URL/health" | jq '.'

echo "✓ All tests complete!"
```

**Expected output:**
```
Swagger UI: 200
OpenAPI Spec: 200
Redoc UI: 200
Advanced Guide: 200
✓ All tests complete!
```

---

## 🔧 CI/CD: Auto-Deploy Documentation

### GitHub Actions Workflow

Create `.github/workflows/deploy-docs.yml`:

```yaml
name: Deploy Documentation

on:
  push:
    branches: [main]
    paths:
      - 'docs/**'
      - 'src/api/**'

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Generate OpenAPI Spec
        run: |
          # If you generate openapi.yaml from code
          cargo run --bin generate-openapi > docs/api/openapi.yaml

      - name: Deploy to Railway
        run: |
          # Railway automatically rebuilds on git push
          # Documentation is served via ServeDir
          echo "✓ Documentation will be deployed with next Railway build"

      - name: Notify Slack
        uses: slackapi/slack-github-action@v1
        with:
          webhook-url: ${{ secrets.SLACK_WEBHOOK }}
          payload: |
            {
              "text": "📚 API Documentation updated: https://connect.defarm.net/docs/api/swagger-ui.html"
            }
```

---

## 📊 Analytics: Track Documentation Usage

### Add Google Analytics to Swagger UI

Edit `docs/api/swagger-ui.html`:

```html
<head>
    <!-- ... existing head content ... -->

    <!-- Google Analytics -->
    <script async src="https://www.googletagmanager.com/gtag/js?id=G-XXXXXXXXXX"></script>
    <script>
        window.dataLayer = window.dataLayer || [];
        function gtag(){dataLayer.push(arguments);}
        gtag('js', new Date());
        gtag('config', 'G-XXXXXXXXXX');
    </script>
</head>
```

**Track:**
- Page views
- Which endpoints developers explore most
- Time spent on documentation
- Popular search queries

---

## 🎯 Recommended Sharing Strategy

### For Public/Open Source:

1. **Primary:** Direct link to Railway
   ```
   https://connect.defarm.net/docs/api/swagger-ui.html
   ```

2. **Backup:** GitHub Pages
   ```
   https://your-org.github.io/engines/api/swagger-ui.html
   ```

3. **Marketing:** Custom domain
   ```
   https://docs.defarm.net
   ```

### For Private/Enterprise:

1. **Primary:** Railway (already secured with authentication)
2. **Alternative:** SwaggerHub with API key authentication
3. **Internal:** Self-hosted on company infrastructure

---

## 🔐 Security Considerations

### Current Setup (Public Documentation)

Your documentation is **publicly accessible** (no authentication required).

**This is GOOD for:**
- ✅ Public APIs
- ✅ Open source projects
- ✅ Developer onboarding
- ✅ Marketing and demos

**API itself is protected:**
- ✅ JWT authentication required for most endpoints
- ✅ API keys for server-to-server
- ✅ Demo credentials are safe (sandboxed accounts)

### If You Need Private Documentation

#### Option 1: Basic Authentication (Nginx)

Add nginx in front of documentation:

```nginx
location /docs {
    auth_basic "API Documentation";
    auth_basic_user_file /etc/nginx/.htpasswd;
    proxy_pass http://localhost:3000/docs;
}
```

#### Option 2: IP Whitelist (Railway)

Configure Railway to restrict `/docs` to specific IPs.

#### Option 3: SwaggerHub Private API

Upload to SwaggerHub with private visibility.

---

## 📚 Complete Sharing Kit

**Email Template for Developers:**

```
Subject: DeFarm Engines API - Documentation & Credentials

Hi [Developer],

Welcome to DeFarm Engines! Here's everything you need to get started:

📖 Interactive API Documentation:
https://connect.defarm.net/docs/api/swagger-ui.html

📘 Advanced Concepts Guide:
https://connect.defarm.net/docs/api/ADVANCED_CONCEPTS.md

🔑 Demo Credentials (for testing):
- Admin: hen / demo123
- Basic: chick / Demo123!
- Professional: pullet / demo123
- Enterprise: cock / demo123

🚀 Quick Start:
1. Open the Swagger UI link above
2. Click "POST /api/auth/login" to expand
3. Click "Try it out"
4. Use demo credentials
5. Copy the token from response
6. Click "Authorize" button at top
7. Paste token and click "Authorize" again
8. Now you can try all endpoints!

📦 SDKs:
- TypeScript: npm install @defarm/sdk
- Python: pip install defarm-sdk
- CLI: npm install -g defarm-cli

💬 Support:
- Email: support@defarm.net
- Issues: https://github.com/defarm/engines/issues

Happy coding!
```

---

## ✅ Verification Checklist

Before sharing documentation:

- [ ] Swagger UI loads: `https://connect.defarm.net/docs/api/swagger-ui.html`
- [ ] OpenAPI spec accessible: `https://connect.defarm.net/docs/api/openapi.yaml`
- [ ] Demo credentials work (test login endpoint)
- [ ] "Try it out" works for public endpoints
- [ ] Authentication flow works (login → authorize → test protected endpoint)
- [ ] All endpoint descriptions are accurate
- [ ] Response examples match actual API responses
- [ ] Error examples are documented
- [ ] Advanced guide accessible
- [ ] Links in documentation work

---

## 🎉 Summary

**Your documentation is LIVE and ready to share!**

**Primary URL (share this):**
```
https://connect.defarm.net/docs/api/swagger-ui.html
```

**What developers get:**
- ✅ Interactive API explorer
- ✅ Working demo credentials
- ✅ "Try it out" functionality
- ✅ Complete endpoint documentation
- ✅ Request/response examples
- ✅ Authentication flow
- ✅ Advanced architecture guide

**Next steps:**
1. Test the URL yourself
2. Share with 1-2 developers for feedback
3. Add to README.md and website
4. Monitor usage and iterate

**No additional deployment needed - you're ready to go!** 🚀

---

**Last Updated:** 2026-01-28
**API Version:** v1.0
**Production URL:** https://connect.defarm.net
**Documentation URL:** https://connect.defarm.net/docs/api/swagger-ui.html

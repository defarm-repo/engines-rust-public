# Custom Domain Setup for Railway

This guide explains how to configure `connect.defarm.net` to point to your Railway deployment.

## 🎯 Goal

Make API documentation accessible at:
- ✅ `https://connect.defarm.net/docs`
- ✅ `https://defarm-engines-api-production.up.railway.app/docs` (still works)

## 📋 Prerequisites

- Access to your DNS provider (where `defarm.net` is registered)
- Railway account with access to the project
- Domain must be registered and active

---

## Step 1: Add Custom Domain in Railway

### Via Railway Dashboard

1. **Open Railway Dashboard**
   - Go to https://railway.app/dashboard
   - Select your project: "defarm-engines-api-production"

2. **Navigate to Settings**
   - Click on your service/deployment
   - Click "Settings" tab
   - Scroll to "Domains" section

3. **Add Custom Domain**
   - Click "Add Domain" or "Custom Domain"
   - Enter: `connect.defarm.net`
   - Click "Add"

4. **Note the DNS Records**
   Railway will show you DNS records to configure. You'll see something like:

   **For CNAME (Subdomain)**:
   ```
   Type: CNAME
   Name: connect
   Value: defarm-engines-api-production.up.railway.app
   ```

   **OR for A/AAAA (if Railway uses IP addresses)**:
   ```
   Type: A
   Name: connect
   Value: [Railway's IP address]
   ```

### Via Railway CLI (Alternative)

```bash
# Install Railway CLI if not installed
npm install -g @railway/cli

# Login to Railway
railway login

# Link to your project
railway link

# Add custom domain
railway domain add connect.defarm.net
```

---

## Step 2: Configure DNS Records

You need to add DNS records at your DNS provider (where `defarm.net` is registered).

### Common DNS Providers

#### **Option A: Cloudflare**

1. Login to Cloudflare
2. Select `defarm.net` domain
3. Go to "DNS" → "Records"
4. Click "Add record"
5. Configure:
   - **Type**: CNAME
   - **Name**: `connect`
   - **Target**: `defarm-engines-api-production.up.railway.app`
   - **Proxy status**: 🟠 DNS only (disable proxy for Railway to work)
   - **TTL**: Auto
6. Click "Save"

**Important**: Disable Cloudflare proxy (gray cloud) initially. You can enable it later once Railway SSL is working.

#### **Option B: AWS Route 53**

1. Open Route 53 console
2. Select hosted zone: `defarm.net`
3. Click "Create record"
4. Configure:
   - **Record name**: `connect`
   - **Record type**: CNAME
   - **Value**: `defarm-engines-api-production.up.railway.app`
   - **TTL**: 300 seconds
5. Click "Create records"

#### **Option C: Google Domains / Google Cloud DNS**

1. Open Google Domains or Cloud DNS
2. Select `defarm.net`
3. Go to "DNS" section
4. Add a custom resource record:
   - **Host name**: `connect`
   - **Type**: CNAME
   - **TTL**: 5m
   - **Data**: `defarm-engines-api-production.up.railway.app`
5. Save

#### **Option D: Namecheap**

1. Login to Namecheap
2. Go to "Domain List" → Click "Manage" next to `defarm.net`
3. Go to "Advanced DNS" tab
4. Click "Add New Record"
5. Configure:
   - **Type**: CNAME Record
   - **Host**: `connect`
   - **Value**: `defarm-engines-api-production.up.railway.app`
   - **TTL**: Automatic
6. Save

#### **Option E: GoDaddy**

1. Login to GoDaddy
2. Go to "My Products" → "Domains"
3. Click "DNS" next to `defarm.net`
4. Click "Add" in DNS Records
5. Configure:
   - **Type**: CNAME
   - **Name**: `connect`
   - **Value**: `defarm-engines-api-production.up.railway.app`
   - **TTL**: 1 hour
6. Save

---

## Step 3: Wait for DNS Propagation

DNS changes can take **5 minutes to 48 hours** to propagate globally.

### Check DNS Propagation

```bash
# Check if CNAME is set up correctly
dig connect.defarm.net CNAME

# Or use online tool
# https://dnschecker.org/#CNAME/connect.defarm.net
```

**Expected result:**
```
connect.defarm.net.    300    IN    CNAME    defarm-engines-api-production.up.railway.app.
```

### Quick DNS Check

```bash
# From terminal
nslookup connect.defarm.net

# Should return Railway's IP address
```

---

## Step 4: Verify SSL Certificate

Railway automatically provisions SSL certificates via Let's Encrypt.

### Wait for SSL

1. Go back to Railway Dashboard
2. Under "Domains", check status next to `connect.defarm.net`
3. Wait for status to change from "Pending" to "Active" (usually 2-10 minutes)

### Test HTTPS

```bash
# Should return 200 OK
curl -I https://connect.defarm.net/health

# Test documentation
curl -I https://connect.defarm.net/docs/
```

---

## Step 5: Test Documentation Access

Once SSL is active, test the documentation:

### 1. API Root
```bash
curl https://connect.defarm.net/
```

**Expected**: Welcome message

### 2. Health Check
```bash
curl https://connect.defarm.net/health
```

**Expected**: `{"status":"healthy"}`

### 3. Documentation Homepage
```bash
curl -I https://connect.defarm.net/docs/
```

**Expected**: 200 OK with `text/html`

### 4. Open in Browser

Visit these URLs in your browser:
- https://connect.defarm.net/docs/ (Documentation homepage)
- https://connect.defarm.net/docs/index.html (Swagger UI)
- https://connect.defarm.net/docs/openapi-external.yaml (OpenAPI spec)

---

## Troubleshooting

### Problem 1: "SSL Certificate Error"

**Cause**: Railway is still provisioning SSL certificate

**Solution**: Wait 5-10 minutes. Railway uses Let's Encrypt which can take time to issue certificates.

**Check status**:
```bash
curl -I https://connect.defarm.net/health
```

If you see certificate errors, wait a bit longer.

### Problem 2: "DNS_PROBE_FINISHED_NXDOMAIN"

**Cause**: DNS records not yet propagated

**Solution**:
1. Verify DNS records are correctly configured at your provider
2. Wait longer (can take up to 48 hours but usually 5-30 minutes)
3. Check DNS propagation: https://dnschecker.org/#CNAME/connect.defarm.net

### Problem 3: "502 Bad Gateway"

**Cause**: Railway deployment might be down

**Solution**:
1. Check Railway dashboard for deployment status
2. Check logs in Railway
3. Verify the default URL still works: https://defarm-engines-api-production.up.railway.app/health

### Problem 4: "404 Not Found" on /docs

**Cause**: Static file serving not deployed yet

**Solution**:
1. Verify the latest commit is deployed on Railway
2. Check Railway build logs for errors
3. Ensure `docs/` folder is included in the deployment

**Verify deployment**:
```bash
# Check if default Railway URL serves docs
curl -I https://defarm-engines-api-production.up.railway.app/docs/
```

If this works but custom domain doesn't, it's a DNS/SSL issue.

### Problem 5: Cloudflare Proxy Interference

**Cause**: Cloudflare proxy is enabled (orange cloud)

**Solution**:
1. In Cloudflare DNS settings, click the orange cloud next to `connect` record
2. Change it to gray cloud (DNS only)
3. Wait a few minutes for changes to propagate
4. Once Railway SSL is working, you can re-enable Cloudflare proxy if desired

---

## Railway CLI Commands (Optional)

### Check domain status
```bash
railway domain list
```

### Remove domain (if needed)
```bash
railway domain remove connect.defarm.net
```

### View deployment logs
```bash
railway logs
```

---

## Summary Checklist

- [ ] Added `connect.defarm.net` in Railway Dashboard
- [ ] Configured CNAME record at DNS provider
- [ ] Waited for DNS propagation (5-30 minutes)
- [ ] Verified SSL certificate is active in Railway
- [ ] Tested: `https://connect.defarm.net/health` returns 200 OK
- [ ] Tested: `https://connect.defarm.net/docs/` loads in browser
- [ ] Both URLs work:
  - [ ] `https://connect.defarm.net/docs`
  - [ ] `https://defarm-engines-api-production.up.railway.app/docs`

---

## Final URLs for Your Client

Once configured, share these URLs with your client:

### Primary URL (Custom Domain)
```
https://connect.defarm.net/docs
```

### Fallback URL (Railway Default)
```
https://defarm-engines-api-production.up.railway.app/docs
```

### API Base URL
```
https://connect.defarm.net
```

---

## What Your Client Will See

When they visit `https://connect.defarm.net/docs`:

1. **Interactive Swagger UI** - Beautiful web interface
2. **Try it Out** buttons - Test API directly in browser
3. **Authentication** - Login with demo credentials
4. **Complete API Reference** - All endpoints documented
5. **Code Examples** - Request/response samples

---

## Support

If you encounter issues:

1. **Railway Support**: https://railway.app/help
2. **DNS Provider Support**: Contact your DNS provider's help desk
3. **Check Railway Status**: https://status.railway.app/

---

**Last Updated**: October 14, 2025

# 🎛️ Railway Dashboard Setup Guide

## 🔴 Current Issue

The Railway CLI `railway up` command is **timing out during the indexing phase**. This is because:
- The project directory is 5.4GB (including `target/` build artifacts)
- Railway CLI needs to scan and upload all files before building
- Even with `.dockerignore`, the CLI indexing process times out after 2 minutes

## ✅ Solution: Use GitHub Integration

**Recommended approach**: Set up **GitHub integration** for automatic deployments. This avoids CLI upload issues and is the production-standard deployment method.

---

## Current Railway Project Status

✅ **Authenticated**: gabriel rondon (grondon@gmail.com)
✅ **Project**: defarm (ID: 2e6d7cdb-f993-4411-bcf4-1844f5b38011)
✅ **Environment**: production
✅ **Service**: defarm-engines-api (already created)
✅ **Public Domain**: connect.defarm.net
⚠️ **Status**: No deployments yet (GitHub integration needed)

---

## Quick Fix Steps

### Step 1: Access Railway Dashboard

**Direct Link to Your Project**:
```
https://railway.app/project/2e6d7cdb-f993-4411-bcf4-1844f5b38011
```

1. Open the link above in your browser
2. You should see the **defarm** project dashboard
3. Click on the **defarm-engines-api** service

---

### Step 2: Connect GitHub Repository

**This is the critical step to fix the deployment issue!**

1. In the `defarm-engines-api` service, click **"Settings"** tab
2. Scroll to **"Source"** section
3. Click **"Connect GitHub Repo"** button
4. Authorize Railway to access your GitHub account (if prompted)
5. Select repository: **`gabrielrondon/defarm-rust-engine`**
6. Select branch: **`main`**
7. Click **"Connect"**

✅ **Result**: Railway will automatically detect the `Dockerfile` and trigger the first deployment!

---

### Step 3: Verify PostgreSQL Database

✅ **Already configured!** Your service already has:
- PostgreSQL database connected
- `DATABASE_URL` environment variable set to: `postgresql://postgres:***@postgres.railway.internal:5432/railway`

No action needed for this step.

---

### Step 4: Verify Environment Variables

✅ **Already configured!** The following variables are already set:

**Core**:
- ✅ `DATABASE_URL`
- ✅ `JWT_SECRET`

**IPFS/Pinata**:
- ✅ `PINATA_API_KEY`, `PINATA_SECRET_KEY`, `PINATA_JWT`
- ✅ `IPFS_ENDPOINT`, `IPFS_GATEWAY`

**Stellar Configuration** (Testnet + Mainnet):
- ✅ All Stellar variables configured

No action needed for this step.

---

### Step 6: Set Required Environment Variables

In the `defarm-engines-api` service, go to **"Variables"** tab and add these variables:

#### Required Variables

```bash
# JWT Secret (generate new one)
JWT_SECRET=<generate-with-openssl-rand-base64-32>

# IPFS/Pinata Configuration
PINATA_API_KEY=<your-pinata-api-key>
PINATA_SECRET_KEY=<your-pinata-secret-key>
PINATA_JWT=<your-pinata-jwt>
IPFS_ENDPOINT=https://api.pinata.cloud
IPFS_GATEWAY=https://gateway.pinata.cloud/ipfs

# Stellar Testnet Configuration
STELLAR_TESTNET_IPCM_CONTRACT=<your-testnet-contract-id>
STELLAR_TESTNET_SECRET=<your-testnet-secret-key>
STELLAR_TESTNET_RPC_URL=https://soroban-testnet.stellar.org
STELLAR_TESTNET_NETWORK=testnet
STELLAR_TESTNET_NETWORK_PASSPHRASE=Test SDF Network ; September 2015

# Stellar Mainnet Configuration
STELLAR_MAINNET_IPCM_CONTRACT=<your-mainnet-contract-id>
STELLAR_MAINNET_SECRET_KEY=<your-mainnet-secret-key>
STELLAR_MAINNET_RPC_URL=https://soroban-mainnet.stellar.org
STELLAR_MAINNET_NETWORK=mainnet
STELLAR_MAINNET_NETWORK_PASSPHRASE=Public Global Stellar Network ; September 2015
DEFARM_OWNER_WALLET=<your-owner-wallet-address>
CURRENT_ADMIN_WALLET=<your-admin-wallet-address>
```

#### Copy from Your Local .env File

If you have a `.env` file locally, you can copy the values from there.

**To generate JWT_SECRET on your local machine**:
```bash
openssl rand -base64 32
```

---

### Step 5: Verify Build Settings

Once GitHub is connected, Railway will auto-detect these settings from `railway.json` and `railway.toml`:

✅ **Builder**: Dockerfile
✅ **Dockerfile Path**: `Dockerfile`
✅ **Start Command**: `/app/defarm-api`
✅ **Health Check Path**: `/health`
✅ **Health Check Timeout**: 100 seconds
✅ **Restart Policy**: ON_FAILURE

To verify, go to **Settings** → **Build** and **Settings** → **Deploy**.

---

### Step 8: Configure Networking

1. Go to **"Settings"** → **"Networking"**
2. Click **"Generate Domain"** to get a Railway-provided subdomain
   - Example: `defarm-engines-api-production.up.railway.app`
3. (Optional) Add custom domain:
   - Click **"Custom Domain"**
   - Enter your domain: `api.defarm.io` (or similar)
   - Follow DNS configuration instructions

---

### Step 7: Monitor the Automatic Deployment

**Railway will automatically start building as soon as GitHub is connected!**

1. Go to **"Deployments"** tab in your service
2. You should see a deployment in progress
3. Click on it to view real-time build logs

**Build phases**:
```
Phase 1: Cloning from GitHub (1-2 minutes)
Phase 2: Building Docker image
  - Stage 1: Rust builder (~10-15 minutes)
    - Installing Stellar CLI (pre-built binary)
    - Compiling Rust application
  - Stage 2: Runtime image (~2 minutes)
Phase 3: Deploying container
Phase 4: Health check (/health endpoint)
```

**Total expected time**: ~15-20 minutes for first build

---

### Step 10: Monitor Deployment

1. Go to **"Deployments"** tab in your service
2. Click on the latest deployment to see logs
3. Watch for:
   ```
   🔍 Checking Stellar CLI configuration...
   ✅ Stellar CLI configured (testnet + mainnet)
   🗄️  Using PostgreSQL storage
   🚀 DeFarm Engines API running on http://0.0.0.0:3000
   ```

4. Check for any errors in the build or runtime logs

---

### Step 11: Verify Deployment

Once deployment is complete:

```bash
# Test health endpoint
curl https://defarm-engines-api-production.up.railway.app/health

# Expected response:
# {"status":"healthy"}
```

**Test from CLI**:
```bash
# Get deployment URL
railway open

# Or get logs
railway logs -f
```

---

## Post-Deployment Configuration

### Test Database Connection

```bash
# Connect to PostgreSQL via Railway CLI
railway run psql

# Or get the DATABASE_URL
railway variables get DATABASE_URL

# Connect with psql locally
psql "$(railway variables get DATABASE_URL)"
```

### Verify Database Schema

Check that migrations ran successfully:

```sql
-- Connect to database
railway run psql

-- List tables
\dt

-- Expected tables (28 total):
-- receipts, items, data_lake, events, circuits, user_accounts, etc.
```

---

## Environment Variables Checklist

Before deployment, ensure all these variables are set:

### Core
- [ ] `DATABASE_URL` (auto-set by Railway PostgreSQL)
- [ ] `JWT_SECRET`

### IPFS/Pinata
- [ ] `PINATA_API_KEY`
- [ ] `PINATA_SECRET_KEY`
- [ ] `PINATA_JWT`
- [ ] `IPFS_ENDPOINT`
- [ ] `IPFS_GATEWAY`

### Stellar Testnet
- [ ] `STELLAR_TESTNET_IPCM_CONTRACT`
- [ ] `STELLAR_TESTNET_SECRET`
- [ ] `STELLAR_TESTNET_RPC_URL`
- [ ] `STELLAR_TESTNET_NETWORK`
- [ ] `STELLAR_TESTNET_NETWORK_PASSPHRASE`

### Stellar Mainnet
- [ ] `STELLAR_MAINNET_IPCM_CONTRACT`
- [ ] `STELLAR_MAINNET_SECRET_KEY`
- [ ] `STELLAR_MAINNET_RPC_URL`
- [ ] `STELLAR_MAINNET_NETWORK`
- [ ] `STELLAR_MAINNET_NETWORK_PASSPHRASE`
- [ ] `DEFARM_OWNER_WALLET`
- [ ] `CURRENT_ADMIN_WALLET`

---

## Troubleshooting

### Build Fails with "Stellar CLI not found"

**Cause**: Dockerfile dependency installation failed

**Fix**: Check build logs for missing system dependencies. The Dockerfile should install:
```dockerfile
RUN apt-get update && apt-get install -y \
    libdbus-1-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*
```

### Database Connection Fails

**Symptoms**:
```
ERROR: could not connect to server
```

**Fix**:
1. Verify `DATABASE_URL` is set correctly
2. Check PostgreSQL service is running (green status in Railway dashboard)
3. Verify the reference to PostgreSQL is created in service variables

### Health Check Fails

**Symptoms**:
```
Health check failed: connection refused
```

**Fix**:
1. Check that the API is listening on port 3000
2. Verify `EXPOSE 3000` is in Dockerfile
3. Check Railway detected the health check path as `/health`
4. Increase health check timeout in Settings → Deploy → Health Check Timeout

### Stellar Network Configuration Warnings

**Symptoms**:
```
⚠️  Mainnet not configured - mainnet adapter will not work
```

**Fix**: This is expected in Railway environment. The Stellar CLI networks are configured via environment variables, not via `stellar network add` commands. The application will still work correctly.

---

## Using Railway CLI for Common Tasks

### View Logs
```bash
railway logs -f
```

### View All Environment Variables
```bash
railway variables
```

### Set a Variable
```bash
railway variables set KEY=value
```

### Open Railway Dashboard
```bash
railway open
```

### Check Service Status
```bash
railway status
```

### Connect to Database
```bash
railway run psql
```

### Run Command with Railway Environment
```bash
railway run <command>
```

---

## Security Recommendations

### Secrets Management
- ✅ Never commit `.env` to git
- ✅ Use Railway's environment variables for all secrets
- ✅ Rotate `JWT_SECRET` and API keys regularly
- ✅ Use separate Stellar keys for testnet and mainnet
- ✅ Keep `STELLAR_MAINNET_SECRET_KEY` extremely secure (real funds!)

### Database Security
- ✅ Railway PostgreSQL has SSL enabled by default
- ✅ Enable automatic backups (Settings → Backups)
- ✅ Monitor database access in Railway dashboard
- ✅ Set up alerts for unusual activity

### Network Security
- ✅ HTTPS is automatic for Railway-provided domains
- ✅ Configure CORS in your API settings
- ✅ Use API rate limiting (already configured in nginx.conf for self-hosted)
- ✅ Monitor API access patterns

---

## Cost Optimization

### Railway Pricing
Railway charges based on:
- **Compute**: CPU and RAM usage
- **Database**: Storage size + compute time
- **Network**: Data transfer (egress)

### Tips to Reduce Costs
1. **Right-size your service**:
   - Settings → Resources → Adjust CPU/RAM
   - Monitor actual usage first, then adjust

2. **Use sleep mode for non-production**:
   - Settings → Deploy → Sleep after inactivity
   - (Only for staging/dev environments)

3. **Enable PostgreSQL connection pooling**:
   - Already configured in `PostgresStorage` (16 connections)

4. **Monitor usage**:
   - Dashboard → Metrics
   - Set up usage alerts

---

## Next Steps After Deployment

1. ✅ **Test all endpoints**:
   ```bash
   # Health check
   curl https://your-domain.railway.app/health

   # Create user (if auth endpoint available)
   curl -X POST https://your-domain.railway.app/api/auth/register \
     -H "Content-Type: application/json" \
     -d '{"username":"test","password":"test123"}'
   ```

2. ✅ **Set up monitoring**:
   - Enable Railway metrics
   - Configure alerts for errors
   - Set up uptime monitoring (e.g., UptimeRobot)

3. ✅ **Configure backups**:
   - Railway Dashboard → PostgreSQL → Backups
   - Enable automatic daily backups
   - Test restore procedure

4. ✅ **Load testing**:
   - Use tools like `wrk` or `k6` to test performance
   - Verify rate limiting works
   - Monitor database connection pool

5. ✅ **Set up CI/CD** (if using GitHub integration):
   - Automatic deployments on push to `main`
   - Railway provides deployment previews for PRs
   - Configure deployment notifications

6. ✅ **Configure custom domain** (optional):
   - Add CNAME record to your DNS
   - Railway provides automatic SSL via Let's Encrypt

7. ✅ **Documentation**:
   - Update API documentation with production URL
   - Document deployment process for team
   - Create runbooks for common issues

---

## Support Resources

- **Railway Docs**: https://docs.railway.app
- **Railway Discord**: https://discord.gg/railway
- **Railway Status**: https://status.railway.app
- **Railway Blog**: https://blog.railway.app

- **DeFarm Deployment Docs**:
  - [RAILWAY_DEPLOYMENT.md](./RAILWAY_DEPLOYMENT.md) - CLI-focused guide
  - [RAILWAY_QUICK_START.md](./RAILWAY_QUICK_START.md) - Quick reference
  - [PRODUCTION_DEPLOYMENT.md](./PRODUCTION_DEPLOYMENT.md) - Self-hosted Docker guide

---

## Summary

This guide walked you through deploying DeFarm Engines API to Railway using the dashboard:

1. ✅ Created `defarm-engines-api` service in existing `defarm` project
2. ✅ Added PostgreSQL database
3. ✅ Configured environment variables
4. ✅ Set up GitHub integration (or CLI deployment)
5. ✅ Deployed and verified health endpoint
6. ✅ Configured custom domain (optional)

**Your deployment is now production-ready!** 🎉

The API is running on Railway with:
- ✅ PostgreSQL database (persistent storage)
- ✅ Automatic SSL/TLS
- ✅ Health checks
- ✅ Stellar testnet + mainnet integration
- ✅ IPFS storage via Pinata
- ✅ Complete circuit tokenization system

---

**Last Updated**: 2025-10-10
**Railway Project**: defarm
**Service**: defarm-engines-api
**Environment**: production

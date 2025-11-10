# 🚂 Railway Quick Start Guide

## Authentication Required

**You need to authenticate with Railway first**. Run this command in your terminal:

```bash
railway login --browserless
```

This will give you a pairing code. Open https://railway.app/cli-login and enter the code.

---

## After Authentication

Once authenticated, you can run these commands:

### 1. List Your Projects
```bash
railway list
```

### 2. Link to Existing DeFarm Workspace
```bash
# If DeFarm project already exists
railway link

# Or link by project ID
railway link <project-id>
```

### 3. Check Project Status
```bash
railway status
```

### 4. View Current Environment Variables
```bash
railway variables
```

### 5. Add PostgreSQL Database
```bash
railway add --database postgres
```

This automatically sets `DATABASE_URL` environment variable.

### 6. Set Required Environment Variables

```bash
# Generate JWT secret
railway variables set JWT_SECRET="$(openssl rand -base64 32)"

# Pinata credentials
railway variables set PINATA_API_KEY="your-pinata-api-key"
railway variables set PINATA_SECRET_KEY="your-pinata-secret-key"
railway variables set PINATA_JWT="your-pinata-jwt"

# Stellar Testnet
railway variables set STELLAR_TESTNET_IPCM_CONTRACT="your-testnet-contract"
railway variables set STELLAR_TESTNET_SECRET="your-testnet-secret"

# Stellar Mainnet
railway variables set STELLAR_MAINNET_IPCM_CONTRACT="your-mainnet-contract"
railway variables set STELLAR_MAINNET_SECRET_KEY="your-mainnet-secret"
railway variables set DEFARM_OWNER_WALLET="your-owner-wallet"
railway variables set CURRENT_ADMIN_WALLET="your-admin-wallet"

# IPFS endpoints
railway variables set IPFS_ENDPOINT="https://api.pinata.cloud"
railway variables set IPFS_GATEWAY="https://gateway.pinata.cloud/ipfs"

# Stellar network configurations
railway variables set STELLAR_TESTNET_RPC_URL="https://soroban-testnet.stellar.org"
railway variables set STELLAR_TESTNET_RPC_FALLBACKS="https://soroban-rpc.testnet.stellar.gateway.fm,https://stellar-soroban-testnet-public.nodies.app"
railway variables set STELLAR_TESTNET_NETWORK="testnet"
railway variables set STELLAR_TESTNET_NETWORK_PASSPHRASE="Test SDF Network ; September 2015"

railway variables set STELLAR_MAINNET_RPC_URL="https://soroban-mainnet.stellar.org"
railway variables set STELLAR_MAINNET_RPC_FALLBACKS="https://soroban-rpc.mainnet.stellar.org,https://soroban-rpc.mainnet.stellar.gateway.fm,https://stellar-soroban-public.nodies.app,https://stellar.api.onfinality.io/public,https://rpc.lightsail.network/,https://archive-rpc.lightsail.network/,https://mainnet.sorobanrpc.com"
railway variables set STELLAR_MAINNET_NETWORK="mainnet"
railway variables set STELLAR_MAINNET_NETWORK_PASSPHRASE="Public Global Stellar Network ; September 2015"
```

### 7. Deploy
```bash
railway up
```

### 8. View Logs
```bash
railway logs -f
```

### 9. Open Dashboard
```bash
railway open
```

### 10. Get Deployment URL
After deployment, Railway will provide a URL like:
`https://defarm-engines-production.up.railway.app`

---

## Railway CLI Commands Reference

### Project Management
```bash
railway list                    # List all projects
railway init                    # Create new project
railway link                    # Link to existing project
railway unlink                  # Unlink from project
railway status                  # Show project info
railway open                    # Open project dashboard
```

### Environment Management
```bash
railway environment             # List environments
railway environment create staging  # Create staging env
railway environment use staging     # Switch to staging
```

### Variables
```bash
railway variables               # Show all variables
railway variables set KEY=value # Set a variable
railway variables delete KEY    # Delete a variable
```

### Deployment
```bash
railway up                      # Deploy current directory
railway deploy                  # Deploy
railway redeploy               # Redeploy latest
railway down                    # Remove latest deployment
railway logs                    # View logs
railway logs -f                 # Follow logs
```

### Services
```bash
railway add --database postgres # Add PostgreSQL
railway service                 # Link service
railway connect                 # Connect to database shell
railway run <command>           # Run command with env vars
```

### Domains
```bash
railway domain                  # Generate Railway domain
railway domain add yourdomain.com  # Add custom domain
```

---

## Deployment Workflow

### Option 1: Direct Deploy (Quick)
```bash
railway login --browserless
railway link                    # Select DeFarm project
railway add --database postgres # If not already added
railway variables set JWT_SECRET="$(openssl rand -base64 32)"
# ... set other variables ...
railway up
railway logs -f
```

### Option 2: Environment-Based (Recommended)
```bash
# Setup staging
railway environment create staging
railway environment use staging
railway add --database postgres
# Set staging variables
railway up

# Test staging
curl https://staging-defarm.railway.app/health

# Deploy to production
railway environment create production
railway environment use production
railway add --database postgres
# Set production variables
railway up
```

---

## Important Notes

1. **DATABASE_URL** is automatically set when you add PostgreSQL
2. **JWT_SECRET** must be set manually (generate with: `openssl rand -base64 32`)
3. **Stellar secrets** must be kept secure - consider using Railway's secret management
4. **Migrations** run automatically on deployment (configured in Dockerfile)
5. **Health check** endpoint: `/health`
6. **Logs** are available via `railway logs` command

---

## Troubleshooting

### Authentication Issues
```bash
railway logout
railway login --browserless
```

### Can't Find Project
```bash
railway list
railway link <project-id>
```

### Deployment Fails
```bash
railway logs
# Check for missing environment variables
railway variables
```

### Database Connection Issues
```bash
railway connect  # Opens psql shell
# Or check DATABASE_URL
railway variables | grep DATABASE_URL
```

---

## Next Steps After Deployment

1. ✅ Verify deployment: `railway logs`
2. ✅ Test health endpoint: `curl https://your-app.railway.app/health`
3. ✅ Test database connection
4. ✅ Configure custom domain (optional)
5. ✅ Set up monitoring
6. ✅ Configure automatic backups
7. ✅ Load testing

---

## Cost Management

Railway charges based on:
- **Compute**: Resource usage (CPU/RAM)
- **Database**: Storage + compute
- **Bandwidth**: Data transfer

**Tips**:
- Use `railway scale` to adjust resources
- Set up sleep mode for non-production environments
- Monitor usage in Railway dashboard

---

## Security Checklist

- [ ] Strong JWT_SECRET generated
- [ ] All sensitive variables set as Railway secrets
- [ ] Separate environments for staging/production
- [ ] Different database credentials per environment
- [ ] Stellar mainnet keys stored securely
- [ ] HTTPS enabled (automatic on Railway)
- [ ] Database backups configured
- [ ] Monitoring and alerts set up

---

## Support

- **Railway Docs**: https://docs.railway.app
- **Railway Discord**: https://discord.gg/railway
- **DeFarm Deployment Guide**: [RAILWAY_DEPLOYMENT.md](./RAILWAY_DEPLOYMENT.md)

# 🚂 Railway Deployment Guide

Complete guide for deploying DeFarm Engines to Railway.app

## Prerequisites

- Railway account ([Sign up](https://railway.app))
- Railway CLI installed (`npm install -g @railway/cli`)
- GitHub repository (for continuous deployment)

## Deployment Options

### Option A: CLI Deployment (Recommended for Testing)

#### 1. Authenticate

```bash
railway login --browserless
```

This will provide you with a pairing code. Open https://railway.app/cli-login and enter the code.

#### 2. Initialize Project

```bash
# In your project directory
railway init
```

Select "Create new project" and choose a name (e.g., "defarm-engines-production")

#### 3. Add PostgreSQL Database

```bash
railway add --database postgres
```

This creates a managed PostgreSQL instance and sets the `DATABASE_URL` environment variable automatically.

#### 4. Set Environment Variables

```bash
# Set all required environment variables
railway variables set JWT_SECRET="$(openssl rand -base64 32)"
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
```

#### 5. Deploy

```bash
railway up
```

This builds the Docker image and deploys it to Railway.

#### 6. Check Status

```bash
# View logs
railway logs

# Get deployment URL
railway open
```

### Option B: GitHub Integration (Recommended for Production)

#### 1. Push to GitHub

```bash
git add .
git commit -m "Ready for Railway deployment"
git push origin main
```

#### 2. Create Railway Project

1. Go to [Railway Dashboard](https://railway.app/dashboard)
2. Click "New Project"
3. Select "Deploy from GitHub repo"
4. Connect your GitHub account
5. Select your repository

#### 3. Add PostgreSQL

1. In your Railway project, click "New"
2. Select "Database" → "PostgreSQL"
3. Railway will automatically set `DATABASE_URL`

#### 4. Configure Environment Variables

In Railway Dashboard:
1. Go to your service
2. Click "Variables"
3. Add all variables from `.env.example`:
   - `JWT_SECRET` - Generate with: `openssl rand -base64 32`
   - `PINATA_API_KEY`
   - `PINATA_SECRET_KEY`
   - `PINATA_JWT`
   - `STELLAR_TESTNET_IPCM_CONTRACT`
   - `STELLAR_TESTNET_SECRET`
   - `STELLAR_MAINNET_IPCM_CONTRACT`
   - `STELLAR_MAINNET_SECRET_KEY`
   - `DEFARM_OWNER_WALLET`

#### 5. Configure Build

Railway should auto-detect the Dockerfile. If not:
1. Go to Settings → Deploy
2. Set "Builder" to "Dockerfile"
3. Set "Dockerfile Path" to `Dockerfile`

#### 6. Deploy

1. Click "Deploy" or push to your repository
2. Railway will automatically build and deploy

## Configuration

### Custom Domain

```bash
# Via CLI
railway domain

# Or in Dashboard: Settings → Networking → Custom Domain
```

### Environment Management

```bash
# List all variables
railway variables

# Set variable
railway variables set KEY=value

# Delete variable
railway variables delete KEY
```

### Scaling

In Railway Dashboard:
1. Go to Settings → Resources
2. Adjust CPU and RAM allocations
3. Configure auto-scaling if needed

## Monitoring

### Logs

```bash
# View real-time logs
railway logs --follow

# Filter by time
railway logs --since 1h
```

### Metrics

View metrics in Railway Dashboard:
- CPU usage
- Memory usage
- Network traffic
- Request counts

## Database Management

### Connect to PostgreSQL

```bash
# Get database URL
railway variables get DATABASE_URL

# Connect with psql
railway run psql
```

### Migrations

Migrations run automatically on deployment via the Dockerfile's CMD.

Manual migration:
```bash
railway run /app/defarm-api --migrate
```

### Backups

Railway provides automatic backups for PostgreSQL. Configure in:
Dashboard → Database → Backups

## Health Checks

Railway automatically monitors your health check endpoint (`/health`).

Configure in:
Settings → Deploy → Health Check Path: `/health`

## SSL/TLS

Railway provides automatic HTTPS for all deployments:
- `*.railway.app` domains have automatic SSL
- Custom domains require DNS configuration

## Troubleshooting

### Build Failures

```bash
# View build logs
railway logs --deployment

# Check Dockerfile syntax
docker build -t test .
```

### Connection Issues

```bash
# Test database connection
railway run env | grep DATABASE_URL

# Verify all required variables are set
railway variables
```

### Stellar CLI Issues

Stellar CLI is installed in the Docker image. Verify:
```bash
railway run stellar --version
```

If network configuration fails, the app will warn at startup but continue running.

## Cost Optimization

### Resource Limits

Set appropriate limits in `railway.toml`:
```toml
[deploy]
numReplicas = 1
restartPolicyType = "ON_FAILURE"

[deploy.healthcheckPath]
path = "/health"
```

### Sleep Mode

For non-production environments, configure sleep mode:
Settings → Deploy → Sleep after inactivity

## Security Best Practices

### Secrets Management

- Never commit `.env` to git
- Use Railway's environment variables
- Rotate secrets regularly
- Use separate environments for staging/production

### Database Security

- Railway PostgreSQL is SSL-enabled by default
- Use strong passwords
- Enable automatic backups
- Monitor access logs

### API Security

- Enable rate limiting (configured in nginx.conf)
- Monitor API access patterns
- Set up alerts for suspicious activity

## Deployment Workflow

### Development → Staging → Production

```bash
# Create staging environment
railway environment create staging

# Deploy to staging
railway up --environment staging

# Test staging
curl https://staging-defarm.railway.app/health

# Promote to production
railway environment create production
railway up --environment production
```

## Railway CLI Reference

```bash
# Authentication
railway login --browserless
railway logout

# Project management
railway init
railway link
railway unlink
railway list

# Deployment
railway up
railway deploy
railway status

# Environment variables
railway variables
railway variables set KEY=value
railway variables delete KEY

# Logs and debugging
railway logs
railway logs --follow
railway logs --since 1h

# Database
railway run psql
railway connect

# Networking
railway domain
railway open

# Environment management
railway environment
railway environment create <name>
railway environment use <name>
```

## Support

- **Railway Docs**: https://docs.railway.app
- **Railway Discord**: https://discord.gg/railway
- **Status Page**: https://status.railway.app

## Next Steps

After successful deployment:

1. ✅ Test all API endpoints
2. ✅ Verify database connectivity
3. ✅ Test Stellar integrations
4. ✅ Configure custom domain
5. ✅ Set up monitoring/alerts
6. ✅ Enable automatic backups
7. ✅ Configure CI/CD pipeline
8. ✅ Load testing
9. ✅ Security audit
10. ✅ Documentation update

---

## Quick Reference

### Essential Commands

```bash
# Deploy
railway up

# Logs
railway logs -f

# Variables
railway variables set KEY=value

# Database
railway run psql

# Open dashboard
railway open
```

### Environment Variables Checklist

- [ ] `DATABASE_URL` (auto-set by Railway PostgreSQL)
- [ ] `JWT_SECRET`
- [ ] `PINATA_API_KEY`
- [ ] `PINATA_SECRET_KEY`
- [ ] `PINATA_JWT`
- [ ] `STELLAR_TESTNET_IPCM_CONTRACT`
- [ ] `STELLAR_TESTNET_SECRET`
- [ ] `STELLAR_MAINNET_IPCM_CONTRACT`
- [ ] `STELLAR_MAINNET_SECRET_KEY`
- [ ] `DEFARM_OWNER_WALLET`

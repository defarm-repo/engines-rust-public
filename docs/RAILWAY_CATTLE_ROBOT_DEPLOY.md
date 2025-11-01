# Deploy Cattle Robot to Railway

## Quick Deploy Steps

### Option 1: Railway Dashboard (Recommended)

1. **Go to Railway Dashboard**
   - Visit https://railway.app/dashboard
   - Select your project (same project as defarm-engines-api)

2. **Create New Service**
   - Click "+ New Service"
   - Select "GitHub Repo"
   - Choose your `engines` repository
   - Name it: `cattle-robot`

3. **Configure Service**
   - Go to service Settings
   - Under "Build", set:
     - **Build Command**: `cargo build --release --bin cattle-robot`
     - **Start Command**: `./target/release/cattle-robot`
   - Under "Deploy", set:
     - **Restart Policy**: `ON_FAILURE`
     - **Max Retries**: `10`

4. **Set Environment Variables**

   Go to "Variables" tab and add:

   ```bash
   # Required - Copy from your main API service
   DATABASE_URL=${{Postgres.DATABASE_URL}}  # Reference your existing PostgreSQL

   # Required - Create API key first (see below)
   ROBOT_API_KEY=dfm_your_robot_key_here

   # Optional (has good defaults)
   RAILWAY_API_URL=https://defarm-engines-api-production.up.railway.app
   ROBOT_MODE=production
   ROBOT_SCHEDULE=weekday-heavy
   RUST_LOG=info
   ```

5. **Deploy**
   - Railway will automatically deploy
   - Watch logs for successful startup

### Option 2: Railway CLI

```bash
# Login to Railway
railway login

# Link to your project
railway link

# Create new service
railway service create cattle-robot

# Set environment variables
railway variables set DATABASE_URL="${DATABASE_URL}"
railway variables set ROBOT_API_KEY="dfm_your_key_here"
railway variables set RAILWAY_API_URL="https://defarm-engines-api-production.up.railway.app"
railway variables set ROBOT_MODE="production"
railway variables set ROBOT_SCHEDULE="weekday-heavy"

# Deploy
railway up --service cattle-robot
```

## Step-by-Step with Screenshots

### 1. Create API Key for Robot

First, create a dedicated API key for the robot:

```bash
# Get admin JWT token
TOKEN=$(curl -s -X POST "https://defarm-engines-api-production.up.railway.app/api/auth/login" \
  -H "Content-Type: application/json" \
  -d '{"username": "hen", "password": "demo123"}' | jq -r '.token')

# Create robot API key
curl -X POST "https://defarm-engines-api-production.up.railway.app/api/api-keys" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Cattle Robot Service",
    "permissions": ["read", "write"],
    "expires_at": null
  }'
```

Save the API key from the response (starts with `dfm_`).

### 2. Create Service in Railway

**Dashboard → Your Project → + New Service → GitHub Repo**

Settings to configure:
- **Name**: `cattle-robot`
- **Source**: Same GitHub repo as your main API
- **Root Directory**: `/` (same as main project)

### 3. Configure Build

**Settings → Build**

```
Build Command: cargo build --release --bin cattle-robot
Start Command: ./target/release/cattle-robot
```

### 4. Set Environment Variables

**Variables Tab → New Variable**

| Variable | Value | Note |
|----------|-------|------|
| `DATABASE_URL` | Reference Postgres service | Click "Reference" → Select Postgres → `DATABASE_URL` |
| `ROBOT_API_KEY` | `dfm_xxx` | API key from step 1 |
| `RAILWAY_API_URL` | `https://defarm-engines-api-production.up.railway.app` | Your API URL |
| `ROBOT_MODE` | `production` | Or `dry-run` for testing |
| `ROBOT_SCHEDULE` | `weekday-heavy` | Or `uniform` |
| `RUST_LOG` | `info` | Or `debug` for verbose |

**Important**: Use the same PostgreSQL instance as your main API. The robot tables are separate and won't conflict.

### 5. Deploy & Monitor

**Deployments Tab → Trigger Deploy**

Watch the logs:
```
🤖 Cattle Robot Starting...
✓ Database connected
✓ API is healthy
Creating new robot circuit...
✓ Circuit created: 002ea6db-6b7b-4a69-8780-1f01ae074265
✓ Adapter configured
🚀 Robot is now running
```

## Verify Deployment

### Check Logs

In Railway dashboard:
- Go to your `cattle-robot` service
- Click "View Logs"
- Look for successful minting operations

Expected logs:
```
[INFO] 🤖 Cattle Robot Starting...
[INFO] ✓ Database connected
[INFO] ✓ API is healthy
[INFO] 🚀 Robot is now running
[INFO] ⏰ 2025-01-20 15:42:13 UTC - Weekday Business hours (Mon)
[INFO] 🎲 Selected operation: NewMint
[INFO] ✅ MINT SUCCESS: SISBOV=BR547891234567, DFID=DFID-20250120-000001-A7B3
[INFO] 📊 Stats: Mints=1, Updates=0, Errors=0, Uptime=12s
```

### Check Database

Connect to your Railway PostgreSQL:

```sql
-- See robot cattle
SELECT COUNT(*) as total_cattle FROM robot_cattle;

-- See latest mints
SELECT c.sisbov, c.breed, c.state, m.dfid, m.created_at
FROM robot_cattle c
JOIN robot_mints m ON m.cattle_id = c.id
ORDER BY m.created_at DESC
LIMIT 5;

-- See events
SELECT c.sisbov, e.event_type, e.event_date, e.dfid
FROM robot_events e
JOIN robot_cattle c ON e.cattle_id = c.id
ORDER BY e.created_at DESC
LIMIT 5;
```

### Check API

```bash
# List circuits (should see robot circuit)
curl "https://defarm-engines-api-production.up.railway.app/api/circuits" \
  -H "X-API-Key: dfm_your_robot_key"

# Check specific cattle DFID
curl "https://defarm-engines-api-production.up.railway.app/api/items/DFID-20250120-000001-A7B3" \
  -H "X-API-Key: dfm_your_robot_key"
```

## Troubleshooting

### Issue: Build Fails

**Error**: `cargo: command not found`

**Solution**: Railway should automatically detect Rust. If not, add to Variables:
```
NIXPACKS_BUILD_CMD=cargo build --release --bin cattle-robot
```

### Issue: Database Connection Failed

**Error**: `connection refused` or `invalid connection string`

**Solution**:
1. Ensure PostgreSQL service is running
2. Check `DATABASE_URL` references the correct Postgres service
3. Verify robot service is in same Railway project

### Issue: API Key Invalid

**Error**: `Authentication failed: 401 Unauthorized`

**Solution**:
1. Verify API key format (must start with `dfm_`)
2. Check key has `write` permissions
3. Ensure key hasn't expired
4. Create new key if needed

### Issue: Migration Not Run

**Error**: `relation "robot_cattle" does not exist`

**Solution**: Run migration manually:

```bash
# Connect to Railway Postgres
railway connect Postgres

# In psql, check if tables exist
\dt robot_*

# If not, run migration from local:
export DATABASE_URL="your_railway_postgres_url"
cargo sqlx migrate run
```

Or add migration to build command:
```
cargo sqlx migrate run && cargo build --release --bin cattle-robot
```

### Issue: Robot Keeps Restarting

**Error**: Service restarts frequently

**Solution**: Check logs for errors. Common causes:
- Wrong `DATABASE_URL`
- Invalid `ROBOT_API_KEY`
- API unreachable
- Missing migration

### Issue: No Operations Happening

**Symptom**: Robot running but no mints in database

**Check**:
1. Verify `ROBOT_MODE=production` (not `dry-run`)
2. Check logs for errors
3. Verify API key has write permissions
4. Check circuit was created successfully

## Monitoring

### Railway Metrics

Railway Dashboard shows:
- **CPU Usage**: Should be low (1-2%)
- **Memory**: ~50-100MB
- **Network**: Intermittent spikes during operations
- **Restarts**: Should be 0 (if restarting, check logs)

### Custom Monitoring

Add logging to track operations:

```bash
# In Railway logs, search for:
- "MINT SUCCESS" - Count successful mints
- "UPDATE SUCCESS" - Count successful updates
- "ERROR" - Find issues
```

### Database Monitoring

Query for statistics:

```sql
-- Hourly mint rate
SELECT
  DATE_TRUNC('hour', created_at) as hour,
  COUNT(*) as mints
FROM robot_mints
GROUP BY hour
ORDER BY hour DESC
LIMIT 24;

-- Operations by state
SELECT state, COUNT(*) as cattle_count
FROM robot_cattle
GROUP BY state
ORDER BY cattle_count DESC;

-- Recent activity
SELECT
  DATE_TRUNC('day', created_at) as day,
  COUNT(*) as operations
FROM robot_events
GROUP BY day
ORDER BY day DESC
LIMIT 7;
```

## Scaling

### Single Instance (Current)
- Handles 50-150 operations/day
- Sufficient for most use cases
- Low resource usage

### Multiple Instances (Future)
- Create multiple robot services
- Each with different circuit
- Different schedules (e.g., one for weekdays, one for weekends)
- Distribute across regions/value chains

## Maintenance

### Stop Robot

In Railway Dashboard:
- Go to service settings
- Click "Stop Service"

Or set `ROBOT_MODE=dry-run` to test without minting.

### Restart Robot

In Railway Dashboard:
- Go to service deployments
- Click "Restart"

Robot will resume with fresh state.

### Update Robot

1. Push changes to GitHub
2. Railway auto-deploys on push
3. Robot restarts automatically

Or manually trigger:
- Go to Deployments
- Click "Deploy"

### Clear Data

To reset and start fresh:

```sql
-- Clear robot data (WARNING: Deletes all robot cattle)
TRUNCATE robot_mints CASCADE;
TRUNCATE robot_events CASCADE;
TRUNCATE robot_cattle CASCADE;
```

Then restart robot service.

## Cost Estimation

Railway charges based on usage:

**Cattle Robot Service**:
- **Compute**: ~$1-2/month (very low CPU)
- **Database**: Shared with main API (minimal extra cost)
- **Network**: Minimal (mostly outbound API calls)

**Total**: ~$1-2/month additional cost

**Stellar Testnet**: Free (no charges)
**IPFS (Pinata)**: Uses existing account

## Production Checklist

Before running in production:

- [x] PostgreSQL migration run (V7 tables created)
- [x] Robot API key created with write permissions
- [x] Environment variables configured in Railway
- [x] Service deployed and logs show successful startup
- [x] Circuit created and adapter configured
- [x] First mint successful and visible in database
- [x] Logs show expected timing patterns
- [x] No errors in logs
- [x] Database queries return robot data

## Support

If issues persist:

1. **Check Railway Status**: https://railway.app/status
2. **Review Logs**: Railway Dashboard → cattle-robot → View Logs
3. **Check Database**: Verify migration ran successfully
4. **Verify API**: Test API endpoint manually
5. **Check Documentation**: `docs/CATTLE_ROBOT.md`

## Next Steps After Deployment

1. **Monitor First Hour**: Watch logs for 5-10 operations
2. **Verify Data**: Check database for cattle/events/mints
3. **Check Blockchain**: View transactions on Stellar testnet
4. **Set Alerts**: Configure Railway notifications for errors
5. **Document Circuit ID**: Save robot circuit ID for reference

---

**Status**: Ready for deployment to Railway
**Estimated Deploy Time**: 5-10 minutes
**Estimated Build Time**: 3-5 minutes (Rust compilation)

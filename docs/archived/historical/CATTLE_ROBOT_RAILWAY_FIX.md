# 🔧 Cattle Robot Railway Deployment Fix

## Problem
The cattle-robot service was running `defarm-api` instead of the `cattle-robot` binary because it was using the wrong Dockerfile.

## Solution
I've created a dedicated `Dockerfile.cattle-robot` that builds only the cattle-robot binary.

## How to Fix in Railway Dashboard

### Option 1: Via Railway Dashboard (Recommended)

1. **Go to Railway Dashboard**: https://railway.app/dashboard
2. **Select your project**: defarm
3. **Click on the cattle-robot service**
4. **Go to Settings tab**
5. **Scroll to Build section**:
   - **Builder**: Select "Dockerfile"
   - **Dockerfile Path**: Enter `Dockerfile.cattle-robot`
6. **Scroll to Deploy section**:
   - **Start Command**: Enter `/app/cattle-robot`
   - **Restart Policy**: ON_FAILURE (should already be set)
   - **Max Retries**: 10
7. **Click "Deploy" button** (top right)

### Option 2: Via Railway CLI

```bash
# Link to cattle-robot service
railway service cattle-robot

# Update the configuration
railway up --dockerfile Dockerfile.cattle-robot

# Or set via environment variable
railway variables --set RAILWAY_DOCKERFILE_PATH=Dockerfile.cattle-robot
```

## Verification

After deployment completes (5-10 minutes), check the logs:

```bash
railway logs --service cattle-robot
```

**Expected output:**
```
🤖 Cattle Robot Starting...
✓ Database connected
✓ API is healthy
Circuit ID: <some-uuid>
🚀 Robot is now running
```

**Wrong output (what you saw before):**
```
[INFO] defarm_api: 🌟 Stellar SDK integration enabled
[INFO] defarm_api: 🗄️  Initializing PostgreSQL...
```

## Stellar Testnet Address

Once the robot is running, cattle NFTs will be minted to:

**Address**: `GDGKI43T7QJYOIPNFDH64M2LHICXTQO5NOKI523VZJSROKR34AEB5CKE`

**View on Stellar Explorer**:
https://stellar.expert/explorer/testnet/account/GDGKI43T7QJYOIPNFDH64M2LHICXTQO5NOKI523VZJSROKR34AEB5CKE

**Current Balance**: ~9,999 XLM (testnet)

## Files Created

- ✅ `Dockerfile.cattle-robot` - Dedicated Dockerfile for cattle-robot binary
- ✅ `railway.cattle-robot.json` - Railway configuration (for reference)
- ✅ Already committed and pushed to GitHub

## Next Steps

1. Update Railway dashboard settings (see above)
2. Wait for deployment (5-10 minutes)
3. Check logs to confirm "🤖 Cattle Robot Starting..."
4. Monitor first mint operation (will happen within 5-120 minutes based on schedule)
5. Check Stellar Explorer for NFT transactions

## Expected Behavior

**Operating Schedule**:
- Weekdays 9am-5pm BRT: ~4 operations/hour
- Weekdays off-hours: ~2 operations/hour
- Weekends: ~1 operation/hour
- Random jitter: ±20% variance

**Operations Mix**:
- 70% new cattle mints
- 30% updates to existing cattle

**Data Privacy**:
- All personal data hashed with BLAKE3
- Format: `hash:owner:blake3(...)`, `hash:vet:blake3(...)`, etc.

## Troubleshooting

**If logs still show defarm-api:**
1. Verify Dockerfile Path is set to `Dockerfile.cattle-robot` in Railway dashboard
2. Click "Redeploy" to force new build
3. Check that GitHub push succeeded

**If build fails:**
1. Check Railway build logs for errors
2. Verify all environment variables are set (see cattle-robot service variables)
3. Ensure DATABASE_URL, ROBOT_API_KEY, JWT_SECRET are present

**If robot starts but crashes:**
1. Check that V7 migration ran (robot_cattle, robot_events, robot_mints tables)
2. Verify RAILWAY_API_URL points to main API service
3. Check that ROBOT_API_KEY has read/write permissions

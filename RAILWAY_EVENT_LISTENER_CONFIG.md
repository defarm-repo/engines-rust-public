# Railway Event Listener Configuration Guide

## IPCM Event Listener Service - Dual Network Setup

This document provides the complete environment variable configuration for deploying the IPCM event listener service on Railway with dual network support (testnet + mainnet).

### Prerequisites

1. **PostgreSQL Database**: The event listener shares the same PostgreSQL database with the `defarm-engines-api` service
2. **Railway CLI**: Install with `npm install -g @railway/cli` or use Railway web dashboard
3. **GitHub Integration**: Code pushed to `main` branch auto-deploys

### Service Name
`ipcm-event-listener`

---

## Environment Variables Configuration

Copy these variables into the Railway dashboard for the `ipcm-event-listener` service:

### 1. Database Configuration (Shared with API)

```bash
DATABASE_URL=postgresql://postgres:OWlsOeBLeDQnHSfpOQDFcMQXQeipFtom@postgres.railway.internal:5432/railway
```

**Note**: This is the same PostgreSQL instance used by `defarm-engines-api`. Both networks write to the same `item_cid_timeline` table.

---

### 2. Testnet Configuration (Default: Enabled)

```bash
ENABLE_TESTNET_LISTENER=true
STELLAR_TESTNET_IPCM_CONTRACT=CCDJV6VAFC2MSSDSL4AEJB5BAMGDA5PMCUIZ3UF6AYIJL467PQTBZ7BS
STELLAR_TESTNET_RPC_URL=https://soroban-testnet.stellar.org
STELLAR_TESTNET_RPC_FALLBACKS=https://soroban-rpc.testnet.stellar.gateway.fm,https://stellar-soroban-testnet-public.nodies.app
TESTNET_POLL_INTERVAL=10
TESTNET_BATCH_SIZE=100
```

**Details**:
- `ENABLE_TESTNET_LISTENER`: Set to `true` to monitor testnet blockchain
- `STELLAR_TESTNET_IPCM_CONTRACT`: IPCM v2.1.0 contract address on testnet (with event emission)
- `STELLAR_TESTNET_RPC_URL`: Primary Soroban RPC endpoint for testnet
- `STELLAR_TESTNET_RPC_FALLBACKS`: Optional comma/space separated list of backup RPC hosts.
  Defaults use the providers recommended in [Stellar’s public RPC catalog](https://developers.stellar.org/docs/data/apis/rpc/providers).
- `TESTNET_POLL_INTERVAL`: Seconds between blockchain queries (default: 10)
- `TESTNET_BATCH_SIZE`: Number of ledgers to fetch per query (default: 100)

---

### 3. Mainnet Configuration (Default: Disabled)

```bash
ENABLE_MAINNET_LISTENER=false
STELLAR_MAINNET_IPCM_CONTRACT=CBHYQKSG2ZADD7NXZPLFZIH7ZK766VA3YWRLISKJ6PH6KXJ4JZ52OLNZ
STELLAR_MAINNET_RPC_URL=https://soroban-mainnet.stellar.org
STELLAR_MAINNET_RPC_FALLBACKS=https://soroban-rpc.mainnet.stellar.org,https://soroban-rpc.mainnet.stellar.gateway.fm,https://stellar-soroban-public.nodies.app,https://stellar.api.onfinality.io/public,https://rpc.lightsail.network/,https://archive-rpc.lightsail.network/,https://mainnet.sorobanrpc.com
MAINNET_POLL_INTERVAL=10
MAINNET_BATCH_SIZE=100
```

**Details**:
- `ENABLE_MAINNET_LISTENER`: Set to `false` initially (enable when ready for production)
- `STELLAR_MAINNET_IPCM_CONTRACT`: IPCM v2.1.0 contract address on mainnet (with event emission)
- `STELLAR_MAINNET_RPC_URL`: Primary Soroban RPC endpoint for mainnet
- `STELLAR_MAINNET_RPC_FALLBACKS`: Optional comma/space separated list of backup RPC hosts (defaults align with Stellar’s provider list linked above)
- `MAINNET_POLL_INTERVAL`: Seconds between blockchain queries (default: 10)
- `MAINNET_BATCH_SIZE`: Number of ledgers to fetch per query (default: 100)

---

## Quick Setup via Railway Web Dashboard

1. **Navigate to Railway Dashboard**:
   - Go to https://railway.app
   - Select project: `defarm`
   - Select environment: `production`

2. **Create/Select Service**:
   - If service doesn't exist, create new service: `ipcm-event-listener`
   - Link to GitHub repository: `gabrielrondon/defarm-rust-engine`
   - Set build command: `cargo build --release --bin ipcm-event-listener`
   - Set start command: `./target/release/ipcm-event-listener`

3. **Set Environment Variables**:
   - Click on the `ipcm-event-listener` service
   - Go to "Variables" tab
   - Click "New Variable" and add each variable from sections above

4. **Deploy**:
   - Service will auto-deploy after variables are set
   - Check logs for successful startup

---

## Deployment Options

### Option 1: Manual Variable Configuration (Recommended for Initial Setup)

Use the Railway web dashboard to manually set all variables listed above.

### Option 2: Railway CLI (Advanced)

```bash
# Set each variable individually
railway service --name ipcm-event-listener
railway variables --kv DATABASE_URL="postgresql://postgres:..."
railway variables --kv ENABLE_TESTNET_LISTENER="true"
railway variables --kv STELLAR_TESTNET_IPCM_CONTRACT="CCDJV6VAFC2MSSDSL4AEJB5BAMGDA5PMCUIZ3UF6AYIJL467PQTBZ7BS"
# ... etc
```

---

## Verifying Deployment

### 1. Check Service Status

```bash
railway status --service ipcm-event-listener
```

### 2. View Logs

```bash
railway logs --service ipcm-event-listener
```

**Expected Output**:
```
🚀 Starting IPCM Event Listener Daemon (Dual Network Support)
🗄️  Connecting to PostgreSQL...
✅ PostgreSQL connected
📋 Network Configuration:
   Testnet Listener: ✅ ENABLED
   Mainnet Listener: ❌ DISABLED
🌐 Testnet Configuration:
   IPCM Contract: CCDJV6VAFC2MSSDSL4AEJB5BAMGDA5PMCUIZ3UF6AYIJL467PQTBZ7BS
   Soroban RPC endpoints: https://soroban-testnet.stellar.org, https://soroban-rpc.testnet.stellar.gateway.fm, https://stellar-soroban-testnet-public.nodies.app
   Poll Interval: 10s
   Batch Size: 100 ledgers
🎧 Starting testnet event listener...
```

### 3. Verify Timeline Population

```bash
# After pushing an item with Stellar adapter, check PostgreSQL
railway run --service defarm-engines-api -- psql $DATABASE_URL -c "SELECT * FROM item_cid_timeline ORDER BY created_at DESC LIMIT 5;"
```

---

## Enabling Mainnet Listener

When ready to monitor mainnet blockchain:

1. Go to Railway dashboard → `ipcm-event-listener` service → Variables
2. Change `ENABLE_MAINNET_LISTENER` from `false` to `true`
3. Service will auto-restart and begin monitoring both networks

**Expected Logs After Enabling Mainnet**:
```
📋 Network Configuration:
   Testnet Listener: ✅ ENABLED
   Mainnet Listener: ✅ ENABLED
🎧 Starting testnet event listener...
🎧 Starting mainnet event listener...
```

---

## Architecture Notes

### Dual Network Support
- Single service monitors both testnet and mainnet simultaneously
- Each network runs in its own Tokio task (concurrent monitoring)
- Shared PostgreSQL database for all timeline entries
- Timeline entries include `network` field to distinguish source

### Cost Optimization
- IPCM v2.1.0 contracts emit events (CPU cost only, no storage fees)
- Events are queryable via Soroban RPC for 7 days
- Event listener indexes events into PostgreSQL for permanent storage
- Off-chain timeline eliminates need for expensive on-chain storage queries

### Fail-Safe Design
- If one network fails, the other continues operating
- PostgreSQL connection is shared and managed by connection pool
- Service restarts automatically if both tasks fail
- Rate limiting respects Soroban RPC limits

---

## Troubleshooting

### Service Won't Start

**Check**:
```bash
railway logs --service ipcm-event-listener | grep -i error
```

**Common Issues**:
- Missing `DATABASE_URL` → Verify variable is set
- PostgreSQL connection failed → Check postgres.railway.internal is accessible
- Binary not found → Verify build command compiled `ipcm-event-listener` binary

### No Events Being Indexed

**Check**:
1. Verify items are being pushed with Stellar adapter
2. Confirm contract addresses are v2.1.0 (with event emission)
3. Check Soroban RPC is responding:
   ```bash
   curl -X POST https://soroban-testnet.stellar.org \
     -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'
   ```

### Both Networks Showing Disabled

**Check**:
- Verify `ENABLE_TESTNET_LISTENER=true` and/or `ENABLE_MAINNET_LISTENER=true`
- Variable types should be strings `"true"` not booleans
- Restart service after changing variables

---

## Contract Addresses Reference

### Testnet
- **IPCM v2.1.0**: `CCDJV6VAFC2MSSDSL4AEJB5BAMGDA5PMCUIZ3UF6AYIJL467PQTBZ7BS`
- **Explorer**: https://stellar.expert/explorer/testnet/contract/CCDJV6VAFC2MSSDSL4AEJB5BAMGDA5PMCUIZ3UF6AYIJL467PQTBZ7BS

### Mainnet
- **IPCM v2.1.0**: `CBHYQKSG2ZADD7NXZPLFZIH7ZK766VA3YWRLISKJ6PH6KXJ4JZ52OLNZ`
- **Explorer**: https://stellar.expert/explorer/public/contract/CBHYQKSG2ZADD7NXZPLFZIH7ZK766VA3YWRLISKJ6PH6KXJ4JZ52OLNZ

---

## Related Documentation

- **IPCM V2 Upgrade Summary**: See `IPCM_V2_UPGRADE_SUMMARY.md`
- **Event Listener Source**: See `src/bin/ipcm_event_listener.rs`
- **Event Listener Implementation**: See `src/blockchain_event_listener.rs`
- **Railway Deployment**: See `RAILWAY_DEPLOYMENT.md`

---

**Last Updated**: 2025-10-19
**Contract Version**: v2.1.0
**Service Version**: Dual Network Support

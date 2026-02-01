# DFID Service - Quick Start Guide

## TL;DR

DFID generation is now **optional external service**. Works with or without it.

## Enable DFID Service (Optional)

```bash
# Add to .env
DFID_SERVICE_URL=http://localhost:3001
```

That's it! API automatically uses remote DFID generation.

## Local Development

### Option 1: Docker Compose (Recommended)

```bash
docker-compose up -d
# All services start: Redis, PostgreSQL, DFID Service, API
```

### Option 2: Manual

```bash
# Terminal 1: DFID Service
cd dfid-service && cargo run

# Terminal 2: API
export DFID_SERVICE_URL=http://localhost:3001
cargo run --bin defarm-api
```

## Test It

```bash
# Generate DFID
curl -X POST http://localhost:3001/dfid/generate \
  -H "Content-Type: application/json" \
  -d '{"count": 1}'

# Response:
# {"dfids":["DFID-20250131-000001-A7B2C3"],...}
```

## Disable DFID Service

```bash
# Remove from .env
# DFID_SERVICE_URL=http://localhost:3001

# Restart API
cargo run --bin defarm-api
# Automatically falls back to local generation
```

## How It Works

```rust
// Hybrid mode - automatic fallback
pub struct ItemsEngine {
    dfid_engine: DfidEngine,        // Local (always present)
    dfid_client: Option<DfidClient>, // Remote (optional)
}

// Uses remote if available, local otherwise
async fn generate_dfid_internal() -> Result<String> {
    if let Some(ref client) = self.dfid_client {
        client.generate_dfid(None).await  // Remote
    } else {
        Ok(self.dfid_engine.generate_dfid())  // Local
    }
}
```

## DFID Format

### Old (16-bit checksum)
```
DFID-20250131-000001-A7B2
         │         │      │
         │         │      └─ 4 hex chars
         │         └──────── 6-digit sequence
         └────────────────── YYYYMMDD
```

### New (24-bit BLAKE3)
```
DFID-20250131-000001-A7B2C3
         │         │      │
         │         │      └─ 6 hex chars (BLAKE3)
         │         └──────── 6-digit sequence
         └────────────────── YYYYMMDD
```

Both formats are valid!

## Troubleshooting

### DFID Service not reachable

```bash
# Check service is running
curl http://localhost:3001/health

# Check environment
echo $DFID_SERVICE_URL
```

**Fix**: Start DFID Service or remove DFID_SERVICE_URL

### Sequence counter resets

```bash
# Ensure Redis is configured
REDIS_URL=redis://localhost:6379 cargo run
```

**Fix**: Configure Redis for persistence

### API errors

```bash
# Check logs
docker-compose logs api

# Look for:
# ✨ DFID Service client enabled (good)
# 📍 DFID Service client disabled (fallback)
```

## Railway Deployment

```bash
# Create service
railway service create defarm-dfid-service

# Configure
railway variables set PORT=3001
railway variables set REDIS_URL=${{Redis.REDIS_URL}}

# Deploy
railway up

# Enable for API
railway service defarm-engines-api
railway variables set DFID_SERVICE_URL=https://defarm-dfid-service.up.railway.app
```

## Verification

```bash
# Run checks
./scripts/verify_dfid_uniformity.sh

# Should see:
# ✨ All checks passed! DFID architecture is uniform.
```

## Documentation

- Full Guide: `DFID_SERVICE_DEPLOYMENT.md`
- Migration: `MIGRATION_GUIDE.md`
- Summary: `DFID_SEPARATION_SUMMARY.md`
- Status: `IMPLEMENTATION_STATUS.md`

## Key Points

✅ **Backward Compatible**: Works without DFID Service
✅ **Zero Downtime**: Enable/disable anytime
✅ **Automatic Fallback**: If remote fails, uses local
✅ **Better Security**: BLAKE3 checksums
✅ **Horizontal Scaling**: Multiple DFID Service instances

## Emergency Rollback

```bash
# Remove environment variable
unset DFID_SERVICE_URL

# Or in Railway:
railway variables unset DFID_SERVICE_URL
railway redeploy
```

**Impact**: None - seamless fallback to local

---

That's it! DFID Service is optional, transparent, and safe.

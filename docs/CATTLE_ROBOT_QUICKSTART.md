# Cattle Robot - Quick Start Guide

## 5-Minute Setup

### 1. Prerequisites Check

```bash
# Check Rust installation
rustc --version  # Should be 1.70+

# Check PostgreSQL
psql --version

# Check database connection
psql $DATABASE_URL -c "SELECT 1"
```

### 2. Run Database Migration

```bash
# From engines directory
cargo sqlx migrate run
```

This creates three tables:
- `robot_cattle` - Cattle records
- `robot_events` - Lifecycle events
- `robot_mints` - Blockchain tracking

### 3. Get API Key

Option A: Create via API (if you have admin access):
```bash
curl -X POST "https://defarm-engines-api-production.up.railway.app/api/api-keys" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Cattle Robot Key",
    "permissions": ["read", "write"],
    "expires_at": null
  }'
```

Option B: Use existing demo account:
```bash
# Login to get JWT
curl -X POST "https://defarm-engines-api-production.up.railway.app/api/auth/login" \
  -H "Content-Type: application/json" \
  -d '{"username": "hen", "password": "demo123"}'
```

### 4. Set Environment Variables

Create `.env.robot`:
```bash
# Required
DATABASE_URL=postgresql://user:pass@localhost:5432/defarm_dev
ROBOT_API_KEY=dfm_your_api_key_here

# Optional (has defaults)
RAILWAY_API_URL=https://defarm-engines-api-production.up.railway.app
ROBOT_MODE=production
ROBOT_SCHEDULE=weekday-heavy
```

Load variables:
```bash
source .env.robot
# or
export $(cat .env.robot | xargs)
```

### 5. Build & Run

```bash
# Build (release mode for production)
cargo build --release --bin cattle-robot

# Run
./target/release/cattle-robot
```

Or use the script:
```bash
./scripts/start_cattle_robot.sh
```

## Expected Output

```
🤖 Cattle Robot Starting...
================================
✓ Database connected
✓ API is healthy
Creating new robot circuit...
✓ Circuit created: 002ea6db-6b7b-4a69-8780-1f01ae074265
✓ Adapter configured
🚀 Robot is now running
Circuit ID: 002ea6db-6b7b-4a69-8780-1f01ae074265
Press Ctrl+C to stop
----------------------------------------
⏰ 2025-01-20 15:42:13 UTC - Weekday Business hours (Mon)
🎲 Selected operation: NewMint
✅ MINT SUCCESS: SISBOV=BR547891234567, DFID=DFID-20250120-000001-A7B3, CID=QmX...
📊 Stats: Mints=1, Updates=0, Errors=0, Uptime=12s
⏳ Next operation in 15m 23s
----------------------------------------
```

## Verify It's Working

### Check Database

```sql
-- See minted cattle
SELECT sisbov, breed, state, owner_hash, created_at
FROM robot_cattle
ORDER BY created_at DESC
LIMIT 5;

-- See events
SELECT c.sisbov, e.event_type, e.event_date, e.dfid
FROM robot_events e
JOIN robot_cattle c ON e.cattle_id = c.id
ORDER BY e.created_at DESC
LIMIT 5;

-- Statistics
SELECT
  COUNT(*) as total_cattle,
  COUNT(DISTINCT state) as states,
  MIN(created_at) as first_mint,
  MAX(created_at) as last_mint
FROM robot_cattle;
```

### Check API

```bash
# List circuits (should see robot circuit)
curl "https://defarm-engines-api-production.up.railway.app/api/circuits" \
  -H "X-API-Key: $ROBOT_API_KEY"

# Get specific DFID
curl "https://defarm-engines-api-production.up.railway.app/api/items/DFID-20250120-000001-A7B3" \
  -H "X-API-Key: $ROBOT_API_KEY"
```

### Check Blockchain

```bash
# View Stellar transaction (use stellar_tx from robot_mints)
# https://stellar.expert/explorer/testnet/tx/{stellar_tx}

# View IPFS content (use cid from robot_mints)
# https://gateway.pinata.cloud/ipfs/{cid}
```

## Common First-Time Issues

### Issue: "Connection refused" to database
**Solution**: Start PostgreSQL
```bash
# macOS
brew services start postgresql@14

# Linux
sudo systemctl start postgresql
```

### Issue: "API key invalid"
**Solution**: Check key format
- Must start with `dfm_`
- Copy exact key from creation response
- Don't include quotes or whitespace

### Issue: "Circuit not found"
**Solution**: Let robot create circuit automatically
- Don't set `ROBOT_CIRCUIT_ID` on first run
- Robot will create and configure circuit
- Save circuit ID from logs for future runs

### Issue: Compilation errors with sqlx
**Solution**: Already handled in code (uses runtime queries)
- No need to set DATABASE_URL during build
- Migration can run after build

## Stop the Robot

```bash
# Graceful shutdown (press in terminal)
Ctrl+C

# From another terminal
pkill cattle-robot

# Kill with signal
kill -SIGTERM $(pgrep cattle-robot)
```

Robot will log final statistics and close gracefully.

## Next Steps

1. **Monitor Operations**: Watch logs to see minting and updates
2. **Query Data**: Explore robot_cattle and robot_events tables
3. **Adjust Schedule**: Change ROBOT_SCHEDULE if needed
4. **Scale Up**: Run multiple instances with different circuits
5. **Integrate**: Use minted data for testing and demos

## Dry Run Mode

Test without actual minting:

```bash
export ROBOT_MODE=dry-run
./target/release/cattle-robot
```

Output:
```
DRY RUN: Would execute NewMint
📊 Stats: Mints=0, Updates=0, Errors=0, Uptime=45s
```

## Production Checklist

- [ ] Database backed up
- [ ] API key created with proper permissions
- [ ] Environment variables configured
- [ ] Migrations run successfully
- [ ] Test run in dry-run mode
- [ ] Monitoring/logging configured
- [ ] Graceful shutdown tested

## Getting Help

**Logs**:
```bash
# Save logs to file
./target/release/cattle-robot 2>&1 | tee robot.log

# Filter errors
grep "ERROR" robot.log
```

**Database Query**:
```sql
-- Find recent errors (if robot stores error logs)
SELECT * FROM robot_events
WHERE metadata->>'error' IS NOT NULL
ORDER BY created_at DESC
LIMIT 10;
```

**Documentation**:
- Full docs: `docs/CATTLE_ROBOT.md`
- API docs: `docs/development/GERBOV_UPDATED_DOC.md`
- Architecture: `CLAUDE.md`

## Example Session

```bash
$ export DATABASE_URL=postgresql://localhost/defarm_dev
$ export ROBOT_API_KEY=dfm_abc123...
$ cargo build --release --bin cattle-robot
   Compiling defarm-engine v0.1.0
    Finished release [optimized] target(s) in 45.2s

$ ./target/release/cattle-robot
🤖 Cattle Robot Starting...
✓ Database connected
✓ API is healthy
✓ Circuit created: 002ea6db-6b7b-4a69-8780-1f01ae074265
🚀 Robot is now running
----------------------------------------
⏰ 2025-01-20 15:42:13 UTC - Weekday Business hours
🎲 Selected operation: NewMint
✅ MINT SUCCESS: SISBOV=BR547891234567
📊 Stats: Mints=1, Updates=0, Errors=0
⏳ Next operation in 15m 23s
----------------------------------------
... (continues)
^C
🛑 Shutting down gracefully...
Final stats: Mints=5, Updates=2, Errors=0
👋 Robot stopped
```

Ready to mint! 🐄🚀

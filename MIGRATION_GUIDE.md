# DFID Service Migration Guide

## Overview

This guide explains how to migrate from embedded DFID generation to the separated DFID Service architecture with **zero downtime**.

## Migration Strategy

The system supports **dual-mode operation**:

1. **Phase 1**: Both systems run in parallel (local + remote)
2. **Phase 2**: Gradual traffic shift to DFID Service
3. **Phase 3**: Full cutover (optional - can stay in dual mode)

## Pre-Migration Checklist

- [ ] PostgreSQL database is running and accessible
- [ ] Redis is running and accessible
- [ ] Environment variables are documented
- [ ] Backup of current database
- [ ] Monitoring dashboards configured
- [ ] Rollback plan documented

## Migration Steps

### Phase 1: Deploy DFID Service (2 weeks)

#### Week 1: Local Testing

**Day 1-2: Build and Test DFID Service**

```bash
# Clone and build DFID Service
cd dfid-service
cargo test
cargo build --release

# Start Redis
docker run -d -p 6379:6379 redis:7-alpine

# Start DFID Service locally
REDIS_URL=redis://localhost:6379 cargo run
```

**Verification:**
```bash
# Test DFID generation
curl -X POST http://localhost:3001/dfid/generate \
  -H "Content-Type: application/json" \
  -d '{"count": 1}'

# Expected output:
{
  "dfids": ["DFID-20250131-000001-A7B2C3"],
  "format_version": "1.0",
  "generated_at": "2025-01-31T12:00:00Z"
}

# Test validation
curl http://localhost:3001/dfid/DFID-20250131-000001-A7B2C3/validate

# Expected: {"valid":true,"checksum_ok":true,...}
```

**Day 3-4: Integration Testing**

```bash
# Start Item Registry with DFID Service
export DFID_SERVICE_URL=http://localhost:3001
export DATABASE_URL=postgresql://defarm:defarm123@localhost:5432/defarm
export REDIS_URL=redis://localhost:6379

cargo run --bin defarm-api
```

**Verification:**
```bash
# Check logs for:
# "✨ DFID Service client enabled - using remote DFID generation"

# Create test item
curl -X POST http://localhost:3000/api/items/local \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "identifiers": [{"namespace": "test", "key": "id", "value": "123"}]
  }'

# Verify DFID format (should have 6-char checksum)
```

**Day 5: Load Testing**

```bash
# Install Apache Bench
brew install ab  # macOS
# OR
sudo apt-get install apache2-utils  # Linux

# Test DFID generation throughput
ab -n 10000 -c 100 -p generate.json \
  http://localhost:3001/dfid/generate

# Expected: >1000 requests/sec

# Test item creation with DFID Service
ab -n 1000 -c 10 -H "Authorization: Bearer $TOKEN" \
  -p item.json http://localhost:3000/api/items/local
```

#### Week 2: Production Deployment

**Day 1: Deploy to Railway Staging**

```bash
# Create DFID Service on Railway
railway service create defarm-dfid-service-staging

# Configure environment
railway variables set PORT=3001
railway variables set REDIS_URL=${{Redis.REDIS_URL}}

# Deploy
cd dfid-service
railway up
```

**Day 2: Enable for Staging API**

```bash
# Update API service
railway service defarm-engines-api-staging
railway variables set DFID_SERVICE_URL=https://defarm-dfid-service-staging.up.railway.app

# Verify logs show DFID Service enabled
railway logs
```

**Day 3-4: Staging Validation**

```bash
# Run integration tests against staging
npm run test:integration:staging

# Monitor metrics
railway logs --service defarm-dfid-service-staging | grep "sequence"

# Check error rates
railway logs --service defarm-engines-api-staging | grep "DFID generation failed"
```

**Day 5: Production Deployment**

```bash
# Deploy DFID Service to production
railway service create defarm-dfid-service

# Configure
railway variables set PORT=3001
railway variables set REDIS_URL=${{Redis.REDIS_URL}}

# Deploy
cd dfid-service
railway up

# Enable for production API (GRADUAL!)
railway service defarm-engines-api
railway variables set DFID_SERVICE_URL=https://defarm-dfid-service.up.railway.app

# Monitor closely
railway logs -f
```

### Phase 2: Gradual Traffic Shift (2 weeks)

#### Week 1: Monitor and Validate

**Metrics to Monitor:**

1. **DFID Service Health**
   - Uptime: Should be >99.9%
   - Response time: <10ms p95
   - Error rate: <0.1%

2. **Sequence Counter**
   - Redis persistence working
   - No sequence gaps
   - No collisions

3. **API Performance**
   - Item creation latency unchanged
   - No authentication issues
   - Circuit operations normal

**Daily Checks:**

```bash
# Check DFID Service health
curl https://defarm-dfid-service.up.railway.app/health | jq

# Verify sequence is incrementing
# (Run this multiple times, sequence should increase)

# Check for errors in API logs
railway logs --service defarm-engines-api | grep "DFID generation failed"

# Verify checksum format (should be 6 hex chars)
railway logs --service defarm-engines-api | grep "DFID-" | tail -10
```

#### Week 2: Performance Optimization

**Optimize DFID Service:**

```bash
# Scale if needed
railway service defarm-dfid-service
railway scale --replicas 2  # If traffic is high

# Verify Redis persistence interval
# Should persist every 10 seconds
railway logs | grep "Persisted sequence"
```

**Monitor Costs:**

- DFID Service instances
- Redis memory usage
- Network bandwidth

### Phase 3: Full Cutover (Optional)

At this point, you can:

**Option A: Stay in Dual Mode** (Recommended)
- Keep both DfidEngine and DfidClient
- Allows instant rollback if needed
- Minimal code changes

**Option B: Remove Local DfidEngine**
- Simplify code by removing embedded generation
- Requires DFID Service to be critical dependency
- More "microservices-native" approach

For Option A (recommended), **no further action needed**.

For Option B:

```rust
// In items_engine.rs and circuits_engine.rs
// Remove dfid_engine field
// Make dfid_client required instead of Option<>
// Update generate_dfid_internal to always use client
```

**We recommend Option A** for production resilience.

## Rollback Procedures

### Immediate Rollback (< 5 minutes)

If DFID Service fails catastrophically:

```bash
# Remove DFID_SERVICE_URL
railway service defarm-engines-api
railway variables unset DFID_SERVICE_URL

# Redeploy
railway redeploy

# Verify fallback to local generation
railway logs | grep "DFID Service client disabled"
```

**Impact**: None - system automatically falls back to local generation

### Partial Rollback (< 30 minutes)

If DFID Service has intermittent issues:

```bash
# Add circuit breaker logic (future enhancement)
# For now, monitor and disable if error rate >5%

railway logs --service defarm-dfid-service | grep "error" | wc -l

# If errors are frequent, rollback as above
```

## Validation Checklist

After each phase, verify:

- [ ] DFID Service health endpoint returns 200
- [ ] New DFIDs have 6-character checksums (BLAKE3 24-bit)
- [ ] Sequence counter persists across DFID Service restarts
- [ ] Item creation API works normally
- [ ] Circuit tokenization works normally
- [ ] No error spikes in logs
- [ ] Response times within SLA (<100ms p95)
- [ ] Redis memory usage stable
- [ ] PostgreSQL performance normal

## Monitoring and Alerts

### Key Metrics

1. **DFID Service**
   - `dfid_generation_rate`: DFIDs/second
   - `dfid_validation_rate`: Validations/second
   - `dfid_sequence_current`: Current sequence value
   - `dfid_redis_persist_errors`: Failed persistence attempts

2. **API Integration**
   - `dfid_client_errors`: Failed DFID Service calls
   - `dfid_local_fallback_count`: Times fell back to local
   - `item_creation_latency`: End-to-end latency

### Recommended Alerts

```yaml
alerts:
  - name: DFID Service Down
    condition: health_check_fails > 3
    severity: critical
    action: Auto-rollback

  - name: DFID Generation Errors
    condition: error_rate > 5%
    severity: high
    action: Page on-call

  - name: Sequence Persistence Failures
    condition: redis_persist_errors > 10
    severity: medium
    action: Investigate Redis

  - name: High Latency
    condition: p95_latency > 100ms
    severity: medium
    action: Check DFID Service
```

## Troubleshooting Guide

### Issue: DFID Service Returns 500

**Symptoms:**
- Item creation fails
- Logs show "DFID generation failed"

**Diagnosis:**
```bash
railway logs --service defarm-dfid-service | grep "error"
```

**Common Causes:**
1. Redis connection lost
2. Out of memory
3. Service crashed

**Resolution:**
```bash
# Quick fix: Restart service
railway service defarm-dfid-service
railway restart

# Long-term: Check Redis health
railway logs --service redis | grep "error"
```

### Issue: Sequence Counter Reset

**Symptoms:**
- DFID sequences restart from 1
- Possible collisions (rare)

**Diagnosis:**
```bash
# Check Redis persistence
railway logs --service defarm-dfid-service | grep "Persisted sequence"

# Verify Redis is running
railway service redis
railway status
```

**Resolution:**
```bash
# Ensure Redis URL is set
railway variables get REDIS_URL

# Restart DFID Service to reload from Redis
railway restart
```

### Issue: Performance Degradation

**Symptoms:**
- Item creation slower than before
- Timeouts on DFID generation

**Diagnosis:**
```bash
# Check DFID Service response time
time curl https://defarm-dfid-service.up.railway.app/dfid/generate

# Should be <50ms
```

**Resolution:**
```bash
# Scale up DFID Service
railway scale --replicas 2

# Or upgrade instance size
railway upgrade
```

## Success Criteria

Migration is considered successful when:

✅ DFID Service uptime >99.9% for 2 weeks
✅ Zero DFID collisions detected
✅ API latency unchanged (within 10%)
✅ No increase in error rates
✅ Redis persistence working (sequence survives restarts)
✅ Monitoring and alerts configured
✅ Team trained on new architecture
✅ Documentation updated

## Post-Migration Tasks

1. **Update Documentation**
   - Architecture diagrams
   - API documentation
   - Runbooks

2. **Team Training**
   - New deployment process
   - Troubleshooting procedures
   - Rollback procedures

3. **Future Enhancements**
   - Index Service (Layer 4)
   - Adapter Registration API
   - Merkle Tree anchoring

## Timeline Summary

| Phase | Duration | Activities | Rollback Risk |
|-------|----------|------------|---------------|
| 1.1 - Local Testing | 1 week | Build, test locally | None |
| 1.2 - Production Deploy | 1 week | Deploy to Railway | Low |
| 2 - Validation | 2 weeks | Monitor, optimize | Medium |
| 3 - Full Cutover | Optional | Remove fallback | High |

**Total: 4-6 weeks** for complete, zero-downtime migration.

## Support

For issues during migration:

1. Check logs: `railway logs`
2. Run verification: `./scripts/verify_dfid_uniformity.sh`
3. Review this guide
4. Rollback if critical

## Conclusion

This migration provides:

- ✅ **Improved Security**: BLAKE3 checksums
- ✅ **Better Persistence**: Redis-backed sequences
- ✅ **Horizontal Scaling**: Multiple DFID Service instances
- ✅ **Zero Downtime**: Gradual, reversible migration
- ✅ **Future Proof**: Foundation for 4-layer architecture

The dual-mode approach ensures safety while enabling the benefits of service separation.

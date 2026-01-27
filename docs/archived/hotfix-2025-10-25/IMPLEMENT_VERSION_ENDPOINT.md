# Implementation Guide: /version Endpoint with GIT_HASH

## Overview
Add deployment tracking endpoint to correlate production behavior with specific commits.

## Step 1: Create Version API Handler

**File**: `src/api/version.rs` (NEW)

```rust
use axum::{
    http::HeaderMap,
    response::Json,
};
use serde_json::json;

/// GET /version - Returns deployment version information
pub async fn get_version() -> (HeaderMap, Json<serde_json::Value>) {
    let mut headers = HeaderMap::new();

    let commit_hash = option_env!("GIT_HASH").unwrap_or("unknown");

    // Add X-Commit header for easy tracking
    if let Ok(header_value) = commit_hash.parse() {
        headers.insert("X-Commit", header_value);
    }

    let response = json!({
        "service": "DeFarm Engines API",
        "version": env!("CARGO_PKG_VERSION"),
        "commit": commit_hash,
        "build_time": option_env!("BUILD_TIME").unwrap_or("unknown"),
        "rust_version": option_env!("RUSTC_VERSION").unwrap_or("unknown"),
    });

    (headers, Json(response))
}
```

## Step 2: Register Module in src/api/mod.rs

```rust
pub mod version;  // Add this line
```

## Step 3: Add Route in src/bin/api.rs

Find the router setup and add:

```rust
use engines::api::version;  // Add to imports

// In the router configuration:
.route("/version", get(version::get_version))
.route("/api/version", get(version::get_version))  // Both paths for flexibility
```

## Step 4: Update Dockerfile for Build Args

**File**: `Dockerfile`

Find the build stage and add build arguments:

```dockerfile
# Build stage
FROM rust:1.83 AS builder

WORKDIR /app

# Add build arguments
ARG GIT_HASH=unknown
ARG BUILD_TIME=unknown
ARG RUSTC_VERSION=unknown

# Set environment variables for build
ENV GIT_HASH=${GIT_HASH}
ENV BUILD_TIME=${BUILD_TIME}
ENV RUSTC_VERSION=${RUSTC_VERSION}

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# ... rest of Dockerfile
```

## Step 5: Update Railway Deployment

**Option A: Using Railway CLI**

Create `scripts/deploy_railway.sh`:

```bash
#!/bin/bash
set -e

echo "🚂 Deploying to Railway with version tracking"

# Get current commit hash
GIT_HASH=$(git rev-parse HEAD)
BUILD_TIME=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
RUSTC_VERSION=$(rustc --version | cut -d' ' -f2)

echo "Commit: $GIT_HASH"
echo "Build Time: $BUILD_TIME"
echo "Rust Version: $RUSTC_VERSION"

# Set Railway environment variables
railway variables set GIT_HASH="$GIT_HASH"
railway variables set BUILD_TIME="$BUILD_TIME"
railway variables set RUSTC_VERSION="$RUSTC_VERSION"

# Deploy
railway up --detach

echo "✅ Deployment started"
echo "Monitor at: https://railway.app/project/..."
```

**Option B: GitHub Actions** (if using CI/CD)

Create `.github/workflows/deploy.yml`:

```yaml
name: Deploy to Railway

on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Set version variables
        id: version
        run: |
          echo "GIT_HASH=$(git rev-parse HEAD)" >> $GITHUB_OUTPUT
          echo "BUILD_TIME=$(date -u +"%Y-%m-%dT%H:%M:%SZ")" >> $GITHUB_OUTPUT
          echo "RUSTC_VERSION=1.83.0" >> $GITHUB_OUTPUT

      - name: Deploy to Railway
        uses: bervProject/railway-deploy@main
        with:
          railway_token: ${{ secrets.RAILWAY_TOKEN }}
          service: defarm-engines-api-production
        env:
          GIT_HASH: ${{ steps.version.outputs.GIT_HASH }}
          BUILD_TIME: ${{ steps.version.outputs.BUILD_TIME }}
          RUSTC_VERSION: ${{ steps.version.outputs.RUSTC_VERSION }}
```

## Step 6: Test Locally

```bash
# Set environment variables for local testing
export GIT_HASH=$(git rev-parse HEAD)
export BUILD_TIME=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
export RUSTC_VERSION=$(rustc --version | cut -d' ' -f2)

# Build with env vars
cargo build

# Run the server
cargo run --bin api

# Test the endpoint
curl http://localhost:3000/version

# Expected response:
# {
#   "service": "DeFarm Engines API",
#   "version": "0.1.0",
#   "commit": "0c707a9...",
#   "build_time": "2025-10-25T12:34:56Z",
#   "rust_version": "1.83.0"
# }

# Check X-Commit header
curl -I http://localhost:3000/version | grep X-Commit
```

## Step 7: Verify Production Deployment

After deploying to Railway:

```bash
# Check version endpoint
curl https://defarm-engines-api-production.up.railway.app/version

# Verify commit hash matches
git rev-parse HEAD

# Check headers
curl -I https://defarm-engines-api-production.up.railway.app/version
```

## Step 8: Update Health Check Script

Create `scripts/check_deployment.sh`:

```bash
#!/bin/bash
API_BASE="${API_BASE:-https://defarm-engines-api-production.up.railway.app}"

echo "🔍 Checking deployment status..."

# Get version info
VERSION_RESPONSE=$(curl -s "$API_BASE/version")

echo "Deployed Version Info:"
echo "$VERSION_RESPONSE" | jq .

# Extract commit
DEPLOYED_COMMIT=$(echo "$VERSION_RESPONSE" | jq -r .commit)
LOCAL_COMMIT=$(git rev-parse HEAD)

echo ""
echo "Local commit:    $LOCAL_COMMIT"
echo "Deployed commit: $DEPLOYED_COMMIT"

if [ "$DEPLOYED_COMMIT" = "$LOCAL_COMMIT" ]; then
    echo "✅ Deployment is up to date"
else
    echo "⚠️  Deployment is out of date"
    echo "Run: railway up --detach"
fi

# Check health
HEALTH=$(curl -s "$API_BASE/health")
echo ""
echo "Health Status:"
echo "$HEALTH" | jq .
```

## Step 9: Integration with Metrics

Update smoke test and load test scripts to include version tracking:

```bash
#!/bin/bash
# scripts/smoke_test_with_version.sh

API_BASE="https://defarm-engines-api-production.up.railway.app"

# Get deployment version
VERSION=$(curl -s "$API_BASE/version" | jq -r .commit)
BUILD_TIME=$(curl -s "$API_BASE/version" | jq -r .build_time)

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🧪 Smoke Test - Deployment Verification"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Commit: $VERSION"
echo "Build Time: $BUILD_TIME"
echo ""

# Run smoke test...
# (rest of smoke test logic)

# Save results with version
cat > /tmp/smoke_test_results.json <<EOF
{
  "test_type": "smoke",
  "commit": "$VERSION",
  "build_time": "$BUILD_TIME",
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "results": {
    "total_requests": $TOTAL,
    "successful": $SUCCESS,
    "failed": $FAILED,
    "success_rate": "$SUCCESS_RATE%"
  }
}
EOF
```

## Benefits

1. **Deployment Tracking**: Instantly see which commit is deployed
2. **Correlation**: Match production errors to specific code changes
3. **Rollback Verification**: Confirm successful rollbacks
4. **Multi-Environment**: Compare versions across dev/staging/prod
5. **CI/CD Integration**: Automate version verification in pipelines
6. **Debugging**: Include commit hash in error reports

## Example Usage in Production

### Scenario 1: Performance Regression
```bash
# Check deployed version
curl https://api.defarm.net/version

# Compare with git log
git log --oneline | grep <commit_hash>

# If regression found, rollback to previous commit
git checkout <previous_working_commit>
railway up --detach
```

### Scenario 2: Feature Verification
```bash
# After deploying feature in commit abc123
VERSION=$(curl -s https://api.defarm.net/version | jq -r .commit)

if [[ "$VERSION" == "abc123"* ]]; then
  echo "✅ Feature deployed, running E2E tests..."
  ./scripts/e2e_tests.sh
else
  echo "❌ Wrong version deployed!"
fi
```

### Scenario 3: Metrics Correlation
```bash
# Include version in all load test reports
DEPLOYED_VERSION=$(curl -s https://api.defarm.net/version | jq -r .commit)

echo "# Load Test Results - Commit $DEPLOYED_VERSION" > report.md
echo "Build Time: $(curl -s https://api.defarm.net/version | jq -r .build_time)" >> report.md
# ... rest of metrics
```

## Completion Checklist

- [ ] Create `src/api/version.rs`
- [ ] Add `pub mod version;` to `src/api/mod.rs`
- [ ] Add routes to `src/bin/api.rs`
- [ ] Update `Dockerfile` with build args
- [ ] Create `scripts/deploy_railway.sh`
- [ ] Test locally with env vars
- [ ] Deploy to Railway
- [ ] Verify `/version` endpoint responds
- [ ] Update smoke/load test scripts
- [ ] Create `scripts/check_deployment.sh`
- [ ] Document in CLAUDE.md or README

## Estimated Time
- Implementation: 30 minutes
- Testing: 15 minutes
- Deployment: 10 minutes
- Documentation: 15 minutes
**Total**: ~70 minutes

---

This completes the /version endpoint implementation. The next step would be adding detailed tracing spans for lock acquisition timing.

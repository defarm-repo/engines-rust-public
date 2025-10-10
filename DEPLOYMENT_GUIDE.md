# Deployment Guide: Stellar Adapter Configuration

## Overview

The engines project uses Stellar CLI as a subprocess to interact with Stellar blockchain. This requires proper Stellar CLI network configuration on the deployment server.

## ✅ What's Working (No Action Needed)

- **IPFS-IPFS Adapter**: Production ready, no infrastructure dependencies
- **Stellar Testnet-IPFS Adapter**: Production ready, verified working
- **Database Configuration**: All adapters read from database (no code changes needed)

## ⚠️ Mainnet Configuration (Action Required for Production)

### Problem

The Stellar mainnet adapter requires the Stellar CLI to have a properly configured `mainnet` network. This is an infrastructure/deployment requirement, not a code issue.

### Solution Options

#### Option 1: Configure Stellar CLI (Recommended for Production)

On the deployment server, ensure Stellar CLI is installed and configured:

```bash
# 1. Install Stellar CLI (if not already installed)
# See: https://developers.stellar.org/docs/tools/developer-tools/cli

# 2. Configure mainnet network
stellar network add mainnet \
  --rpc-url https://soroban-rpc.mainnet.stellar.org \
  --network-passphrase "Public Global Stellar Network ; September 2015"

# 3. Verify configuration
stellar network ls

# 4. Test connectivity
stellar contract invoke \
  --network mainnet \
  --source <YOUR_SECRET_KEY> \
  --id <CONTRACT_ID> \
  -- <function_name>
```

**Add to deployment checklist:**
- [ ] Stellar CLI installed
- [ ] Mainnet network configured
- [ ] Testnet network configured (if using testnet)
- [ ] Network connectivity verified

#### Option 2: Use Alternative RPC Endpoints

If the default RPC endpoint is unreachable, try alternative providers:

```bash
# Option A: Stellar.org RPC
stellar network add mainnet \
  --rpc-url https://soroban-rpc.mainnet.stellar.org \
  --network-passphrase "Public Global Stellar Network ; September 2015"

# Option B: Alternative provider (if available)
# stellar network add mainnet \
#   --rpc-url https://your-alternative-rpc-provider.com \
#   --network-passphrase "Public Global Stellar Network ; September 2015"
```

#### Option 3: Docker Container with Pre-configured CLI

Create a Docker image with Stellar CLI pre-configured:

```dockerfile
FROM rust:1.75

# Install Stellar CLI
RUN cargo install --locked stellar-cli

# Configure networks
RUN stellar network add testnet \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015"

RUN stellar network add mainnet \
  --rpc-url https://soroban-rpc.mainnet.stellar.org \
  --network-passphrase "Public Global Stellar Network ; September 2015"

# Your app
COPY . /app
WORKDIR /app

CMD ["cargo", "run", "--bin", "defarm-api"]
```

#### Option 4: Health Check at Startup

Add a startup health check to verify Stellar CLI configuration:

```rust
// In src/bin/api.rs or startup code

async fn verify_stellar_configuration() -> Result<(), Box<dyn std::error::Error>> {
    // Check if stellar CLI is available
    let output = tokio::process::Command::new("stellar")
        .args(&["network", "ls"])
        .output()
        .await?;

    let networks = String::from_utf8_lossy(&output.stdout);

    if !networks.contains("testnet") {
        eprintln!("⚠️  WARNING: Stellar testnet network not configured");
        eprintln!("   Run: stellar network add testnet --rpc-url https://soroban-testnet.stellar.org --network-passphrase \"Test SDF Network ; September 2015\"");
    }

    if !networks.contains("mainnet") {
        eprintln!("⚠️  WARNING: Stellar mainnet network not configured");
        eprintln!("   Run: stellar network add mainnet --rpc-url https://soroban-rpc.mainnet.stellar.org --network-passphrase \"Public Global Stellar Network ; September 2015\"");
    }

    Ok(())
}
```

#### Option 5: Disable Mainnet Adapter Until Configured

Update database to disable mainnet adapter:

```sql
UPDATE adapter_configs
SET is_active = false
WHERE adapter_type = 'StellarMainnetIpfs';
```

Re-enable once infrastructure is ready:

```sql
UPDATE adapter_configs
SET is_active = true
WHERE adapter_type = 'StellarMainnetIpfs';
```

## Environment-Specific Configuration

### Development Environment

```bash
# Local development - testnet only is fine
stellar network add testnet \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015"
```

### Staging Environment

```bash
# Staging - configure both testnet and mainnet
stellar network add testnet \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015"

stellar network add mainnet \
  --rpc-url https://soroban-rpc.mainnet.stellar.org \
  --network-passphrase "Public Global Stellar Network ; September 2015"
```

### Production Environment

```bash
# Production - both networks with production RPC endpoints
stellar network add testnet \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015"

stellar network add mainnet \
  --rpc-url https://soroban-rpc.mainnet.stellar.org \
  --network-passphrase "Public Global Stellar Network ; September 2015"

# Verify
stellar network ls
```

## Troubleshooting

### Issue: "invalid rpc url: invalid uri character"

**Cause:** Malformed RPC URL in network configuration

**Solution:**
```bash
stellar network rm mainnet
stellar network add mainnet \
  --rpc-url https://soroban-rpc.mainnet.stellar.org \
  --network-passphrase "Public Global Stellar Network ; September 2015"
```

### Issue: "dns error: failed to lookup address information"

**Cause:** Network connectivity issue or DNS resolution problem

**Solutions:**
1. Check internet connectivity
2. Try alternative DNS servers
3. Use alternative RPC endpoint
4. Check firewall rules
5. Verify the RPC URL is correct and reachable:
   ```bash
   curl https://soroban-rpc.mainnet.stellar.org/health
   ```

### Issue: "Failed to find config identity"

**Cause:** Using `--source-account` with identity name instead of secret key

**Solution:** Our code now uses `--source` with the secret key directly (fixed in latest version)

### Issue: "UnreachableCodeReached" error

**Cause:** Interface address not authorized on IPCM contract

**Solution:** Already handled by backbone initialization

## Deployment Checklist

- [ ] Stellar CLI installed on server
- [ ] Testnet network configured
- [ ] Mainnet network configured
- [ ] Network connectivity verified
- [ ] IPCM contracts initialized (done by backbone)
- [ ] Interface addresses authorized (done by backbone)
- [ ] Database adapter configurations populated
- [ ] Environment variables set (if any fallbacks used)
- [ ] Health checks passing

## Production Recommendations

### 1. Infrastructure as Code

Include Stellar CLI configuration in your infrastructure setup:

**Ansible:**
```yaml
- name: Install Stellar CLI
  shell: cargo install --locked stellar-cli

- name: Configure Stellar networks
  shell: |
    stellar network add testnet \
      --rpc-url https://soroban-testnet.stellar.org \
      --network-passphrase "Test SDF Network ; September 2015"
    stellar network add mainnet \
      --rpc-url https://soroban-rpc.mainnet.stellar.org \
      --network-passphrase "Public Global Stellar Network ; September 2015"
```

**Kubernetes ConfigMap:**
```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: stellar-config
data:
  testnet.toml: |
    rpc_url = "https://soroban-testnet.stellar.org"
    network_passphrase = "Test SDF Network ; September 2015"
  mainnet.toml: |
    rpc_url = "https://soroban-rpc.mainnet.stellar.org"
    network_passphrase = "Public Global Stellar Network ; September 2015"
```

### 2. Monitoring

Add monitoring for Stellar adapter health:

```rust
// Periodic health check
async fn monitor_stellar_adapters() {
    // Check if networks are configured
    // Check if RPC endpoints are reachable
    // Alert if issues detected
}
```

### 3. Fallback Strategy

Consider implementing fallback RPC endpoints:

```rust
// Try primary RPC, fallback to secondary if needed
const PRIMARY_RPC: &str = "https://soroban-rpc.mainnet.stellar.org";
const FALLBACK_RPC: &str = "https://alternative-rpc-provider.com";
```

## Future Enhancement: Native Stellar SDK

**Long-term recommendation:** Replace Stellar CLI subprocess with native Rust SDK (`stellar-xdr` crate)

**Benefits:**
- No external CLI dependency
- Faster execution (no subprocess overhead)
- Better error handling
- No network configuration files needed

**Trade-off:**
- More complex implementation
- Need to handle XDR encoding/signing manually

**Current approach is production-ready** - CLI integration works well and is what backbone uses.

## Summary

The mainnet configuration issue is **purely infrastructural** - the code is production-ready. Choose the solution that fits your deployment model:

- **Quick fix:** Run `stellar network add mainnet ...` on deployment server
- **Docker:** Use pre-configured container
- **IaC:** Add to Ansible/Terraform/Kubernetes configs
- **Temporary:** Disable mainnet adapter until infrastructure is ready

**Testnet is fully working** and proves the architecture is correct. Mainnet will work identically once CLI is configured.
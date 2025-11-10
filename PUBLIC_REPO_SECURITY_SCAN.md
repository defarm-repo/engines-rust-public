# Public Repository Security Scan Report

**Date**: November 9, 2025
**Repository**: https://github.com/defarm-repo/engines-rust-public
**Total Files**: 150 (only Rust source code + minimal config)

## 1. Documentation Files Scan ✅

### Files Found:
- `README.md` - Minimal public-facing documentation (SAFE)
- **No other documentation files present**

### Verdict: CLEAN
Only the intended minimal README exists. No internal documentation leaked.

## 2. Sensitive Data Pattern Scan ⚠️

### Findings:

#### a) Stellar Wallet Address (PUBLIC - OK)
- **Pattern**: `GANDYZQQ3OQBXHZQXJHZ7AQ2GDBFUQIR4ZLMUPD3P2B7PLIYQNFG54XQ`
- **Location**: Multiple files (api.rs, db_init.rs, adapters)
- **Context**: Default fallback for DEFARM_OWNER_WALLET environment variable
- **Risk**: LOW - This appears to be a public Stellar wallet address used as a default
- **Recommendation**: Consider removing or using a placeholder like "YOUR_STELLAR_ADDRESS"

#### b) Contract Addresses (PUBLIC - OK)
- **Pattern**: `CCDJV6VAFC2MSSDSL4AEJB5BAMGDA5PMCUIZ3UF6AYIJL467PQTBZ7BS`
- **Location**: stellar_client.rs
- **Context**: Testnet IPCM contract address
- **Risk**: LOW - Public testnet contracts are meant to be known

#### c) Test IP Addresses (LOW RISK)
- **Patterns**: `192.168.1.1`, `192.168.1.100`, `192.168.1.200`
- **Location**: Test code in audit_engine.rs, api_key_engine.rs
- **Context**: Used in unit tests
- **Risk**: LOW - Standard private IP range used for testing

## 3. Hardcoded Credentials Scan ✅

### Findings:
- **No hardcoded passwords found**
- **No API keys found**
- **No JWT tokens found**
- **No private keys found**

### Verdict: CLEAN

## 4. Internal References Scan ⚠️

### Findings:

#### a) TODO/FIXME Comments (32 occurrences)
- **Examples**:
  - `// TODO: Implement generic cache invalidation`
  - `// TODO: PostgresPersistence needs store_lid_dfid_mapping method`
- **Risk**: LOW - These reveal implementation gaps but no sensitive info
- **Recommendation**: Consider removing TODOs before public sync

#### b) Module Name References
- **Pattern**: `defarm_engine::*` in use statements
- **Context**: Standard Rust module imports
- **Risk**: NONE - This is the expected crate name

## 5. Client/Demo Information Scan ✅

### Findings:
- **No client names found** (gerbov, etc.)
- **No demo credentials found** (demo123, etc.)
- **No internal user references found**

### Verdict: CLEAN

## 6. Infrastructure Details Scan ✅

### Findings:
- **No Railway URLs or configs**
- **No production URLs**
- **No database connection strings**
- **No email service configurations**

### Verdict: CLEAN

## OVERALL ASSESSMENT: MOSTLY SECURE ✅

### Clean Areas:
- ✅ No internal documentation
- ✅ No shell scripts or test files
- ✅ No client information
- ✅ No demo credentials
- ✅ No infrastructure details
- ✅ No actual secrets or API keys

### Minor Issues to Consider:
1. **Default Stellar wallet address** - Consider using placeholder
2. **TODO/FIXME comments** - Consider filtering these out
3. **Test IP addresses** - Harmless but could be removed

### Recommended Actions:

1. **Update sync workflow to filter out**:
   ```bash
   # Remove TODO/FIXME comments
   find . -name "*.rs" -exec sed -i '/\/\/.*TODO/d; /\/\/.*FIXME/d' {} \;

   # Replace default wallet with placeholder
   sed -i 's/GANDYZQQ3OQBXHZQXJHZ7AQ2GDBFUQIR4ZLMUPD3P2B7PLIYQNFG54XQ/YOUR_STELLAR_WALLET_ADDRESS/g' src/**/*.rs
   ```

2. **No critical issues found** - The public repository is safe for external viewing

## Summary

The public repository contains **ONLY** the essential Rust source code with:
- No sensitive credentials
- No internal documentation
- No client references
- No infrastructure details
- Only minor cosmetic issues (TODOs and default values)

The current filtering strategy is working effectively.
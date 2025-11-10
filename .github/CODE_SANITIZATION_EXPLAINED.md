# Code Sanitization Explained

This document explains how we filter sensitive content from code files during the public repo sync.

## The Challenge

You can't simply delete lines from source code files because:
1. It might break the code structure
2. Comments might be inline with actual code
3. String literals are part of the compiled program

## Our Solution: Text Replacement

We use `sed` (stream editor) to find and replace patterns within files:

### 1. Filtering TODO/FIXME Comments

**Command:**
```bash
find . -name "*.rs" -exec sed -i 's|//.*TODO.*|// Implementation pending|g; s|//.*FIXME.*|// Under review|g' {} \;
```

**How it works:**
- `find . -name "*.rs"` - Find all Rust files
- `-exec sed -i` - Execute sed in-place editing mode
- `s|pattern|replacement|g` - Replace all occurrences

**Example transformation:**
```rust
// Before:
let result = calculate(); // TODO: add error handling
fn process() { // FIXME: this is broken

// After:
let result = calculate(); // Implementation pending
fn process() { // Under review
```

### 2. Replacing Hardcoded Values

**Command:**
```bash
find . -name "*.rs" -exec sed -i 's/GANDYZQQ3OQBXHZQXJHZ7AQ2GDBFUQIR4ZLMUPD3P2B7PLIYQNFG54XQ/STELLAR_WALLET_PLACEHOLDER/g' {} \;
```

**Example transformation:**
```rust
// Before:
let wallet = env::var("WALLET").unwrap_or_else(|_| {
    "GANDYZQQ3OQBXHZQXJHZ7AQ2GDBFUQIR4ZLMUPD3P2B7PLIYQNFG54XQ".to_string()
});

// After:
let wallet = env::var("WALLET").unwrap_or_else(|_| {
    "STELLAR_WALLET_PLACEHOLDER".to_string()
});
```

### 3. Redacting Test Credentials

**Command:**
```bash
find . -name "*.rs" -exec sed -i 's/demo123/[REDACTED]/g; s/Demo123/[REDACTED]/g' {} \;
```

## Why This Approach Works

1. **Preserves Code Structure**: We replace text, not delete lines
2. **Maintains Functionality**: The code still compiles (placeholders are valid strings)
3. **Context Aware**: We only replace specific patterns
4. **Safe**: Uses conservative patterns to avoid breaking code

## Alternative Approaches (Not Used)

### Approach 1: Delete Entire Lines
```bash
sed -i '/\/\/.*TODO/d'  # DANGEROUS: Could delete code lines
```
Problem: Would delete `let x = 5; // TODO: optimize`

### Approach 2: Remove Comment Content Only
```bash
sed -i 's|//.*TODO.*||g'  # Leaves empty comments
```
Result: `let x = 5; ` (trailing space)

### Approach 3: Use AST Parser
Would require a Rust parser to understand code structure - too complex for CI/CD.

## Testing the Sanitization

You can test these commands locally:
```bash
# Make a test copy
cp src/bin/api.rs test.rs

# Test the replacement
sed 's/GANDYZQQ3OQBXHZQXJHZ7AQ2GDBFUQIR4ZLMUPD3P2B7PLIYQNFG54XQ/PLACEHOLDER/g' test.rs

# Check if code still compiles
cargo check
```

## Important Notes

1. **Order Matters**: Replace specific strings before generic ones
2. **Escape Special Characters**: Use `|` as delimiter instead of `/` for paths
3. **Test First**: Always test replacements on copies before applying
4. **Backup**: The original private repo remains unchanged

## What Gets Sanitized

| Pattern | Replacement | Why |
|---------|------------|-----|
| `//.*TODO.*` | `// Implementation pending` | Hides internal task tracking |
| `//.*FIXME.*` | `// Under review` | Hides known issues |
| Specific wallet addresses | `STELLAR_WALLET_PLACEHOLDER` | Removes real addresses |
| `demo123`, `Demo123!` | `[REDACTED]` | Removes test credentials |

## What Doesn't Get Sanitized

- Contract addresses (they're public on blockchain)
- Module names (`defarm_engine`)
- Function names and APIs
- Error messages (needed for debugging)
- Non-sensitive constants

This approach ensures the public repository contains clean, functional code without internal comments or test data.
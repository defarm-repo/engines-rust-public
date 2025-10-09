#!/bin/bash

# DeFarm Engines Setup Script
# This script ensures all required environment variables are set before starting the server

set -e

echo "🚀 DeFarm Engines Setup"
echo "======================="
echo ""

# Check if JWT_SECRET is set
if [ -z "$JWT_SECRET" ]; then
    echo "❌ ERROR: JWT_SECRET environment variable is not set"
    echo ""
    echo "The JWT_SECRET is required for secure authentication."
    echo "It must be at least 32 characters long."
    echo ""
    echo "To set it, run:"
    echo "  export JWT_SECRET=\"your-secure-secret-key-here-at-least-32-chars-long\""
    echo ""
    echo "Or create a .env file:"
    echo "  echo 'JWT_SECRET=your-secure-secret-key-here-at-least-32-chars-long' > .env"
    echo "  source .env"
    echo ""
    exit 1
fi

# Validate JWT_SECRET length
if [ ${#JWT_SECRET} -lt 32 ]; then
    echo "❌ ERROR: JWT_SECRET must be at least 32 characters long"
    echo "Current length: ${#JWT_SECRET}"
    echo ""
    exit 1
fi

echo "✅ JWT_SECRET is set and valid (${#JWT_SECRET} characters)"
echo ""

# Check if Cargo is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ ERROR: Cargo is not installed"
    echo "Please install Rust from https://rustup.rs/"
    exit 1
fi

echo "✅ Cargo is installed"
echo ""

# Run cargo check
echo "🔍 Running cargo check..."
if cargo check --lib 2>&1 | grep -q "error:"; then
    echo "❌ Compilation errors detected. Please fix them before running."
    cargo check --lib
    exit 1
fi

echo "✅ Compilation successful"
echo ""

# Check for warnings
WARNING_COUNT=$(cargo check --lib 2>&1 | grep -c "warning:" || true)
if [ "$WARNING_COUNT" -gt 0 ]; then
    echo "⚠️  $WARNING_COUNT warning(s) detected (non-critical)"
fi

echo ""
echo "✨ Setup complete! Ready to run the server."
echo ""
echo "To start the server, run:"
echo "  cargo run --bin api"
echo ""
echo "Or for production (optimized build):"
echo "  cargo build --release"
echo "  ./target/release/api"
echo ""

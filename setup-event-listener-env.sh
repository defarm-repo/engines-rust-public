#!/bin/bash
# Setup environment variables for IPCM Event Listener on Railway (Dual Network Support)

echo "🚀 Setting up IPCM Event Listener environment variables (Dual Network)..."

# Get DATABASE_URL from the API service (same PostgreSQL instance)
echo "📦 Fetching DATABASE_URL from defarm-engines-api service..."
DATABASE_URL=$(railway variables --service defarm-engines-api 2>&1 | grep "DATABASE_URL" | awk -F'│' '{print $3}' | xargs)

if [ -z "$DATABASE_URL" ]; then
    echo "❌ Failed to fetch DATABASE_URL from defarm-engines-api service"
    exit 1
fi

echo "✅ DATABASE_URL fetched successfully"

# Note: Railway CLI doesn't support 'set' command - variables must be set via web dashboard
# This script generates the configuration that should be applied

echo ""
echo "═══════════════════════════════════════════════════════════"
echo "  📋 IPCM Event Listener Environment Variables"
echo "═══════════════════════════════════════════════════════════"
echo ""
echo "Please set the following variables in Railway dashboard:"
echo ""
echo "# Shared Configuration"
echo "DATABASE_URL=$DATABASE_URL"
echo ""
echo "# Testnet Configuration (Default: Enabled)"
echo "ENABLE_TESTNET_LISTENER=true"
echo "STELLAR_TESTNET_IPCM_CONTRACT=CCDJV6VAFC2MSSDSL4AEJB5BAMGDA5PMCUIZ3UF6AYIJL467PQTBZ7BS"
echo "STELLAR_TESTNET_RPC_URL=https://soroban-testnet.stellar.org"
echo "TESTNET_POLL_INTERVAL=10"
echo "TESTNET_BATCH_SIZE=100"
echo ""
echo "# Mainnet Configuration (Default: Disabled for safety)"
echo "ENABLE_MAINNET_LISTENER=false"
echo "STELLAR_MAINNET_IPCM_CONTRACT=CBHYQKSG2ZADD7NXZPLFZIH7ZK766VA3YWRLISKJ6PH6KXJ4JZ52OLNZ"
echo "STELLAR_MAINNET_RPC_URL=https://soroban-mainnet.stellar.org"
echo "MAINNET_POLL_INTERVAL=10"
echo "MAINNET_BATCH_SIZE=100"
echo ""
echo "═══════════════════════════════════════════════════════════"
echo ""
echo "📝 Notes:"
echo "  • Both networks use the same PostgreSQL database"
echo "  • Testnet enabled by default, mainnet disabled (change when ready)"
echo "  • Contract addresses are v2.1.0 with event emission support"
echo "  • Poll interval: seconds between blockchain queries"
echo "  • Batch size: number of ledgers to fetch per query"
echo ""
echo "🌐 To enable mainnet monitoring later:"
echo "  Set ENABLE_MAINNET_LISTENER=true in Railway dashboard"
echo ""

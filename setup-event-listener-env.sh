#!/bin/bash
# Setup environment variables for IPCM Event Listener on Railway

echo "Setting up IPCM Event Listener environment variables..."

# Get DATABASE_URL from the API service (same PostgreSQL instance)
DATABASE_URL=$(railway variables --service defarm-engines-api 2>&1 | grep "DATABASE_URL" | awk -F'│' '{print $3}' | xargs)

# Set environment variables for event listener service
railway variables --service ipcm-event-listener set DATABASE_URL="$DATABASE_URL"
railway variables --service ipcm-event-listener set STELLAR_NETWORK="testnet"
railway variables --service ipcm-event-listener set SOROBAN_RPC_URL="https://soroban-testnet.stellar.org"
railway variables --service ipcm-event-listener set IPCM_CONTRACT_ADDRESS="CAALVDSF7RLM7IRGE3GQKPRHWWZSPDSNHOBEIEDJU5MAM4I4PVFWJXLS"
railway variables --service ipcm-event-listener set LISTENER_POLL_INTERVAL="10"
railway variables --service ipcm-event-listener set LISTENER_BATCH_SIZE="100"

echo "✅ Environment variables configured"
echo "DATABASE_URL: (from API service - shared PostgreSQL)"
echo "STELLAR_NETWORK: testnet"
echo "SOROBAN_RPC_URL: https://soroban-testnet.stellar.org"
echo "IPCM_CONTRACT: CAALVDSF7RLM7IRGE3GQKPRHWWZSPDSNHOBEIEDJU5MAM4I4PVFWJXLS"
echo "POLL_INTERVAL: 10s"
echo "BATCH_SIZE: 100 ledgers"

#!/bin/bash

# Start Cattle Robot - Autonomous NFT Minting Service
# This script starts the cattle robot with proper configuration

set -e

echo "🤖 Starting Cattle Robot..."
echo "================================"

# Check environment variables
if [ -z "$DATABASE_URL" ]; then
    echo "❌ ERROR: DATABASE_URL is not set"
    exit 1
fi

if [ -z "$ROBOT_API_KEY" ]; then
    echo "❌ ERROR: ROBOT_API_KEY is not set"
    exit 1
fi

# Set default values if not provided
export RAILWAY_API_URL="${RAILWAY_API_URL:-https://defarm-engines-api-production.up.railway.app}"
export ROBOT_MODE="${ROBOT_MODE:-production}"
export ROBOT_SCHEDULE="${ROBOT_SCHEDULE:-weekday-heavy}"

echo "Configuration:"
echo "  API URL: $RAILWAY_API_URL"
echo "  Mode: $ROBOT_MODE"
echo "  Schedule: $ROBOT_SCHEDULE"
echo "  Circuit ID: ${ROBOT_CIRCUIT_ID:-Will be created}"
echo ""

# Build the robot if not already built
if [ ! -f "target/release/cattle-robot" ]; then
    echo "📦 Building cattle-robot (release mode)..."
    cargo build --release --bin cattle-robot
fi

echo "🚀 Launching cattle robot..."
echo "Press Ctrl+C to stop"
echo "================================"
echo ""

# Run the robot
./target/release/cattle-robot

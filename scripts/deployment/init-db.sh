#!/bin/bash
# Database initialization script
# This script runs when PostgreSQL container is first created

set -e

echo "🗄️  Initializing DeFarm database..."

# Database is already created by POSTGRES_DB environment variable
# This script can be used for additional initialization if needed

echo "✅ Database initialization complete"

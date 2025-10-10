# Production Deployment Plan

## Overview
Transform the current in-memory storage system to production-ready PostgreSQL deployment with proper infrastructure.

## Phase 1: PostgreSQL Implementation (Largest effort - ~150 methods)

### 1.1 Add Dependencies to Cargo.toml
```toml
tokio-postgres = "0.7"
deadpool-postgres = "0.12"  # Connection pooling
refinery = "0.8"            # Database migrations
```

### 1.2 Create Database Schema (migrations/V1__initial_schema.sql)
Create PostgreSQL schema for all entities:
- receipts, data_lake, items, events, circuits, users
- adapter_configs, storage_history, webhooks
- notifications, api_keys, credit_transactions
- Indexes on frequently queried columns (dfid, user_id, circuit_id)

### 1.3 Implement PostgresStorage (src/postgres_storage.rs)
- Implement all 150 StorageBackend trait methods
- Use connection pooling with deadpool-postgres
- Proper transaction handling for multi-step operations
- Error mapping from postgres errors to StorageError

### 1.4 Migration Runner
Add migration execution to startup in src/bin/api.rs

## Phase 2: Configuration & Environment

### 2.1 Update .env file
Add:
```
DATABASE_URL=postgres://user:password@localhost/defarm
DATABASE_POOL_SIZE=10
```

### 2.2 Update src/bin/api.rs
- Initialize PostgreSQL connection pool
- Run migrations on startup
- Replace Arc<Mutex<InMemoryStorage>> with PostgresStorage

## Phase 3: Infrastructure Setup

### 3.1 Create Dockerfile
```dockerfile
FROM rust:1.75 as builder
# Build binary
FROM debian:bookworm-slim
# Install Stellar CLI
# Copy binary
# Runtime setup
```

### 3.2 Create docker-compose.yml
Services:
- postgres (with volume for persistence)
- defarm-api (with proper env vars)
- nginx (reverse proxy with SSL)

### 3.3 Create nginx configuration
- SSL termination
- Reverse proxy to API
- WebSocket support for notifications
- Rate limiting
- CORS headers

### 3.4 Database Initialization Script
- Create database
- Run migrations
- Seed initial adapter configs (from db_init.rs)

## Phase 4: Deployment Checklist

### 4.1 Server Setup
- Install Docker & Docker Compose
- Install Stellar CLI
- Configure Stellar networks (testnet + mainnet)
- Set up SSL certificates (Let's Encrypt)

### 4.2 Environment Variables (Production)
Copy from .env template, update:
- DATABASE_URL → production PostgreSQL
- JWT_SECRET → new secure random key
- PINATA_* → production credentials
- STELLAR_*_SECRET → production wallets

### 4.3 Database Setup
- Create production PostgreSQL instance
- Run migrations
- Initialize adapter configs
- Create admin user

### 4.4 Security
- Enable PostgreSQL SSL
- Firewall rules (only 443, 80, postgres port from API)
- Secret management (consider Vault or AWS Secrets Manager)
- Backup strategy for PostgreSQL

## Phase 5: Testing & Validation

### 5.1 Local Testing
- Test PostgresStorage implementation
- Verify all 150 methods work
- Run integration tests
- Test with real blockchain transactions

### 5.2 Staging Deployment
- Deploy to staging environment
- Run full test suite
- Performance testing
- Load testing

### 5.3 Production Deployment
- Blue-green deployment strategy
- Monitor logs during rollout
- Health check validation
- Rollback plan ready

## Estimated Effort

- **Phase 1 (PostgreSQL)**: 5-7 days (largest effort - 150 methods + testing)
- **Phase 2 (Config)**: 1 day
- **Phase 3 (Infrastructure)**: 2-3 days
- **Phase 4 (Deployment)**: 1-2 days
- **Phase 5 (Testing)**: 2-3 days

**Total**: 11-16 days for complete production deployment

## Quick Win Alternative: Use EncryptedFileStorage

Current codebase already has EncryptedFileStorage implemented. For faster deployment:
- Skip PostgreSQL implementation
- Use EncryptedFileStorage with shared volume
- Deploy quickly (2-3 days instead of 11-16)
- Migrate to PostgreSQL later when needed

## Implementation Status

- [ ] Phase 1: PostgreSQL Implementation
- [ ] Phase 2: Configuration & Environment
- [ ] Phase 3: Infrastructure Setup
- [ ] Phase 4: Deployment Checklist
- [ ] Phase 5: Testing & Validation

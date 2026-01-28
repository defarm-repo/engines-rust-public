# DeFarm Engines

<div align="center">

**Enterprise-grade traceability and data sharing platform with blockchain integration**

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)](https://github.com/defarm/engines)
[![API Status](https://img.shields.io/badge/API-online-success)](https://connect.defarm.net/health)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)

[Production API](https://connect.defarm.net) •
[Documentation](docs/api/COMPLETE_DEVELOPER_GUIDE.md) •
[Interactive Docs](docs/api/swagger-ui.html) •
[Contributing](CONTRIBUTING.md)

</div>

---

## 🌟 Overview

DeFarm Engines is a comprehensive traceability platform that enables organizations to create tamper-proof records of their products, processes, and supply chains. Built with Rust for maximum performance and security, it combines cryptographic verification, blockchain storage, and a powerful circuit-based sharing model.

### Key Features

- 🔐 **Dual Authentication** - JWT tokens + API keys for flexible integration
- 🎯 **Item Tokenization** - Generate globally unique DFIDs for any entity
- 🔄 **Circuit-Based Sharing** - Permission-controlled data repositories
- 📦 **Blockchain Storage** - IPFS + Stellar for immutable records
- 🌲 **Merkle State Trees** - Cryptographic verification of data integrity
- 📊 **Complete Audit Trail** - Every change tracked with events
- 🔔 **Real-Time Notifications** - WebSocket + REST API
- 🌍 **Multi-Tenant** - Workspace isolation and management

---

## 🚀 Quick Start

### Option 1: Using the CLI (Recommended for Testing)

```bash
# Install CLI
npm install -g defarm-cli

# Login
defarm login

# Create an item
defarm items create --key product --value ABC123

# List circuits
defarm circuits list

# View help
defarm --help
```

### Option 2: Using TypeScript SDK

```typescript
import { DefarmClient } from '@defarm/sdk';

const client = new DefarmClient({
  BASE: 'https://connect.defarm.net'
});

// Login
const { token } = await client.auth.login({
  username: 'your_username',
  password: 'your_password'
});

client.request.config.TOKEN = token;

// Create item and push to circuit
const item = await client.items.createLocalItem({...});
const result = await client.circuits.pushLocalItemToCircuit({...});
```

### Option 3: Using Python SDK

```python
from defarm import ApiClient, Configuration
from defarm.api import ItemsApi

config = Configuration(host="https://connect.defarm.net")
config.access_token = "your_token"

with ApiClient(config) as api_client:
    items_api = ItemsApi(api_client)
    items = items_api.list_items()
```

### Option 4: Direct API Calls

```bash
# Login
curl -X POST https://connect.defarm.net/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"demo","password":"demo123"}'

# Use token for subsequent requests
curl -H "Authorization: Bearer $TOKEN" \
  https://connect.defarm.net/api/items
```

---

## 📚 Documentation

### 🌐 Live Interactive Documentation

**Try the API right now - no installation required:**

- **[Swagger UI (Interactive)](https://connect.defarm.net/docs/api/swagger-ui.html)** - Click "Try it out" on any endpoint
- **[OpenAPI Spec](https://connect.defarm.net/docs/api/openapi.yaml)** - Import into Postman, Insomnia, etc.
- **[Advanced Concepts](https://connect.defarm.net/docs/api/ADVANCED_CONCEPTS.md)** - Architecture deep dive

💡 Use demo credentials: `hen` / `demo123` or `chick` / `Demo123!`

### For Developers

| Document | Description |
|----------|-------------|
| **[Complete Developer Guide](docs/api/COMPLETE_DEVELOPER_GUIDE.md)** | Everything you need to get started |
| **[API Reference](docs/api/API_GUIDE.md)** | Complete API documentation |
| **[Advanced Concepts](docs/api/ADVANCED_CONCEPTS.md)** | Deep dive: identifiers, deduplication, blockchain, events |
| **[Interactive Docs (Swagger UI)](https://connect.defarm.net/docs/api/swagger-ui.html)** | 🌐 Try the API in your browser |
| **[OpenAPI Spec](https://connect.defarm.net/docs/api/openapi.yaml)** | Machine-readable API contract |
| **[SDK Documentation](sdk/)** | TypeScript and Python SDKs |
| **[CLI Documentation](cli/README.md)** | Command-line tool guide |

### For Operations

| Document | Description |
|----------|-------------|
| **[Deployment Guide](docs/deployment/PRODUCTION_DEPLOYMENT.md)** | Production deployment steps |
| **[Railway Setup](docs/deployment/RAILWAY_DEPLOYMENT.md)** | Deploy to Railway.app |
| **[Swagger Deployment](docs/deployment/SWAGGER_DEPLOYMENT.md)** | Share API documentation (already live!) |
| **[Security Checklist](docs/security/SECURITY_CHECKLIST.md)** | Security best practices |

### For System Architects

| Document | Description |
|----------|-------------|
| **[System Principles](CLAUDE.md)** | Architecture and design principles |
| **[Concurrency Model](docs/adr/001-concurrency-model.md)** | Thread safety patterns |
| **[Scalability Guide](docs/guides/SCALABILITY.md)** | Scaling strategies |

---

## 🏗️ Architecture

DeFarm Engines is built on **9 specialized engines**, each handling a specific domain:

```
┌─────────────────────────────────────────────────────────┐
│                    DeFarm Engines                        │
├─────────────────────────────────────────────────────────┤
│  Reception → Storage → DFID → Verification → Items      │
│     ↓          ↓        ↓          ↓           ↓        │
│  Events → Circuits → API Keys → Merkle Tree             │
└─────────────────────────────────────────────────────────┘
```

### Core Engines

1. **Reception Engine** - Data intake with cryptographic receipts
2. **Storage Engine** - Multi-backend encrypted storage
3. **DFID Engine** - Globally unique identifier generation
4. **Verification Engine** - Deduplication and conflict resolution
5. **Items Engine** - Canonical item management
6. **Events Engine** - Complete audit trail
7. **Circuits Engine** - Permission-controlled sharing
8. **API Key Engine** - Authentication and rate limiting
9. **Merkle State Tree Engine** - Cryptographic verification

---

## 💻 Development Setup

### Prerequisites

- Rust 1.75+ ([Install](https://rustup.rs/))
- PostgreSQL 14+ ([Install](https://www.postgresql.org/download/))
- Redis 7+ (optional, for production)
- Node.js 16+ (for CLI and SDK development)

### Local Development

```bash
# Clone repository
git clone https://github.com/defarm/engines.git
cd engines

# Copy environment file
cp .env.example .env

# Edit .env with your database credentials
# DATABASE_URL=postgresql://user:pass@localhost/defarm_engines

# Build and run
cargo build
cargo run

# Run tests
cargo test

# Run with auto-reload (requires cargo-watch)
cargo install cargo-watch
cargo watch -x run
```

### Using Docker

```bash
# Build image
docker build -t defarm-engines .

# Run with docker-compose
docker-compose up
```

### Database Setup

```bash
# Create database
createdb defarm_engines

# Run migrations (automatic on first startup)
cargo run
```

---

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run integration tests
cargo test --test integration

# Run with coverage
cargo tarpaulin --out Html
```

### Testing the API

```bash
# Using the CLI
defarm login
defarm items list

# Using test scripts
./scripts/testing/test-api-keys.sh

# Using Swagger UI
open docs/api/swagger-ui.html
```

---

## 🚢 Deployment

### Deploy to Railway (Recommended)

```bash
# Install Railway CLI
npm i -g @railway/cli

# Login
railway login

# Link to project
railway link

# Deploy
railway up
```

See [Railway Deployment Guide](docs/deployment/RAILWAY_DEPLOYMENT.md) for details.

### Deploy to Production Server

See [Production Deployment Guide](docs/deployment/PRODUCTION_DEPLOYMENT.md).

---

## 📊 API Status

### Production API

- **Base URL**: https://connect.defarm.net
- **Status**: https://connect.defarm.net/health
- **Interactive Docs**: [Swagger UI](docs/api/swagger-ui.html)

### Demo Credentials

Try the API with these test accounts:

| Username | Password | Tier | Purpose |
|----------|----------|------|---------|
| hen | demo123 | Admin | Admin operations |
| chick | Demo123! | Basic | Basic tier features |
| pullet | demo123 | Professional | Professional tier |
| cock | demo123 | Enterprise | Enterprise features |

---

## 🛠️ Technology Stack

### Backend

- **Language**: Rust 1.75+
- **Web Framework**: Axum
- **Database**: PostgreSQL 14+
- **Cache**: Redis 7+ (optional)
- **Authentication**: JWT + API Keys
- **Cryptography**: BLAKE3

### Blockchain

- **Network**: Stellar (Testnet + Mainnet)
- **Storage**: IPFS
- **Smart Contracts**: Soroban (IPCM)

### Infrastructure

- **Hosting**: Railway.app
- **CI/CD**: GitHub Actions
- **Monitoring**: Built-in health checks
- **Logging**: Structured logging with tracing

---

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Development Workflow

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Write or update tests
5. Ensure all tests pass (`cargo test`)
6. Commit your changes (`git commit -m 'feat: add amazing feature'`)
7. Push to the branch (`git push origin feature/amazing-feature`)
8. Open a Pull Request

### Coding Standards

- Follow Rust naming conventions
- Write tests for new features
- Update documentation
- Use conventional commits
- Run `cargo fmt` before committing
- Ensure `cargo clippy` passes

---

## 📦 Project Structure

```
engines/
├── src/                    # Rust source code
│   ├── api/               # API endpoints
│   ├── engines/           # Core engines
│   ├── types/             # Data types
│   └── utils/             # Utilities
├── docs/                  # Documentation
│   ├── api/              # API documentation
│   ├── deployment/       # Deployment guides
│   ├── development/      # Development guides
│   ├── guides/           # How-to guides
│   └── security/         # Security documentation
├── sdk/                   # SDK generators
│   ├── typescript/       # TypeScript SDK
│   └── python/           # Python SDK
├── cli/                   # CLI tool
├── scripts/               # Utility scripts
├── tests/                 # Integration tests
└── config/                # Configuration files
```

---

## 🔒 Security

### Reporting Security Issues

Please report security vulnerabilities to: **security@defarm.net**

Do NOT create public GitHub issues for security vulnerabilities.

### Security Features

- 🔐 End-to-end encryption for sensitive data
- 🔑 Secure API key storage (BLAKE3 hashed)
- 🛡️ Rate limiting per user/API key
- 📝 Complete audit trail
- 🔒 IP restrictions for API keys
- ✅ Input validation and sanitization

See [Security Checklist](docs/security/SECURITY_CHECKLIST.md) for more details.

---

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## 🙏 Acknowledgments

- Built with [Rust](https://www.rust-lang.org/)
- Web framework: [Axum](https://github.com/tokio-rs/axum)
- Database: [PostgreSQL](https://www.postgresql.org/)
- Blockchain: [Stellar](https://www.stellar.org/)
- Storage: [IPFS](https://ipfs.io/)

---

## 📞 Support

- **Documentation**: [Complete Developer Guide](docs/api/COMPLETE_DEVELOPER_GUIDE.md)
- **API Status**: https://status.defarm.net
- **Email**: support@defarm.net
- **Issues**: [GitHub Issues](https://github.com/defarm/engines/issues)

---

## 🗺️ Roadmap

### Current Version: 1.0

- ✅ Core traceability engines
- ✅ Circuit-based sharing
- ✅ Blockchain integration (Stellar + IPFS)
- ✅ Merkle state trees
- ✅ Complete API (200+ endpoints)
- ✅ TypeScript & Python SDKs
- ✅ Professional CLI tool

### Upcoming Features

- 🔄 Merkle root blockchain anchoring
- 🔒 Advanced Zero-Knowledge Proofs
- 📱 Mobile SDKs (iOS, Android)
- 🌐 GraphQL API
- 📊 Analytics dashboard
- 🔌 Webhook system enhancements
- 🌍 Multi-region deployment

---

<div align="center">

**Built with ❤️ by the DeFarm Team**

[Website](https://defarm.net) •
[Documentation](docs/) •
[API](https://connect.defarm.net) •
[Support](mailto:support@defarm.net)

</div>

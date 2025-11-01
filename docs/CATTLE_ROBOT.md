# Cattle Robot - Autonomous NFT Minting Service

## Overview

The Cattle Robot is an autonomous service that continuously generates realistic Brazilian cattle data and mints it as NFTs on Stellar Testnet + IPFS. It simulates real-world cattle tracking operations with authentic SISBOV identifiers, lifecycle events, and privacy-preserving data handling.

## Features

### 🐄 Realistic Data Generation
- **SISBOV Numbers**: Valid Brazilian cattle traceability system identifiers (BR + 12 digits)
- **Geographic Distribution**: Focused on Mato Grosso do Sul (60%) with realistic spread across MT, SP, GO, RS
- **Cattle Breeds**: Nelore, Angus, Brahman, Senepol, Simmental, Hereford, Canchim, Caracu, Guzerá
- **Municipalities**: Real IBGE codes for 50+ Brazilian municipalities
- **Events**: Birth, weight measurements, ownership transfers, vaccinations, movements

### 🔐 Privacy-First Architecture
- **Hashed Identifiers**: All owner names, CPF/CNPJ, veterinarian names hashed with BLAKE3
- **Format**: `hash:owner:blake3(name)`, `hash:vet:blake3(name)`, `hash:cpf:blake3(number)`
- **Irreversible**: Original data never stored, only cryptographic hashes
- **Realistic but Anonymous**: Data feels authentic without compromising privacy

### ⏰ Intelligent Scheduling
- **Weekdays**: 2-5 operations/hour (higher during business hours 9am-5pm)
- **Weekends**: ~1 operation/hour
- **Random Jitter**: ±20% variance to avoid detection patterns
- **Poisson Distribution**: Natural temporal spacing of events

### 🔄 Operation Types
- **70% New Mints**: Generate new cattle with birth events
- **30% Updates**: Add events to existing cattle (weight, transfer, vaccination, movement)

### 🌐 Blockchain Integration
- **Stellar Testnet**: NFT minting and IPCM event registration
- **IPFS**: Permanent storage via Pinata
- **Circuit-Based**: Tokenization through DeFarm circuit system
- **Event-Only Mode**: Cost-effective blockchain evidence (90% cheaper)

## Architecture

```
┌─────────────────┐
│  Cattle Robot   │
│   (Daemon)      │
└────────┬────────┘
         │
         ├─► Scheduler (Poisson intervals)
         │
         ├─► Data Generator (SISBOV, events, hashes)
         │
         ├─► API Client (Railway API)
         │   ├─► POST /api/items/local
         │   └─► POST /api/circuits/{id}/push-local
         │
         ├─► PostgreSQL (robot_cattle, robot_events, robot_mints)
         │
         └─► Stellar Testnet + IPFS
             ├─► NFT Contract (mint_nft)
             ├─► IPCM Contract (emit_update_event)
             └─► Pinata (JSON upload)
```

## Installation

### Prerequisites
- Rust 1.70+
- PostgreSQL 12+
- Railway API access
- API key with write permissions

### Build
```bash
cargo build --release --bin cattle-robot
```

### Database Setup
```bash
# Run migration to create robot tables
sqlx migrate run
```

The migration creates:
- `robot_cattle`: Cattle records with SISBOV and hashed owners
- `robot_events`: Lifecycle events (birth, vaccination, transfer, etc.)
- `robot_mints`: Tracking of DFID assignment and blockchain transactions

## Configuration

### Environment Variables

```bash
# Required
DATABASE_URL=postgresql://user:pass@host:5432/database
ROBOT_API_KEY=dfm_your_api_key_here

# Optional
RAILWAY_API_URL=https://defarm-engines-api-production.up.railway.app
ROBOT_CIRCUIT_ID=your-circuit-id  # If not set, will create new circuit
ROBOT_MODE=production              # or dry-run
ROBOT_SCHEDULE=weekday-heavy       # or uniform
```

### Configuration Options

**ROBOT_MODE**:
- `production`: Actually performs minting operations
- `dry-run`: Simulates operations without blockchain interaction

**ROBOT_SCHEDULE**:
- `weekday-heavy`: 2-5 ops/hour weekdays, ~1/hour weekends (default)
- `uniform`: Constant rate regardless of day/time

## Usage

### Start Robot

```bash
# Using script
./scripts/start_cattle_robot.sh

# Direct execution
./target/release/cattle-robot
```

### Stop Robot

Press `Ctrl+C` or send `SIGTERM`:
```bash
pkill cattle-robot
```

The robot will gracefully shutdown, closing database connections and logging final statistics.

### Monitoring

The robot logs all operations:
```
🤖 Cattle Robot Starting...
✓ Database connected
✓ API is healthy
✓ Circuit created: 002ea6db-6b7b-4a69-8780-1f01ae074265
✓ Adapter configured
🚀 Robot is now running
----------------------------------------
⏰ 2025-01-20 14:23:45 UTC - Weekday Business hours (Mon)
🎲 Selected operation: NewMint
✅ MINT SUCCESS: SISBOV=BR547891234567, DFID=DFID-20250120-000001-A7B3, CID=QmX...
📊 Stats: Mints=1, Updates=0, Errors=0, Uptime=45s
⏳ Next operation in 12m 34s
----------------------------------------
```

## Database Schema

### robot_cattle
```sql
id UUID PRIMARY KEY
sisbov VARCHAR(14) UNIQUE          -- BR + 12 digits
birth_date DATE
breed VARCHAR(50)                   -- Nelore, Angus, etc.
gender VARCHAR(10)                  -- Male, Female
state VARCHAR(2)                    -- MS, MT, SP, GO, RS
municipality_code VARCHAR(7)        -- IBGE code
owner_hash VARCHAR(100)             -- hash:owner:blake3(name)
status VARCHAR(20)                  -- active, sold, deceased
created_at TIMESTAMPTZ
updated_at TIMESTAMPTZ
```

### robot_events
```sql
id UUID PRIMARY KEY
cattle_id UUID REFERENCES robot_cattle(id)
event_type VARCHAR(50)              -- birth, vaccination, transfer, weight, movement
event_date DATE
from_owner_hash VARCHAR(100)        -- For transfers
to_owner_hash VARCHAR(100)          -- For transfers
vet_hash VARCHAR(100)               -- For vaccinations
metadata JSONB                      -- Event-specific data
dfid VARCHAR(50)                    -- Set after push
local_id UUID                       -- Set after local creation
created_at TIMESTAMPTZ
```

### robot_mints
```sql
id UUID PRIMARY KEY
cattle_id UUID REFERENCES robot_cattle(id)
dfid VARCHAR(50)
local_id UUID
cid VARCHAR(100)                    -- IPFS CID
stellar_tx VARCHAR(100)             -- Stellar transaction hash
operation_id UUID                   -- Circuit operation ID
created_at TIMESTAMPTZ
```

## API Flow

### 1. New Mint Operation

```
1. Generate cattle data (SISBOV, breed, birth date, owner hash)
2. POST /api/items/local
   Body: {
     "enhanced_identifiers": [
       { "namespace": "bovino", "key": "sisbov", "value": "BR547891234567", "id_type": "Canonical" },
       { "namespace": "bovino", "key": "owner", "value": "hash:owner:a7b3...", "id_type": "Contextual" }
     ],
     "enriched_data": { "breed": "Nelore", "birth_date": "2024-01-15", ... }
   }
   Response: { "local_id": "uuid" }

3. POST /api/circuits/{circuit_id}/push-local
   Body: { "local_id": "uuid", "requester_id": "robot-system" }
   Response: {
     "dfid": "DFID-20250120-000001-A7B3",
     "operation_id": "uuid",
     "storage_metadata": { "cid": "QmX...", "stellar_tx": "abc..." }
   }

4. Store in database (robot_cattle, robot_events, robot_mints)
```

### 2. Update Operation

```
1. Select random active cattle from database
2. Generate event (weight, transfer, vaccination, movement)
3. POST /api/items/local (with event data)
4. POST /api/circuits/{circuit_id}/push-local
5. Store event in robot_events
6. Update cattle owner if transfer event
```

## Data Examples

### SISBOV Numbers (Sample)
```
BR547891234567
BR823456789012
BR914567890123
BR756789012345
BR634890123456
```

### Owner Hashes (Sample)
```
hash:owner:af1349fb3d8a4e0a1b5c6d7e8f90a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8
hash:owner:2b4c6d8e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7
hash:farm:e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f
hash:company:5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d
```

### Veterinarian Hashes (Sample)
```
hash:vet:9a0b1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0
hash:vet:4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c
```

### Event Metadata Examples

**Birth Event**:
```json
{
  "birth_weight_kg": 32,
  "mother_sisbov": "BR456789012345",
  "location": "Campo Grande"
}
```

**Weight Event**:
```json
{
  "weight_kg": 385,
  "age_days": 547
}
```

**Transfer Event**:
```json
{
  "transfer_type": "sale",
  "new_owner_type": "company"
}
```

**Vaccination Event**:
```json
{
  "vaccine_type": "FMD",
  "batch": "VAC5847"
}
```

**Movement Event**:
```json
{
  "from_location": "Campo Grande",
  "to_location": "Dourados",
  "reason": "pasture_rotation"
}
```

## Statistics

### Expected Operations (24h)
- **Weekdays**: 48-120 operations (2-5/hour)
- **Weekends**: 24-36 operations (~1/hour)
- **Mix**: ~70% new mints, ~30% updates

### Geographic Distribution
- **MS (Mato Grosso do Sul)**: 60%
- **MT (Mato Grosso)**: 15%
- **SP (São Paulo)**: 10%
- **GO (Goiás)**: 10%
- **RS (Rio Grande do Sul)**: 5%

### Breed Distribution
- **Nelore**: 50%
- **Angus**: 15%
- **Brahman**: 12%
- **Others**: 23%

## Troubleshooting

### Common Issues

**1. Database Connection Failed**
```
Error: connection to server failed
```
Solution: Check `DATABASE_URL` and ensure PostgreSQL is running

**2. API Key Invalid**
```
Error: Authentication failed: 401 Unauthorized
```
Solution: Verify `ROBOT_API_KEY` is valid and has write permissions

**3. No Circuit Configured**
```
Error: Circuit not configured
```
Solution: Either set `ROBOT_CIRCUIT_ID` or allow robot to create new circuit

**4. Rate Limited**
```
Error: Rate limit exceeded
```
Solution: API key hit rate limits. Wait or upgrade tier.

**5. No Cattle Available**
```
Warning: No cattle available for update, skipping...
```
Solution: Normal on startup. Robot will mint new cattle first.

### Logs

Logs are written to stdout with timestamps:
```bash
# View logs
tail -f robot.log

# Search for errors
grep "ERROR" robot.log

# Count operations
grep "MINT SUCCESS" robot.log | wc -l
grep "UPDATE SUCCESS" robot.log | wc -l
```

## Deployment

### Railway Deployment

1. Add Cattle Robot service to Railway
2. Configure environment variables
3. Deploy from GitHub (automatic build)
4. Start service

### Docker (Future)

```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin cattle-robot

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libpq5 ca-certificates
COPY --from=builder /app/target/release/cattle-robot /usr/local/bin/
CMD ["cattle-robot"]
```

## Security

### Privacy Guarantees
- **No Personal Data**: All identifiable information hashed before storage
- **Irreversible Hashing**: BLAKE3 produces one-way cryptographic hashes
- **Separation**: Robot data isolated in separate tables from production

### Access Control
- **API Key Required**: Robot needs valid API key with write permissions
- **Circuit Permissions**: Respects circuit access control and adapter requirements
- **Rate Limiting**: Subject to API key rate limits

## Performance

### Resource Usage
- **CPU**: Low (~1-2% average)
- **Memory**: ~50-100MB
- **Database**: Minimal (few queries per operation)
- **Network**: Intermittent bursts during operations

### Scalability
- Single instance handles 50-150 operations/day comfortably
- Can scale horizontally with multiple circuits
- Database can support millions of cattle records

## Future Enhancements

- [ ] Web dashboard for monitoring
- [ ] Admin API for pause/resume/stats
- [ ] Configurable event type weights
- [ ] Support for multiple value chains (aves, suino, soja)
- [ ] Historical data seeding (bulk create cattle from 2020-2024)
- [ ] Circuit rotation (distribute across multiple circuits)
- [ ] Prometheus metrics export
- [ ] Grafana dashboard templates

## License

Same as parent project (DeFarm Engines)

## Support

For issues or questions:
1. Check logs for error messages
2. Review this documentation
3. Open GitHub issue with logs and configuration

# DFID Service

Layer 1 of the DeFarm architecture - Universal ID generation service.

## Overview

DFID Service is a stateless (except for sequence counter) microservice that generates and validates DeFarm IDs (DFIDs). It provides cryptographically secure, globally unique identifiers using BLAKE3 checksums.

## Features

- **DFID Generation**: Single or batch generation with optional context
- **BLAKE3 Checksums**: 24-bit cryptographic checksums (upgrade from 16-bit polynomial)
- **Redis Persistence**: Optional sequence counter persistence across restarts
- **Stateless Design**: Horizontal scaling with shared Redis backend
- **REST API**: Simple HTTP interface for integration

## API Endpoints

### POST /dfid/generate

Generate one or more DFIDs.

**Request:**
```json
{
  "context": "bovino",  // optional
  "count": 1            // default: 1, max: 1000
}
```

**Response:**
```json
{
  "dfids": ["DFID-20250131-000001-A7B2C3"],
  "format_version": "1.0",
  "generated_at": "2025-01-31T12:00:00Z"
}
```

### POST /dfid/batch

Generate large batches of DFIDs (up to 10,000).

**Request:**
```json
{
  "count": 100,
  "context": "soja"
}
```

**Response:**
```json
{
  "dfids": ["DFID-...", "DFID-...", ...],
  "format_version": "1.0",
  "generated_at": "2025-01-31T12:00:00Z"
}
```

### GET /dfid/:id/validate

Validate DFID format and extract metadata.

**Response:**
```json
{
  "valid": true,
  "checksum_ok": true,
  "metadata": {
    "year": 2025,
    "month": 1,
    "day": 31,
    "sequence": 42,
    "full_dfid": "DFID-20250131-000042-A7B2C3"
  }
}
```

### GET /health

Health check endpoint.

**Response:**
```json
{
  "status": "healthy",
  "current_sequence": 1234567,
  "uptime_seconds": 86400
}
```

## DFID Format

```
DFID-{YYYYMMDD}-{SEQUENCE}-{CHECKSUM}
```

- **YYYYMMDD**: Date of generation (8 digits)
- **SEQUENCE**: 6-digit sequence number (000001-999999)
- **CHECKSUM**: 6-character BLAKE3 hash (24-bit, hex)

Example: `DFID-20250131-000042-A7B2C3`

## Environment Variables

```bash
PORT=3001                           # HTTP port (default: 3001)
REDIS_URL=redis://localhost:6379    # Optional Redis for persistence
RUST_LOG=info                       # Log level
```

## Running Locally

### Without Redis
```bash
cargo run
```

### With Redis
```bash
docker run -d -p 6379:6379 redis:7-alpine
REDIS_URL=redis://localhost:6379 cargo run
```

## Docker

```bash
# Build
docker build -t dfid-service .

# Run
docker run -p 3001:3001 -e REDIS_URL=redis://redis:6379 dfid-service
```

## Testing

```bash
cargo test

# Generate DFID
curl -X POST http://localhost:3001/dfid/generate \
  -H "Content-Type: application/json" \
  -d '{"context": "bovino"}'

# Validate DFID
curl http://localhost:3001/dfid/DFID-20250131-000001-A7B2C3/validate

# Health check
curl http://localhost:3001/health
```

## Architecture

- **Stateless**: Only atomic sequence counter in memory
- **Redis Optional**: Sequence persisted every 10 seconds
- **Horizontal Scaling**: Multiple instances share Redis sequence
- **BLAKE3**: Fast, cryptographically secure checksums

## Performance

- **Throughput**: >10,000 DFIDs/second per instance
- **Latency**: <5ms per request (in-memory)
- **Batch Generation**: Linear O(n) with minimal overhead

## Upgrading from Legacy

The DFID Service maintains backward compatibility with existing DFIDs while providing:

1. **Stronger Checksums**: BLAKE3 24-bit vs polynomial 16-bit
2. **Persistence**: Redis-backed sequence counter
3. **Scalability**: Horizontal scaling with shared state
4. **API-First**: HTTP interface instead of embedded library

## License

Part of the DeFarm ecosystem.

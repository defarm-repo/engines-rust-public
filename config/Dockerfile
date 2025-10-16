# Multi-stage Dockerfile for DeFarm Engines Production Deployment

# ============================================================================
# Stage 1: Builder
# ============================================================================
# Use explicit Rust 1.90 (Stellar CLI 23.1.4 requires rustc 1.89.0+)
FROM rust:1.90-bookworm as builder

WORKDIR /app

# Install system dependencies including libsodium for encryption
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    libsodium-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Stellar SDK integration - no CLI needed (native Rust)

# Copy dependency manifests
COPY Cargo.toml Cargo.lock ./

# Create dummy main to cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    mkdir -p src/bin && \
    echo "fn main() {}" > src/bin/api.rs

# Build dependencies (this layer will be cached)
RUN cargo build --release --bin defarm-api && \
    rm -rf src

# Copy actual source code
COPY . .

# Build the actual application
RUN cargo build --release --bin defarm-api

# ============================================================================
# Stage 2: Runtime
# ============================================================================
FROM debian:bookworm-slim

# Install runtime dependencies including libsodium for encryption
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libsodium23 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -m -u 1000 defarm

WORKDIR /app

# Copy the built binary (Stellar SDK compiled in - no CLI needed)
COPY --from=builder /app/target/release/defarm-api /app/defarm-api

# Copy migrations (needed for database setup)
COPY migrations /app/migrations

# Copy API documentation (served via /docs endpoint)
COPY docs /app/docs

# Set ownership
RUN chown -R defarm:defarm /app

# Switch to app user
USER defarm

# Stellar SDK uses environment variables for configuration (no CLI setup needed)

# Expose API port (Railway provides dynamic PORT via environment variable)
# Note: Railway sets PORT dynamically, this is just documentation
EXPOSE 8080

# Railway handles health checks via HTTP (see railway.json)
# Docker HEALTHCHECK removed to avoid conflicts with Railway's healthcheck system
# Railway will query /health endpoint directly using its own mechanism

# Run the API
CMD ["/app/defarm-api"]

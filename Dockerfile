# Multi-stage Dockerfile for DeFarm Engines Production Deployment

# ============================================================================
# Stage 1: Builder
# ============================================================================
# Use explicit Rust 1.90 (Stellar CLI 23.1.4 requires rustc 1.89.0+)
FROM rust:1.90-bookworm as builder

WORKDIR /app

# Install system dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install Stellar CLI (pre-built binary - much faster than cargo install)
RUN curl -L https://github.com/stellar/stellar-cli/releases/download/v23.1.4/stellar-cli-23.1.4-x86_64-unknown-linux-gnu.tar.gz \
    | tar -xz -C /usr/local/bin && \
    chmod +x /usr/local/bin/stellar

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

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -m -u 1000 defarm

WORKDIR /app

# Copy Stellar CLI from builder
COPY --from=builder /usr/local/bin/stellar /usr/local/bin/stellar

# Copy the built binary
COPY --from=builder /app/target/release/defarm-api /app/defarm-api

# Copy migrations (needed for database setup)
COPY migrations /app/migrations

# Set ownership
RUN chown -R defarm:defarm /app

# Switch to app user
USER defarm

# Configure Stellar networks at runtime (done in entrypoint.sh)
# This will be handled by docker-compose environment variables

# Expose API port (Railway provides dynamic PORT via environment variable)
EXPOSE 3000

# Health check - use PORT environment variable if set, fallback to 3000
HEALTHCHECK --interval=30s --timeout=3s --start-period=40s --retries=3 \
    CMD curl -f http://localhost:${PORT:-3000}/health || exit 1

# Run the API
CMD ["/app/defarm-api"]

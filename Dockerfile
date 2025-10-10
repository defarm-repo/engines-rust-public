# Multi-stage Dockerfile for DeFarm Engines Production Deployment

# ============================================================================
# Stage 1: Builder
# ============================================================================
FROM rust:1.75-bookworm as builder

WORKDIR /app

# Install Stellar CLI (required for blockchain integration)
RUN cargo install --locked stellar-cli

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
COPY --from=builder /usr/local/cargo/bin/stellar /usr/local/bin/stellar

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

# Expose API port
EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=40s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

# Run the API
CMD ["/app/defarm-api"]

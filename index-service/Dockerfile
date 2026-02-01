# ============================================================================
# Build Stage
# ============================================================================
FROM rust:1.88-bookworm AS builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Copy dependency files
COPY Cargo.toml Cargo.lock ./

# Create dummy source to build dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs

# Build dependencies (will be cached)
RUN cargo build --release && \
    rm -rf src

# Copy actual source code
COPY . .

# Build the application
RUN cargo build --release --bin index-service

# ============================================================================
# Runtime Stage
# ============================================================================
FROM debian:bookworm-slim

# Create app user
RUN useradd -m -u 1000 indexuser

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/index-service /usr/local/bin/index-service

# Copy migrations
COPY --from=builder /app/migrations ./migrations

# Set ownership
RUN chown -R indexuser:indexuser /app

# Switch to non-root user
USER indexuser

# Expose port
EXPOSE 3002

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3002/health || exit 1

# Run the application
CMD ["index-service"]

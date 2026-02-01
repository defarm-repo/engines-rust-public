FROM rust:1.83 as builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml ./

# Create dummy source to cache dependencies
RUN mkdir -p src/api && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy actual source code
COPY src ./src

# Build for release
RUN touch src/main.rs && \
    cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/dfid-service /usr/local/bin/dfid-service

EXPOSE 3001

ENV PORT=3001
ENV RUST_LOG=info

CMD ["dfid-service"]

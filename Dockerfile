# Multi-stage build for Gradience API
FROM rust:1.80-slim-bookworm AS builder

WORKDIR /app

# Install dependencies for sqlx and openssl
RUN apt-get update && apt-get install -y pkg-config libssl-dev

# Copy workspace manifests first for better caching
COPY Cargo.toml Cargo.lock ./
COPY crates/gradience-core/Cargo.toml crates/gradience-core/
COPY crates/gradience-db/Cargo.toml crates/gradience-db/
COPY crates/gradience-api/Cargo.toml crates/gradience-api/
COPY crates/gradience-cli/Cargo.toml crates/gradience-cli/
COPY crates/gradience-mcp/Cargo.toml crates/gradience-mcp/
COPY crates/gradience-sdk-node/Cargo.toml crates/gradience-sdk-node/

# Fetch dependencies
RUN mkdir -p crates/gradience-core/src crates/gradience-db/src crates/gradience-api/src \
    crates/gradience-cli/src crates/gradience-mcp/src crates/gradience-sdk-node/src \
    && echo 'fn main() {}' > crates/gradience-api/src/main.rs \
    && echo 'fn main() {}' > crates/gradience-cli/src/main.rs \
    && echo 'fn main() {}' > crates/gradience-mcp/src/main.rs \
    && touch crates/gradience-core/src/lib.rs \
    && touch crates/gradience-db/src/lib.rs \
    && touch crates/gradience-sdk-node/src/lib.rs \
    && cargo fetch

# Copy full source and build
COPY . .
RUN cargo build --release -p gradience-api

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/gradience-api /usr/local/bin/gradience-api

ENV DATABASE_URL="sqlite:/app/data/gradience.db?mode=rwc"
ENV GRADIENCE_DATA_DIR="/app/data"
ENV ORIGIN="https://wallet.example.com"
ENV RP_ID="wallet.example.com"

EXPOSE 8080

VOLUME ["/app/data"]

ENTRYPOINT ["gradience-api"]

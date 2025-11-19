# Multi-stage Dockerfile for Gold/USD Trading Bot
# Produces a minimal production image (~50MB)

# Stage 1: Builder
FROM rust:1.75-slim as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY migrations ./migrations

# Build release binary
RUN cargo build --release --bin trade

# Stage 2: Runtime
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/trade /usr/local/bin/trading-bot

# Copy migrations
COPY migrations /app/migrations

# Create non-root user
RUN useradd -m -u 1000 trader && \
    chown -R trader:trader /app

USER trader

# Health check
HEALTHCHECK --interval=60s --timeout=10s --start-period=30s --retries=3 \
    CMD pgrep -x trading-bot || exit 1

# Default command
CMD ["trading-bot"]

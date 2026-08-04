# ------------------------------------------------------------------------------
# Stage 1: Build static binary with Rust & Musl
# ------------------------------------------------------------------------------
FROM rust:1.94-alpine AS builder

RUN apk add --no-cache musl-dev gcc pkgconfig

WORKDIR /app

# Copy Cargo definitions
COPY Cargo.toml Cargo.lock ./

# Copy source files
COPY src/ ./src/

# Compile static release binary
RUN cargo build --release

# ------------------------------------------------------------------------------
# Stage 2: Ultra-lightweight Minimal Runtime Container (< 25MB total size)
# ------------------------------------------------------------------------------
FROM alpine:latest

RUN apk add --no-cache ca-certificates tzdata

WORKDIR /app

# Copy compiled binary from builder stage
COPY --from=builder /app/target/release/vexta-bridge-v2 /app/vexta-bridge-v2

# Persistent volume for SQLite WAL database
VOLUME ["/app/data"]

ENV RUST_LOG=info \
    DATABASE_PATH=/app/data/vexta_bridge_v2.db

EXPOSE 8000

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
  CMD wget -q -O /dev/null http://localhost:8000/api/announcements/ || exit 1

ENTRYPOINT ["/app/vexta-bridge-v2"]

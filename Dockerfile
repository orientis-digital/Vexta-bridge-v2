# ------------------------------------------------------------------------------
# Stage 1: Build React Admin UI
# ------------------------------------------------------------------------------
FROM node:20-alpine AS ui-builder

WORKDIR /app/admin-ui

COPY admin-ui/package*.json ./
RUN npm install --legacy-peer-deps

COPY admin-ui/ ./
RUN npm run build

# ------------------------------------------------------------------------------
# Stage 2: Build static binary with Rust & Musl
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
# Stage 3: Ultra-lightweight Minimal Runtime Container
# ------------------------------------------------------------------------------
FROM alpine:latest

RUN apk add --no-cache ca-certificates tzdata \
    && addgroup -g 10001 -S vexta \
    && adduser -u 10001 -S vexta -G vexta

WORKDIR /app

# Copy compiled Rust binary
COPY --from=builder /app/target/release/vexta-bridge-v2 /app/vexta-bridge-v2

# Copy compiled React Admin UI static bundle
COPY --from=ui-builder /app/admin-ui/dist /app/admin-ui/dist

# Create data directory and set permissions for non-root user
RUN mkdir -p /app/data && chown -R vexta:vexta /app

USER vexta:vexta

# Persistent volume for SQLite WAL database
VOLUME ["/app/data"]

ENV RUST_LOG=info \
    DATABASE_PATH=/app/data/vexta_bridge_v2.db

EXPOSE 8000

# Health check
HEALTHCHECK --interval=10s --timeout=3s --start-period=5s --retries=3 \
  CMD wget --no-verbose --tries=1 --spider http://127.0.0.1:8000/health || exit 1

ENTRYPOINT ["/app/vexta-bridge-v2"]

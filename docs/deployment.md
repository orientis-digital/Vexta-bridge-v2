# Vexta Bridge V2 Production Deployment Guide

This document explains how to compile, deploy, and run **Vexta Bridge V2** in a production environment.

---

## 1. Static Binary Release Compilation

To build a standalone, ultra-optimized static binary:

```bash
cd vexta-bridge-v2
cargo build --release
```

The output binary is placed at `target/release/vexta-bridge-v2` (~10 MB).

---

## 2. Docker Deployment

Create a `Dockerfile` in the root of `vexta-bridge-v2`:

```dockerfile
FROM rust:1.94-alpine as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM alpine:latest
WORKDIR /app
COPY --from=builder /app/target/release/vexta-bridge-v2 /app/vexta-bridge-v2
EXPOSE 8000
CMD ["/app/vexta-bridge-v2"]
```

Build and run container:
```bash
docker build -t vexta-bridge-v2 .
docker run -d -p 8000:8000 --name vexta-bridge vexta-bridge-v2
```

---

## 3. NGINX Reverse Proxy Configuration (WSS Support)

Sample NGINX configuration with SSL termination and WebSocket upgrade support:

```nginx
server {
    listen 443 ssl http2;
    server_name vexta-api.nexusec.space;

    ssl_certificate /etc/letsencrypt/live/vexta-api.nexusec.space/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/vexta-api.nexusec.space/privkey.pem;

    location /ws/ {
        proxy_pass http://127.0.0.1:8000;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "Upgrade";
        proxy_set_header Host $host;
        proxy_read_timeout 86400s;
        proxy_send_timeout 86400s;
    }

    location /api/ {
        proxy_pass http://127.0.0.1:8000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

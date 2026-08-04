# Vexta Bridge V2 — Cloudflare Tunnel (`cloudflared`) Setup Guide

This document details how **`vexta-bridge-v2`** is configured to run securely behind a **Cloudflare Tunnel** (`cloudflared`) at **`vexta-api.nexusec.space`**.

---

## 1. Cloudflare Tunnel Architecture

```
User (Tauri / Web)
   │
   │  HTTPS / WSS (Port 443)
   ▼
Cloudflare Edge WAF (vexta-api.nexusec.space)
   │
   │  Encrypted Cloudflare Tunnel (cloudflared container / daemon)
   ▼
vexta-bridge-v2 (Rust Server listening on 127.0.0.1:8000)
```

---

## 2. Docker Compose Deployment (Recommended)

`docker-compose.yml` includes the official `cloudflare/cloudflared:latest` container service:

```yaml
services:
  vexta-bridge-v2:
    build:
      context: .
      dockerfile: Dockerfile
    container_name: vexta-bridge-v2
    restart: unless-stopped
    ports:
      - "8000:8000"

  cloudflared:
    image: cloudflare/cloudflared:latest
    container_name: vexta-cloudflared
    restart: unless-stopped
    command: tunnel --no-autoupdate run
    environment:
      - TUNNEL_TOKEN=${TUNNEL_TOKEN}
    depends_on:
      vexta-bridge-v2:
        condition: service_healthy
```

### Steps:
1. Open Cloudflare Zero Trust Dashboard -> Networks -> Tunnels.
2. Create a tunnel for `vexta-api.nexusec.space` pointing to `http://vexta-bridge-v2:8000`.
3. Copy your `TUNNEL_TOKEN` into `.env`:
   ```bash
   TUNNEL_TOKEN=eyJhIjoiY2I...
   ```
4. Start both containers:
   ```bash
   docker compose up -d
   ```

---

## 3. Host System Daemon Alternative (`config.yml`)

If running `cloudflared` directly on the host system:

`/etc/cloudflared/config.yml`:
```yaml
tunnel: <YOUR-TUNNEL-UUID>
credentials-file: /etc/cloudflared/<YOUR-TUNNEL-UUID>.json

ingress:
  - hostname: vexta-api.nexusec.space
    service: http://127.0.0.1:8000
  - service: http_status:404
```

Start system service:
```bash
sudo systemctl start cloudflared
```

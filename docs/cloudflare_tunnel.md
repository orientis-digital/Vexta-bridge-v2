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
   │  Encrypted Cloudflare Tunnel (cloudflared daemon)
   ▼
vexta-bridge-v2 (Rust Server listening on 127.0.0.1:8000)
```

---

## 2. Cloudflare Tunnel Configuration (`config.yml`)

Create or update `/etc/cloudflared/config.yml` on your server:

```yaml
tunnel: <YOUR-TUNNEL-UUID>
credentials-file: /etc/cloudflared/<YOUR-TUNNEL-UUID>.json

ingress:
  # Route WebSocket & REST API to vexta-bridge-v2
  - hostname: vexta-api.nexusec.space
    service: http://127.0.0.1:8000
    originRequest:
      noTLSVerify: true
      connectTimeout: 30s

  # Catch-all 404 rule
  - service: http_status:404
```

---

## 3. Rust Server Proxy Header Support

`vexta-bridge-v2` natively parses Cloudflare Tunnel headers:
* **`CF-Connecting-IP`**: Real client IP address passed through Cloudflare.
* **`X-Forwarded-For`**: Standard proxy chain forwarding header.
* **`X-Forwarded-Proto`**: Set to `https` / `wss`.

---

## 4. Running `cloudflared` Service

To start the tunnel daemon as a system service:

```bash
sudo cloudflared service install
sudo systemctl start cloudflared
sudo systemctl enable cloudflared
```

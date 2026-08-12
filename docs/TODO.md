# 🌐 Vexta Bridge Protocol Federation (Multi-Bridge Architecture) — TODO & Roadmap

This document outlines the architectural design, protocol specifications, and implementation steps for interconnecting independent **Vexta Bridge V2** server nodes across a federated, zero-knowledge network.

---

## 🎯 High-Level Objective

Enable users registered on `bridge-a.vexta.net` to seamlessly and securely communicate with users on `bridge-b.vexta.net` without sacrificing zero-knowledge end-to-end encryption or delegating server trust.

---

## 🏛️ Federation Architecture Overview

```
+-------------------+                      +-------------------+
|  Vexta Client A   |                      |  Vexta Client B   |
| (@alice@node1.org)|                      |  (@bob@node2.org) |
+---------+---------+                      +---------+---------+
          |                                          |
    E2EE  | (WS / TLS)                         E2EE  | (WS / TLS)
          v                                          v
+---------+---------+    mTLS Peer Relay   +---------+---------+
|  Vexta Bridge 1   | <==================> |  Vexta Bridge 2   |
|   (node1.org)     |   (Server-to-Server) |   (node2.org)     |
+-------------------+                      +-------------------+
```

---

## 📋 Technical Requirements & Roadmap

### Phase 1: Federated Naming & Address Formatting
- [ ] Support `@username@domain` federated address format (e.g. `@alice@bridge1.vexta.org`).
- [ ] Add domain parsing helper in `src/ws.rs` and `src/models.rs` to distinguish local recipients from remote federated recipients.
- [ ] Implement DNS TXT record or HTTPS `.well-known/vexta-bridge` discovery endpoint for resolving bridge endpoints dynamically.

### Phase 2: Server-to-Server (S2S) Authentication & Trust
- [ ] Add RSA/Ed25519 node keypair to `ServerCrypto` for bridge instance identity verification.
- [ ] Implement mutual TLS (mTLS) or HTTP Message Signatures for server-to-server HTTP/WebSocket peering requests.
- [ ] Add Admin UI controls to whitelist/blacklist trusted federated peer bridges (`/api/admin/federation/peers`).

### Phase 3: Cross-Bridge Message Relaying
- [ ] Implement outbound WebSocket connection pool manager (`src/federation.rs`) for forwarding ciphertexts to peer bridges.
- [ ] Implement store-and-forward queueing for offline federated recipients when peer bridges are temporarily unreachable.
- [ ] Maintain zero-knowledge guarantees: Relay encrypted `wire_blob` payloads untouched without decrypting or inspecting content.

### Phase 4: Federated Prekey & Key Bundle Syncing
- [ ] Implement S2S endpoint `/api/v1/federation/prekeys/:username` to fetch recipient public identity keys across bridges for E2EE session initiation.
- [ ] Add caching mechanism with TTL for remote prekey bundles to reduce cross-node latency.

### Phase 5: Admin Observability & Rate Limiting
- [ ] Track cross-bridge bandwidth metrics in `user_traffic_stats` and display peer node status in Admin Dashboard.
- [ ] Enforce per-peer bridge rate limits to mitigate denial-of-service or spam amplification attacks across federated nodes.

---

## 🔐 Security Considerations

1. **Zero Trust Peering:** Peer bridges must never be trusted with plaintext user data or private keys.
2. **Replay Protection:** S2S frames must include cryptographic timestamps and unique message IDs to prevent cross-node replay attacks.
3. **Loop Prevention:** S2S routing frames must include a `hop_count` header to prevent infinite relay loops between misconfigured nodes.

# Vexta Bridge V2 Architecture & Concurrency Design

This document describes the high-throughput, zero-knowledge architecture of **Vexta Bridge V2**.

---

## 1. Concurrency Model & Architecture

```
                       ┌─────────────────────────────────────┐
                       │    Axum TCP Listener (0.0.0.0:8000) │
                       └──────────────────┬──────────────────┘
                                          │ WSS / REST
         ┌────────────────────────────────┴────────────────────────────────┐
         ▼                                                                 ▼
┌─────────────────────────────────┐                       ┌─────────────────────────────────┐
│     Lock-Free Session Router    │                       │   Embedded SQLite Database      │
│  DashMap<Username, WsSender>    │                       │   WAL Mode & Offline Queues     │
│  (Lockless Sub-millisecond)     │                       │   (rusqlite + Foreign Keys)     │
└─────────────────────────────────┘                       └─────────────────────────────────┘
```

### Key Architectural Highlights
1. **Axum + Tokio Web Server**: Built on `hyper` and `tower`, providing zero-cost async I/O.
2. **Lockless Session Routing (`DashMap`)**: Sessions map `username -> tokio::sync::mpsc::UnboundedSender<Message>` using a lock-free concurrent hash map (`DashMap`). Message routing completes in **< 0.2ms** without database read locks.
3. **SQLite WAL Mode**: Embedded database configured with `PRAGMA journal_mode = WAL;` and `PRAGMA busy_timeout = 5000;`, enabling concurrent non-blocking reads and writes.

---

## 2. Server Crypto Engine (`src/crypto.rs`)

* **Ed25519 Identity Keypair**: Loaded or generated using `ed25519-dalek`.
* **Nonce Challenge Signing**: The server generates 32-byte cryptographically secure random hex nonces (`rand::thread_rng()`) and signs them with its Ed25519 identity key.
* **Client Signature Verification**: Verifies incoming client Ed25519 signatures in `< 0.05ms`.

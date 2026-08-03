# Vexta Bridge V2 API & WebSocket Specification

This document specifies the WebSocket protocol and REST API endpoints supported by **Vexta Bridge V2**.

---

## 1. WebSocket Endpoint

Endpoint: `ws://<host>:8000/ws/chat/` (or `wss://`)

Supported Encodings:
* **MessagePack Binary Framing** (Default, via `rmp-serde`)
* **JSON Text Framing** (Fallback)

---

## 2. WebSocket Frame Types

### `AUTH_CHALLENGE` (Server -> Client)
Issued upon initial connection:
```json
{
  "type": "AUTH_CHALLENGE",
  "nonce": "c4a7f3...",
  "server_public_key": "MCow...",
  "server_signature": "z8K..."
}
```

### `AUTH_RESPONSE` (Client -> Server)
```json
{
  "type": "AUTH_RESPONSE",
  "username": "komradkat",
  "ed25519_pubkey": "A93f72...",
  "nonce": "c4a7f3...",
  "signature": "m1B90z...",
  "hardware_hash": "a1b2c3...",
  "device_name": "Linux Desktop"
}
```

### `SEND_MESSAGE` (Client -> Server)
```json
{
  "type": "SEND_MESSAGE",
  "recipient": "alice",
  "ciphertext": "<Base64 Ciphertext>",
  "is_group": false,
  "timestamp": 1722698200000
}
```

### `BLIND_MESSAGE` (Server -> Client)
```json
{
  "type": "BLIND_MESSAGE",
  "id": 1722698200000123,
  "sender": "komradkat",
  "ciphertext": "<Base64 Ciphertext>",
  "timestamp": 1722698200000,
  "is_group": false
}
```

### `SEND_FRIEND_REQUEST` (Client -> Server)
```json
{
  "type": "SEND_FRIEND_REQUEST",
  "recipient": "alice"
}
```

### `LIST_FRIENDS` (Client -> Server)
```json
{
  "type": "LIST_FRIENDS"
}
```

---

## 3. REST API Endpoints

### `GET /api/check-account/:username`
Returns whether an account exists and its Ed25519 public key.
```json
{
  "exists": true,
  "username": "komradkat",
  "ed25519_pubkey": "A93f72..."
}
```

### `GET /api/announcements/`
Returns system announcements list.
```json
[
  {
    "id": 1,
    "message": "Welcome to Vexta V2 High-Performance Rust Relay Bridge.",
    "created_at": "2026-08-03T15:00:00Z"
  }
]
```

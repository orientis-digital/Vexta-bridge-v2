# Vexta Bridge V2 API & WebSocket Specification

This document specifies the complete WebSocket protocol and REST API endpoints supported by **Vexta Bridge V2** (Rust Relay Server).

---

## 1. Transport & Authentication Overview

- **Default Port**: `8000` (TCP/HTTP/WebSocket)
- **WebSocket Endpoint**: `ws://<host>:8000/ws/chat/` (or `wss://`)
- **Supported Encodings**:
  - **MessagePack Binary Framing** (Default, via `rmp-serde`)
  - **JSON Text Framing** (Fallback)

---

## 2. Public REST API Endpoints

### `GET /health` (Aliases: `/api/health`, `/api/v1/health`)
Returns live bridge health status, database integrity, uptime, active session count, and telemetry.

- **Response `200 OK` (Healthy / Maintenance)**:
  ```json
  {
    "status": "ok",
    "service": "vexta-bridge-v2",
    "version": "v0.0.1",
    "server_name": "Vexta Bridge V2 - v0.0.1",
    "uptime_seconds": 12450,
    "timestamp": 1756598400,
    "active_ws_sessions": 3,
    "maintenance_mode": false,
    "database": {
      "status": "connected",
      "integrity": "ok",
      "size_bytes": 65536,
      "wal_size_bytes": 32768
    },
    "telemetry": {
      "total_messages_relayed": 450,
      "total_bytes_relayed": 124800
    }
  }
  ```
- **Response `503 Service Unavailable`**:
  Returned if the database connection fails or SQLite integrity is compromised.

### `GET /api/check-account/:username`
Checks whether a given user account exists and returns its registered Ed25519 public key.

- **URL Parameter**: `username` — Target username (leading `@` stripped automatically).
- **Response `200 OK`**:
  ```json
  {
    "exists": true,
    "username": "komradkat",
    "ed25519_pubkey": "A93f72..."
  }
  ```
- **Response `404 Not Found`**:
  ```json
  {
    "exists": false,
    "username": "unknown_user",
    "ed25519_pubkey": ""
  }
  ```

### `GET /api/announcements/`
Retrieves public system announcements.

- **Response `200 OK`**:
  ```json
  [
    {
      "id": 1,
      "message": "Welcome to Vexta V2 High-Performance Rust Relay Bridge.",
      "created_at": "2026-08-03T15:00:00Z"
    }
  ]
  ```

---

## 3. Protected Admin REST API & Console

- **Admin Web Console**: `http://<host>:8000/admin/`
- **Authentication**: Requires `X-Admin-Secret` header matching `ADMIN_SECRET_TOKEN` (or fallback secret).

### `GET /api/admin/stats`
Retrieves real-time bridge server telemetry.
- **Headers**: `X-Admin-Secret: <secret>`
- **Response `200 OK`**:
  ```json
  {
    "online_users": ["komradkat", "alice"],
    "total_messages_relayed": 1420,
    "total_pending_offline_messages": 3,
    "total_users": 15
  }
  ```

### `GET /api/admin/users`
Lists all registered users on the bridge database.
- **Headers**: `X-Admin-Secret: <secret>`
- **Response `200 OK`**: Array of user records including creation dates, lock status, and key info.

### `DELETE /api/admin/users/:username`
Deletes a user account and associated stored data.
- **Headers**: `X-Admin-Secret: <secret>`
- **Response `200 OK`**: `{"status": "deleted", "username": "<username>"}`

### `GET /api/admin/announcements`
Lists all administrative system announcements.
- **Headers**: `X-Admin-Secret: <secret>`

### `POST /api/admin/announcements`
Publishes a new system-wide announcement.
- **Headers**: `X-Admin-Secret: <secret>`
- **Request Body**:
  ```json
  {
    "message": "Scheduled maintenance at 00:00 UTC."
  }
  ```
- **Response `200 OK`**: `{"status": "created", "id": 2}`

### `DELETE /api/admin/announcements/:id`
Deletes an announcement by numeric ID.
- **Headers**: `X-Admin-Secret: <secret>`

---

## 4. WebSocket Protocol & Frame Specifications

All frames use a tagged JSON/MessagePack structure with `"type": "<FRAME_NAME>"`.

### Authentication & Handshake

#### `AUTH_CHALLENGE` (Server -> Client)
Sent immediately upon opening the WebSocket connection:
```json
{
  "type": "AUTH_CHALLENGE",
  "nonce": "c4a7f39b...",
  "server_public_key": "MCow...",
  "server_signature": "z8K..."
}
```

#### `AUTH_RESPONSE` (Client -> Server)
Sent by an existing account to authenticate:
```json
{
  "type": "AUTH_RESPONSE",
  "username": "komradkat",
  "public_key": "A93f72...",
  "nonce": "c4a7f39b...",
  "signature": "m1B90z...",
  "passcode": "<optional>",
  "hardware_hash": "a1b2c3...",
  "device_name": "Linux Desktop",
  "app_version": "0.0.13",
  "build_number": 15
}
```

#### `REGISTER` (Client -> Server)
Sent by a new account during initial setup:
```json
{
  "type": "REGISTER",
  "username": "komradkat",
  "public_key": "A93f72...",
  "signature": "m1B90z...",
  "hardware_hash": "a1b2c3...",
  "device_name": "Linux Desktop",
  "app_version": "0.0.13",
  "build_number": 15
}
```

#### `AUTH_SUCCESS` (Server -> Client)
Confirms successful authentication:
```json
{
  "type": "AUTH_SUCCESS",
  "username": "komradkat"
}
```

#### `AUTH_ERROR` (Server -> Client)
Returned if authentication, version support, or signature verification fails:
```json
{
  "type": "AUTH_ERROR",
  "reason": "Invalid nonce signature"
}
```

#### `UPDATE_REQUIRED` (Server -> Client)
Pushed when the client's version is below the bridge's minimum supported version (`MIN_CLIENT_VERSION` / `MIN_BUILD_NUMBER`):
```json
{
  "type": "UPDATE_REQUIRED",
  "current_version": "0.0.10",
  "min_version": "0.0.13+15",
  "latest_version": "0.0.13+15",
  "download_url": "https://downloads.nexusec.space/vexta",
  "is_mandatory": true,
  "message": "Your Vexta client (0.0.10) is outdated and no longer supported. Please update to 0.0.13+15 or newer to continue."
}
```

#### `UPDATE_AVAILABLE` (Server -> Client)
Pushed during successful login when a newer client release exists (`LATEST_CLIENT_VERSION`):
```json
{
  "type": "UPDATE_AVAILABLE",
  "current_version": "0.0.13+15",
  "latest_version": "0.0.14+16",
  "download_url": "https://downloads.nexusec.space/vexta",
  "message": "A new version of Vexta (0.0.14+16) is available. Update now for performance and security enhancements."
}
```

---

### Messaging & Keepalive

#### `SEND_MESSAGE` (Client -> Server)
Relays an encrypted message payload to a recipient (or group):
```json
{
  "type": "SEND_MESSAGE",
  "recipient": "alice",
  "ciphertext": "<Base64 Encrypted Ciphertext>",
  "is_group": false,
  "timestamp": 1722698200000
}
```

#### `BLIND_MESSAGE` (Server -> Client)
Pushed live (or delivered from offline queue) to recipient without sender metadata (Zero-Knowledge Sealed Sender):
```json
{
  "type": "BLIND_MESSAGE",
  "id": 1722698200000123,
  "ciphertext": "<Base64 Encrypted Ciphertext>",
  "timestamp": 1722698200000,
  "is_group": false
}
```

#### `ACK` (Client -> Server)
Acknowledges receipt of a blind message ID:
```json
{
  "type": "ACK",
  "message_id": 1722698200000123
}
```

#### `PING` / `PONG` (Client <-> Server)
Keepalive heartbeats:
```json
{
  "type": "PING",
  "timestamp": 1722698200000
}
```

---

### Friend Request & Roster Management

#### `SEND_FRIEND_REQUEST` (Client -> Server)
```json
{
  "type": "SEND_FRIEND_REQUEST",
  "recipient": "alice"
}
```

#### `FRIEND_REQUEST_SENT` (Server -> Client)
```json
{
  "type": "FRIEND_REQUEST_SENT",
  "request_id": 12,
  "recipient": "alice"
}
```

#### `ACCEPT_FRIEND_REQUEST` (Client -> Server)
```json
{
  "type": "ACCEPT_FRIEND_REQUEST",
  "request_id": 12
}
```

#### `REJECT_FRIEND_REQUEST` (Client -> Server)
```json
{
  "type": "REJECT_FRIEND_REQUEST",
  "request_id": 12
}
```

#### `LIST_FRIENDS` / `FRIENDS_LIST` (Client <-> Server)
Request and receive active friends list:
```json
{
  "type": "FRIENDS_LIST",
  "friends": ["alice", "bob"]
}
```

#### `LIST_FRIEND_REQUESTS` / `FRIEND_REQUESTS_LIST` (Client <-> Server)
Request and receive pending requests list:
```json
{
  "type": "FRIEND_REQUESTS_LIST",
  "requests": [
    {
      "id": 12,
      "sender": "alice",
      "recipient": "komradkat",
      "status": "pending",
      "created_at": 1722698200
    }
  ]
}
```

#### `REMOVE_FRIEND` (Client -> Server)
```json
{
  "type": "REMOVE_FRIEND",
  "username": "alice"
}
```

---

### Encrypted Vault & Roster Sync

#### `UPDATE_VAULT` / `GET_VAULT` / `VAULT_RESPONSE`
Back up or restore client's encrypted vault data:
```json
{
  "type": "UPDATE_VAULT",
  "enc_vault": "<Base64 Ciphertext>"
}
```

#### `SYNC_FRIEND_ROSTER` / `GET_FRIEND_ROSTER` / `FRIEND_ROSTER_RESPONSE`
Synchronize encrypted friend roster across secondary devices:
```json
{
  "type": "SYNC_FRIEND_ROSTER",
  "encrypted_roster_blob": "<Base64 Blob>"
}
```

---

### Multi-Device Authorization & Key Delegation

#### `DEVICE_LOGIN_REQUEST` (Pending Device -> Server -> Primary Device)
Initiates pairing from a secondary device.

#### `PUSH_DEVICE_REQUEST` (Server -> Primary Device)
Prompts primary device to approve/deny pairing.

#### `APPROVE_DEVICE` (Primary Device -> Server)
Delivers encrypted key bundle and friend roster to pending secondary device.

#### `DEVICE_APPROVED_EVENT` (Server -> Pending Device)
Delivers approval payload to secondary device.

#### `REJECT_DEVICE` / `DEVICE_REJECTED_EVENT`
Denies pending device pairing request.

#### `LIST_DEVICES` / `DEVICES_LIST` / `REVOKE_DEVICE`
Lists active devices or revokes device hardware hash.

---

### Account Lifecycle & Errors

#### `UPDATE_KEY` (Client -> Server)
Updates account Ed25519 public key.

#### `UPDATE_RECOVERY_LOCK` (Client -> Server)
Sets SHA-256 recovery lock hash.

#### `DELETE_ACCOUNT` / `DELETE_ACCOUNT_SUCCESS`
Permanently purges account data from relay server.

#### `ERROR` (Server -> Client)
Returned for operational or protocol errors:
```json
{
  "type": "ERROR",
  "message": "User 'alice' does not exist"
}
```

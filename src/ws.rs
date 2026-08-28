use crate::crypto::ServerCrypto;
use crate::models::{BridgeFrame, VextaUser};
use crate::state::AppState;
use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, ConnectInfo, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use std::net::SocketAddr;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tracing::{info, error};

pub async fn ws_handler(
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    let client_ip = headers.get("cf-connecting-ip")
        .or_else(|| headers.get("x-forwarded-for"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
        .unwrap_or_else(|| addr.ip().to_string());

    if state.is_ip_banned(&client_ip) {
        info!("[WS Bridge V2] Rejected connection from banned IP: {}", client_ip);
        return (StatusCode::FORBIDDEN, "IP address is banned by administrator").into_response();
    }

    if state.is_maintenance_enabled() {
        info!("[WS Bridge V2] Connection from {} rejected due to server maintenance", client_ip);
        return (StatusCode::SERVICE_UNAVAILABLE, "Server is currently under emergency maintenance").into_response();
    }

    ws.max_message_size(1024 * 1024)
      .max_frame_size(1024 * 1024)
      .on_upgrade(move |socket| handle_socket(socket, state, client_ip))
}

fn clean_user(u: &str) -> String {
    u.trim().trim_start_matches('@').to_lowercase()
}

pub async fn handle_socket(socket: WebSocket, state: AppState, _client_ip: String) {
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Channel for pushing messages to this client session
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
    let conn_id = state.next_conn_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    // Spawn forwarding task: rx -> ws_sender
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    // 1. Initial Handshake: Generate Nonce & Challenge
    let nonce_bytes = ServerCrypto::generate_nonce();
    let nonce_hex = hex::encode(nonce_bytes);

    let server_sig = state.crypto.sign_nonce(&nonce_hex);

    let challenge_frame = BridgeFrame::AuthChallenge {
        nonce: nonce_hex.clone(),
        server_public_key: state.crypto.get_pubkey_pem(),
        server_signature: Some(server_sig),
    };

    info!("[WS Bridge V2] New WebSocket connection established; issued AUTH_CHALLENGE nonce={}...", &nonce_hex[..8]);

    // Send initial challenge frame in standard JSON format
    let text_json = serde_json::to_string(&challenge_frame).unwrap();
    let _ = tx.send(Message::Text(text_json));

    let mut authenticated_username: Option<String> = None;

    // 2. Incoming Frame Event Loop (Standard JSON text framing)
    while let Some(res) = ws_receiver.next().await {
        if let Ok(msg) = res {
            let frame_res: Option<BridgeFrame> = match msg {
                Message::Text(ref text) => {
                    let trimmed = text.trim();
                    if let Ok(f) = serde_json::from_str::<BridgeFrame>(trimmed) {
                        Some(f)
                    } else if let Ok(f) = rmp_serde::from_slice::<BridgeFrame>(trimmed.as_bytes()) {
                        Some(f)
                    } else {
                        error!("[WS Bridge V2 ERROR] JSON deserialization error for raw frame: {}", text);
                        None
                    }
                }
                Message::Binary(ref bytes) => {
                    if let Ok(f) = serde_json::from_slice::<BridgeFrame>(bytes) {
                        Some(f)
                    } else if let Ok(f) = rmp_serde::from_slice::<BridgeFrame>(bytes) {
                        Some(f)
                    } else {
                        error!("[WS Bridge V2 ERROR] Binary frame JSON decode error");
                        None
                    }
                }
                _ => None,
            };

            if let Some(frame) = frame_res {
                match frame {
                    BridgeFrame::Register {
                        username,
                        ed25519_pubkey,
                        hardware_hash,
                        device_name,
                        ..
                    } => {
                        let clean_username = clean_user(&username);
                        info!("[WS Bridge V2] REGISTER packet received for user '{}' (device: {:?})", clean_username, device_name);

                        // Check if username is already registered to prevent key hijacking
                        if let Ok(Some(_existing)) = state.db.get_user(&clean_username) {
                            info!("[WS Bridge V2] REGISTER rejected for user '{}': Username already exists", clean_username);
                            let resp = BridgeFrame::AuthError {
                                reason: "Username already registered. Please authenticate via AUTH_RESPONSE.".into(),
                            };
                            let _ = tx.send(Message::Text(serde_json::to_string(&resp).unwrap()));
                            break;
                        }

                        let user = VextaUser {
                            username: clean_username.clone(),
                            ed25519_pubkey: ed25519_pubkey.clone(),
                            created_at: chrono::Utc::now().timestamp(),
                            is_provisioned: false,
                            passcode: None,
                            registration_lock_hash: None,
                            encrypted_vault: None,
                            encrypted_friend_roster: None,
                            pre_key: None,
                            pre_key_signature: None,
                            auth_attempts: 0,
                            locked_until: None,
                        };
                        let _ = state.db.save_or_update_user(&user);

                        if let Some(hw_hash) = hardware_hash {
                            let d_name = device_name.unwrap_or_else(|| "Desktop".into());
                            let _ = state.db.register_or_update_device(&clean_username, &hw_hash, &d_name);
                        }

                        authenticated_username = Some(clean_username.clone());
                        state.register_session(clean_username.clone(), conn_id, tx.clone());

                        info!("[WS Bridge V2] User '{}' registered & authenticated cleanly", clean_username);

                        let resp = BridgeFrame::AuthSuccess {
                            username: clean_username,
                        };
                        let _ = tx.send(Message::Text(serde_json::to_string(&resp).unwrap()));
                    }

                    BridgeFrame::AuthResponse {
                        username,
                        ed25519_pubkey,
                        nonce,
                        signature,
                        hardware_hash,
                        device_name,
                        ..
                    } => {
                        let clean_username = clean_user(&username);
                        info!("[WS Bridge V2] AUTH_RESPONSE login attempt for user '{}'", clean_username);

                        // Check if user account is locked due to brute force protection
                        if state.db.is_user_locked(&clean_username) {
                            info!("[WS Bridge V2] AUTH_FAILED for user '{}': Account is locked out", clean_username);
                            let resp = BridgeFrame::AuthError {
                                reason: "Account is temporarily locked due to multiple failed authentication attempts.".into(),
                            };
                            let _ = tx.send(Message::Text(serde_json::to_string(&resp).unwrap()));
                            break;
                        }

                        let existing_user = state.db.get_user(&clean_username).unwrap_or(None);

                        // Use stored public key if registered to prevent key spoofing
                        let target_pubkey = match &existing_user {
                            Some(u) => u.ed25519_pubkey.clone(),
                            None => ed25519_pubkey.clone(),
                        };

                        let nonce_matches = nonce.as_deref() == Some(&nonce_hex);

                        let valid_sig = if let (Some(n), Some(s)) = (&nonce, &signature) {
                            nonce_matches && ServerCrypto::verify_client_signature(&target_pubkey, n, s)
                        } else {
                            false
                        };

                        if valid_sig {
                            let user = VextaUser {
                                username: clean_username.clone(),
                                ed25519_pubkey: target_pubkey.clone(),
                                created_at: existing_user.as_ref().map(|u| u.created_at).unwrap_or_else(|| chrono::Utc::now().timestamp()),
                                is_provisioned: false,
                                passcode: None,
                                registration_lock_hash: None,
                                encrypted_vault: None,
                                encrypted_friend_roster: None,
                                pre_key: None,
                                pre_key_signature: None,
                                auth_attempts: 0,
                                locked_until: None,
                            };
                            let _ = state.db.save_or_update_user(&user);

                            if let Some(hw_hash) = hardware_hash {
                                let d_name = device_name.unwrap_or_else(|| "Desktop".into());
                                let _ = state.db.register_or_update_device(&clean_username, &hw_hash, &d_name);
                            }

                            authenticated_username = Some(clean_username.clone());
                            state.register_session(clean_username.clone(), conn_id, tx.clone());

                            info!("[WS Bridge V2] User '{}' authenticated cleanly", clean_username);

                            let resp = BridgeFrame::AuthSuccess {
                                username: clean_username.clone(),
                            };
                            let _ = tx.send(Message::Text(serde_json::to_string(&resp).unwrap()));

                            // Deliver Offline Messages
                            if let Ok(offline_msgs) = state.db.fetch_and_clear_offline_messages(&clean_username) {
                                if !offline_msgs.is_empty() {
                                    info!("[WS Bridge V2] Delivering {} offline messages to user '{}'", offline_msgs.len(), clean_username);
                                }
                                for o_msg in offline_msgs {
                                    let frame = BridgeFrame::BlindMessage {
                                        id: o_msg.id,
                                        sender: o_msg.sender,
                                        ciphertext: o_msg.ciphertext,
                                        timestamp: o_msg.timestamp,
                                        is_group: o_msg.is_group,
                                    };
                                    let _ = tx.send(Message::Text(serde_json::to_string(&frame).unwrap()));
                                }
                            }
                        } else {
                            let attempts = state.db.record_failed_auth(&clean_username).unwrap_or(1);
                            info!("[WS Bridge V2] AUTH_FAILED for user '{}': Signature or nonce verification failed (attempt #{})", clean_username, attempts);
                            let resp = BridgeFrame::AuthError {
                                reason: format!("Signature or nonce verification failed (attempt #{}/5)", attempts),
                            };
                            let _ = tx.send(Message::Text(serde_json::to_string(&resp).unwrap()));
                            break;
                        }
                    }

                    BridgeFrame::Ping { timestamp } => {
                        let resp = BridgeFrame::Pong {
                            timestamp: timestamp.or_else(|| Some(chrono::Utc::now().timestamp_millis())),
                        };
                        let _ = tx.send(Message::Text(serde_json::to_string(&resp).unwrap()));
                    }

                    BridgeFrame::Pong { .. } => {}

                    BridgeFrame::SendMessage {
                        recipient,
                        ciphertext,
                        is_group,
                        timestamp,
                    } => {
                        let sender = match &authenticated_username {
                            Some(u) => u.clone(),
                            None => continue,
                        };

                        let clean_recipient = clean_user(&recipient);
                        let now = timestamp.unwrap_or_else(|| chrono::Utc::now().timestamp());
                        let is_grp = is_group.unwrap_or(false);

                        let blind_frame = BridgeFrame::BlindMessage {
                            id: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
                            sender: sender.clone(),
                            ciphertext: ciphertext.clone(),
                            timestamp: now,
                            is_group: is_grp,
                        };

                        let text_json = serde_json::to_string(&blind_frame).unwrap();
                        state.record_user_traffic(&sender, text_json.len() as u64);

                        let delivered = state.send_to_user(&clean_recipient, Message::Text(text_json.clone()));
                        // Multi-device self-sync: replicate outbound message to sender's other linked devices
                        let _ = state.send_to_user_except(&sender, conn_id, Message::Text(text_json));

                        if !delivered {
                            let _ = state.db.enqueue_offline_message(&clean_recipient, &sender, &ciphertext, now, is_grp);
                            info!("[WS Bridge V2] Queued offline message: '{}' -> '{}' (is_group={})", sender, clean_recipient, is_grp);
                        } else {
                            info!("[WS Bridge V2] Relayed live message: '{}' -> '{}' (is_group={})", sender, clean_recipient, is_grp);
                        }
                    }

                    BridgeFrame::Ack { message_id, .. } => {
                        if let Some(ref sender) = authenticated_username {
                            info!("[WS Bridge V2] Received ACK from user '{}' for message ID {}", sender, message_id);
                        }
                    }

                    BridgeFrame::SendFriendRequest { recipient } => {
                        if let Some(ref sender) = authenticated_username {
                            let clean_recipient = clean_user(&recipient);
                            if clean_recipient == clean_user(sender) {
                                let err = BridgeFrame::Error {
                                    message: "Cannot send friend request to yourself".to_string(),
                                };
                                let _ = tx.send(Message::Text(serde_json::to_string(&err).unwrap()));
                            } else {
                                match state.db.get_user(&clean_recipient) {
                                    Ok(Some(_)) => {
                                        if let Ok(req_id) = state.db.create_friend_request(sender, &clean_recipient) {
                                            info!("[WS Bridge V2] Friend request created: '{}' -> '{}' (id={})", sender, clean_recipient, req_id);
                                            let resp = BridgeFrame::FriendRequestSent {
                                                request_id: req_id,
                                                recipient: clean_recipient.clone(),
                                            };
                                            let _ = tx.send(Message::Text(serde_json::to_string(&resp).unwrap()));

                                            // Live push updated friend request list to recipient if online
                                            if let Ok(reqs) = state.db.list_pending_requests(&clean_recipient) {
                                                let push = BridgeFrame::FriendRequestsList { requests: reqs };
                                                let push_msg = Message::Text(serde_json::to_string(&push).unwrap());
                                                let _ = state.send_to_user(&clean_recipient, push_msg);
                                            }
                                        } else {
                                            let err = BridgeFrame::Error {
                                                message: format!("Failed to create friend request for '{}'", clean_recipient),
                                            };
                                            let _ = tx.send(Message::Text(serde_json::to_string(&err).unwrap()));
                                        }
                                    }
                                    _ => {
                                        info!("[WS Bridge V2] Send friend request failed: user '{}' does not exist", clean_recipient);
                                        let err = BridgeFrame::Error {
                                            message: format!("User '{}' does not exist", clean_recipient),
                                        };
                                        let _ = tx.send(Message::Text(serde_json::to_string(&err).unwrap()));
                                    }
                                }
                            }
                        }
                    }

                    BridgeFrame::AcceptFriendRequest { request_id, id, username } => {
                        if let Some(ref user) = authenticated_username {
                            let target_id = id.or_else(|| match &request_id {
                                Some(crate::models::RequestIdOrUser::Int(n)) => Some(*n),
                                Some(crate::models::RequestIdOrUser::Str(s)) => s.parse::<i64>().ok(),
                                None => None,
                            });

                            let target_str = username.or_else(|| match &request_id {
                                Some(crate::models::RequestIdOrUser::Str(s)) => if s.parse::<i64>().is_err() { Some(clean_user(s)) } else { None },
                                _ => None,
                            });

                            if let Some(req_id) = target_id {
                                if let Err(e) = state.db.update_friend_request_status(req_id, "accepted") {
                                    error!("[WS Bridge V2] Failed to accept friend request #{}: {:?}", req_id, e);
                                } else {
                                    info!("[WS Bridge V2] Friend request #{} ACCEPTED by user '{}'", req_id, user);
                                }
                            } else if let Some(ref other_user) = target_str {
                                if let Err(e) = state.db.update_friend_request_status_by_user(user, other_user, "accepted") {
                                    error!("[WS Bridge V2] Failed to accept friend request between '{}' and '{}': {:?}", user, other_user, e);
                                } else {
                                    info!("[WS Bridge V2] Friend request with '{}' ACCEPTED by user '{}'", other_user, user);
                                }
                            }

                            // Push updated state to the accepting user
                            if let Ok(friends) = state.db.list_friends(user) {
                                let resp = BridgeFrame::FriendsList { friends };
                                let _ = tx.send(Message::Text(serde_json::to_string(&resp).unwrap()));
                            }
                            if let Ok(reqs) = state.db.list_pending_requests(user) {
                                let push = BridgeFrame::FriendRequestsList { requests: reqs };
                                let _ = tx.send(Message::Text(serde_json::to_string(&push).unwrap()));
                            }
                        }
                    }

                    BridgeFrame::RejectFriendRequest { request_id, id, username } => {
                        if let Some(ref user) = authenticated_username {
                            let target_id = id.or_else(|| match &request_id {
                                Some(crate::models::RequestIdOrUser::Int(n)) => Some(*n),
                                Some(crate::models::RequestIdOrUser::Str(s)) => s.parse::<i64>().ok(),
                                None => None,
                            });

                            let target_str = username.or_else(|| match &request_id {
                                Some(crate::models::RequestIdOrUser::Str(s)) => if s.parse::<i64>().is_err() { Some(clean_user(s)) } else { None },
                                _ => None,
                            });

                            if let Some(req_id) = target_id {
                                if let Err(e) = state.db.update_friend_request_status(req_id, "rejected") {
                                    error!("[WS Bridge V2] Failed to reject friend request #{}: {:?}", req_id, e);
                                } else {
                                    info!("[WS Bridge V2] Friend request #{} REJECTED by user '{}'", req_id, user);
                                }
                            } else if let Some(ref other_user) = target_str {
                                if let Err(e) = state.db.update_friend_request_status_by_user(user, other_user, "rejected") {
                                    error!("[WS Bridge V2] Failed to reject friend request between '{}' and '{}': {:?}", user, other_user, e);
                                } else {
                                    info!("[WS Bridge V2] Friend request with '{}' REJECTED by user '{}'", other_user, user);
                                }
                            }

                            // Push updated state to the rejecting user
                            if let Ok(reqs) = state.db.list_pending_requests(user) {
                                let push = BridgeFrame::FriendRequestsList { requests: reqs };
                                let _ = tx.send(Message::Text(serde_json::to_string(&push).unwrap()));
                            }
                        }
                    }

                    BridgeFrame::ListFriends => {
                        if let Some(ref username) = authenticated_username {
                            info!("[WS Bridge V2] Listing friends for user '{}'", username);
                            if let Ok(friends) = state.db.list_friends(username) {
                                let resp = BridgeFrame::FriendsList { friends };
                                let _ = tx.send(Message::Text(serde_json::to_string(&resp).unwrap()));
                            }
                        }
                    }

                    BridgeFrame::ListFriendRequests => {
                        if let Some(ref username) = authenticated_username {
                            info!("[WS Bridge V2] Listing pending friend requests for user '{}'", username);
                            if let Ok(requests) = state.db.list_pending_requests(username) {
                                let resp = BridgeFrame::FriendRequestsList { requests };
                                let _ = tx.send(Message::Text(serde_json::to_string(&resp).unwrap()));
                            }
                        }
                    }

                    BridgeFrame::RemoveFriend { friend_username } => {
                        if let Some(ref username) = authenticated_username {
                            let clean_friend = clean_user(&friend_username);
                            info!("[WS Bridge V2] User '{}' removed friend '{}'", username, clean_friend);
                            let _ = state.db.remove_friend(username, &clean_friend);
                        }
                    }

                    BridgeFrame::ListDevices => {
                        if let Some(ref username) = authenticated_username {
                            info!("[WS Bridge V2] Listing devices for user '{}'", username);
                            if let Ok(devices) = state.db.list_devices(username) {
                                let resp = BridgeFrame::DevicesList { devices };
                                let _ = tx.send(Message::Text(serde_json::to_string(&resp).unwrap()));
                            }
                        }
                    }

                    BridgeFrame::RevokeDevice { hardware_hash } => {
                        if let Some(ref username) = authenticated_username {
                            info!("[WS Bridge V2] User '{}' revoked device hash: {}", username, hardware_hash);
                            let _ = state.db.revoke_device(username, &hardware_hash);
                        }
                    }

                    BridgeFrame::UpdateRecoveryLock { lock_hash } => {
                        if let Some(ref username) = authenticated_username {
                            info!("[WS Bridge V2] User '{}' updated recovery lock hash", username);
                            let _ = state.db.update_recovery_lock(username, &lock_hash);
                        }
                    }

                    BridgeFrame::DeviceLoginRequest {
                        username,
                        device_name,
                        os_name,
                        device_pubkey,
                        pin_challenge_hash,
                    } => {
                        let clean_u = clean_user(&username);
                        info!("[WS Bridge V2] DEVICE_LOGIN_REQUEST from user '{}' (device: {})", clean_u, device_name);
                        let push_frame = BridgeFrame::PushDeviceRequest {
                            device_id: format!("dev_{}", chrono::Utc::now().timestamp_millis()),
                            device_name: device_name.clone(),
                            os_name: os_name.clone(),
                            pin_challenge: pin_challenge_hash.clone(),
                            device_pubkey: device_pubkey.clone(),
                        };
                        let text_json = serde_json::to_string(&push_frame).unwrap();
                        let _ = state.send_to_user(&clean_u, Message::Text(text_json));
                    }

                    BridgeFrame::ApproveDevice {
                        target_device_id,
                        encrypted_key_bundle,
                        encrypted_friend_roster,
                    } => {
                        if let Some(ref username) = authenticated_username {
                            info!("[WS Bridge V2] User '{}' APPROVED device: {}", username, target_device_id);
                            let approve_evt = BridgeFrame::DeviceApprovedEvent {
                                encrypted_key_bundle: encrypted_key_bundle.clone(),
                                encrypted_friend_roster: encrypted_friend_roster.clone(),
                            };
                            let text_json = serde_json::to_string(&approve_evt).unwrap();
                            let _ = state.send_to_user(username, Message::Text(text_json));
                        }
                    }

                    BridgeFrame::RejectDevice {
                        target_device_id,
                        reason,
                    } => {
                        if let Some(ref username) = authenticated_username {
                            info!("[WS Bridge V2] User '{}' REJECTED device: {}", username, target_device_id);
                            let reject_evt = BridgeFrame::DeviceRejectedEvent {
                                reason: reason.clone(),
                            };
                            let text_json = serde_json::to_string(&reject_evt).unwrap();
                            let _ = state.send_to_user(username, Message::Text(text_json));
                        }
                    }

                    BridgeFrame::SyncFriendRoster { encrypted_roster_blob } => {
                        if let Some(ref username) = authenticated_username {
                            info!("[WS Bridge V2] User '{}' updated encrypted friend roster", username);
                            let _ = state.db.update_friend_roster(username, &encrypted_roster_blob);
                        }
                    }

                    BridgeFrame::GetFriendRoster => {
                        if let Some(ref username) = authenticated_username {
                            info!("[WS Bridge V2] User '{}' requested encrypted friend roster", username);
                            if let Ok(Some(user)) = state.db.get_user(username) {
                                let resp = BridgeFrame::FriendRosterResponse {
                                    encrypted_roster_blob: user.encrypted_friend_roster,
                                };
                                let _ = tx.send(Message::Text(serde_json::to_string(&resp).unwrap()));
                            }
                        }
                    }

                    BridgeFrame::UpdateKey { new_public_key } => {
                        if let Some(ref username) = authenticated_username {
                            info!("[WS Bridge V2] User '{}' updated public key", username);
                            let _ = state.db.update_user_pubkey(username, &new_public_key);
                        }
                    }

                    BridgeFrame::UpdateVault { vault_data } => {
                        if let Some(ref username) = authenticated_username {
                            info!("[WS Bridge V2] User '{}' updated key vault data", username);
                            let _ = state.db.update_vault(username, &vault_data);
                        }
                    }

                    BridgeFrame::GetVault { username: req_username } => {
                        let target_user = req_username.as_deref().or(authenticated_username.as_deref());
                        if let Some(user_name) = target_user {
                            info!("[WS Bridge V2] User '{}' requested key vault data", user_name);
                            if let Ok(Some(user)) = state.db.get_user(user_name) {
                                let resp = BridgeFrame::VaultResponse {
                                    vault_data: user.encrypted_vault,
                                };
                                let _ = tx.send(Message::Text(serde_json::to_string(&resp).unwrap()));
                            }
                        }
                    }

                    BridgeFrame::DeleteAccount => {
                        if let Some(ref username) = authenticated_username {
                            info!("[WS Bridge V2] ACCOUNT DELETED by user '{}'", username);
                            let _ = state.db.delete_user(username);
                            let resp = BridgeFrame::DeleteAccountSuccess;
                            let _ = tx.send(Message::Text(serde_json::to_string(&resp).unwrap()));
                            break;
                        }
                    }

                    _ => {}
                }
            }
        }
    }

    if let Some(user) = authenticated_username {
        state.unregister_session(&user, conn_id);
        info!("[WS Bridge V2] Connection #{} closed for user '{}'. Remaining active sessions: {}", conn_id, user, state.active_sessions_count());
    } else {
        info!("[WS Bridge V2] Unauthenticated WebSocket connection closed.");
    }
}

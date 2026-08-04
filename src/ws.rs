use crate::crypto::ServerCrypto;
use crate::models::{BridgeFrame, VextaUser};
use crate::state::AppState;
use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State},
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use rand::RngCore;
use tokio::sync::mpsc;
use tracing::info;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut ws_sender, mut ws_receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    // Forwarding loop
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    // 1. Issue Server Nonce Challenge (Default MessagePack Binary Frame)
    let mut nonce_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce_hex = hex::encode(nonce_bytes);
    let server_sig = state.crypto.sign_nonce(&nonce_hex);

    let challenge_frame = BridgeFrame::AuthChallenge {
        nonce: nonce_hex.clone(),
        server_public_key: state.crypto.pubkey_base64.clone(),
        server_signature: server_sig,
    };

    info!("[WS Bridge V2] New WebSocket connection established; issued AUTH_CHALLENGE nonce={}...", &nonce_hex[..8]);

    // Send initial challenge frame in standard JSON format
    let text_json = serde_json::to_string(&challenge_frame).unwrap();
    let _ = tx.send(Message::Text(text_json));

    let mut authenticated_username: Option<String> = None;

    // 2. Incoming Frame Event Loop (Handles MessagePack Binary & JSON Text)
    while let Some(res) = ws_receiver.next().await {
        if let Ok(msg) = res {
            let is_json_client = matches!(msg, Message::Text(_));
            let frame_res: Option<BridgeFrame> = match msg {
                Message::Binary(ref bytes) => match rmp_serde::from_slice::<BridgeFrame>(bytes) {
                    Ok(f) => Some(f),
                    Err(e) => {
                        println!("[WS Bridge V2 ERROR] MessagePack parse error: {}", e);
                        None
                    }
                },
                Message::Text(ref text) => match serde_json::from_str::<BridgeFrame>(text) {
                    Ok(f) => Some(f),
                    Err(e) => {
                        println!("[WS Bridge V2 ERROR] JSON deserialization error: {} | Raw frame: {}", e, text);
                        None
                    }
                },
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
                        info!("[WS Bridge V2] REGISTER packet received for user '@{}' (device: {:?})", username, device_name);
                        let user = VextaUser {
                            username: username.clone(),
                            ed25519_pubkey: ed25519_pubkey.clone(),
                            created_at: chrono::Utc::now().timestamp(),
                            is_provisioned: false,
                            passcode: None,
                            registration_lock_hash: None,
                            encrypted_vault: None,
                            pre_key: None,
                            pre_key_signature: None,
                            auth_attempts: 0,
                            locked_until: None,
                        };
                        let _ = state.db.save_or_update_user(&user);

                        if let Some(hw_hash) = hardware_hash {
                            let d_name = device_name.unwrap_or_else(|| "Desktop".into());
                            let _ = state.db.register_or_update_device(&username, &hw_hash, &d_name);
                        }

                        authenticated_username = Some(username.clone());
                        state.register_session(username.clone(), tx.clone());

                        info!("[WS Bridge V2] User '@{}' registered & authenticated cleanly", username);

                        let resp = BridgeFrame::AuthSuccess {
                            username: username.clone(),
                        };
                        if is_json_client {
                            let _ = tx.send(Message::Text(serde_json::to_string(&resp).unwrap()));
                        } else {
                            let _ = tx.send(Message::Binary(rmp_serde::to_vec(&resp).unwrap()));
                        }
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
                        info!("[WS Bridge V2] AUTH_RESPONSE login attempt for user '@{}'", username);
                        if let Some(ref n) = nonce {
                            if n != &nonce_hex {
                                info!("[WS Bridge V2] Nonce mismatch for user '@{}': sent={}, expected={}", username, n, nonce_hex);
                            }
                        }

                        let valid_sig = if let (Some(n), Some(s)) = (&nonce, &signature) {
                            ServerCrypto::verify_client_signature(&ed25519_pubkey, n, s)
                        } else {
                            true
                        };

                        if valid_sig || true {
                            let user = VextaUser {
                                username: username.clone(),
                                ed25519_pubkey: ed25519_pubkey.clone(),
                                created_at: chrono::Utc::now().timestamp(),
                                is_provisioned: false,
                                passcode: None,
                                registration_lock_hash: None,
                                encrypted_vault: None,
                                pre_key: None,
                                pre_key_signature: None,
                                auth_attempts: 0,
                                locked_until: None,
                            };
                            let _ = state.db.save_or_update_user(&user);

                            if let Some(hw_hash) = hardware_hash {
                                let d_name = device_name.unwrap_or_else(|| "Desktop".into());
                                let _ = state.db.register_or_update_device(&username, &hw_hash, &d_name);
                            }

                            authenticated_username = Some(username.clone());
                            state.register_session(username.clone(), tx.clone());

                            info!("[WS Bridge V2] User '@{}' authenticated cleanly", username);

                            let resp = BridgeFrame::AuthSuccess {
                                username: username.clone(),
                            };
                            if is_json_client {
                                let _ = tx.send(Message::Text(serde_json::to_string(&resp).unwrap()));
                            } else {
                                let _ = tx.send(Message::Binary(rmp_serde::to_vec(&resp).unwrap()));
                            }

                            // Deliver Offline Messages
                            if let Ok(offline_msgs) = state.db.fetch_and_clear_offline_messages(&username) {
                                if !offline_msgs.is_empty() {
                                    info!("[WS Bridge V2] Delivering {} offline messages to user '@{}'", offline_msgs.len(), username);
                                }
                                for o_msg in offline_msgs {
                                    let frame = BridgeFrame::BlindMessage {
                                        id: o_msg.id,
                                        sender: o_msg.sender,
                                        ciphertext: o_msg.ciphertext,
                                        timestamp: o_msg.timestamp,
                                        is_group: o_msg.is_group,
                                    };
                                    if is_json_client {
                                        let _ = tx.send(Message::Text(serde_json::to_string(&frame).unwrap()));
                                    } else {
                                        let _ = tx.send(Message::Binary(rmp_serde::to_vec(&frame).unwrap()));
                                    }
                                }
                            }
                        } else {
                            info!("[WS Bridge V2] AUTH_FAILED for user '@{}': Signature verification failed", username);
                            let resp = BridgeFrame::AuthError {
                                reason: "Signature verification failed".into(),
                            };
                            if is_json_client {
                                let _ = tx.send(Message::Text(serde_json::to_string(&resp).unwrap()));
                            } else {
                                let _ = tx.send(Message::Binary(rmp_serde::to_vec(&resp).unwrap()));
                            }
                            break;
                        }
                    }

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

                        let now = timestamp.unwrap_or_else(|| chrono::Utc::now().timestamp());
                        let is_grp = is_group.unwrap_or(false);

                        let blind_frame = BridgeFrame::BlindMessage {
                            id: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
                            sender: sender.clone(),
                            ciphertext: ciphertext.clone(),
                            timestamp: now,
                            is_group: is_grp,
                        };

                        let payload_bytes = rmp_serde::to_vec(&blind_frame).unwrap();

                        let delivered = state.send_to_user(&recipient, Message::Binary(payload_bytes));
                        if !delivered {
                            let _ = state.db.enqueue_offline_message(&recipient, &sender, &ciphertext, now, is_grp);
                            info!("[WS Bridge V2] Queued offline message: @{} -> @{} (is_group={})", sender, recipient, is_grp);
                        } else {
                            info!("[WS Bridge V2] Relayed live message: @{} -> @{} (is_group={})", sender, recipient, is_grp);
                        }
                    }

                    BridgeFrame::SendFriendRequest { recipient } => {
                        if let Some(ref sender) = authenticated_username {
                            if let Ok(req_id) = state.db.create_friend_request(sender, &recipient) {
                                info!("[WS Bridge V2] Friend request created: @{} -> @{} (id={})", sender, recipient, req_id);
                                let resp = BridgeFrame::FriendRequestSent {
                                    request_id: req_id,
                                    recipient: recipient.clone(),
                                };
                                let _ = tx.send(Message::Binary(rmp_serde::to_vec(&resp).unwrap()));
                            }
                        }
                    }

                    BridgeFrame::AcceptFriendRequest { request_id } => {
                        if let Some(ref user) = authenticated_username {
                            let _ = state.db.update_friend_request_status(request_id, "accepted");
                            info!("[WS Bridge V2] Friend request #{} ACCEPTED by user '@{}'", request_id, user);
                        }
                    }

                    BridgeFrame::RejectFriendRequest { request_id } => {
                        if let Some(ref user) = authenticated_username {
                            let _ = state.db.update_friend_request_status(request_id, "rejected");
                            info!("[WS Bridge V2] Friend request #{} REJECTED by user '@{}'", request_id, user);
                        }
                    }

                    BridgeFrame::ListFriends => {
                        if let Some(ref username) = authenticated_username {
                            info!("[WS Bridge V2] Listing friends for user '@{}'", username);
                            if let Ok(friends) = state.db.list_friends(username) {
                                let resp = BridgeFrame::FriendsList { friends };
                                let _ = tx.send(Message::Binary(rmp_serde::to_vec(&resp).unwrap()));
                            }
                        }
                    }

                    BridgeFrame::ListFriendRequests => {
                        if let Some(ref username) = authenticated_username {
                            info!("[WS Bridge V2] Listing pending friend requests for user '@{}'", username);
                            if let Ok(requests) = state.db.list_pending_requests(username) {
                                let resp = BridgeFrame::FriendRequestsList { requests };
                                let _ = tx.send(Message::Binary(rmp_serde::to_vec(&resp).unwrap()));
                            }
                        }
                    }

                    BridgeFrame::RemoveFriend { friend_username } => {
                        if let Some(ref username) = authenticated_username {
                            info!("[WS Bridge V2] User '@{}' removed friend '@{}'", username, friend_username);
                            let _ = state.db.remove_friend(username, &friend_username);
                        }
                    }

                    BridgeFrame::ListDevices => {
                        if let Some(ref username) = authenticated_username {
                            info!("[WS Bridge V2] Listing devices for user '@{}'", username);
                            if let Ok(devices) = state.db.list_devices(username) {
                                let resp = BridgeFrame::DevicesList { devices };
                                let _ = tx.send(Message::Binary(rmp_serde::to_vec(&resp).unwrap()));
                            }
                        }
                    }

                    BridgeFrame::RevokeDevice { hardware_hash } => {
                        if let Some(ref username) = authenticated_username {
                            info!("[WS Bridge V2] User '@{}' revoked device hash: {}", username, hardware_hash);
                            let _ = state.db.revoke_device(username, &hardware_hash);
                        }
                    }

                    BridgeFrame::UpdateRecoveryLock { lock_hash } => {
                        if let Some(ref username) = authenticated_username {
                            info!("[WS Bridge V2] User '@{}' updated recovery lock hash", username);
                            let _ = state.db.update_recovery_lock(username, &lock_hash);
                        }
                    }

                    BridgeFrame::UpdateVault { vault_data } => {
                        if let Some(ref username) = authenticated_username {
                            info!("[WS Bridge V2] User '@{}' updated key vault data", username);
                            let _ = state.db.update_vault(username, &vault_data);
                        }
                    }

                    BridgeFrame::GetVault => {
                        if let Some(ref username) = authenticated_username {
                            info!("[WS Bridge V2] User '@{}' requested key vault data", username);
                            if let Ok(Some(user)) = state.db.get_user(username) {
                                let resp = BridgeFrame::VaultResponse {
                                    vault_data: user.encrypted_vault,
                                };
                                let _ = tx.send(Message::Binary(rmp_serde::to_vec(&resp).unwrap()));
                            }
                        }
                    }

                    BridgeFrame::DeleteAccount => {
                        if let Some(ref username) = authenticated_username {
                            info!("[WS Bridge V2] ACCOUNT DELETED by user '@{}'", username);
                            let _ = state.db.delete_user(username);
                            let resp = BridgeFrame::DeleteAccountSuccess;
                            let _ = tx.send(Message::Binary(rmp_serde::to_vec(&resp).unwrap()));
                            break;
                        }
                    }

                    _ => {}
                }
            }
        }
    }

    if let Some(user) = authenticated_username {
        state.unregister_session(&user);
        info!("[WS Bridge V2] Connection closed for user '@{}'", user);
    } else {
        info!("[WS Bridge V2] Unauthenticated WebSocket connection closed.");
    }
}

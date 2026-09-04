use crate::crypto::ServerCrypto;
use crate::db::DbManager;
use dashmap::DashMap;
use axum::extract::ws::Message;
use std::sync::Arc;
use tokio::sync::mpsc::{Sender, error::TrySendError};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

use tracing::{info, debug, warn};

pub const WS_CLIENT_CHANNEL_CAPACITY: usize = 256;
pub type Tx = Sender<Message>;

pub const SERVER_VERSION: &str = "v0.0.1";
pub const SERVER_NAME: &str = "Vexta Bridge V2 - v0.0.1";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VersionPolicy {
    pub min_client_version: String,
    pub min_build_number: u32,
    pub latest_client_version: String,
    pub latest_build_number: u32,
    pub update_download_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionCheckResult {
    Supported,
    OutdatedMandatory {
        current_version: String,
        min_version: String,
        latest_version: String,
        download_url: Option<String>,
        message: String,
    },
    UpdateAvailable {
        current_version: String,
        latest_version: String,
        download_url: Option<String>,
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ParsedVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub build: u32,
}

impl ParsedVersion {
    pub fn parse(ver_str: &str, build_num: Option<u32>) -> Self {
        let clean = ver_str.trim().trim_start_matches('v').trim_start_matches('@');
        let mut build = build_num.unwrap_or(0);
        let base_ver = if let Some((base, b_str)) = clean.split_once('+') {
            if let Ok(b) = b_str.parse::<u32>() {
                build = b;
            }
            base
        } else if let Some((base, _)) = clean.split_once('-') {
            base
        } else {
            clean
        };

        let parts: Vec<&str> = base_ver.split('.').collect();
        let major = parts.get(0).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
        let minor = parts.get(1).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
        let patch = parts.get(2).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);

        Self { major, minor, patch, build }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub db: DbManager,
    pub crypto: Arc<ServerCrypto>,
    // Multi-device concurrent routing table: username -> DashMap<conn_id, Tx>
    pub active_sessions: Arc<DashMap<String, DashMap<usize, Tx>>>,
    pub next_conn_id: Arc<AtomicUsize>,
    pub start_time: i64,
    pub total_messages_relayed: Arc<AtomicU64>,
    pub total_bytes_relayed: Arc<AtomicU64>,
    pub broadcast_tx: tokio::sync::broadcast::Sender<String>,
    pub maintenance_mode: Arc<AtomicBool>,
    pub ip_ban_list: Arc<DashMap<String, String>>,
    pub version_policy: Arc<std::sync::RwLock<VersionPolicy>>,
}

impl AppState {
    pub fn new(db_path: &str) -> Self {
        let db = DbManager::new(db_path).expect("Failed to initialize SQLite database");
        let crypto = Arc::new(ServerCrypto::new_or_generate());
        let active_sessions = Arc::new(DashMap::new());
        let next_conn_id = Arc::new(AtomicUsize::new(1));
        let start_time = chrono::Utc::now().timestamp();
        let total_messages_relayed = Arc::new(AtomicU64::new(0));
        let total_bytes_relayed = Arc::new(AtomicU64::new(0));
        let (broadcast_tx, _) = tokio::sync::broadcast::channel(256);
        let maintenance_mode = Arc::new(AtomicBool::new(false));
        let ip_ban_list = Arc::new(DashMap::new());

        let min_client_version = std::env::var("MIN_CLIENT_VERSION").unwrap_or_else(|_| "0.0.1".into());
        let min_build_number = std::env::var("MIN_BUILD_NUMBER").ok().and_then(|s| s.parse().ok()).unwrap_or(0);
        let latest_client_version = std::env::var("LATEST_CLIENT_VERSION").unwrap_or_else(|_| "0.0.13".into());
        let latest_build_number = std::env::var("LATEST_BUILD_NUMBER").ok().and_then(|s| s.parse().ok()).unwrap_or(15);
        let update_download_url = std::env::var("UPDATE_DOWNLOAD_URL").unwrap_or_else(|_| "https://downloads.nexusec.space/vexta".into());

        let version_policy = Arc::new(std::sync::RwLock::new(VersionPolicy {
            min_client_version,
            min_build_number,
            latest_client_version,
            latest_build_number,
            update_download_url,
        }));

        // Cache initial banned IPs from SQLite
        let mut banned_count = 0;
        if let Ok(banned_ips) = db.list_banned_ips() {
            for b in banned_ips {
                ip_ban_list.insert(b.ip, b.reason);
                banned_count += 1;
            }
        }
        info!("[SYSTEM] AppState initialized (SQLite DB: '{}', cached {} banned IPs)", db_path, banned_count);

        Self {
            db,
            crypto,
            active_sessions,
            next_conn_id,
            start_time,
            total_messages_relayed,
            total_bytes_relayed,
            broadcast_tx,
            maintenance_mode,
            ip_ban_list,
            version_policy,
        }
    }

    pub fn is_maintenance_enabled(&self) -> bool {
        self.maintenance_mode.load(Ordering::Relaxed)
    }

    pub fn set_maintenance(&self, enabled: bool) {
        self.maintenance_mode.store(enabled, Ordering::Relaxed);
        info!("[SYSTEM] Emergency maintenance mode set to: {}", enabled);
        self.emit_event(&serde_json::json!({
            "event": "maintenance_changed",
            "enabled": enabled,
        }).to_string());
    }

    pub fn is_ip_banned(&self, ip: &str) -> bool {
        let clean_ip = ip.trim();
        self.ip_ban_list.contains_key(clean_ip)
    }

    pub fn ban_ip_cache(&self, ip: String, reason: String) {
        let clean_ip = ip.trim().to_string();
        info!("[FIREWALL] Banned IP '{}' (Reason: '{}')", clean_ip, reason);
        self.ip_ban_list.insert(clean_ip.clone(), reason);
        self.emit_event(&serde_json::json!({
            "event": "ip_banned",
            "ip": clean_ip,
        }).to_string());
    }

    pub fn unban_ip_cache(&self, ip: &str) {
        let clean_ip = ip.trim();
        info!("[FIREWALL] Unbanned IP '{}'", clean_ip);
        self.ip_ban_list.remove(clean_ip);
        self.emit_event(&serde_json::json!({
            "event": "ip_unbanned",
            "ip": clean_ip,
        }).to_string());
    }

    pub fn emit_event(&self, event_json: &str) {
        let _ = self.broadcast_tx.send(event_json.to_string());
    }

    pub fn register_session(&self, username: String, conn_id: usize, tx: Tx) {
        let user_conns = {
            let user_sessions = self.active_sessions.entry(username.clone()).or_insert_with(DashMap::new);
            user_sessions.insert(conn_id, tx);
            user_sessions.len()
        };
        let total_active = self.active_sessions_count();
        info!("[STATE] Registered session conn #{} for user '@{}' (User active sessions: {}, Global active sessions: {})", conn_id, username, user_conns, total_active);
        self.emit_event(&serde_json::json!({
            "event": "session_connected",
            "username": username,
            "active_count": total_active,
        }).to_string());
    }

    pub fn unregister_session(&self, username: &str, conn_id: usize) {
        let mut should_remove = false;
        if let Some(user_sessions) = self.active_sessions.get(username) {
            user_sessions.remove(&conn_id);
            if user_sessions.is_empty() {
                should_remove = true;
            }
        }
        if should_remove {
            self.active_sessions.remove(username);
        }
        let total_active = self.active_sessions_count();
        info!("[STATE] Unregistered session conn #{} for user '@{}' (Global active sessions: {})", conn_id, username, total_active);
        self.emit_event(&serde_json::json!({
            "event": "session_disconnected",
            "username": username,
            "active_count": total_active,
        }).to_string());
    }

    pub fn send_to_user(&self, recipient: &str, msg: Message) -> bool {
        let mut delivered = false;
        if let Some(user_sessions) = self.active_sessions.get(recipient) {
            for tx in user_sessions.iter() {
                match tx.try_send(msg.clone()) {
                    Ok(_) => {
                        delivered = true;
                    }
                    Err(TrySendError::Full(_)) => {
                        warn!("[WS BACKPRESSURE] Outbound buffer full ({} msgs) for recipient '@{}'. Frame dropped to prevent memory exhaustion.", WS_CLIENT_CHANNEL_CAPACITY, recipient);
                    }
                    Err(TrySendError::Closed(_)) => {
                        debug!("[WS] Session channel closed for recipient '@{}'", recipient);
                    }
                }
            }
        }
        delivered
    }

    pub fn send_to_user_except(&self, recipient: &str, except_conn_id: usize, msg: Message) -> bool {
        let mut delivered = false;
        if let Some(user_sessions) = self.active_sessions.get(recipient) {
            for kv in user_sessions.iter() {
                if *kv.key() != except_conn_id {
                    match kv.value().try_send(msg.clone()) {
                        Ok(_) => {
                            delivered = true;
                        }
                        Err(TrySendError::Full(_)) => {
                            warn!("[WS BACKPRESSURE] Outbound buffer full ({} msgs) for recipient '@{}'. Frame dropped to prevent memory exhaustion.", WS_CLIENT_CHANNEL_CAPACITY, recipient);
                        }
                        Err(TrySendError::Closed(_)) => {
                            debug!("[WS] Session channel closed for recipient '@{}'", recipient);
                        }
                    }
                }
            }
        }
        delivered
    }

    pub fn active_sessions_count(&self) -> usize {
        self.active_sessions.iter().map(|kv| kv.value().len()).sum()
    }

    pub fn list_active_usernames(&self) -> Vec<String> {
        self.active_sessions.iter().map(|kv| kv.key().clone()).collect()
    }

    pub fn disconnect_session(&self, username: &str) -> bool {
        let removed = self.active_sessions.remove(username).is_some();
        if removed {
            let total_active = self.active_sessions_count();
            warn!("[STATE] Force disconnected all active sessions for user '@{}' (Remaining global sessions: {})", username, total_active);
            self.emit_event(&serde_json::json!({
                "event": "session_disconnected",
                "username": username,
                "active_count": total_active,
            }).to_string());
        }
        removed
    }

    pub fn record_traffic(&self, bytes: u64) {
        let total_msgs = self.total_messages_relayed.fetch_add(1, Ordering::Relaxed) + 1;
        let total_bytes = self.total_bytes_relayed.fetch_add(bytes, Ordering::Relaxed) + bytes;
        debug!("[STATE] Relayed {} bytes (Lifetime total msgs: {}, Lifetime traffic: {} bytes)", bytes, total_msgs, total_bytes);

        self.emit_event(&serde_json::json!({
            "event": "traffic_recorded",
            "total_messages": total_msgs,
            "total_bytes": total_bytes,
        }).to_string());
    }

    pub fn record_user_traffic(&self, sender: &str, bytes: u64) {
        self.record_traffic(bytes);
        if !sender.trim().is_empty() {
            let _ = self.db.record_user_traffic_stat(sender, bytes);
        }
    }

    pub fn get_traffic_stats(&self) -> (u64, u64) {
        (
            self.total_messages_relayed.load(Ordering::Relaxed),
            self.total_bytes_relayed.load(Ordering::Relaxed),
        )
    }

    pub fn evaluate_version_policy(&self, app_version: Option<&str>, build_number: Option<u32>) -> VersionCheckResult {
        let client_ver_str = app_version.unwrap_or("0.0.0");
        let client_ver = ParsedVersion::parse(client_ver_str, build_number);

        let policy = self.version_policy.read().unwrap();
        let min_ver = ParsedVersion::parse(&policy.min_client_version, Some(policy.min_build_number));
        let latest_ver = ParsedVersion::parse(&policy.latest_client_version, Some(policy.latest_build_number));

        let display_client = if client_ver.build > 0 {
            format!("{}.{}.{}+{}", client_ver.major, client_ver.minor, client_ver.patch, client_ver.build)
        } else {
            format!("{}.{}.{}", client_ver.major, client_ver.minor, client_ver.patch)
        };

        let display_min = if policy.min_build_number > 0 {
            format!("{}+{}", policy.min_client_version, policy.min_build_number)
        } else {
            policy.min_client_version.clone()
        };

        let display_latest = if policy.latest_build_number > 0 {
            format!("{}+{}", policy.latest_client_version, policy.latest_build_number)
        } else {
            policy.latest_client_version.clone()
        };

        if client_ver < min_ver {
            VersionCheckResult::OutdatedMandatory {
                current_version: display_client.clone(),
                min_version: display_min.clone(),
                latest_version: display_latest,
                download_url: Some(policy.update_download_url.clone()),
                message: format!("Your Vexta client ({}) is outdated and no longer supported. Please update to {} or newer to continue.", display_client, display_min),
            }
        } else if client_ver < latest_ver {
            VersionCheckResult::UpdateAvailable {
                current_version: display_client,
                latest_version: display_latest.clone(),
                download_url: Some(policy.update_download_url.clone()),
                message: format!("A new version of Vexta ({}) is available. Update now for performance and security enhancements.", display_latest),
            }
        } else {
            VersionCheckResult::Supported
        }
    }

    pub fn get_version_policy(&self) -> VersionPolicy {
        self.version_policy.read().unwrap().clone()
    }

    pub fn set_version_policy(&self, policy: VersionPolicy) {
        info!("[SYSTEM] Updated version policy: min={}+{}, latest={}+{}", policy.min_client_version, policy.min_build_number, policy.latest_client_version, policy.latest_build_number);
        let mut p = self.version_policy.write().unwrap();
        *p = policy;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parsed_version_comparison() {
        let v1 = ParsedVersion::parse("0.0.12", Some(14));
        let v2 = ParsedVersion::parse("0.0.13-tauri", Some(15));
        let v3 = ParsedVersion::parse("v0.0.13+15", None);
        let v4 = ParsedVersion::parse("0.1.0", None);

        assert!(v1 < v2);
        assert_eq!(v2, v3);
        assert!(v3 < v4);
    }

    #[test]
    fn test_bounded_channel_backpressure() {
        use axum::extract::ws::Message;
        use tokio::sync::mpsc;

        let (tx, _rx) = mpsc::channel::<Message>(2);
        assert!(tx.try_send(Message::Text("first".into())).is_ok());
        assert!(tx.try_send(Message::Text("second".into())).is_ok());
        // Channel capacity is 2; 3rd item must fail non-blocking try_send
        let err = tx.try_send(Message::Text("third".into()));
        assert!(err.is_err());
    }
}

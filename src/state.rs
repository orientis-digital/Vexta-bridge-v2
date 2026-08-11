use crate::crypto::ServerCrypto;
use crate::db::DbManager;
use dashmap::DashMap;
use axum::extract::ws::Message;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

use std::sync::atomic::{AtomicU64, Ordering};

pub type Tx = UnboundedSender<Message>;

#[derive(Clone)]
pub struct AppState {
    pub db: DbManager,
    pub crypto: Arc<ServerCrypto>,
    // Lock-free lockless user routing table: username -> channel sender
    pub active_sessions: Arc<DashMap<String, Tx>>,
    pub start_time: i64,
    pub total_messages_relayed: Arc<AtomicU64>,
    pub total_bytes_relayed: Arc<AtomicU64>,
    pub broadcast_tx: tokio::sync::broadcast::Sender<String>,
}

impl AppState {
    pub fn new(db_path: &str) -> Self {
        let db = DbManager::new(db_path).expect("Failed to initialize SQLite database");
        let crypto = Arc::new(ServerCrypto::new_or_generate());
        let active_sessions = Arc::new(DashMap::new());
        let start_time = chrono::Utc::now().timestamp();
        let total_messages_relayed = Arc::new(AtomicU64::new(0));
        let total_bytes_relayed = Arc::new(AtomicU64::new(0));
        let (broadcast_tx, _) = tokio::sync::broadcast::channel(256);

        Self {
            db,
            crypto,
            active_sessions,
            start_time,
            total_messages_relayed,
            total_bytes_relayed,
            broadcast_tx,
        }
    }

    pub fn emit_event(&self, event_json: &str) {
        let _ = self.broadcast_tx.send(event_json.to_string());
    }

    pub fn register_session(&self, username: String, tx: Tx) {
        self.active_sessions.insert(username.clone(), tx);
        self.emit_event(&serde_json::json!({
            "event": "session_connected",
            "username": username,
            "active_count": self.active_sessions_count(),
        }).to_string());
    }

    pub fn unregister_session(&self, username: &str) {
        if self.active_sessions.remove(username).is_some() {
            self.emit_event(&serde_json::json!({
                "event": "session_disconnected",
                "username": username,
                "active_count": self.active_sessions_count(),
            }).to_string());
        }
    }

    pub fn send_to_user(&self, recipient: &str, msg: Message) -> bool {
        if let Some(tx) = self.active_sessions.get(recipient) {
            tx.send(msg).is_ok()
        } else {
            false
        }
    }

    pub fn active_sessions_count(&self) -> usize {
        self.active_sessions.len()
    }

    pub fn list_active_usernames(&self) -> Vec<String> {
        self.active_sessions.iter().map(|kv| kv.key().clone()).collect()
    }

    pub fn disconnect_session(&self, username: &str) -> bool {
        let removed = self.active_sessions.remove(username).is_some();
        if removed {
            self.emit_event(&serde_json::json!({
                "event": "session_disconnected",
                "username": username,
                "active_count": self.active_sessions_count(),
            }).to_string());
        }
        removed
    }

    pub fn record_traffic(&self, bytes: u64) {
        let total_msgs = self.total_messages_relayed.fetch_add(1, Ordering::Relaxed) + 1;
        let total_bytes = self.total_bytes_relayed.fetch_add(bytes, Ordering::Relaxed) + bytes;

        self.emit_event(&serde_json::json!({
            "event": "traffic_recorded",
            "total_messages": total_msgs,
            "total_bytes": total_bytes,
        }).to_string());
    }

    pub fn get_traffic_stats(&self) -> (u64, u64) {
        (
            self.total_messages_relayed.load(Ordering::Relaxed),
            self.total_bytes_relayed.load(Ordering::Relaxed),
        )
    }
}



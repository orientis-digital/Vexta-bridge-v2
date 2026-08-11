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
}

impl AppState {
    pub fn new(db_path: &str) -> Self {
        let db = DbManager::new(db_path).expect("Failed to initialize SQLite database");
        let crypto = Arc::new(ServerCrypto::new_or_generate());
        let active_sessions = Arc::new(DashMap::new());
        let start_time = chrono::Utc::now().timestamp();
        let total_messages_relayed = Arc::new(AtomicU64::new(0));
        let total_bytes_relayed = Arc::new(AtomicU64::new(0));

        Self {
            db,
            crypto,
            active_sessions,
            start_time,
            total_messages_relayed,
            total_bytes_relayed,
        }
    }

    pub fn register_session(&self, username: String, tx: Tx) {
        self.active_sessions.insert(username, tx);
    }

    pub fn unregister_session(&self, username: &str) {
        self.active_sessions.remove(username);
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
        self.active_sessions.remove(username).is_some()
    }

    pub fn record_traffic(&self, bytes: u64) {
        self.total_messages_relayed.fetch_add(1, Ordering::Relaxed);
        self.total_bytes_relayed.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn get_traffic_stats(&self) -> (u64, u64) {
        (
            self.total_messages_relayed.load(Ordering::Relaxed),
            self.total_bytes_relayed.load(Ordering::Relaxed),
        )
    }
}



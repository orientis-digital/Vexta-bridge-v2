use crate::crypto::ServerCrypto;
use crate::db::DbManager;
use dashmap::DashMap;
use axum::extract::ws::Message;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

pub type Tx = UnboundedSender<Message>;

#[derive(Clone)]
pub struct AppState {
    pub db: DbManager,
    pub crypto: Arc<ServerCrypto>,
    // Lock-free lockless user routing table: username -> channel sender
    pub active_sessions: Arc<DashMap<String, Tx>>,
}

impl AppState {
    pub fn new(db_path: &str) -> Self {
        let db = DbManager::new(db_path).expect("Failed to initialize SQLite database");
        let crypto = Arc::new(ServerCrypto::new_or_generate());
        let active_sessions = Arc::new(DashMap::new());

        Self {
            db,
            crypto,
            active_sessions,
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
}

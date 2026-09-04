use crate::models::{BlindMessage, BridgeMigrationData, FriendRequest, MigrationImportStats, PlatformStats, UserDevice, VextaUser};
use rusqlite::{params, Connection, Result};
use std::sync::{Arc, Mutex};
use tracing::{info, debug, warn};

fn clean_user(u: &str) -> String {
    u.trim().trim_start_matches('@').to_lowercase()
}

#[derive(Clone)]
pub struct DbManager {
    writer: Arc<Mutex<Connection>>,
    reader_pool: Arc<Mutex<Vec<Connection>>>,
    db_path: String,
}

pub struct ReaderGuard<'a> {
    conn: Option<Connection>,
    pool: &'a Mutex<Vec<Connection>>,
}

impl<'a> std::ops::Deref for ReaderGuard<'a> {
    type Target = Connection;
    fn deref(&self) -> &Self::Target {
        self.conn.as_ref().unwrap()
    }
}

impl<'a> std::ops::DerefMut for ReaderGuard<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.conn.as_mut().unwrap()
    }
}

impl<'a> Drop for ReaderGuard<'a> {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take() {
            if let Ok(mut pool) = self.pool.lock() {
                if pool.len() < 16 {
                    pool.push(conn);
                }
            }
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct AdminStats {
    pub total_users: i64,
    pub total_queued_offline_messages: i64,
    pub total_registered_devices: i64,
    pub total_announcements: i64,
    pub database_size_bytes: u64,
    pub wal_size_bytes: u64,
    pub provisioned_users: i64,
    pub locked_users: i64,
    pub users_with_vault: i64,
    pub users_with_prekey: i64,
    pub users_with_offline_msgs: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct OfflineQueueSummary {
    pub recipient: String,
    pub message_count: i64,
    pub oldest_timestamp: i64,
    pub newest_timestamp: i64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct Announcement {
    pub id: i64,
    pub message: String,
    pub created_at: i64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct BannedIp {
    pub ip: String,
    pub reason: String,
    pub banned_by: String,
    pub created_at: i64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct AuditLog {
    pub id: i64,
    pub action: String,
    pub target: String,
    pub details: String,
    pub timestamp: i64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct UserAnalytics {
    pub username: String,
    pub message_count: i64,
    pub byte_count: i64,
    pub last_active: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct DbHealth {
    pub page_count: i64,
    pub page_size: i64,
    pub total_size_bytes: i64,
    pub wal_size_bytes: u64,
    pub integrity_check: String,
}

#[allow(dead_code)]
impl DbManager {
    fn open_connection(db_path: &str) -> Result<Connection> {
        let conn = Connection::open(db_path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA busy_timeout = 5000;
             PRAGMA cache_size = -64000;
             PRAGMA temp_store = MEMORY;
             PRAGMA mmap_size = 268435456;
             PRAGMA foreign_keys = ON;",
        )?;
        Ok(conn)
    }

    #[inline]
    fn writer(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.writer.lock().unwrap()
    }

    #[inline]
    fn reader(&self) -> Result<ReaderGuard<'_>> {
        let maybe_conn = {
            let mut pool = self.reader_pool.lock().unwrap();
            pool.pop()
        };
        let conn = match maybe_conn {
            Some(c) => c,
            None => Self::open_connection(&self.db_path)?,
        };
        Ok(ReaderGuard {
            conn: Some(conn),
            pool: &self.reader_pool,
        })
    }

    pub fn new(db_path: &str) -> Result<Self> {
        let writer = Self::open_connection(db_path)?;

        // Initialize Schema
        writer.execute_batch(
            "CREATE TABLE IF NOT EXISTS users (
                username TEXT PRIMARY KEY,
                ed25519_pubkey TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                is_provisioned INTEGER NOT NULL DEFAULT 0,
                passcode TEXT,
                registration_lock_hash TEXT,
                encrypted_vault TEXT,
                encrypted_friend_roster TEXT,
                pre_key TEXT,
                pre_key_signature TEXT,
                auth_attempts INTEGER NOT NULL DEFAULT 0,
                locked_until INTEGER
            );

            CREATE TABLE IF NOT EXISTS friend_requests (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                sender TEXT NOT NULL,
                recipient TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                created_at INTEGER NOT NULL,
                UNIQUE(sender, recipient),
                FOREIGN KEY(sender) REFERENCES users(username) ON DELETE CASCADE,
                FOREIGN KEY(recipient) REFERENCES users(username) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS user_devices (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT NOT NULL,
                hardware_hash TEXT NOT NULL,
                device_name TEXT NOT NULL,
                device_type TEXT NOT NULL DEFAULT 'Desktop',
                registered_at INTEGER NOT NULL,
                last_active INTEGER NOT NULL,
                UNIQUE(username, hardware_hash),
                FOREIGN KEY(username) REFERENCES users(username) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS offline_messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                recipient TEXT NOT NULL,
                ciphertext TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                is_group INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY(recipient) REFERENCES users(username) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS announcements (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                message TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS ip_bans (
                ip TEXT PRIMARY KEY,
                reason TEXT NOT NULL,
                banned_by TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS admin_audit_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                action TEXT NOT NULL,
                target TEXT NOT NULL,
                details TEXT NOT NULL,
                timestamp INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS user_traffic_stats (
                username TEXT PRIMARY KEY,
                message_count INTEGER NOT NULL DEFAULT 0,
                byte_count INTEGER NOT NULL DEFAULT 0,
                last_active INTEGER NOT NULL
            );",
        )?;

        // Column migration for existing db files
        let _ = writer.execute("ALTER TABLE users ADD COLUMN encrypted_friend_roster TEXT", []);

        // Username normalization migration for existing data
        let _ = writer.execute_batch(
            "UPDATE users SET username = LOWER(LTRIM(username, '@'));
             UPDATE friend_requests SET sender = LOWER(LTRIM(sender, '@')), recipient = LOWER(LTRIM(recipient, '@'));
             UPDATE user_devices SET username = LOWER(LTRIM(username, '@'));
             UPDATE offline_messages SET recipient = LOWER(LTRIM(recipient, '@'));"
        );

        let mut readers = Vec::with_capacity(4);
        for _ in 0..2 {
            if let Ok(r) = Self::open_connection(db_path) {
                readers.push(r);
            }
        }

        info!("[DB] SQLite database initialized at '{}' (WAL mode, synchronous=NORMAL, foreign keys ON, read pool ready)", db_path);

        Ok(Self {
            writer: Arc::new(Mutex::new(writer)),
            reader_pool: Arc::new(Mutex::new(readers)),
            db_path: db_path.to_string(),
        })
    }

    pub fn save_or_update_user(&self, user: &VextaUser) -> Result<()> {
        let conn = self.writer();
        let clean_username = clean_user(&user.username);
        conn.execute(
            "INSERT INTO users (username, ed25519_pubkey, created_at, is_provisioned, passcode, registration_lock_hash, encrypted_vault, encrypted_friend_roster, pre_key, pre_key_signature, auth_attempts, locked_until)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(username) DO UPDATE SET
             ed25519_pubkey=excluded.ed25519_pubkey,
             encrypted_vault=COALESCE(excluded.encrypted_vault, users.encrypted_vault),
             encrypted_friend_roster=COALESCE(excluded.encrypted_friend_roster, users.encrypted_friend_roster)",
            params![
                clean_username,
                user.ed25519_pubkey,
                user.created_at,
                if user.is_provisioned { 1 } else { 0 },
                user.passcode,
                user.registration_lock_hash,
                user.encrypted_vault,
                user.encrypted_friend_roster,
                user.pre_key,
                user.pre_key_signature,
                user.auth_attempts,
                user.locked_until,
            ],
        )?;
        debug!("[DB] Saved/updated account for user '@{}'", clean_username);
        Ok(())
    }

    pub fn get_user(&self, username: &str) -> Result<Option<VextaUser>> {
        let conn = self.reader()?;
        let clean_username = clean_user(username);
        let mut stmt = conn.prepare(
            "SELECT username, ed25519_pubkey, created_at, is_provisioned, passcode, registration_lock_hash, encrypted_vault, encrypted_friend_roster, pre_key, pre_key_signature, auth_attempts, locked_until FROM users WHERE LOWER(LTRIM(username, '@')) = ?1",
        )?;

        let mut rows = stmt.query(params![clean_username])?;
        if let Some(row) = rows.next()? {
            let is_prov: i32 = row.get(3)?;
            Ok(Some(VextaUser {
                username: row.get(0)?,
                ed25519_pubkey: row.get(1)?,
                created_at: row.get(2)?,
                is_provisioned: is_prov == 1,
                passcode: row.get(4)?,
                registration_lock_hash: row.get(5)?,
                encrypted_vault: row.get(6)?,
                encrypted_friend_roster: row.get(7)?,
                pre_key: row.get(8)?,
                pre_key_signature: row.get(9)?,
                auth_attempts: row.get(10)?,
                locked_until: row.get(11)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn update_vault(&self, username: &str, vault_data: &str) -> Result<()> {
        let conn = self.writer();
        let clean_username = clean_user(username);
        conn.execute(
            "UPDATE users SET encrypted_vault = ?1 WHERE LOWER(LTRIM(username, '@')) = ?2",
            params![vault_data, clean_username],
        )?;
        Ok(())
    }

    pub fn update_friend_roster(&self, username: &str, roster_data: &str) -> Result<()> {
        let conn = self.writer();
        let clean_username = clean_user(username);
        conn.execute(
            "UPDATE users SET encrypted_friend_roster = ?1 WHERE LOWER(LTRIM(username, '@')) = ?2",
            params![roster_data, clean_username],
        )?;
        Ok(())
    }

    pub fn update_recovery_lock(&self, username: &str, lock_hash: &str) -> Result<()> {
        let conn = self.writer();
        let clean_username = clean_user(username);
        conn.execute(
            "UPDATE users SET registration_lock_hash = ?1 WHERE LOWER(LTRIM(username, '@')) = ?2",
            params![lock_hash, clean_username],
        )?;
        Ok(())
    }

    pub fn update_user_pubkey(&self, username: &str, new_pubkey: &str) -> Result<()> {
        let conn = self.writer();
        let clean_username = clean_user(username);
        conn.execute(
            "UPDATE users SET ed25519_pubkey = ?1 WHERE LOWER(LTRIM(username, '@')) = ?2",
            params![new_pubkey, clean_username],
        )?;
        Ok(())
    }

    pub fn delete_user(&self, username: &str) -> Result<()> {
        let conn = self.writer();
        let clean_username = clean_user(username);
        conn.execute("DELETE FROM users WHERE LOWER(LTRIM(username, '@')) = ?1", params![clean_username])?;
        Ok(())
    }

    // --- Admin Operations ---
    pub fn get_admin_stats(&self) -> Result<AdminStats> {
        let conn = self.reader()?;
        let total_users: i64 = conn.query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0)).unwrap_or(0);
        let total_queued_offline_messages: i64 = conn.query_row("SELECT COUNT(*) FROM offline_messages", [], |r| r.get(0)).unwrap_or(0);
        let total_registered_devices: i64 = conn.query_row("SELECT COUNT(*) FROM user_devices", [], |r| r.get(0)).unwrap_or(0);
        let total_announcements: i64 = conn.query_row("SELECT COUNT(*) FROM announcements", [], |r| r.get(0)).unwrap_or(0);
        let provisioned_users: i64 = conn.query_row("SELECT COUNT(*) FROM users WHERE is_provisioned = 1", [], |r| r.get(0)).unwrap_or(0);
        let now_ts = chrono::Utc::now().timestamp();
        let locked_users: i64 = conn.query_row("SELECT COUNT(*) FROM users WHERE (locked_until IS NOT NULL AND locked_until > ?1) OR auth_attempts >= 5", params![now_ts], |r| r.get(0)).unwrap_or(0);
        let users_with_vault: i64 = conn.query_row("SELECT COUNT(*) FROM users WHERE encrypted_vault IS NOT NULL AND LENGTH(encrypted_vault) > 0", [], |r| r.get(0)).unwrap_or(0);
        let users_with_prekey: i64 = conn.query_row("SELECT COUNT(*) FROM users WHERE pre_key IS NOT NULL AND LENGTH(pre_key) > 0", [], |r| r.get(0)).unwrap_or(0);
        let users_with_offline_msgs: i64 = conn.query_row("SELECT COUNT(DISTINCT recipient) FROM offline_messages", [], |r| r.get(0)).unwrap_or(0);

        let database_size_bytes = std::fs::metadata(&self.db_path).map(|m| m.len()).unwrap_or(0);
        let wal_path = format!("{}-wal", self.db_path);
        let wal_size_bytes = std::fs::metadata(&wal_path).map(|m| m.len()).unwrap_or(0);

        Ok(AdminStats {
            total_users,
            total_queued_offline_messages,
            total_registered_devices,
            total_announcements,
            database_size_bytes,
            wal_size_bytes,
            provisioned_users,
            locked_users,
            users_with_vault,
            users_with_prekey,
            users_with_offline_msgs,
        })
    }

    pub fn list_all_users(&self) -> Result<Vec<VextaUser>> {
        let conn = self.reader()?;
        let mut stmt = conn.prepare(
            "SELECT username, ed25519_pubkey, created_at, is_provisioned, passcode, registration_lock_hash, encrypted_vault, encrypted_friend_roster, pre_key, pre_key_signature, auth_attempts, locked_until FROM users ORDER BY created_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            let is_prov: i32 = row.get(3)?;
            Ok(VextaUser {
                username: row.get(0)?,
                ed25519_pubkey: row.get(1)?,
                created_at: row.get(2)?,
                is_provisioned: is_prov == 1,
                passcode: row.get(4)?,
                registration_lock_hash: row.get(5)?,
                encrypted_vault: row.get(6)?,
                encrypted_friend_roster: row.get(7)?,
                pre_key: row.get(8)?,
                pre_key_signature: row.get(9)?,
                auth_attempts: row.get(10)?,
                locked_until: row.get(11)?,
            })
        })?;

        let mut users = Vec::new();
        for r in rows {
            users.push(r?);
        }
        Ok(users)
    }

    pub fn create_announcement(&self, message: &str) -> Result<i64> {
        let conn = self.writer();
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "INSERT INTO announcements (message, created_at) VALUES (?1, ?2)",
            params![message, now],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn list_announcements(&self) -> Result<Vec<Announcement>> {
        let conn = self.reader()?;
        let mut stmt = conn.prepare(
            "SELECT id, message, created_at FROM announcements ORDER BY created_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(Announcement {
                id: row.get(0)?,
                message: row.get(1)?,
                created_at: row.get(2)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn delete_announcement(&self, id: i64) -> Result<()> {
        let conn = self.writer();
        conn.execute("DELETE FROM announcements WHERE id = ?1", params![id])?;
        Ok(())
    }

    // --- Friend Requests & Friends Roster ---
    pub fn create_friend_request(&self, sender: &str, recipient: &str) -> Result<i64> {
        let conn = self.writer();
        let now = chrono::Utc::now().timestamp();
        let clean_sender = clean_user(sender);
        let clean_recipient = clean_user(recipient);
        conn.execute(
            "INSERT INTO friend_requests (sender, recipient, status, created_at)
             VALUES (?1, ?2, 'pending', ?3)
             ON CONFLICT(sender, recipient) DO UPDATE SET status='pending', created_at=?3",
            params![clean_sender, clean_recipient, now],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn update_friend_request_status(&self, req_id: i64, status: &str) -> Result<()> {
        let conn = self.writer();
        conn.execute(
            "UPDATE friend_requests SET status = ?1 WHERE id = ?2",
            params![status, req_id],
        )?;
        Ok(())
    }

    pub fn update_friend_request_status_by_user(&self, user: &str, other_user: &str, status: &str) -> Result<()> {
        let conn = self.writer();
        let clean_u = clean_user(user);
        let clean_other = clean_user(other_user);
        conn.execute(
            "UPDATE friend_requests SET status = ?1 
             WHERE ((LOWER(LTRIM(sender, '@')) = ?2 AND LOWER(LTRIM(recipient, '@')) = ?3) OR 
                    (LOWER(LTRIM(sender, '@')) = ?3 AND LOWER(LTRIM(recipient, '@')) = ?2))",
            params![status, clean_u, clean_other],
        )?;
        Ok(())
    }

    pub fn list_friends(&self, username: &str) -> Result<Vec<String>> {
        let conn = self.reader()?;
        let clean_username = clean_user(username);
        let mut stmt = conn.prepare(
            "SELECT CASE WHEN LOWER(LTRIM(sender, '@')) = ?1 THEN recipient ELSE sender END AS friend
             FROM friend_requests
             WHERE (LOWER(LTRIM(sender, '@')) = ?1 OR LOWER(LTRIM(recipient, '@')) = ?1) AND status = 'accepted'",
        )?;

        let rows = stmt.query_map(params![clean_username], |row| row.get(0))?;
        let mut friends = Vec::new();
        for r in rows {
            friends.push(r?);
        }
        Ok(friends)
    }

    pub fn list_pending_requests(&self, username: &str) -> Result<Vec<FriendRequest>> {
        let conn = self.reader()?;
        let clean_username = clean_user(username);
        let mut stmt = conn.prepare(
            "SELECT id, sender, recipient, status, created_at
             FROM friend_requests 
             WHERE (LOWER(LTRIM(recipient, '@')) = ?1 OR LOWER(LTRIM(sender, '@')) = ?1) AND status = 'pending'",
        )?;

        let rows = stmt.query_map(params![clean_username], |row| {
            Ok(FriendRequest {
                id: row.get(0)?,
                sender: row.get(1)?,
                recipient: row.get(2)?,
                status: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;

        let mut reqs = Vec::new();
        for r in rows {
            reqs.push(r?);
        }
        Ok(reqs)
    }

    /// Returns the other party's username for a given request id.
    /// If current_user is the recipient, returns the sender, and vice-versa.
    pub fn get_friend_request_other_party(&self, req_id: i64, current_user: &str) -> Result<Option<String>> {
        let conn = self.reader()?;
        let clean_current = clean_user(current_user);
        let result = conn.query_row(
            "SELECT sender, recipient FROM friend_requests WHERE id = ?1",
            params![req_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        );
        match result {
            Ok((sender, recipient)) => {
                let clean_sender = clean_user(&sender);
                if clean_sender == clean_current {
                    Ok(Some(recipient))
                } else {
                    Ok(Some(sender))
                }
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Returns the sender of a pending request between two users.
    pub fn get_friend_request_sender_by_users(&self, user_a: &str, user_b: &str) -> Result<Option<String>> {
        let conn = self.reader()?;
        let clean_a = clean_user(user_a);
        let clean_b = clean_user(user_b);
        let result = conn.query_row(
            "SELECT sender FROM friend_requests
             WHERE ((LOWER(LTRIM(sender, '@')) = ?1 AND LOWER(LTRIM(recipient, '@')) = ?2) OR 
                    (LOWER(LTRIM(sender, '@')) = ?2 AND LOWER(LTRIM(recipient, '@')) = ?1))
             AND status = 'pending'",
            params![clean_a, clean_b],
            |row| row.get::<_, String>(0),
        );
        match result {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn remove_friend(&self, username: &str, friend_username: &str) -> Result<()> {
        let conn = self.writer();
        let clean_username = clean_user(username);
        let clean_friend = clean_user(friend_username);
        conn.execute(
            "DELETE FROM friend_requests
             WHERE ((LOWER(LTRIM(sender, '@')) = ?1 AND LOWER(LTRIM(recipient, '@')) = ?2) OR 
                    (LOWER(LTRIM(sender, '@')) = ?2 AND LOWER(LTRIM(recipient, '@')) = ?1))",
            params![clean_username, clean_friend],
        )?;
        Ok(())
    }

    // --- Device Management ---
    pub fn register_or_update_device(
        &self,
        username: &str,
        hardware_hash: &str,
        device_name: &str,
    ) -> Result<()> {
        let conn = self.writer();
        let now = chrono::Utc::now().timestamp();
        let clean_username = clean_user(username);
        conn.execute(
            "INSERT INTO user_devices (username, hardware_hash, device_name, device_type, registered_at, last_active)
             VALUES (?1, ?2, ?3, 'Desktop', ?4, ?4)
             ON CONFLICT(username, hardware_hash) DO UPDATE SET
             last_active = ?4, device_name = ?3",
            params![clean_username, hardware_hash, device_name, now],
        )?;
        Ok(())
    }

    pub fn list_devices(&self, username: &str) -> Result<Vec<UserDevice>> {
        let conn = self.reader()?;
        let clean_username = clean_user(username);
        let mut stmt = conn.prepare(
            "SELECT id, username, hardware_hash, device_name, device_type, registered_at, last_active
             FROM user_devices WHERE LOWER(LTRIM(username, '@')) = ?1",
        )?;

        let rows = stmt.query_map(params![clean_username], |row| {
            Ok(UserDevice {
                id: row.get(0)?,
                username: row.get(1)?,
                hardware_hash: row.get(2)?,
                device_name: row.get(3)?,
                device_type: row.get(4)?,
                registered_at: row.get(5)?,
                last_active: row.get(6)?,
            })
        })?;

        let mut devices = Vec::new();
        for r in rows {
            devices.push(r?);
        }
        Ok(devices)
    }

    pub fn revoke_device(&self, username: &str, hardware_hash: &str) -> Result<()> {
        let conn = self.writer();
        let clean_username = clean_user(username);
        conn.execute(
            "DELETE FROM user_devices WHERE LOWER(LTRIM(username, '@')) = ?1 AND hardware_hash = ?2",
            params![clean_username, hardware_hash],
        )?;
        Ok(())
    }

    // --- Offline Messages ---
    pub fn enqueue_offline_message(
        &self,
        recipient: &str,
        ciphertext: &str,
        timestamp: i64,
        is_group: bool,
    ) -> Result<i64> {
        let conn = self.writer();
        let clean_recipient = clean_user(recipient);
        conn.execute(
            "INSERT INTO offline_messages (recipient, ciphertext, timestamp, is_group)
             VALUES (?1, ?2, ?3, ?4)",
            params![clean_recipient, ciphertext, timestamp, if is_group { 1 } else { 0 }],
        )?;
        let msg_id = conn.last_insert_rowid();
        debug!("[DB] Enqueued blind offline message #{} for recipient '@{}' (group: {})", msg_id, clean_recipient, is_group);
        Ok(msg_id)
    }

    pub fn fetch_and_clear_offline_messages(&self, recipient: &str) -> Result<Vec<BlindMessage>> {
        let conn = self.writer();
        let clean_recipient = clean_user(recipient);
        let mut stmt = conn.prepare(
            "SELECT id, recipient, ciphertext, timestamp, is_group
             FROM offline_messages WHERE LOWER(LTRIM(recipient, '@')) = ?1 ORDER BY timestamp ASC",
        )?;

        let rows = stmt.query_map(params![clean_recipient], |row| {
            let is_group_int: i32 = row.get(4)?;
            Ok(BlindMessage {
                id: row.get(0)?,
                recipient: row.get(1)?,
                ciphertext: row.get(2)?,
                timestamp: row.get(3)?,
                is_group: is_group_int == 1,
            })
        })?;

        let mut msgs = Vec::new();
        for r in rows {
            msgs.push(r?);
        }

        conn.execute(
            "DELETE FROM offline_messages WHERE LOWER(LTRIM(recipient, '@')) = ?1",
            params![clean_recipient],
        )?;

        debug!("[DB] Fetched and cleared {} blind offline messages for recipient '@{}'", msgs.len(), clean_recipient);
        Ok(msgs)
    }

    pub fn get_offline_messages_summary(&self) -> Result<Vec<OfflineQueueSummary>> {
        let conn = self.reader()?;
        let mut stmt = conn.prepare(
            "SELECT recipient, COUNT(*) as msg_count, MIN(timestamp) as min_ts, MAX(timestamp) as max_ts
             FROM offline_messages
             GROUP BY recipient
             ORDER BY msg_count DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(OfflineQueueSummary {
                recipient: row.get(0)?,
                message_count: row.get(1)?,
                oldest_timestamp: row.get(2)?,
                newest_timestamp: row.get(3)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn unlock_user_account(&self, username: &str) -> Result<()> {
        let conn = self.writer();
        let clean_username = clean_user(username);
        conn.execute(
            "UPDATE users SET auth_attempts = 0, locked_until = NULL WHERE LOWER(LTRIM(username, '@')) = ?1",
            params![clean_username],
        )?;
        Ok(())
    }

    pub fn record_failed_auth(&self, username: &str) -> Result<i64> {
        let conn = self.writer();
        let clean_username = clean_user(username);
        let now_ts = chrono::Utc::now().timestamp();

        let attempts: i64 = conn.query_row(
            "SELECT auth_attempts FROM users WHERE LOWER(LTRIM(username, '@')) = ?1",
            params![clean_username],
            |r| r.get(0),
        ).unwrap_or(0) + 1;

        // Lock account for 15 minutes (900 seconds) if 5 consecutive failures occur
        let locked_until = if attempts >= 5 {
            Some(now_ts + 900)
        } else {
            None
        };

        conn.execute(
            "UPDATE users SET auth_attempts = ?1, locked_until = COALESCE(?2, locked_until) WHERE LOWER(LTRIM(username, '@')) = ?3",
            params![attempts, locked_until, clean_username],
        )?;

        if attempts >= 5 {
            warn!("[DB] User '@{}' locked out for 15 minutes after {} failed auth attempts", clean_username, attempts);
        } else {
            debug!("[DB] Recorded failed auth attempt #{}/5 for user '@{}'", attempts, clean_username);
        }

        Ok(attempts)
    }

    pub fn is_user_locked(&self, username: &str) -> bool {
        let conn = match self.reader() {
            Ok(c) => c,
            Err(_) => return false,
        };
        let clean_username = clean_user(username);
        let now_ts = chrono::Utc::now().timestamp();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM users WHERE LOWER(LTRIM(username, '@')) = ?1 AND (locked_until > ?2 OR auth_attempts >= 5)",
            params![clean_username, now_ts],
            |r| r.get(0),
        ).unwrap_or(0);
        count > 0
    }

    pub fn purge_stale_offline_messages(&self, older_than_timestamp: i64) -> Result<usize> {
        let conn = self.writer();
        let deleted = conn.execute(
            "DELETE FROM offline_messages WHERE timestamp < ?1",
            params![older_than_timestamp],
        )?;
        info!("[DB] Purged {} stale offline messages older than timestamp {}", deleted, older_than_timestamp);
        Ok(deleted)
    }

    // ── IP Firewall Methods ──
    pub fn ban_ip(&self, ip: &str, reason: &str, banned_by: &str) -> Result<()> {
        let conn = self.writer();
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "INSERT OR REPLACE INTO ip_bans (ip, reason, banned_by, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![ip.trim(), reason, banned_by, now],
        )?;
        info!("[DB] Stored IP ban for '{}' (Reason: '{}', Banned By: '{}')", ip.trim(), reason, banned_by);
        Ok(())
    }

    pub fn unban_ip(&self, ip: &str) -> Result<()> {
        let conn = self.writer();
        conn.execute("DELETE FROM ip_bans WHERE ip = ?1", params![ip.trim()])?;
        info!("[DB] Removed IP ban for '{}'", ip.trim());
        Ok(())
    }

    pub fn list_banned_ips(&self) -> Result<Vec<BannedIp>> {
        let conn = self.reader()?;
        let mut stmt = conn.prepare("SELECT ip, reason, banned_by, created_at FROM ip_bans ORDER BY created_at DESC")?;
        let rows = stmt.query_map([], |row| {
            Ok(BannedIp {
                ip: row.get(0)?,
                reason: row.get(1)?,
                banned_by: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn is_ip_banned(&self, ip: &str) -> Result<bool> {
        let conn = self.reader()?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM ip_bans WHERE ip = ?1",
            params![ip.trim()],
            |r| r.get(0),
        ).unwrap_or(0);
        Ok(count > 0)
    }

    // ── Audit Log Methods ──
    pub fn log_audit_action(&self, action: &str, target: &str, details: &str) -> Result<()> {
        let conn = self.writer();
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "INSERT INTO admin_audit_logs (action, target, details, timestamp) VALUES (?1, ?2, ?3, ?4)",
            params![action, target, details, now],
        )?;
        Ok(())
    }

    pub fn list_audit_logs(&self, limit: usize) -> Result<Vec<AuditLog>> {
        let conn = self.reader()?;
        let mut stmt = conn.prepare(
            "SELECT id, action, target, details, timestamp FROM admin_audit_logs ORDER BY timestamp DESC LIMIT ?1"
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(AuditLog {
                id: row.get(0)?,
                action: row.get(1)?,
                target: row.get(2)?,
                details: row.get(3)?,
                timestamp: row.get(4)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    // ── Per-User Traffic Analytics Methods ──
    pub fn record_user_traffic_stat(&self, username: &str, bytes: u64) -> Result<()> {
        let conn = self.writer();
        let clean_username = clean_user(username);
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "INSERT INTO user_traffic_stats (username, message_count, byte_count, last_active)
             VALUES (?1, 1, ?2, ?3)
             ON CONFLICT(username) DO UPDATE SET
               message_count = message_count + 1,
               byte_count = byte_count + ?2,
               last_active = ?3",
            params![clean_username, bytes as i64, now],
        )?;
        Ok(())
    }

    pub fn get_top_user_analytics(&self, limit: usize) -> Result<Vec<UserAnalytics>> {
        let conn = self.reader()?;
        let mut stmt = conn.prepare(
            "SELECT username, message_count, byte_count, last_active FROM user_traffic_stats ORDER BY byte_count DESC LIMIT ?1"
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(UserAnalytics {
                username: row.get(0)?,
                message_count: row.get(1)?,
                byte_count: row.get(2)?,
                last_active: row.get(3)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    // ── Database Maintenance & Vacuum ──
    pub fn ping(&self) -> Result<()> {
        let conn = self.reader()?;
        conn.query_row("SELECT 1", [], |_| Ok(()))?;
        Ok(())
    }

    pub fn vacuum_database(&self) -> Result<()> {
        let conn = self.writer();
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); VACUUM;")?;
        info!("[DB] Executed PRAGMA wal_checkpoint(TRUNCATE) and VACUUM on '{}'", self.db_path);
        Ok(())
    }

    pub fn get_db_health_fast(&self) -> Result<DbHealth> {
        let conn = self.reader()?;
        let page_count: i64 = conn.query_row("PRAGMA page_count", [], |r| r.get(0)).unwrap_or(0);
        let page_size: i64 = conn.query_row("PRAGMA page_size", [], |r| r.get(0)).unwrap_or(0);
        let total_size_bytes = page_count * page_size;

        let wal_path = format!("{}-wal", self.db_path);
        let wal_size_bytes = std::fs::metadata(&wal_path).map(|m| m.len()).unwrap_or(0);

        Ok(DbHealth {
            page_count,
            page_size,
            total_size_bytes,
            wal_size_bytes,
            integrity_check: "ok".into(),
        })
    }

    pub fn get_db_health(&self) -> Result<DbHealth> {
        let conn = self.reader()?;
        let page_count: i64 = conn.query_row("PRAGMA page_count", [], |r| r.get(0)).unwrap_or(0);
        let page_size: i64 = conn.query_row("PRAGMA page_size", [], |r| r.get(0)).unwrap_or(0);
        let total_size_bytes = page_count * page_size;
        let integrity: String = conn.query_row("PRAGMA integrity_check", [], |r| r.get(0)).unwrap_or_else(|_| "ok".into());

        let wal_path = format!("{}-wal", self.db_path);
        let wal_size_bytes = std::fs::metadata(&wal_path).map(|m| m.len()).unwrap_or(0);

        Ok(DbHealth {
            page_count,
            page_size,
            total_size_bytes,
            wal_size_bytes,
            integrity_check: integrity,
        })
    }

    pub fn list_all_devices(&self) -> Result<Vec<UserDevice>> {
        let conn = self.reader()?;
        let mut stmt = conn.prepare(
            "SELECT id, username, hardware_hash, device_name, device_type, registered_at, last_active FROM user_devices ORDER BY last_active DESC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(UserDevice {
                id: row.get(0)?,
                username: row.get(1)?,
                hardware_hash: row.get(2)?,
                device_name: row.get(3)?,
                device_type: row.get(4)?,
                registered_at: row.get(5)?,
                last_active: row.get(6)?,
            })
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn list_all_friend_requests(&self) -> Result<Vec<FriendRequest>> {
        let conn = self.reader()?;
        let mut stmt = conn.prepare(
            "SELECT id, sender, recipient, status, created_at FROM friend_requests"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(FriendRequest {
                id: row.get(0)?,
                sender: row.get(1)?,
                recipient: row.get(2)?,
                status: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn export_migration_data(&self, server_version: &str, policy: Option<crate::state::VersionPolicy>) -> Result<BridgeMigrationData> {
        let users = self.list_all_users()?;
        let devices = self.list_all_devices()?;
        let friend_requests = self.list_all_friend_requests()?;
        let announcements = self.list_announcements()?;
        let banned_ips = self.list_banned_ips()?;
        let exported_at = chrono::Utc::now().timestamp();

        info!("[DB] Exported bridge migration data (Users: {}, Devices: {}, Requests: {}, Announcements: {}, Bans: {})",
            users.len(), devices.len(), friend_requests.len(), announcements.len(), banned_ips.len());

        Ok(BridgeMigrationData {
            exported_at,
            server_version: server_version.to_string(),
            users,
            devices,
            friend_requests,
            announcements,
            banned_ips,
            version_policy: policy,
        })
    }

    pub fn import_migration_data(&self, data: &BridgeMigrationData) -> Result<MigrationImportStats> {
        let mut conn = self.writer();
        let tx = conn.transaction()?;

        let mut imported_users = 0;
        let mut imported_devices = 0;
        let mut imported_friend_requests = 0;
        let mut imported_announcements = 0;
        let mut imported_banned_ips = 0;

        // 1. Users
        for user in &data.users {
            let clean_username = clean_user(&user.username);
            tx.execute(
                "INSERT INTO users (username, ed25519_pubkey, created_at, is_provisioned, passcode, registration_lock_hash, encrypted_vault, encrypted_friend_roster, pre_key, pre_key_signature, auth_attempts, locked_until)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                 ON CONFLICT(username) DO UPDATE SET
                 ed25519_pubkey=excluded.ed25519_pubkey,
                 is_provisioned=excluded.is_provisioned,
                 passcode=COALESCE(excluded.passcode, users.passcode),
                 registration_lock_hash=COALESCE(excluded.registration_lock_hash, users.registration_lock_hash),
                 encrypted_vault=COALESCE(excluded.encrypted_vault, users.encrypted_vault),
                 encrypted_friend_roster=COALESCE(excluded.encrypted_friend_roster, users.encrypted_friend_roster),
                 pre_key=COALESCE(excluded.pre_key, users.pre_key),
                 pre_key_signature=COALESCE(excluded.pre_key_signature, users.pre_key_signature),
                 auth_attempts=excluded.auth_attempts,
                 locked_until=excluded.locked_until",
                params![
                    clean_username,
                    user.ed25519_pubkey,
                    user.created_at,
                    if user.is_provisioned { 1 } else { 0 },
                    user.passcode,
                    user.registration_lock_hash,
                    user.encrypted_vault,
                    user.encrypted_friend_roster,
                    user.pre_key,
                    user.pre_key_signature,
                    user.auth_attempts,
                    user.locked_until,
                ],
            )?;
            imported_users += 1;
        }

        // 2. Devices
        for dev in &data.devices {
            let clean_username = clean_user(&dev.username);
            tx.execute(
                "INSERT INTO user_devices (username, hardware_hash, device_name, device_type, registered_at, last_active)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(username, hardware_hash) DO UPDATE SET
                 device_name=excluded.device_name,
                 device_type=excluded.device_type,
                 last_active=excluded.last_active",
                params![
                    clean_username,
                    dev.hardware_hash,
                    dev.device_name,
                    dev.device_type,
                    dev.registered_at,
                    dev.last_active,
                ],
            )?;
            imported_devices += 1;
        }

        // 3. Friend Requests
        for req in &data.friend_requests {
            let clean_sender = clean_user(&req.sender);
            let clean_recipient = clean_user(&req.recipient);
            tx.execute(
                "INSERT INTO friend_requests (sender, recipient, status, created_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(sender, recipient) DO UPDATE SET
                 status=excluded.status",
                params![
                    clean_sender,
                    clean_recipient,
                    req.status,
                    req.created_at,
                ],
            )?;
            imported_friend_requests += 1;
        }

        // 4. Announcements
        for ann in &data.announcements {
            tx.execute(
                "INSERT INTO announcements (message, created_at)
                 VALUES (?1, ?2)",
                params![ann.message, ann.created_at],
            )?;
            imported_announcements += 1;
        }

        // 5. Banned IPs
        for b in &data.banned_ips {
            tx.execute(
                "INSERT INTO ip_bans (ip, reason, banned_by, created_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(ip) DO UPDATE SET
                 reason=excluded.reason,
                 banned_by=excluded.banned_by",
                params![b.ip, b.reason, b.banned_by, b.created_at],
            )?;
            imported_banned_ips += 1;
        }

        tx.commit()?;

        info!("[DB] Successfully imported migration data (Users: {}, Devices: {}, Requests: {}, Announcements: {}, Bans: {})",
            imported_users, imported_devices, imported_friend_requests, imported_announcements, imported_banned_ips);

        Ok(MigrationImportStats {
            imported_users,
            imported_devices,
            imported_friend_requests,
            imported_announcements,
            imported_banned_ips,
        })
    }

    pub fn get_platform_distribution(&self) -> Result<Vec<PlatformStats>> {
        let devices = self.list_all_devices()?;
        let total = devices.len();
        if total == 0 {
            return Ok(vec![
                PlatformStats { platform: "Windows".into(), count: 0, percentage: 0.0 },
                PlatformStats { platform: "Linux".into(), count: 0, percentage: 0.0 },
                PlatformStats { platform: "macOS".into(), count: 0, percentage: 0.0 },
                PlatformStats { platform: "Android".into(), count: 0, percentage: 0.0 },
                PlatformStats { platform: "iOS".into(), count: 0, percentage: 0.0 },
                PlatformStats { platform: "Other".into(), count: 0, percentage: 0.0 },
            ]);
        }

        let mut windows = 0;
        let mut macos = 0;
        let mut linux = 0;
        let mut android = 0;
        let mut ios = 0;
        let mut other = 0;

        for d in &devices {
            let name_lower = d.device_name.to_lowercase();
            let type_lower = d.device_type.to_lowercase();

            if name_lower.contains("win") || type_lower.contains("win") {
                windows += 1;
            } else if name_lower.contains("mac") || name_lower.contains("apple") || type_lower.contains("mac") {
                macos += 1;
            } else if name_lower.contains("android") || type_lower.contains("android") {
                android += 1;
            } else if name_lower.contains("ios") || name_lower.contains("iphone") || name_lower.contains("ipad") {
                ios += 1;
            } else if name_lower.contains("linux") || type_lower.contains("linux") {
                linux += 1;
            } else {
                other += 1;
            }
        }

        let make_stat = |name: &str, count: usize| PlatformStats {
            platform: name.to_string(),
            count,
            percentage: ((count as f64 / total as f64) * 100.0 * 10.0).round() / 10.0,
        };

        Ok(vec![
            make_stat("Windows", windows),
            make_stat("Linux", linux),
            make_stat("macOS", macos),
            make_stat("Android", android),
            make_stat("iOS", ios),
            make_stat("Other", other),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_ping_and_health() {
        let temp_dir = std::env::temp_dir();
        let db_file = temp_dir.join(format!("test_health_{}.db", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)));
        let db_path = db_file.to_str().unwrap();

        let db = DbManager::new(db_path).expect("Failed to create test DB");
        assert!(db.ping().is_ok());

        let health = db.get_db_health().expect("Failed to get DB health");
        assert_eq!(health.integrity_check, "ok");
        assert!(health.page_size > 0);
        assert!(health.page_count > 0);
        assert!(health.total_size_bytes > 0);

        assert!(db.vacuum_database().is_ok());

        let stats = db.get_admin_stats().expect("Failed to get admin stats");
        assert_eq!(stats.total_users, 0);

        // Cleanup
        let _ = std::fs::remove_file(&db_file);
        let _ = std::fs::remove_file(format!("{}-wal", db_path));
        let _ = std::fs::remove_file(format!("{}-shm", db_path));
    }

    #[test]
    fn test_sealed_offline_messages() {
        let temp_dir = std::env::temp_dir();
        let db_file = temp_dir.join(format!("test_sealed_msgs_{}.db", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)));
        let db_path = db_file.to_str().unwrap();

        let db = DbManager::new(db_path).expect("Failed to create test DB");

        // Register recipient user first (required by foreign key constraint)
        db.save_or_update_user(&VextaUser {
            username: "alice".to_string(),
            ed25519_pubkey: "test_pubkey".to_string(),
            created_at: 1725000000,
            is_provisioned: true,
            passcode: None,
            registration_lock_hash: None,
            encrypted_vault: None,
            encrypted_friend_roster: None,
            pre_key: None,
            pre_key_signature: None,
            auth_attempts: 0,
            locked_until: None,
        }).expect("Failed to register test user alice");

        // Enqueue sealed offline message without sender
        let msg_id = db.enqueue_offline_message("alice", "sealed_ciphertext_payload_xyz", 1725000000, false)
            .expect("Failed to enqueue offline message");
        assert!(msg_id > 0);

        // Fetch and clear
        let msgs = db.fetch_and_clear_offline_messages("alice").expect("Failed to fetch offline messages");
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].recipient, "alice");
        assert_eq!(msgs[0].ciphertext, "sealed_ciphertext_payload_xyz");
        assert_eq!(msgs[0].timestamp, 1725000000);
        assert_eq!(msgs[0].is_group, false);

        // Verify cleared
        let msgs_empty = db.fetch_and_clear_offline_messages("alice").expect("Failed to fetch cleared msgs");
        assert_eq!(msgs_empty.len(), 0);

        // Cleanup
        let _ = std::fs::remove_file(&db_file);
        let _ = std::fs::remove_file(format!("{}-wal", db_path));
        let _ = std::fs::remove_file(format!("{}-shm", db_path));
    }

    #[test]
    fn test_export_and_import_migration_data() {
        let temp_dir = std::env::temp_dir();
        let db_file_a = temp_dir.join(format!("test_mig_a_{}.db", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)));
        let db_file_b = temp_dir.join(format!("test_mig_b_{}.db", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)));
        let path_a = db_file_a.to_str().unwrap();
        let path_b = db_file_b.to_str().unwrap();

        let db_a = DbManager::new(path_a).expect("Failed to create DB A");
        let db_b = DbManager::new(path_b).expect("Failed to create DB B");

        // Seed data in A
        db_a.save_or_update_user(&VextaUser {
            username: "bob".to_string(),
            ed25519_pubkey: "bob_pubkey_123".to_string(),
            created_at: 1725000000,
            is_provisioned: true,
            passcode: None,
            registration_lock_hash: None,
            encrypted_vault: Some("vault_blob".into()),
            encrypted_friend_roster: Some("roster_blob".into()),
            pre_key: None,
            pre_key_signature: None,
            auth_attempts: 0,
            locked_until: None,
        }).unwrap();

        db_a.register_or_update_device("bob", "hw_hash_99", "Windows 11 Laptop").unwrap();
        db_a.create_announcement("Test migration announcement").unwrap();
        db_a.ban_ip("192.168.1.100", "Abuse", "Admin").unwrap();

        // Export from A
        let export_data = db_a.export_migration_data("v0.0.1", None).expect("Export failed");
        assert_eq!(export_data.users.len(), 1);
        assert_eq!(export_data.devices.len(), 1);
        assert_eq!(export_data.announcements.len(), 1);
        assert_eq!(export_data.banned_ips.len(), 1);

        // Import into B
        let import_stats = db_b.import_migration_data(&export_data).expect("Import failed");
        assert_eq!(import_stats.imported_users, 1);
        assert_eq!(import_stats.imported_devices, 1);
        assert_eq!(import_stats.imported_announcements, 1);
        assert_eq!(import_stats.imported_banned_ips, 1);

        // Verify data in B
        let imported_user = db_b.get_user("bob").unwrap().expect("User bob missing in DB B");
        assert_eq!(imported_user.ed25519_pubkey, "bob_pubkey_123");
        assert_eq!(imported_user.encrypted_vault, Some("vault_blob".into()));

        let platform_dist = db_b.get_platform_distribution().unwrap();
        assert!(platform_dist.iter().any(|p| p.platform == "Windows" && p.count == 1));

        // Cleanup
        let _ = std::fs::remove_file(&db_file_a);
        let _ = std::fs::remove_file(format!("{}-wal", path_a));
        let _ = std::fs::remove_file(format!("{}-shm", path_a));
        let _ = std::fs::remove_file(&db_file_b);
        let _ = std::fs::remove_file(format!("{}-wal", path_b));
        let _ = std::fs::remove_file(format!("{}-shm", path_b));
    }

    #[test]
    fn test_concurrent_readers_and_writer() {
        let temp_dir = std::env::temp_dir();
        let db_file = temp_dir.join(format!("test_concurrency_{}.db", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)));
        let db_path = db_file.to_str().unwrap();

        let db = DbManager::new(db_path).expect("Failed to create test DB");

        // Seed a user
        db.save_or_update_user(&VextaUser {
            username: "charlie".to_string(),
            ed25519_pubkey: "charlie_key".to_string(),
            created_at: 1725000000,
            is_provisioned: false,
            passcode: None,
            registration_lock_hash: None,
            encrypted_vault: None,
            encrypted_friend_roster: None,
            pre_key: None,
            pre_key_signature: None,
            auth_attempts: 0,
            locked_until: None,
        }).unwrap();

        let mut handles = Vec::new();
        // Spawn concurrent readers
        for i in 0..5 {
            let db_clone = db.clone();
            handles.push(std::thread::spawn(move || {
                for _ in 0..20 {
                    let user = db_clone.get_user("charlie").unwrap();
                    assert!(user.is_some());
                    assert_eq!(user.unwrap().ed25519_pubkey, "charlie_key");
                }
                i
            }));
        }

        // Concurrently write
        for j in 0..10 {
            db.create_announcement(&format!("Announcement {}", j)).unwrap();
        }

        for h in handles {
            h.join().unwrap();
        }

        let announcements = db.list_announcements().unwrap();
        assert_eq!(announcements.len(), 10);

        // Cleanup
        let _ = std::fs::remove_file(&db_file);
        let _ = std::fs::remove_file(format!("{}-wal", db_path));
        let _ = std::fs::remove_file(format!("{}-shm", db_path));
    }
}


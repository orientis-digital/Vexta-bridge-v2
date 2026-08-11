use crate::models::{BlindMessage, FriendRequest, UserDevice, VextaUser};
use rusqlite::{params, Connection, Result};
use std::sync::{Arc, Mutex};

fn clean_user(u: &str) -> String {
    u.trim().trim_start_matches('@').to_lowercase()
}

#[derive(Clone)]
pub struct DbManager {
    conn: Arc<Mutex<Connection>>,
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

impl DbManager {
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;

        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA busy_timeout = 5000;
             PRAGMA foreign_keys = ON;",
        )?;

        // Initialize Schema
        conn.execute_batch(
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
                sender TEXT NOT NULL,
                ciphertext TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                is_group INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY(recipient) REFERENCES users(username) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS announcements (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                message TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );",
        )?;

        // Column migration for existing db files
        let _ = conn.execute("ALTER TABLE users ADD COLUMN encrypted_friend_roster TEXT", []);

        // Username normalization migration for existing data
        let _ = conn.execute_batch(
            "UPDATE users SET username = LOWER(LTRIM(username, '@'));
             UPDATE friend_requests SET sender = LOWER(LTRIM(sender, '@')), recipient = LOWER(LTRIM(recipient, '@'));
             UPDATE user_devices SET username = LOWER(LTRIM(username, '@'));
             UPDATE offline_messages SET recipient = LOWER(LTRIM(recipient, '@')), sender = LOWER(LTRIM(sender, '@'));"
        );

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn save_or_update_user(&self, user: &VextaUser) -> Result<()> {
        let conn = self.conn.lock().unwrap();
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
        Ok(())
    }

    pub fn get_user(&self, username: &str) -> Result<Option<VextaUser>> {
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
        let clean_username = clean_user(username);
        conn.execute(
            "UPDATE users SET encrypted_vault = ?1 WHERE LOWER(LTRIM(username, '@')) = ?2",
            params![vault_data, clean_username],
        )?;
        Ok(())
    }

    pub fn update_friend_roster(&self, username: &str, roster_data: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let clean_username = clean_user(username);
        conn.execute(
            "UPDATE users SET encrypted_friend_roster = ?1 WHERE LOWER(LTRIM(username, '@')) = ?2",
            params![roster_data, clean_username],
        )?;
        Ok(())
    }

    pub fn update_recovery_lock(&self, username: &str, lock_hash: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let clean_username = clean_user(username);
        conn.execute(
            "UPDATE users SET registration_lock_hash = ?1 WHERE LOWER(LTRIM(username, '@')) = ?2",
            params![lock_hash, clean_username],
        )?;
        Ok(())
    }

    pub fn update_user_pubkey(&self, username: &str, new_pubkey: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let clean_username = clean_user(username);
        conn.execute(
            "UPDATE users SET ed25519_pubkey = ?1 WHERE LOWER(LTRIM(username, '@')) = ?2",
            params![new_pubkey, clean_username],
        )?;
        Ok(())
    }

    pub fn delete_user(&self, username: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let clean_username = clean_user(username);
        conn.execute("DELETE FROM users WHERE LOWER(LTRIM(username, '@')) = ?1", params![clean_username])?;
        Ok(())
    }

    // --- Admin Operations ---
    pub fn get_admin_stats(&self) -> Result<AdminStats> {
        let conn = self.conn.lock().unwrap();
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

        let db_path = std::env::var("DATABASE_PATH").unwrap_or_else(|_| "vexta_bridge_v2.db".into());
        let database_size_bytes = std::fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0);
        let wal_path = format!("{}-wal", db_path);
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
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "INSERT INTO announcements (message, created_at) VALUES (?1, ?2)",
            params![message, now],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn list_announcements(&self) -> Result<Vec<Announcement>> {
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM announcements WHERE id = ?1", params![id])?;
        Ok(())
    }

    // --- Friend Requests & Friends Roster ---
    pub fn create_friend_request(&self, sender: &str, recipient: &str) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE friend_requests SET status = ?1 WHERE id = ?2",
            params![status, req_id],
        )?;
        Ok(())
    }

    pub fn update_friend_request_status_by_user(&self, user: &str, other_user: &str, status: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
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
            Err(e) => Err(e.into()),
        }
    }

    /// Returns the sender of a pending request between two users.
    pub fn get_friend_request_sender_by_users(&self, user_a: &str, user_b: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
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
            Err(e) => Err(e.into()),
        }
    }

    pub fn remove_friend(&self, username: &str, friend_username: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
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
        sender: &str,
        ciphertext: &str,
        timestamp: i64,
        is_group: bool,
    ) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let clean_recipient = clean_user(recipient);
        let clean_sender = clean_user(sender);
        conn.execute(
            "INSERT INTO offline_messages (recipient, sender, ciphertext, timestamp, is_group)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![clean_recipient, clean_sender, ciphertext, timestamp, if is_group { 1 } else { 0 }],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn fetch_and_clear_offline_messages(&self, recipient: &str) -> Result<Vec<BlindMessage>> {
        let conn = self.conn.lock().unwrap();
        let clean_recipient = clean_user(recipient);
        let mut stmt = conn.prepare(
            "SELECT id, recipient, sender, ciphertext, timestamp, is_group
             FROM offline_messages WHERE LOWER(LTRIM(recipient, '@')) = ?1 ORDER BY timestamp ASC",
        )?;

        let rows = stmt.query_map(params![clean_recipient], |row| {
            let is_group_int: i32 = row.get(5)?;
            Ok(BlindMessage {
                id: row.get(0)?,
                recipient: row.get(1)?,
                sender: row.get(2)?,
                ciphertext: row.get(3)?,
                timestamp: row.get(4)?,
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

        Ok(msgs)
    }

    pub fn get_offline_messages_summary(&self) -> Result<Vec<OfflineQueueSummary>> {
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
        let clean_username = clean_user(username);
        conn.execute(
            "UPDATE users SET auth_attempts = 0, locked_until = NULL WHERE LOWER(LTRIM(username, '@')) = ?1",
            params![clean_username],
        )?;
        Ok(())
    }

    pub fn purge_stale_offline_messages(&self, older_than_timestamp: i64) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let deleted = conn.execute(
            "DELETE FROM offline_messages WHERE timestamp < ?1",
            params![older_than_timestamp],
        )?;
        Ok(deleted)
    }
}

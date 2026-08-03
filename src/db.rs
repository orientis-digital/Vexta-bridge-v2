use crate::models::{BlindMessage, FriendRequest, UserDevice, VextaUser};
use rusqlite::{params, Connection, Result};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct DbManager {
    conn: Arc<Mutex<Connection>>,
}

impl DbManager {
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;

        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA busy_timeout = 5000;
             PRAGMA foreign_keys = ON;",
        )?;

        // Initialize Complete V1 + V2 Unified Schema
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS users (
                username TEXT PRIMARY KEY,
                ed25519_pubkey TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                is_provisioned INTEGER NOT NULL DEFAULT 0,
                passcode TEXT,
                registration_lock_hash TEXT,
                encrypted_vault TEXT,
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

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn save_or_update_user(&self, user: &VextaUser) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO users (username, ed25519_pubkey, created_at, is_provisioned, passcode, registration_lock_hash, encrypted_vault, pre_key, pre_key_signature, auth_attempts, locked_until)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(username) DO UPDATE SET
             ed25519_pubkey=excluded.ed25519_pubkey,
             encrypted_vault=COALESCE(excluded.encrypted_vault, users.encrypted_vault)",
            params![
                user.username,
                user.ed25519_pubkey,
                user.created_at,
                if user.is_provisioned { 1 } else { 0 },
                user.passcode,
                user.registration_lock_hash,
                user.encrypted_vault,
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
        let mut stmt = conn.prepare(
            "SELECT username, ed25519_pubkey, created_at, is_provisioned, passcode, registration_lock_hash, encrypted_vault, pre_key, pre_key_signature, auth_attempts, locked_until FROM users WHERE username = ?1",
        )?;

        let mut rows = stmt.query(params![username])?;
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
                pre_key: row.get(7)?,
                pre_key_signature: row.get(8)?,
                auth_attempts: row.get(9)?,
                locked_until: row.get(10)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn update_vault(&self, username: &str, vault_data: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE users SET encrypted_vault = ?1 WHERE username = ?2",
            params![vault_data, username],
        )?;
        Ok(())
    }

    pub fn update_recovery_lock(&self, username: &str, lock_hash: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE users SET registration_lock_hash = ?1 WHERE username = ?2",
            params![lock_hash, username],
        )?;
        Ok(())
    }

    pub fn delete_user(&self, username: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM users WHERE username = ?1", params![username])?;
        Ok(())
    }

    // --- Friend Requests & Friends Roster ---
    pub fn create_friend_request(&self, sender: &str, recipient: &str) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "INSERT INTO friend_requests (sender, recipient, status, created_at)
             VALUES (?1, ?2, 'pending', ?3)
             ON CONFLICT(sender, recipient) DO UPDATE SET status='pending', created_at=?3",
            params![sender, recipient, now],
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

    pub fn list_friends(&self, username: &str) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT CASE WHEN sender = ?1 THEN recipient ELSE sender END AS friend
             FROM friend_requests
             WHERE (sender = ?1 OR recipient = ?1) AND status = 'accepted'",
        )?;

        let rows = stmt.query_map(params![username], |row| row.get(0))?;
        let mut friends = Vec::new();
        for r in rows {
            friends.push(r?);
        }
        Ok(friends)
    }

    pub fn list_pending_requests(&self, username: &str) -> Result<Vec<FriendRequest>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, sender, recipient, status, created_at
             FROM friend_requests WHERE recipient = ?1 AND status = 'pending'",
        )?;

        let rows = stmt.query_map(params![username], |row| {
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

    pub fn remove_friend(&self, username: &str, friend_username: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM friend_requests
             WHERE (sender = ?1 AND recipient = ?2) OR (sender = ?2 AND recipient = ?1)",
            params![username, friend_username],
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
        conn.execute(
            "INSERT INTO user_devices (username, hardware_hash, device_name, device_type, registered_at, last_active)
             VALUES (?1, ?2, ?3, 'Desktop', ?4, ?4)
             ON CONFLICT(username, hardware_hash) DO UPDATE SET
             last_active = ?4, device_name = ?3",
            params![username, hardware_hash, device_name, now],
        )?;
        Ok(())
    }

    pub fn list_devices(&self, username: &str) -> Result<Vec<UserDevice>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, username, hardware_hash, device_name, device_type, registered_at, last_active
             FROM user_devices WHERE username = ?1",
        )?;

        let rows = stmt.query_map(params![username], |row| {
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
        conn.execute(
            "DELETE FROM user_devices WHERE username = ?1 AND hardware_hash = ?2",
            params![username, hardware_hash],
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
        conn.execute(
            "INSERT INTO offline_messages (recipient, sender, ciphertext, timestamp, is_group)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![recipient, sender, ciphertext, timestamp, if is_group { 1 } else { 0 }],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn fetch_and_clear_offline_messages(&self, recipient: &str) -> Result<Vec<BlindMessage>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, recipient, sender, ciphertext, timestamp, is_group
             FROM offline_messages WHERE recipient = ?1 ORDER BY timestamp ASC",
        )?;

        let rows = stmt.query_map(params![recipient], |row| {
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
            "DELETE FROM offline_messages WHERE recipient = ?1",
            params![recipient],
        )?;

        Ok(msgs)
    }
}

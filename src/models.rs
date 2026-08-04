use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VextaUser {
    pub username: String,
    pub ed25519_pubkey: String,
    pub created_at: i64,
    pub is_provisioned: bool,
    pub passcode: Option<String>,
    pub registration_lock_hash: Option<String>,
    pub encrypted_vault: Option<String>,
    pub pre_key: Option<String>,
    pub pre_key_signature: Option<String>,
    pub auth_attempts: i32,
    pub locked_until: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FriendRequest {
    pub id: i64,
    pub sender: String,
    pub recipient: String,
    pub status: String, // "pending", "accepted", "rejected"
    pub created_at: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserDevice {
    pub id: i64,
    pub username: String,
    pub hardware_hash: String,
    pub device_name: String,
    pub device_type: String,
    pub registered_at: i64,
    pub last_active: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BlindMessage {
    pub id: i64,
    pub recipient: String,
    pub sender: String,
    pub ciphertext: String,
    pub timestamp: i64,
    pub is_group: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum BridgeFrame {
    #[serde(rename = "AUTH_CHALLENGE")]
    AuthChallenge {
        nonce: String,
        server_public_key: String,
        server_signature: String,
    },
    #[serde(rename = "AUTH_RESPONSE")]
    AuthResponse {
        username: String,
        #[serde(alias = "public_key")]
        ed25519_pubkey: String,
        nonce: Option<String>,
        signature: Option<String>,
        passcode: Option<String>,
        passcode_hmac: Option<String>,
        hardware_hash: Option<String>,
        device_name: Option<String>,
        os_name: Option<String>,
        os_version: Option<String>,
        device_type: Option<String>,
        app_version: Option<String>,
    },
    #[serde(rename = "REGISTER")]
    Register {
        username: String,
        #[serde(alias = "public_key")]
        ed25519_pubkey: String,
        signature: Option<String>,
        passcode: Option<String>,
        hardware_hash: Option<String>,
        device_name: Option<String>,
        os_name: Option<String>,
        os_version: Option<String>,
        device_type: Option<String>,
        app_version: Option<String>,
    },
    #[serde(rename = "AUTH_SUCCESS")]
    AuthSuccess {
        username: String,
    },
    #[serde(rename = "AUTH_ERROR")]
    AuthError {
        reason: String,
    },
    #[serde(rename = "SEND_MESSAGE")]
    SendMessage {
        recipient: String,
        ciphertext: String,
        is_group: Option<bool>,
        timestamp: Option<i64>,
    },
    #[serde(rename = "BLIND_MESSAGE")]
    BlindMessage {
        id: i64,
        sender: String,
        ciphertext: String,
        timestamp: i64,
        is_group: bool,
    },
    #[serde(rename = "ACK")]
    Ack {
        message_id: i64,
    },
    #[serde(rename = "UPDATE_VAULT")]
    UpdateVault {
        vault_data: String,
    },
    #[serde(rename = "GET_VAULT")]
    GetVault,
    #[serde(rename = "VAULT_RESPONSE")]
    VaultResponse {
        vault_data: Option<String>,
    },
    #[serde(rename = "UPDATE_RECOVERY_LOCK")]
    UpdateRecoveryLock {
        lock_hash: String,
    },
    // Friend Request Frames
    #[serde(rename = "SEND_FRIEND_REQUEST")]
    SendFriendRequest {
        recipient: String,
    },
    #[serde(rename = "FRIEND_REQUEST_SENT")]
    FriendRequestSent {
        request_id: i64,
        recipient: String,
    },
    #[serde(rename = "ACCEPT_FRIEND_REQUEST")]
    AcceptFriendRequest {
        request_id: i64,
    },
    #[serde(rename = "REJECT_FRIEND_REQUEST")]
    RejectFriendRequest {
        request_id: i64,
    },
    #[serde(rename = "LIST_FRIENDS")]
    ListFriends,
    #[serde(rename = "FRIENDS_LIST")]
    FriendsList {
        friends: Vec<String>,
    },
    #[serde(rename = "LIST_FRIEND_REQUESTS")]
    ListFriendRequests,
    #[serde(rename = "FRIEND_REQUESTS_LIST")]
    FriendRequestsList {
        requests: Vec<FriendRequest>,
    },
    #[serde(rename = "REMOVE_FRIEND")]
    RemoveFriend {
        friend_username: String,
    },

    // Device Management Frames
    #[serde(rename = "LIST_DEVICES")]
    ListDevices,
    #[serde(rename = "DEVICES_LIST")]
    DevicesList {
        devices: Vec<UserDevice>,
    },
    #[serde(rename = "REVOKE_DEVICE")]
    RevokeDevice {
        hardware_hash: String,
    },

    // Account Lifetime
    #[serde(rename = "DELETE_ACCOUNT")]
    DeleteAccount,
    #[serde(rename = "DELETE_ACCOUNT_SUCCESS")]
    DeleteAccountSuccess,

    #[serde(rename = "ERROR")]
    Error {
        message: String,
    },
}

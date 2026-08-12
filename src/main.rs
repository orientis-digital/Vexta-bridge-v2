mod admin_html;
mod crypto;
mod db;
mod models;
mod state;
mod ws;

use axum::{
    extract::{ws::Message, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{sse::{Event, Sse}, Html, IntoResponse, Json},
    routing::{delete, get, post},
    Router,
};
use futures_util::stream::{self, Stream};
use std::convert::Infallible;
use std::time::Duration;
use serde::Deserialize;
use serde_json::{json, Value};
use state::AppState;
use std::net::SocketAddr;

use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() {
    // 0. Load .env environment variables if present
    dotenvy::dotenv().ok();
    dotenvy::from_path("/app/data/.env").ok();

    // 1. Initialize Logging
    tracing_subscriber::registry()
        .with(EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stdout))
        .init();

    println!("====================================================");
    println!(" [Vexta V2 Bridge] Server Logging Initialized (INFO)");
    println!("====================================================");

    // 2. Initialize Shared App State (SQLite DB + Ed25519 Server Crypto)
    let db_path = std::env::var("DATABASE_PATH").unwrap_or_else(|_| "vexta_bridge_v2.db".into());
    if let Some(parent) = std::path::Path::new(&db_path).parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let state = AppState::new(&db_path);

    // 3. Configure Full CORS Middleware (Cloudflare Tunnel Compatible)
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // 4. Build Axum Router with Admin Console & Public APIs
    let base_app = Router::new()
        // WebSocket Relay
        .route("/ws/chat/", get(ws::ws_handler))
        .route("/ws/chat", get(ws::ws_handler))
        
        // Public REST Endpoints
        .route("/api/check-account/:username", get(check_account_handler))
        .route("/api/announcements/", get(public_announcements_handler))
        .route("/api/announcements", get(public_announcements_handler))
        
        .route("/api/admin/events", get(admin_events_sse_handler))
        .route("/api/admin/stats", get(admin_stats_handler))
        .route("/api/admin/sessions", get(admin_list_sessions_handler))
        .route("/api/admin/sessions/:username", delete(admin_disconnect_session_handler))
        .route("/api/admin/users", get(admin_list_users_handler))
        .route("/api/admin/users/:username", delete(admin_delete_user_handler))
        .route("/api/admin/users/:username/unlock", post(admin_unlock_user_handler))
        .route("/api/admin/devices", get(admin_list_devices_handler))
        .route("/api/admin/devices/:username/:hardware_hash", delete(admin_revoke_device_handler))
        .route("/api/admin/offline-messages/summary", get(admin_offline_messages_summary_handler))
        .route("/api/admin/offline-messages/purge", post(admin_purge_offline_messages_handler))
        .route("/api/admin/announcements", get(admin_list_announcements_handler).post(admin_post_announcement_handler))
        .route("/api/admin/announcements/:id", delete(admin_delete_announcement_handler))
        // IP Firewall
        .route("/api/admin/banned-ips", get(admin_list_banned_ips_handler).post(admin_ban_ip_handler))
        .route("/api/admin/banned-ips/:ip", delete(admin_unban_ip_handler))
        // Maintenance Mode
        .route("/api/admin/maintenance", get(admin_get_maintenance_handler))
        .route("/api/admin/maintenance/enable", post(admin_enable_maintenance_handler))
        .route("/api/admin/maintenance/disable", post(admin_disable_maintenance_handler))
        // Audit Logs
        .route("/api/admin/audit-logs", get(admin_list_audit_logs_handler))
        // System Vacuum & Health
        .route("/api/admin/system/vacuum", post(admin_vacuum_db_handler))
        .route("/api/admin/system/db-health", get(admin_db_health_handler))
        // Traffic Analytics
        .route("/api/admin/analytics/top-users", get(admin_top_user_analytics_handler));

    // Admin UI Routing: Serve React bundle from 'admin-ui/dist' if built, else fallback to embedded HTML
    let admin_dist_path = std::path::Path::new("admin-ui/dist");
    let app = if admin_dist_path.exists() {
        info!("📦 Serving compiled React Admin UI from 'admin-ui/dist'");
        let serve_service = ServeDir::new("admin-ui/dist")
            .fallback(ServeFile::new("admin-ui/dist/index.html"));
        base_app
            .nest_service("/admin", serve_service.clone())
            .fallback_service(serve_service)
    } else {
        info!("📄 Serving embedded fallback Admin UI");
        base_app
            .route("/", get(admin_ui_handler))
            .route("/admin", get(admin_ui_handler))
            .route("/admin/", get(admin_ui_handler))
    };

    let app = app.layer(cors).with_state(state);

    // 5. Start TCP Server on Port 8000
    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    info!("🚀 Vexta V2 Rust Bridge listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn get_admin_secret_token() -> String {
    std::env::var("ADMIN_SECRET_TOKEN")
        .or_else(|_| std::env::var("ADMIN_SECRET"))
        .unwrap_or_else(|_| "vexta_admin_secret_key_2026".to_string())
}

// Check Admin Secret Token Header
fn verify_admin_auth(headers: &HeaderMap) -> bool {
    let expected_secret = get_admin_secret_token();
    if let Some(token) = headers.get("x-admin-secret").or_else(|| headers.get("authorization")) {
        if let Ok(str_val) = token.to_str() {
            let clean_val = str_val.trim_start_matches("Bearer ").trim();
            return clean_val == expected_secret;
        }
    }
    false
}

#[derive(Deserialize)]
struct SseAuthQuery {
    token: Option<String>,
}

// Admin Real-time Events SSE Stream Handler
async fn admin_events_sse_handler(
    headers: HeaderMap,
    Query(query): Query<SseAuthQuery>,
    State(state): State<AppState>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    let auth_valid = verify_admin_auth(&headers) || {
        if let Some(token) = &query.token {
            token == &get_admin_secret_token()
        } else {
            false
        }
    };

    if !auth_valid {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let rx = state.broadcast_tx.subscribe();
    let stream = stream::unfold(rx, |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    let event = Event::default().data(msg);
                    return Some((Ok(event), rx));
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => return None,
            }
        }
    });

    Ok(Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::new().interval(Duration::from_secs(15))))
}

// Embedded Admin UI Page
async fn admin_ui_handler() -> Html<&'static str> {
    Html(admin_html::ADMIN_HTML)
}

// Admin Telemetry Handler
async fn admin_stats_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if !verify_admin_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized admin token"})));
    }

    if let Ok(stats) = state.db.get_admin_stats() {
        let active_sessions = state.active_sessions_count();
        let uptime_seconds = chrono::Utc::now().timestamp() - state.start_time;
        let (total_msgs, total_bytes) = state.get_traffic_stats();
        return (
            StatusCode::OK,
            Json(json!({
                "version": state::SERVER_VERSION,
                "server_name": state::SERVER_NAME,
                "active_ws_sessions": active_sessions,
                "maintenance_mode": state.is_maintenance_enabled(),
                "total_users": stats.total_users,
                "total_queued_offline_messages": stats.total_queued_offline_messages,
                "total_registered_devices": stats.total_registered_devices,
                "total_announcements": stats.total_announcements,
                "database_size_bytes": stats.database_size_bytes,
                "wal_size_bytes": stats.wal_size_bytes,
                "provisioned_users": stats.provisioned_users,
                "locked_users": stats.locked_users,
                "users_with_vault": stats.users_with_vault,
                "users_with_prekey": stats.users_with_prekey,
                "users_with_offline_msgs": stats.users_with_offline_msgs,
                "total_messages_relayed": total_msgs,
                "total_bytes_relayed": total_bytes,
                "uptime_seconds": uptime_seconds,
                "server_start_time": state.start_time,
            })),
        );
    }

    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Database error"})))
}

#[derive(Deserialize)]
struct BanIpReq {
    ip: String,
    reason: Option<String>,
}

// IP Firewall Handlers
async fn admin_list_banned_ips_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if !verify_admin_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized admin token"})));
    }
    if let Ok(list) = state.db.list_banned_ips() {
        return (StatusCode::OK, Json(serde_json::to_value(list).unwrap()));
    }
    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Database error"})))
}

async fn admin_ban_ip_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<BanIpReq>,
) -> impl IntoResponse {
    if !verify_admin_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized admin token"})));
    }
    let reason = payload.reason.unwrap_or_else(|| "Banned by administrator".into());
    if let Ok(_) = state.db.ban_ip(&payload.ip, &reason, "Admin") {
        state.ban_ip_cache(payload.ip.clone(), reason.clone());
        let _ = state.db.log_audit_action("BAN_IP", &payload.ip, &reason);
        info!("[Admin Console] Banned IP address '{}'", payload.ip);
        return (StatusCode::OK, Json(json!({"success": true, "banned_ip": payload.ip})));
    }
    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Database error"})))
}

async fn admin_unban_ip_handler(
    Path(ip): Path<String>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if !verify_admin_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized admin token"})));
    }
    if let Ok(_) = state.db.unban_ip(&ip) {
        state.unban_ip_cache(&ip);
        let _ = state.db.log_audit_action("UNBAN_IP", &ip, "Unbanned by admin");
        info!("[Admin Console] Unbanned IP address '{}'", ip);
        return (StatusCode::OK, Json(json!({"success": true, "unbanned_ip": ip})));
    }
    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Database error"})))
}

// Maintenance Mode Handlers
async fn admin_get_maintenance_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if !verify_admin_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized admin token"})));
    }
    (StatusCode::OK, Json(json!({ "maintenance_mode": state.is_maintenance_enabled() })))
}

async fn admin_enable_maintenance_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if !verify_admin_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized admin token"})));
    }
    state.set_maintenance(true);
    let _ = state.db.log_audit_action("ENABLE_MAINTENANCE", "server", "Server entered emergency maintenance mode");
    info!("[Admin Console] Enabled emergency maintenance mode");
    (StatusCode::OK, Json(json!({ "success": true, "maintenance_mode": true })))
}

async fn admin_disable_maintenance_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if !verify_admin_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized admin token"})));
    }
    state.set_maintenance(false);
    let _ = state.db.log_audit_action("DISABLE_MAINTENANCE", "server", "Resumed normal bridge operation");
    info!("[Admin Console] Disabled maintenance mode — resumed operations");
    (StatusCode::OK, Json(json!({ "success": true, "maintenance_mode": false })))
}

// Audit Logs Handler
async fn admin_list_audit_logs_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if !verify_admin_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized admin token"})));
    }
    if let Ok(logs) = state.db.list_audit_logs(100) {
        return (StatusCode::OK, Json(serde_json::to_value(logs).unwrap()));
    }
    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Database error"})))
}

// Database Vacuum & Health Handlers
async fn admin_vacuum_db_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if !verify_admin_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized admin token"})));
    }
    if let Ok(_) = state.db.vacuum_database() {
        let _ = state.db.log_audit_action("VACUUM_DB", "sqlite", "Ran WAL checkpoint & database VACUUM");
        info!("[Admin Console] Executed SQLite WAL checkpoint & VACUUM");
        return (StatusCode::OK, Json(json!({ "success": true, "message": "Database WAL truncated & vacuumed successfully" })));
    }
    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Database error"})))
}

async fn admin_db_health_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if !verify_admin_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized admin token"})));
    }
    if let Ok(health) = state.db.get_db_health() {
        return (StatusCode::OK, Json(serde_json::to_value(health).unwrap()));
    }
    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Database error"})))
}

// Traffic Analytics Handler
async fn admin_top_user_analytics_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if !verify_admin_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized admin token"})));
    }
    if let Ok(analytics) = state.db.get_top_user_analytics(50) {
        return (StatusCode::OK, Json(serde_json::to_value(analytics).unwrap()));
    }
    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Database error"})))
}

// Admin List Active Sessions Handler
async fn admin_list_sessions_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if !verify_admin_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized admin token"})));
    }

    let active_users = state.list_active_usernames();
    (StatusCode::OK, Json(json!({ "active_sessions": active_users, "count": active_users.len() })))
}

// Admin Disconnect Session Handler
async fn admin_disconnect_session_handler(
    Path(username): Path<String>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if !verify_admin_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized admin token"})));
    }

    let disconnected = state.disconnect_session(&username);
    info!("[Admin Console] Disconnected WS session for user '{}' (success: {})", username, disconnected);
    (StatusCode::OK, Json(json!({ "success": true, "disconnected_username": username, "found": disconnected })))
}

// Admin Unlock User Handler
async fn admin_unlock_user_handler(
    Path(username): Path<String>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if !verify_admin_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized admin token"})));
    }

    if let Ok(_) = state.db.unlock_user_account(&username) {
        let _ = state.db.log_audit_action("UNLOCK_ACCOUNT", &username, "Unlocked failed login attempts");
        info!("[Admin Console] Unlocked account '{}'", username);
        return (StatusCode::OK, Json(json!({ "success": true, "unlocked_username": username })));
    }

    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Database error"})))
}

// Admin Revoke Device Handler
async fn admin_revoke_device_handler(
    Path((username, hardware_hash)): Path<(String, String)>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if !verify_admin_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized admin token"})));
    }

    if let Ok(_) = state.db.revoke_device(&username, &hardware_hash) {
        let _ = state.db.log_audit_action("REVOKE_DEVICE", &username, &format!("Revoked device hash {}", hardware_hash));
        info!("[Admin Console] Revoked device '{}' for user '{}'", hardware_hash, username);
        return (StatusCode::OK, Json(json!({ "success": true, "revoked_device": hardware_hash, "username": username })));
    }

    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Database error"})))
}

// Admin Offline Messages Summary Handler
async fn admin_offline_messages_summary_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if !verify_admin_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized admin token"})));
    }

    if let Ok(summary) = state.db.get_offline_messages_summary() {
        return (StatusCode::OK, Json(serde_json::to_value(summary).unwrap()));
    }

    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Database error"})))
}

#[derive(Deserialize)]
struct PurgeOfflineReq {
    older_than_days: Option<i64>,
}

// Admin Purge Stale Offline Messages Handler
async fn admin_purge_offline_messages_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<PurgeOfflineReq>,
) -> impl IntoResponse {
    if !verify_admin_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized admin token"})));
    }

    let days = payload.older_than_days.unwrap_or(30);
    let cutoff = chrono::Utc::now().timestamp() - (days * 86400);

    if let Ok(deleted_count) = state.db.purge_stale_offline_messages(cutoff) {
        let _ = state.db.log_audit_action("PURGE_OFFLINE_MESSAGES", "database", &format!("Deleted {} messages older than {} days", deleted_count, days));
        info!("[Admin Console] Purged {} offline messages older than {} days", deleted_count, days);
        return (StatusCode::OK, Json(json!({ "success": true, "deleted_count": deleted_count, "cutoff_days": days })));
    }

    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Database error"})))
}

// Admin List Users Handler
async fn admin_list_users_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if !verify_admin_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized admin token"})));
    }

    if let Ok(users) = state.db.list_all_users() {
        return (StatusCode::OK, Json(serde_json::to_value(users).unwrap()));
    }

    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Database error"})))
}

// Admin Delete User Handler
async fn admin_delete_user_handler(
    Path(username): Path<String>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if !verify_admin_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized admin token"})));
    }

    let _ = state.db.delete_user(&username);
    state.unregister_session(&username);
    info!("[Admin Console] Deleted user '{}'", username);

    (StatusCode::OK, Json(json!({"success": true, "deleted_username": username})))
}

// Admin List All Devices (across all users)
async fn admin_list_devices_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if !verify_admin_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized admin token"})));
    }

    if let Ok(users) = state.db.list_all_users() {
        let mut all_devices = Vec::new();
        for user in &users {
            if let Ok(devices) = state.db.list_devices(&user.username) {
                for d in devices {
                    all_devices.push(d);
                }
            }
        }
        return (StatusCode::OK, Json(serde_json::to_value(all_devices).unwrap()));
    }

    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Database error"})))
}

#[derive(Deserialize)]
struct CreateAnnouncementReq {
    message: String,
}

// Admin Post Announcement
async fn admin_post_announcement_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<CreateAnnouncementReq>,
) -> impl IntoResponse {
    if !verify_admin_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized admin token"})));
    }

    if let Ok(id) = state.db.create_announcement(&payload.message) {
        let _ = state.db.log_audit_action("POST_ANNOUNCEMENT", &format!("#{}", id), &payload.message);
        info!("[Admin Console] Created broadcast announcement #{}", id);

        // Broadcast announcement to all connected WebSocket sessions in real-time
        let announcement_inner = json!({
            "type": "system_broadcast",
            "announcement": payload.message,
            "id": id,
        });

        let broadcast_frame = json!({
            "type": "message",
            "sender": "Vexta - Global Message",
            "timestamp": chrono::Utc::now().timestamp_millis(),
            "ciphertext": announcement_inner.to_string(),
        });

        let frame_str = broadcast_frame.to_string();
        for item in state.active_sessions.iter() {
            let tx = item.value();
            let _ = tx.send(Message::Text(frame_str.clone()));
        }

        state.emit_event(&json!({
            "event": "announcement_created",
            "id": id,
            "message": payload.message,
        }).to_string());

        return (StatusCode::OK, Json(json!({"success": true, "id": id})));
    }

    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Database error"})))
}

// Admin List Announcements
async fn admin_list_announcements_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if !verify_admin_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized admin token"})));
    }

    if let Ok(list) = state.db.list_announcements() {
        return (StatusCode::OK, Json(serde_json::to_value(list).unwrap()));
    }

    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Database error"})))
}

// Admin Delete Announcement
async fn admin_delete_announcement_handler(
    Path(id): Path<i64>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if !verify_admin_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized admin token"})));
    }

    let _ = state.db.delete_announcement(id);
    (StatusCode::OK, Json(json!({"success": true, "deleted_id": id})))
}

// Public Account Existence Check
async fn check_account_handler(
    Path(username): Path<String>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> (StatusCode, Json<Value>) {
    let client_ip = headers
        .get("cf-connecting-ip")
        .or_else(|| headers.get("x-forwarded-for"))
        .and_then(|h| h.to_str().ok())
        .unwrap_or("127.0.0.1");

    info!("[Cloudflare API] Check account '{}' from IP: {}", username, client_ip);

    match state.db.get_user(&username) {
        Ok(Some(user)) => (
            StatusCode::OK,
            Json(json!({
                "exists": true,
                "username": user.username,
                "ed25519_pubkey": user.ed25519_pubkey,
            })),
        ),
        _ => (
            StatusCode::OK,
            Json(json!({
                "exists": false,
            })),
        ),
    }
}

// Public Announcements REST Endpoint
async fn public_announcements_handler(
    State(state): State<AppState>,
) -> Json<Value> {
    if let Ok(list) = state.db.list_announcements() {
        if !list.is_empty() {
            return Json(serde_json::to_value(list).unwrap());
        }
    }

    Json(json!([
        {
            "id": 1,
            "message": "Welcome to Vexta V2 High-Performance Rust Relay Bridge (vexta-api.nexusec.space).",
            "created_at": chrono::Utc::now().timestamp(),
        }
    ]))
}

mod admin_html;
mod crypto;
mod db;
mod models;
mod state;
mod ws;

use axum::{
    extract::{ws::Message, Path, Query, State},
    http::{HeaderMap, StatusCode},
    middleware,
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
use tracing::{info, warn, error, debug};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

fn clean_user(u: &str) -> String {
    u.trim().trim_start_matches('@').to_lowercase()
}

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

    info!("====================================================");
    info!(" 🚀 Vexta Bridge V2 ({} - {}) Server Starting", state::SERVER_VERSION, state::SERVER_NAME);
    info!("====================================================");

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
        
        // Public REST Endpoints & Healthchecks
        .route("/health", get(health_handler))
        .route("/health/", get(health_handler))
        .route("/api/health", get(health_handler))
        .route("/api/health/", get(health_handler))
        .route("/api/v1/health", get(health_handler))
        .route("/api/v1/health/", get(health_handler))
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
        // Version Policy
        .route("/api/version-policy", get(public_version_policy_handler))
        .route("/api/version-policy/", get(public_version_policy_handler))
        .route("/api/admin/version-policy", get(admin_get_version_policy_handler).post(admin_set_version_policy_handler))
        // Server Migration (Export & Import)
        .route("/api/admin/migration/export", get(admin_export_migration_handler))
        .route("/api/admin/migration/import", post(admin_import_migration_handler))
        // Traffic & Platform Analytics
        .route("/api/admin/analytics/top-users", get(admin_top_user_analytics_handler))
        .route("/api/admin/analytics/platforms", get(admin_platform_analytics_handler));

    // Admin UI Routing: Serve React bundle from 'admin-ui/dist' if built, else fallback to embedded HTML
    let admin_dist_path = std::path::Path::new("admin-ui/dist");
    let app = if admin_dist_path.exists() {
        info!("[SYSTEM] Serving compiled React Admin UI from 'admin-ui/dist'");
        let serve_service = ServeDir::new("admin-ui/dist")
            .fallback(ServeFile::new("admin-ui/dist/index.html"));
        base_app
            .nest_service("/admin", serve_service.clone())
            .fallback_service(serve_service)
    } else {
        info!("[SYSTEM] Serving embedded fallback Admin UI");
        base_app
            .route("/", get(admin_ui_handler))
            .route("/admin", get(admin_ui_handler))
            .route("/admin/", get(admin_ui_handler))
    };

    let app = app
        .layer(cors)
        .layer(middleware::from_fn(request_logging_middleware))
        .layer(middleware::from_fn(add_security_headers))
        .with_state(state.clone());

    // 5. Start TCP Server on Configured Port (Default 8000)
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(8000);
    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into());
    let addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .unwrap_or_else(|_| SocketAddr::from(([0, 0, 0, 0], port)));

    let admin_secret_is_set = std::env::var("ADMIN_SECRET_TOKEN").or_else(|_| std::env::var("ADMIN_SECRET")).is_ok();
    info!("[SYSTEM] Server configuration details:");
    info!("  • Listen Address  : http://{}", addr);
    info!("  • Database Path   : {}", db_path);
    info!("  • Admin Secret    : {}", if admin_secret_is_set { "Configured" } else { "DEFAULT FALLBACK KEY (Warning: configure ADMIN_SECRET in production)" });
    info!("  • Server PubKey   : {}...", &state.crypto.pubkey_base64[..16.min(state.crypto.pubkey_base64.len())]);
    info!("  • Maintenance     : {}", if state.is_maintenance_enabled() { "ENABLED" } else { "DISABLED" });

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    info!("🚀 Vexta Bridge V2 listening on http://{}", addr);
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await.unwrap();
}

async fn request_logging_middleware(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> impl IntoResponse {
    let start = std::time::Instant::now();
    let method = req.method().clone();
    let uri = req.uri().clone();
    let client_ip = req.headers().get("cf-connecting-ip")
        .or_else(|| req.headers().get("x-forwarded-for"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
        .unwrap_or_else(|| "127.0.0.1".to_string());

    let response = next.run(req).await;
    let elapsed = start.elapsed();
    let status = response.status();

    if status.is_server_error() {
        error!("[HTTP] {} {} -> {} ({:?}) [client: {}]", method, uri, status.as_u16(), elapsed, client_ip);
    } else if status.is_client_error() {
        warn!("[HTTP] {} {} -> {} ({:?}) [client: {}]", method, uri, status.as_u16(), elapsed, client_ip);
    } else {
        info!("[HTTP] {} {} -> {} ({:?}) [client: {}]", method, uri, status.as_u16(), elapsed, client_ip);
    }

    response
}

async fn add_security_headers(req: axum::extract::Request, next: axum::middleware::Next) -> impl IntoResponse {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    headers.insert("x-content-type-options", "nosniff".parse().unwrap());
    headers.insert("x-frame-options", "SAMEORIGIN".parse().unwrap());
    headers.insert("x-xss-protection", "1; mode=block".parse().unwrap());
    headers.insert("referrer-policy", "strict-origin-when-cross-origin".parse().unwrap());
    response
}

fn get_admin_secret_token() -> String {
    std::env::var("ADMIN_SECRET_TOKEN")
        .or_else(|_| std::env::var("ADMIN_SECRET"))
        .unwrap_or_else(|_| {
            warn!("[SECURITY WARNING] ADMIN_SECRET environment variable is NOT set! Using default fallback key. Please configure ADMIN_SECRET in production!");
            "vexta_admin_secret_key_2026".to_string()
        })
}

fn constant_time_compare(a: &str, b: &str) -> bool {
    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();
    if a_bytes.len() != b_bytes.len() {
        return false;
    }
    let mut result = 0u8;
    for (x, y) in a_bytes.iter().zip(b_bytes.iter()) {
        result |= x ^ y;
    }
    result == 0
}

// Check Admin Secret Token Header with constant-time timing-attack mitigation
fn verify_admin_auth(headers: &HeaderMap) -> bool {
    let expected_secret = get_admin_secret_token();
    if let Some(token) = headers.get("x-admin-secret").or_else(|| headers.get("authorization")) {
        if let Ok(str_val) = token.to_str() {
            let clean_val = str_val.trim_start_matches("Bearer ").trim();
            let valid = constant_time_compare(clean_val, &expected_secret);
            if !valid {
                warn!("[ADMIN AUTH] Unauthorized admin access attempt: Invalid secret token");
            }
            return valid;
        }
    }
    warn!("[ADMIN AUTH] Unauthorized admin access attempt: Missing secret token header");
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
        warn!("[ADMIN AUTH] Unauthorized SSE subscription attempt");
        return Err(StatusCode::UNAUTHORIZED);
    }

    info!("[ADMIN] Client subscribed to real-time administrative SSE stream");

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
        debug!("[ADMIN] Telemetry metrics queried (Active Sessions: {}, Relayed Msgs: {})", active_sessions, total_msgs);
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
        debug!("[ADMIN] Listed {} banned IPs", list.len());
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
        info!("[ADMIN] Banned IP address '{}' (Reason: '{}')", payload.ip, reason);
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
        info!("[ADMIN] Unbanned IP address '{}'", ip);
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
    info!("[ADMIN] Emergency maintenance mode ENABLED");
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
    info!("[ADMIN] Emergency maintenance mode DISABLED — resumed normal operations");
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
        debug!("[ADMIN] Listed {} audit logs", logs.len());
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
        info!("[ADMIN] Executed SQLite database VACUUM & WAL truncation");
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
        debug!("[ADMIN] Fetched SQLite database health & integrity metrics");
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
        debug!("[ADMIN] Fetched top user analytics ({} entries)", analytics.len());
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
    debug!("[ADMIN] Listed active WebSocket sessions ({} users online)", active_users.len());
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
    info!("[ADMIN] Forcibly disconnected WS sessions for user '@{}' (Found: {})", username, disconnected);
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
        info!("[ADMIN] Unlocked account for user '@{}'", username);
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
        info!("[ADMIN] Revoked device '{}' for user '@{}'", hardware_hash, username);
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
        debug!("[ADMIN] Queried offline message queue summary ({} recipient queues)", summary.len());
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
        info!("[ADMIN] Purged {} offline messages older than {} days", deleted_count, days);
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
        debug!("[ADMIN] Listed all registered users ({} total)", users.len());
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
    state.disconnect_session(&username);
    info!("[ADMIN] Deleted user account '@{}'", username);

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
        debug!("[ADMIN] Listed all user devices ({} total)", all_devices.len());
        return (StatusCode::OK, Json(serde_json::to_value(all_devices).unwrap()));
    }

    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Database error"})))
}

// Public Version Policy Handler
async fn public_version_policy_handler(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let policy = state.get_version_policy();
    (StatusCode::OK, Json(serde_json::to_value(policy).unwrap()))
}

// Admin Get Version Policy Handler
async fn admin_get_version_policy_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if !verify_admin_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized admin token"})));
    }
    let policy = state.get_version_policy();
    (StatusCode::OK, Json(serde_json::to_value(policy).unwrap()))
}

// Admin Set Version Policy Handler
async fn admin_set_version_policy_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<state::VersionPolicy>,
) -> impl IntoResponse {
    if !verify_admin_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized admin token"})));
    }
    state.set_version_policy(payload.clone());
    let _ = state.db.log_audit_action("SET_VERSION_POLICY", "SYSTEM", &format!("Min: {}+{}, Latest: {}+{}", payload.min_client_version, payload.min_build_number, payload.latest_client_version, payload.latest_build_number));
    (StatusCode::OK, Json(json!({"success": true, "policy": payload})))
}

// Admin Export Migration Data Handler
async fn admin_export_migration_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if !verify_admin_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, HeaderMap::new(), Json(json!({"error": "Unauthorized admin token"}))).into_response();
    }

    match state.db.export_migration_data(state::SERVER_VERSION, Some(state.get_version_policy())) {
        Ok(data) => {
            let _ = state.db.log_audit_action("EXPORT_DATABASE", "SYSTEM", &format!("Exported {} users, {} devices", data.users.len(), data.devices.len()));
            info!("[ADMIN] Exported complete database migration archive");
            let mut res_headers = HeaderMap::new();
            let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
            let filename = format!("vexta_bridge_backup_{}.json", timestamp);
            res_headers.insert(
                axum::http::header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", filename).parse().unwrap(),
            );
            (StatusCode::OK, res_headers, Json(serde_json::to_value(data).unwrap())).into_response()
        }
        Err(err) => {
            error!("[ADMIN] Failed to export migration data: {:?}", err);
            (StatusCode::INTERNAL_SERVER_ERROR, HeaderMap::new(), Json(json!({"error": "Failed to export data"}))).into_response()
        }
    }
}

// Admin Import Migration Data Handler
async fn admin_import_migration_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<models::BridgeMigrationData>,
) -> impl IntoResponse {
    if !verify_admin_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized admin token"})));
    }

    if let Some(policy) = &payload.version_policy {
        state.set_version_policy(policy.clone());
    }

    match state.db.import_migration_data(&payload) {
        Ok(stats) => {
            // Re-populate in-memory IP bans
            if let Ok(banned_ips) = state.db.list_banned_ips() {
                state.ip_ban_list.clear();
                for b in banned_ips {
                    state.ip_ban_list.insert(b.ip, b.reason);
                }
            }

            let _ = state.db.log_audit_action("IMPORT_DATABASE", "SYSTEM", &format!("Imported {} users, {} devices, {} announcements", stats.imported_users, stats.imported_devices, stats.imported_announcements));
            info!("[ADMIN] Successfully restored/imported database migration archive");

            state.emit_event(&json!({
                "event": "database_imported",
                "stats": stats,
            }).to_string());

            (StatusCode::OK, Json(json!({ "success": true, "stats": stats })))
        }
        Err(err) => {
            error!("[ADMIN] Failed to import migration data: {:?}", err);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": format!("Import failed: {:?}", err) })))
        }
    }
}

// Admin Platform Analytics Handler
async fn admin_platform_analytics_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if !verify_admin_auth(&headers) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized admin token"})));
    }

    match state.db.get_platform_distribution() {
        Ok(dist) => (StatusCode::OK, Json(serde_json::to_value(dist).unwrap())),
        Err(err) => {
            error!("[ADMIN] Failed to get platform distribution: {:?}", err);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to get platform analytics"})))
        }
    }
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
        info!("[ADMIN] Created & broadcasted announcement #{}", id);

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
        for user_entry in state.active_sessions.iter() {
            for tx in user_entry.value().iter() {
                let _ = tx.send(Message::Text(frame_str.clone()));
            }
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
        debug!("[ADMIN] Listed announcements ({} total)", list.len());
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
    info!("[ADMIN] Deleted announcement #{}", id);
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
        .map(|s| s.split(',').next().unwrap_or(s).trim())
        .unwrap_or("127.0.0.1");

    if state.is_ip_banned(client_ip) {
        warn!("[FIREWALL] Blocked check-account probe from banned IP: {}", client_ip);
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "error": "IP address is banned by administrator",
            })),
        );
    }

    let clean_u = clean_user(&username);
    match state.db.get_user(&clean_u) {
        Ok(Some(user)) => {
            debug!("[HTTP API] Account existence check for '@{}' from IP: {} -> EXISTS", clean_u, client_ip);
            (
                StatusCode::OK,
                Json(json!({
                    "exists": true,
                    "username": user.username,
                    "ed25519_pubkey": user.ed25519_pubkey,
                })),
            )
        }
        _ => {
            debug!("[HTTP API] Account existence check for '@{}' from IP: {} -> NOT FOUND", clean_u, client_ip);
            (
                StatusCode::OK,
                Json(json!({
                    "exists": false,
                })),
            )
        }
    }
}

// Public Announcements REST Endpoint
async fn public_announcements_handler(
    State(state): State<AppState>,
) -> Json<Value> {
    if let Ok(list) = state.db.list_announcements() {
        if !list.is_empty() {
            debug!("[HTTP API] Retrieved {} public announcements", list.len());
            return Json(serde_json::to_value(list).unwrap());
        }
    }

    debug!("[HTTP API] Serving default system announcement");
    Json(json!([
        {
            "id": 1,
            "message": "Welcome to Vexta V2 High-Performance Rust Relay Bridge (vexta-api.nexusec.space).",
            "created_at": chrono::Utc::now().timestamp(),
        }
    ]))
}

// Comprehensive Health Check Endpoint
async fn health_handler(
    State(state): State<AppState>,
) -> (StatusCode, Json<Value>) {
    let db_ping_ok = state.db.ping().is_ok();
    let db_health = state.db.get_db_health_fast().ok();
    let maintenance = state.is_maintenance_enabled();
    let uptime = chrono::Utc::now().timestamp() - state.start_time;
    let active_sessions = state.active_sessions_count();
    let (total_msgs, total_bytes) = state.get_traffic_stats();

    let is_healthy = db_ping_ok;
    let status_str = if !is_healthy {
        "unhealthy"
    } else if maintenance {
        "maintenance"
    } else {
        "ok"
    };

    let http_status = if is_healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    debug!("[HTTP API] Health check evaluated -> status: '{}' (DB: {}, Sessions: {}, Uptime: {}s)", status_str, if db_ping_ok { "connected" } else { "error" }, active_sessions, uptime);

    let payload = json!({
        "status": status_str,
        "service": "vexta-bridge-v2",
        "version": state::SERVER_VERSION,
        "server_name": state::SERVER_NAME,
        "uptime_seconds": uptime,
        "timestamp": chrono::Utc::now().timestamp(),
        "active_ws_sessions": active_sessions,
        "maintenance_mode": maintenance,
        "database": {
            "status": if db_ping_ok { "connected" } else { "error" },
            "integrity": db_health.as_ref().map(|h| h.integrity_check.as_str()).unwrap_or("unknown"),
            "size_bytes": db_health.as_ref().map(|h| h.total_size_bytes).unwrap_or(0),
            "wal_size_bytes": db_health.as_ref().map(|h| h.wal_size_bytes).unwrap_or(0),
        },
        "telemetry": {
            "total_messages_relayed": total_msgs,
            "total_bytes_relayed": total_bytes,
        }
    });

    (http_status, Json(payload))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_handler_normal_and_maintenance() {
        let temp_dir = std::env::temp_dir();
        let db_file = temp_dir.join(format!("test_health_handler_{}.db", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)));
        let db_path = db_file.to_str().unwrap();

        let state = AppState::new(db_path);

        // Normal health check
        let (status, Json(val)) = health_handler(State(state.clone())).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(val["status"], "ok");
        assert_eq!(val["service"], "vexta-bridge-v2");
        assert_eq!(val["version"], state::SERVER_VERSION);
        assert_eq!(val["database"]["status"], "connected");
        assert_eq!(val["database"]["integrity"], "ok");
        assert_eq!(val["maintenance_mode"], false);

        // Maintenance mode health check
        state.set_maintenance(true);
        let (m_status, Json(m_val)) = health_handler(State(state.clone())).await;
        assert_eq!(m_status, StatusCode::OK);
        assert_eq!(m_val["status"], "maintenance");
        assert_eq!(m_val["maintenance_mode"], true);

        // Cleanup
        let _ = std::fs::remove_file(&db_file);
        let _ = std::fs::remove_file(format!("{}-wal", db_path));
        let _ = std::fs::remove_file(format!("{}-shm", db_path));
    }
}


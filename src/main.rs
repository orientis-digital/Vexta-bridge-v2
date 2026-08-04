mod admin_html;
mod crypto;
mod db;
mod models;
mod state;
mod ws;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Json},
    routing::{delete, get},
    Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use state::AppState;
use std::net::SocketAddr;

use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() {
    // 1. Initialize Logging
    tracing_subscriber::registry()
        .with(EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "vexta_bridge_v2=info,tower_http=info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 2. Ensure downloads directory exists
    let downloads_dir = std::path::PathBuf::from("downloads");
    std::fs::create_dir_all(&downloads_dir).ok();

    // 3. Initialize Shared App State (SQLite DB + Ed25519 Server Crypto)
    let state = AppState::new("vexta_bridge_v2.db");

    // 4. Configure Full CORS Middleware (Cloudflare Tunnel Compatible)
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // 5. Build Axum Router with Admin Console, Public APIs & Static Downloads Hosting
    let app = Router::new()
        // WebSocket Relay
        .route("/ws/chat/", get(ws::ws_handler))
        
        // Public REST Endpoints
        .route("/api/check-account/:username", get(check_account_handler))
        .route("/api/announcements/", get(public_announcements_handler))
        
        // Static App Downloads Hosting (e.g. /downloads/vexta-v2_amd64.AppImage)
        .nest_service("/downloads", ServeDir::new("downloads"))
        .nest_service("/static/downloads", ServeDir::new("downloads"))
        
        // Private Embedded Admin UI
        .route("/admin/", get(admin_ui_handler))
        
        // Protected Admin REST APIs (Requires X-Admin-Secret Header)
        .route("/api/admin/stats", get(admin_stats_handler))
        .route("/api/admin/users", get(admin_list_users_handler))
        .route("/api/admin/users/:username", delete(admin_delete_user_handler))
        .route("/api/admin/announcements", get(admin_list_announcements_handler).post(admin_post_announcement_handler))
        .route("/api/admin/announcements/:id", delete(admin_delete_announcement_handler))
        
        .layer(cors)
        .with_state(state);

    // 6. Start TCP Server on Port 8000
    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    info!("🚀 Vexta V2 Rust Bridge listening on http://{} (Downloads: http://127.0.0.1:8000/downloads/)", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// Check Admin Secret Token Header
fn verify_admin_auth(headers: &HeaderMap) -> bool {
    let expected_secret = std::env::var("ADMIN_SECRET_TOKEN").unwrap_or_else(|_| "vexta_admin_secret_key_2026".into());
    if let Some(token) = headers.get("x-admin-secret").or_else(|| headers.get("authorization")) {
        if let Ok(str_val) = token.to_str() {
            let clean_val = str_val.trim_start_matches("Bearer ").trim();
            return clean_val == expected_secret;
        }
    }
    false
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
        return (
            StatusCode::OK,
            Json(json!({
                "active_ws_sessions": active_sessions,
                "total_users": stats.total_users,
                "total_queued_offline_messages": stats.total_queued_offline_messages,
                "total_registered_devices": stats.total_registered_devices,
                "total_announcements": stats.total_announcements,
            })),
        );
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
        info!("[Admin Console] Created broadcast announcement #{}", id);
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

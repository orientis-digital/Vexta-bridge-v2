mod crypto;
mod db;
mod models;
mod state;
mod ws;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::get,
    Router,
};
use serde_json::{json, Value};
use state::AppState;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
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

    // 2. Initialize Shared App State (SQLite DB + Ed25519 Server Crypto)
    let state = AppState::new("vexta_bridge_v2.db");

    // 3. Configure Full CORS Middleware (Cloudflare Tunnel & Cross-Origin Compatible)
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // 4. Build Axum Router
    let app = Router::new()
        .route("/ws/chat/", get(ws::ws_handler))
        .route("/api/check-account/:username", get(check_account_handler))
        .route("/api/announcements/", get(announcements_handler))
        .layer(cors)
        .with_state(state);

    // 5. Start TCP Server on Port 8000
    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    info!("🚀 Vexta V2 Rust Bridge listening on http://{} (Cloudflare Tunnel Ready: vexta-api.nexusec.space)", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// Lightweight HTTP REST Account Existence Check (Cloudflare Proxy Aware)
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

// Announcements REST Endpoint
async fn announcements_handler() -> Json<Value> {
    Json(json!([
        {
            "id": 1,
            "message": "Welcome to Vexta V2 High-Performance Rust Relay Bridge (vexta-api.nexusec.space).",
            "created_at": chrono::Utc::now().to_rfc3339(),
        }
    ]))
}

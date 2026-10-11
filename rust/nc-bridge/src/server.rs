use crate::proxy::bridge_session;
use crate::NodeManager;
use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use nc_dropfile::{CallerInfo, CallerRole, PlatformInfo};
use serde::Deserialize;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::process::Command;
use tower_http::services::ServeDir;
use tracing::{error, info, warn};

#[derive(Debug, Deserialize)]
pub struct ConnectQuery {
    pub handle: Option<String>,
    pub platform_id: Option<String>,
    pub display_name: Option<String>,
}

/// Create the Axum HTTP & WebSocket application router.
pub fn create_router(manager: Arc<NodeManager>) -> Router {
    let static_dir = manager.config().static_dir.clone();
    let serve_dir = ServeDir::new(&static_dir).append_index_html_on_directories(true);

    Router::new()
        .route("/health", get(health_handler))
        .route("/ws/connect", get(ws_connect_handler))
        .fallback_service(serve_dir)
        .with_state(manager)
}

async fn health_handler(State(manager): State<Arc<NodeManager>>) -> Response {
    let active = manager.active_count().await;
    axum::response::Json(serde_json::json!({
        "status": "ok",
        "service": "nc-bridge",
        "active_sessions": active,
        "max_nodes": manager.config().max_nodes,
    }))
    .into_response()
}

async fn ws_connect_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<ConnectQuery>,
    State(manager): State<Arc<NodeManager>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, query, manager))
}

async fn handle_socket(socket: WebSocket, query: ConnectQuery, manager: Arc<NodeManager>) {
    let Some(node_index) = manager.allocate_node().await else {
        warn!("Rejecting connection: maximum concurrent nodes reached");
        return;
    };

    info!(node = node_index, "Allocated virtual node for new session");

    // Bind an ephemeral TCP listener on localhost to receive nc-door's connection
    let tcp_listener = match TcpListener::bind("127.0.0.1:0").await {
        Ok(l) => l,
        Err(e) => {
            error!("Failed to bind loopback TCP listener: {e}");
            manager.release_node(node_index).await;
            return;
        }
    };

    let loopback_port = match tcp_listener.local_addr() {
        Ok(addr) => addr.port(),
        Err(e) => {
            error!("Failed to get local address for listener: {e}");
            manager.release_node(node_index).await;
            return;
        }
    };

    let handle = query.handle.unwrap_or_else(|| format!("guest_{node_index}"));
    let platform_id = query.platform_id.unwrap_or_else(|| format!("anon_{node_index}"));

    let caller = CallerInfo {
        platform_id: platform_id.clone(),
        handle: handle.clone(),
        display_name: query.display_name.or_else(|| Some(handle.clone())),
        public_key: None,
        role: CallerRole::Player,
        locale: Some("en-US".to_string()),
    };

    let platform = Some(PlatformInfo {
        name: "web".to_string(),
        guild_id: None,
        channel_id: None,
        session_id: Some(format!("node_{node_index}_{loopback_port}")),
    });

    let active_node = match manager.prepare_session(node_index, loopback_port, caller, platform).await {
        Ok(n) => n,
        Err(e) => {
            error!("Failed to prepare session dropfile: {e}");
            manager.release_node(node_index).await;
            return;
        }
    };

    // Spawn nc-door child process
    let config = manager.config();
    let mut cmd = Command::new(&config.nc_door_binary);
    cmd.args([
        "--dir",
        config.campaign_dir.to_str().unwrap_or("."),
        "--dropfile",
        active_node.dropfile_path.to_str().unwrap_or(""),
        "--socket-port",
        &loopback_port.to_string(),
        "--encoding",
        "cp437",
        "--color-mode",
        "ansi16",
    ]);

    info!(
        node = node_index,
        port = loopback_port,
        cmd = ?cmd,
        "Spawning nc-door"
    );

    let child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to spawn nc-door binary: {e}");
            manager.release_node(node_index).await;
            return;
        }
    };

    // Accept incoming TCP connection from nc-door
    let tcp_stream = match tokio::time::timeout(std::time::Duration::from_secs(5), tcp_listener.accept()).await {
        Ok(Ok((stream, _))) => stream,
        Ok(Err(e)) => {
            error!("Failed to accept connection from nc-door: {e}");
            manager.release_node(node_index).await;
            return;
        }
        Err(_) => {
            error!("Timed out waiting for nc-door to connect to loopback port {loopback_port}");
            manager.release_node(node_index).await;
            return;
        }
    };

    info!(node = node_index, "nc-door connected. Bridging stream with WebSocket.");

    // Pump I/O between WebSocket and nc-door
    if let Err(e) = bridge_session(socket, tcp_stream, child).await {
        error!(node = node_index, "Bridge session error: {e}");
    }

    info!(node = node_index, "Session finished. Releasing node.");
    manager.release_node(node_index).await;
}

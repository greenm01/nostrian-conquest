use nc_bridge::server::create_router;
use nc_bridge::{BridgeConfig, NodeManager};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = BridgeConfig::default();
    let bind_addr = format!("{}:{}", config.bind_host, config.bind_port);
    let manager = Arc::new(NodeManager::new(config.clone()));

    let app = create_router(manager.clone());
    let listener = TcpListener::bind(&bind_addr).await?;

    info!(
        addr = %bind_addr,
        max_nodes = %config.max_nodes,
        "nc-bridge HTTP & WebSocket server listening"
    );

    println!(
        "nc-bridge: listening on http://{} (max {} concurrent nodes)",
        bind_addr, config.max_nodes
    );

    axum::serve(listener, app).await?;
    Ok(())
}

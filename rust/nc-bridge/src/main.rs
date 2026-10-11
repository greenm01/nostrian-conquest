use nc_bridge::{BridgeConfig, NodeManager};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = BridgeConfig::default();
    let manager = NodeManager::new(config.clone());

    info!(
        host = %config.bind_host,
        port = %config.bind_port,
        max_nodes = %config.max_nodes,
        "nc-bridge web terminal daemon initialized"
    );

    println!(
        "nc-bridge: listening on {}:{} (max {} concurrent nodes)",
        config.bind_host, config.bind_port, config.max_nodes
    );

    let active = manager.active_count().await;
    println!("Active sessions: {active}");

    Ok(())
}

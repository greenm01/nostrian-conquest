//! Platform-agnostic web terminal bridge and virtual node manager for Nostrian Conquest.
//!
//! Bridges incoming WebSocket connections from Discord Activities, Matrix Widgets,
//! and standalone browsers to `nc-door` child processes using `DOOR.KDL` dropfiles.

use nc_dropfile::{CallerInfo, DoorKdl, PlatformInfo, SessionInfo, TerminalInfo, TransportInfo, TransportMode};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Configuration parameters for the bridge server.
#[derive(Debug, Clone)]
pub struct BridgeConfig {
    pub bind_host: String,
    pub bind_port: u16,
    pub campaign_dir: PathBuf,
    pub nc_door_binary: PathBuf,
    pub max_nodes: usize,
    pub session_timeout_seconds: u32,
    pub node_base_dir: PathBuf,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            bind_host: "127.0.0.1".to_string(),
            bind_port: 8080,
            campaign_dir: PathBuf::from("."),
            nc_door_binary: PathBuf::from("nc-door"),
            max_nodes: 16,
            session_timeout_seconds: 1800,
            node_base_dir: PathBuf::from("/tmp/nc-nodes"),
        }
    }
}

/// A currently active virtual node session.
#[derive(Debug, Clone)]
pub struct ActiveNode {
    pub node_index: usize,
    pub caller_handle: String,
    pub platform_id: String,
    pub loopback_port: u16,
    pub dropfile_path: PathBuf,
}

/// Virtual node manager that handles allocation, dropfile generation,
/// and process lifecycle.
pub struct NodeManager {
    config: BridgeConfig,
    active_nodes: Arc<Mutex<HashMap<usize, ActiveNode>>>,
}

impl NodeManager {
    pub fn new(config: BridgeConfig) -> Self {
        Self {
            config,
            active_nodes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Allocate the next free virtual node index.
    pub async fn allocate_node(&self) -> Option<usize> {
        let active = self.active_nodes.lock().await;
        for i in 1..=self.config.max_nodes {
            if !active.contains_key(&i) {
                return Some(i);
            }
        }
        None
    }

    /// Prepare a dropfile for a new session and record the node as active.
    pub async fn prepare_session(
        &self,
        node_index: usize,
        caller: CallerInfo,
        platform: Option<PlatformInfo>,
    ) -> Result<ActiveNode, Box<dyn std::error::Error + Send + Sync>> {
        let loopback_port = 5000 + (node_index as u16);
        let node_dir = self.config.node_base_dir.join(format!("node_{node_index}"));
        std::fs::create_dir_all(&node_dir)?;
        let dropfile_path = node_dir.join("door.kdl");

        let dropfile = DoorKdl {
            version: 1,
            platform,
            caller: caller.clone(),
            session: SessionInfo {
                time_limit_seconds: Some(self.config.session_timeout_seconds),
                idle_timeout_seconds: Some(300),
                node: node_index as u32,
            },
            transport: TransportInfo {
                mode: TransportMode::LoopbackTcp {
                    port: loopback_port,
                    host: Some("127.0.0.1".to_string()),
                },
            },
            terminal: TerminalInfo::default(),
            callbacks: None,
        };

        std::fs::write(&dropfile_path, dropfile.to_kdl_string())?;

        let active = ActiveNode {
            node_index,
            caller_handle: caller.handle,
            platform_id: caller.platform_id,
            loopback_port,
            dropfile_path,
        };

        let mut nodes = self.active_nodes.lock().await;
        nodes.insert(node_index, active.clone());

        Ok(active)
    }

    /// Release a virtual node upon session completion.
    pub async fn release_node(&self, node_index: usize) {
        let mut nodes = self.active_nodes.lock().await;
        if let Some(node) = nodes.remove(&node_index) {
            let _ = std::fs::remove_file(&node.dropfile_path);
        }
    }

    /// Count current active players.
    pub async fn active_count(&self) -> usize {
        let nodes = self.active_nodes.lock().await;
        nodes.len()
    }
}

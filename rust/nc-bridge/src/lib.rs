//! Platform-agnostic web terminal bridge and virtual node manager for Nostrian Conquest.
//!
//! Bridges incoming WebSocket connections from Discord Activities, Matrix Widgets,
//! and standalone browsers to `nc-door` child processes using `DOOR.KDL` dropfiles.

pub mod community;
pub mod proxy;
pub mod server;

use nc_dropfile::{CallbacksInfo, CallerInfo, DoorKdl, PlatformInfo, SessionInfo, TerminalInfo, TransportInfo, TransportMode};
use serde::{Deserialize, Serialize};
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
    pub static_dir: PathBuf,
    pub event_forward_url: Option<String>,
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
            static_dir: PathBuf::from("static"),
            event_forward_url: None,
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
    pub callback_token: String,
}

/// Payload sent by an in-game door process to the internal event webhook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeEventPayload {
    pub player: Option<String>,
    pub seat: Option<usize>,
    pub event_type: Option<String>,
    pub summary: String,
}

/// State of a virtual node slot.
#[derive(Debug, Clone)]
pub enum NodeSlot {
    Reserved,
    Active(ActiveNode),
}

/// Virtual node manager that handles allocation, dropfile generation,
/// and process lifecycle.
pub struct NodeManager {
    config: BridgeConfig,
    active_nodes: Arc<Mutex<HashMap<usize, NodeSlot>>>,
}

impl NodeManager {
    pub fn new(config: BridgeConfig) -> Self {
        Self {
            config,
            active_nodes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn config(&self) -> &BridgeConfig {
        &self.config
    }

    /// Allocate the next free virtual node index, immediately reserving the slot.
    pub async fn allocate_node(&self) -> Option<usize> {
        let mut active = self.active_nodes.lock().await;
        for i in 1..=self.config.max_nodes {
            if !active.contains_key(&i) {
                active.insert(i, NodeSlot::Reserved);
                return Some(i);
            }
        }
        None
    }

    /// Prepare a dropfile for a new session and record the node as active.
    pub async fn prepare_session(
        &self,
        node_index: usize,
        loopback_port: u16,
        caller: CallerInfo,
        platform: Option<PlatformInfo>,
    ) -> Result<ActiveNode, Box<dyn std::error::Error + Send + Sync>> {
        let node_dir = self.config.node_base_dir.join(format!("node_{node_index}"));
        std::fs::create_dir_all(&node_dir)?;
        let callback_token = format!("tok_node_{node_index}_{loopback_port}");

        let callbacks = Some(CallbacksInfo {
            event_webhook: Some(format!(
                "http://{}:{}/internal/events/{}",
                self.config.bind_host, self.config.bind_port, node_index
            )),
            secret: Some(callback_token.clone()),
        });

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
            callbacks,
        };

        let kdl_path = node_dir.join("door.kdl");
        std::fs::write(&kdl_path, dropfile.to_kdl_string())?;

        // Write DOOR32.SYS for standard BBS dropfile compatibility with nc-door
        let door32_path = node_dir.join("DOOR32.SYS");
        let timeout_mins = (self.config.session_timeout_seconds / 60).max(1);
        let real_name = caller.display_name.as_deref().unwrap_or(&caller.handle);
        let door32_content = format!(
            "2\r\n0\r\n38400\r\nNostrian Conquest Bridge\r\n1\r\n{}\r\n{}\r\n30\r\n{}\r\n1\r\n80\r\n25\r\n",
            real_name, caller.handle, timeout_mins
        );
        std::fs::write(&door32_path, door32_content)?;

        let active = ActiveNode {
            node_index,
            caller_handle: caller.handle,
            platform_id: caller.platform_id,
            loopback_port,
            dropfile_path: door32_path,
            callback_token,
        };

        let mut nodes = self.active_nodes.lock().await;
        nodes.insert(node_index, NodeSlot::Active(active.clone()));

        Ok(active)
    }

    /// Verify that an incoming callback request is from a legitimate active node session.
    pub async fn verify_node_token(&self, node_index: usize, token: &str) -> Option<ActiveNode> {
        let nodes = self.active_nodes.lock().await;
        if let Some(NodeSlot::Active(active)) = nodes.get(&node_index) {
            if active.callback_token == token {
                return Some(active.clone());
            }
        }
        None
    }

    /// Release a virtual node upon session completion.
    pub async fn release_node(&self, node_index: usize) {
        let mut nodes = self.active_nodes.lock().await;
        if let Some(slot) = nodes.remove(&node_index) {
            if let NodeSlot::Active(node) = slot {
                let _ = std::fs::remove_file(&node.dropfile_path);
                let _ = std::fs::remove_file(node.dropfile_path.with_file_name("door.kdl"));
            }
        }
    }

    /// Count current active players.
    pub async fn active_count(&self) -> usize {
        let nodes = self.active_nodes.lock().await;
        nodes.values().filter(|s| matches!(s, NodeSlot::Active(_))).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nc_dropfile::{CallerRole, DoorKdl, Dropfile};

    #[tokio::test]
    async fn node_manager_lifecycle() {
        let temp_dir = std::env::temp_dir().join(format!("nc_bridge_test_{}", std::process::id()));
        let config = BridgeConfig {
            max_nodes: 2,
            node_base_dir: temp_dir.clone(),
            ..BridgeConfig::default()
        };

        let manager = NodeManager::new(config);

        // Allocate node 1
        let n1 = manager.allocate_node().await.expect("should allocate node 1");
        assert_eq!(n1, 1);

        let caller1 = CallerInfo {
            platform_id: "test_user_1".to_string(),
            handle: "vorn".to_string(),
            display_name: Some("Lord Vorn".to_string()),
            public_key: None,
            role: CallerRole::Player,
            locale: Some("en-US".to_string()),
        };

        let session1 = manager.prepare_session(n1, 5001, caller1, None).await.expect("session prepared");
        assert_eq!(manager.active_count().await, 1);
        assert!(session1.dropfile_path.exists());

        // Verify that the generated dropfiles are valid
        let loaded_kdl = DoorKdl::load_from_file(&session1.dropfile_path.with_file_name("door.kdl")).expect("valid KDL on disk");
        assert_eq!(loaded_kdl.caller.handle, "vorn");
        assert_eq!(loaded_kdl.session.node, 1);
        assert!(loaded_kdl.callbacks.is_some());

        let loaded_door32 = Dropfile::load_autodetect(&session1.dropfile_path).expect("valid DOOR32 on disk");
        assert_eq!(loaded_door32.alias.as_deref(), Some("vorn"));

        // Verify callback token authentication
        assert!(manager.verify_node_token(n1, &session1.callback_token).await.is_some());
        assert!(manager.verify_node_token(n1, "invalid_secret").await.is_none());
        assert!(manager.verify_node_token(99, &session1.callback_token).await.is_none());

        // Allocate node 2
        let n2 = manager.allocate_node().await.expect("should allocate node 2");
        assert_eq!(n2, 2);

        // Allocate node 3 should fail because max_nodes = 2
        let n3 = manager.allocate_node().await;
        assert!(n3.is_none());

        // Release node 1
        manager.release_node(n1).await;
        assert_eq!(manager.active_count().await, 0);
        assert!(!session1.dropfile_path.exists());

        // Clean up test temp dir
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}

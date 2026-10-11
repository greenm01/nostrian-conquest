//! Dropfile parsing and serialization for Nostrian Conquest.
//!
//! Provides data structures and parsers for both modern `DOOR.KDL` files and
//! legacy BBS dropfiles (`DOOR32.SYS`, `DOOR.SYS`, `CHAIN.TXT`).

use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

/// Errors that can occur when reading or parsing a dropfile.
#[derive(Debug, Error)]
pub enum DropfileError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("KDL parse error: {0}")]
    KdlParse(String),

    #[error("unrecognised dropfile format or filename: '{0}'")]
    UnknownFormat(String),

    #[error("missing required field: '{0}'")]
    MissingField(&'static str),
}

/// Authorization role of the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CallerRole {
    #[default]
    Player,
    Spectator,
    Sysop,
}

impl CallerRole {
    pub fn from_str_name(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "spectator" => Self::Spectator,
            "sysop" => Self::Sysop,
            _ => Self::Player,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Player => "player",
            Self::Spectator => "spectator",
            Self::Sysop => "sysop",
        }
    }
}

/// Target input device category reported by the client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum InputDevice {
    #[default]
    Keyboard,
    Touchscreen,
    Hybrid,
}

impl InputDevice {
    pub fn from_str_name(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "touchscreen" => Self::Touchscreen,
            "hybrid" => Self::Hybrid,
            _ => Self::Keyboard,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Keyboard => "keyboard",
            Self::Touchscreen => "touchscreen",
            Self::Hybrid => "hybrid",
        }
    }
}

/// Transport delivery mechanism between the bridge and `nc-door`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum TransportMode {
    LoopbackTcp {
        port: u16,
        host: Option<String>,
    },
    UnixSocket {
        path: String,
    },
    Stdio,
}

/// Host platform information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformInfo {
    pub name: String,
    pub guild_id: Option<String>,
    pub channel_id: Option<String>,
    pub session_id: Option<String>,
}

/// Player identity and session authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallerInfo {
    pub platform_id: String,
    pub handle: String,
    pub display_name: Option<String>,
    pub public_key: Option<String>,
    pub role: CallerRole,
    pub locale: Option<String>,
}

/// Session duration and concurrency limits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionInfo {
    pub time_limit_seconds: Option<u32>,
    pub idle_timeout_seconds: Option<u32>,
    pub node: u32,
}

/// Transport configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportInfo {
    pub mode: TransportMode,
}

/// Terminal geometry and rendering capabilities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalInfo {
    pub cols: u16,
    pub rows: u16,
    pub resizable: bool,
    pub encoding: String,
    pub color: String,
    pub client: Option<String>,
    pub input_device: InputDevice,
}

impl Default for TerminalInfo {
    fn default() -> Self {
        Self {
            cols: 80,
            rows: 25,
            resizable: false,
            encoding: "cp437".to_string(),
            color: "ansi16".to_string(),
            client: Some("xterm.js".to_string()),
            input_device: InputDevice::Keyboard,
        }
    }
}

/// Outbound webhook callback settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CallbacksInfo {
    pub event_webhook: Option<String>,
    pub secret: Option<String>,
}

/// In-memory representation of a `DOOR.KDL` document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoorKdl {
    pub version: u32,
    pub platform: Option<PlatformInfo>,
    pub caller: CallerInfo,
    pub session: SessionInfo,
    pub transport: TransportInfo,
    pub terminal: TerminalInfo,
    pub callbacks: Option<CallbacksInfo>,
}

impl DoorKdl {
    /// Render the document to a standard KDL string.
    pub fn to_kdl_string(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("version {}\n\n", self.version));

        if let Some(plat) = &self.platform {
            out.push_str(&format!("platform \"{}\" {{\n", plat.name));
            if let Some(g) = &plat.guild_id {
                out.push_str(&format!("    guild_id \"{g}\"\n"));
            }
            if let Some(c) = &plat.channel_id {
                out.push_str(&format!("    channel_id \"{c}\"\n"));
            }
            if let Some(s) = &plat.session_id {
                out.push_str(&format!("    session_id \"{s}\"\n"));
            }
            out.push_str("}\n\n");
        }

        out.push_str("caller {\n");
        out.push_str(&format!("    platform_id \"{}\"\n", self.caller.platform_id));
        out.push_str(&format!("    handle \"{}\"\n", self.caller.handle));
        if let Some(d) = &self.caller.display_name {
            out.push_str(&format!("    display_name \"{d}\"\n"));
        }
        if let Some(pk) = &self.caller.public_key {
            out.push_str(&format!("    public_key \"{pk}\"\n"));
        }
        out.push_str(&format!("    role \"{}\"\n", self.caller.role.as_str()));
        if let Some(loc) = &self.caller.locale {
            out.push_str(&format!("    locale \"{loc}\"\n"));
        }
        out.push_str("}\n\n");

        out.push_str("session {\n");
        if let Some(tl) = self.session.time_limit_seconds {
            out.push_str(&format!("    time_limit_seconds {tl}\n"));
        }
        if let Some(idle) = self.session.idle_timeout_seconds {
            out.push_str(&format!("    idle_timeout_seconds {idle}\n"));
        }
        out.push_str(&format!("    node {}\n", self.session.node));
        out.push_str("}\n\n");

        out.push_str("transport {\n");
        match &self.transport.mode {
            TransportMode::LoopbackTcp { port, host } => {
                let h = host.as_deref().unwrap_or("127.0.0.1");
                out.push_str(&format!("    mode \"loopback-tcp\" port={port} host=\"{h}\"\n"));
            }
            TransportMode::UnixSocket { path } => {
                out.push_str(&format!("    mode \"unix-socket\" path=\"{path}\"\n"));
            }
            TransportMode::Stdio => {
                out.push_str("    mode \"stdio\"\n");
            }
        }
        out.push_str("}\n\n");

        out.push_str("terminal {\n");
        out.push_str(&format!("    cols {}\n", self.terminal.cols));
        out.push_str(&format!("    rows {}\n", self.terminal.rows));
        out.push_str(&format!("    resizable #{}\n", self.terminal.resizable));
        out.push_str(&format!("    encoding \"{}\"\n", self.terminal.encoding));
        out.push_str(&format!("    color \"{}\"\n", self.terminal.color));
        if let Some(cl) = &self.terminal.client {
            out.push_str(&format!("    client \"{cl}\"\n"));
        }
        out.push_str(&format!("    input_device \"{}\"\n", self.terminal.input_device.as_str()));
        out.push_str("}\n");

        if let Some(cb) = &self.callbacks {
            if cb.event_webhook.is_some() {
                out.push_str("\ncallbacks {\n");
                if let Some(wh) = &cb.event_webhook {
                    if let Some(sec) = &cb.secret {
                        out.push_str(&format!("    event_webhook \"{wh}\" secret=\"{sec}\"\n"));
                    } else {
                        out.push_str(&format!("    event_webhook \"{wh}\"\n"));
                    }
                }
                out.push_str("}\n");
            }
        }

        out
    }

    /// Parse a `DOOR.KDL` document string.
    pub fn parse_kdl(text: &str) -> Result<Self, DropfileError> {
        let doc: kdl::KdlDocument = text.parse().map_err(|e: kdl::KdlError| DropfileError::KdlParse(e.to_string()))?;

        let version = doc.get("version")
            .and_then(|n| n.entries().first())
            .and_then(|e| e.value().as_integer())
            .unwrap_or(1) as u32;

        let platform = doc.get("platform").map(|n| {
            let name = n.entries().first().and_then(|e| e.value().as_string()).unwrap_or("unknown").to_string();
            let mut guild_id = None;
            let mut channel_id = None;
            let mut session_id = None;
            if let Some(children) = n.children() {
                guild_id = children.get("guild_id").and_then(|c| c.entries().first()).and_then(|e| e.value().as_string()).map(|s| s.to_string());
                channel_id = children.get("channel_id").and_then(|c| c.entries().first()).and_then(|e| e.value().as_string()).map(|s| s.to_string());
                session_id = children.get("session_id").and_then(|c| c.entries().first()).and_then(|e| e.value().as_string()).map(|s| s.to_string());
            }
            PlatformInfo { name, guild_id, channel_id, session_id }
        });

        let caller_node = doc.get("caller").ok_or(DropfileError::MissingField("caller"))?;
        let caller_children = caller_node.children().ok_or(DropfileError::MissingField("caller fields"))?;

        let platform_id = caller_children.get("platform_id")
            .and_then(|c| c.entries().first())
            .and_then(|e| e.value().as_string())
            .ok_or(DropfileError::MissingField("caller.platform_id"))?
            .to_string();

        let handle = caller_children.get("handle")
            .and_then(|c| c.entries().first())
            .and_then(|e| e.value().as_string())
            .ok_or(DropfileError::MissingField("caller.handle"))?
            .to_string();

        let display_name = caller_children.get("display_name")
            .and_then(|c| c.entries().first())
            .and_then(|e| e.value().as_string())
            .map(|s| s.to_string());

        let public_key = caller_children.get("public_key")
            .and_then(|c| c.entries().first())
            .and_then(|e| e.value().as_string())
            .map(|s| s.to_string());

        let role_str = caller_children.get("role")
            .and_then(|c| c.entries().first())
            .and_then(|e| e.value().as_string())
            .unwrap_or("player");
        let role = CallerRole::from_str_name(role_str);

        let locale = caller_children.get("locale")
            .and_then(|c| c.entries().first())
            .and_then(|e| e.value().as_string())
            .map(|s| s.to_string());

        let caller = CallerInfo {
            platform_id,
            handle,
            display_name,
            public_key,
            role,
            locale,
        };

        let session_node = doc.get("session").ok_or(DropfileError::MissingField("session"))?;
        let session_children = session_node.children().ok_or(DropfileError::MissingField("session fields"))?;

        let time_limit_seconds = session_children.get("time_limit_seconds")
            .and_then(|c| c.entries().first())
            .and_then(|e| e.value().as_integer())
            .map(|i| i as u32);

        let idle_timeout_seconds = session_children.get("idle_timeout_seconds")
            .and_then(|c| c.entries().first())
            .and_then(|e| e.value().as_integer())
            .map(|i| i as u32);

        let node = session_children.get("node")
            .and_then(|c| c.entries().first())
            .and_then(|e| e.value().as_integer())
            .unwrap_or(1) as u32;

        let session = SessionInfo {
            time_limit_seconds,
            idle_timeout_seconds,
            node,
        };

        let transport = if let Some(tn) = doc.get("transport") {
            let mut mode = TransportMode::Stdio;
            if let Some(ch) = tn.children() {
                if let Some(mn) = ch.get("mode") {
                    let mode_name = mn.entries().first().and_then(|e| e.value().as_string()).unwrap_or("stdio");
                    match mode_name {
                        "loopback-tcp" => {
                            let port = mn.get("port").and_then(|v| v.as_integer()).unwrap_or(5000) as u16;
                            let host = mn.get("host").and_then(|v| v.as_string()).map(|s| s.to_string());
                            mode = TransportMode::LoopbackTcp { port, host };
                        }
                        "unix-socket" => {
                            let path = mn.get("path").and_then(|v| v.as_string()).unwrap_or("").to_string();
                            mode = TransportMode::UnixSocket { path };
                        }
                        _ => mode = TransportMode::Stdio,
                    }
                }
            }
            TransportInfo { mode }
        } else {
            TransportInfo { mode: TransportMode::Stdio }
        };

        let terminal = if let Some(tm) = doc.get("terminal") {
            let mut term = TerminalInfo::default();
            if let Some(ch) = tm.children() {
                if let Some(c) = ch.get("cols").and_then(|n| n.entries().first()).and_then(|e| e.value().as_integer()) {
                    term.cols = c as u16;
                }
                if let Some(r) = ch.get("rows").and_then(|n| n.entries().first()).and_then(|e| e.value().as_integer()) {
                    term.rows = r as u16;
                }
                if let Some(b) = ch.get("resizable").and_then(|n| n.entries().first()).and_then(|e| e.value().as_bool()) {
                    term.resizable = b;
                }
                if let Some(enc) = ch.get("encoding").and_then(|n| n.entries().first()).and_then(|e| e.value().as_string()) {
                    term.encoding = enc.to_string();
                }
                if let Some(col) = ch.get("color").and_then(|n| n.entries().first()).and_then(|e| e.value().as_string()) {
                    term.color = col.to_string();
                }
                if let Some(cl) = ch.get("client").and_then(|n| n.entries().first()).and_then(|e| e.value().as_string()) {
                    term.client = Some(cl.to_string());
                }
                if let Some(inp) = ch.get("input_device").and_then(|n| n.entries().first()).and_then(|e| e.value().as_string()) {
                    term.input_device = InputDevice::from_str_name(inp);
                }
            }
            term
        } else {
            TerminalInfo::default()
        };

        let callbacks = doc.get("callbacks").and_then(|cb| {
            cb.children().map(|ch| {
                let mut cbs = CallbacksInfo::default();
                if let Some(wh) = ch.get("event_webhook") {
                    cbs.event_webhook = wh.entries().first().and_then(|e| e.value().as_string()).map(|s| s.to_string());
                    cbs.secret = wh.get("secret").and_then(|v| v.as_string()).map(|s| s.to_string());
                }
                cbs
            })
        });

        Ok(DoorKdl {
            version,
            platform,
            caller,
            session,
            transport,
            terminal,
            callbacks,
        })
    }

    /// Load and parse from file path.
    pub fn load_from_file(path: &Path) -> Result<Self, DropfileError> {
        let text = std::fs::read_to_string(path)?;
        Self::parse_kdl(&text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_door_kdl() {
        let original = DoorKdl {
            version: 1,
            platform: Some(PlatformInfo {
                name: "discord".to_string(),
                guild_id: Some("12345".to_string()),
                channel_id: Some("67890".to_string()),
                session_id: Some("sess_abc".to_string()),
            }),
            caller: CallerInfo {
                platform_id: "user_99".to_string(),
                handle: "starmaster".to_string(),
                display_name: Some("Lord Vorn".to_string()),
                public_key: Some("npub1xyz".to_string()),
                role: CallerRole::Player,
                locale: Some("en-US".to_string()),
            },
            session: SessionInfo {
                time_limit_seconds: Some(1800),
                idle_timeout_seconds: Some(300),
                node: 3,
            },
            transport: TransportInfo {
                mode: TransportMode::LoopbackTcp {
                    port: 5042,
                    host: Some("127.0.0.1".to_string()),
                },
            },
            terminal: TerminalInfo {
                cols: 80,
                rows: 25,
                resizable: false,
                encoding: "cp437".to_string(),
                color: "ansi16".to_string(),
                client: Some("xterm.js".to_string()),
                input_device: InputDevice::Keyboard,
            },
            callbacks: Some(CallbacksInfo {
                event_webhook: Some("http://localhost:8080/events".to_string()),
                secret: Some("tok_123".to_string()),
            }),
        };

        let kdl_str = original.to_kdl_string();
        let parsed = DoorKdl::parse_kdl(&kdl_str).expect("should parse generated KDL");

        assert_eq!(original.caller.handle, parsed.caller.handle);
        assert_eq!(original.session.node, parsed.session.node);
        assert_eq!(original.session.time_limit_seconds, parsed.session.time_limit_seconds);
        assert_eq!(original.caller.role, parsed.caller.role);
    }
}

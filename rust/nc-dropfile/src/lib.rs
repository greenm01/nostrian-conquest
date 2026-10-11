//! Dropfile parsing and serialization for Nostrian Conquest.
//!
//! Provides data structures and parsers for both modern `DOOR.KDL` files and
//! legacy BBS dropfiles (`DOOR32.SYS`, `DOOR.SYS`, `CHAIN.TXT`).

pub mod cp437;
pub mod legacy;

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

/// The dropfile format specification variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DropfileFormat {
    #[default]
    DoorKdl,
    Door32,
    DoorSys,
    ChainTxt,
}

/// Transport connection type reported by the BBS dropfile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DoorConnectionType {
    #[default]
    Local,
    Serial,
    TelnetSocket,
    Unknown(u32),
}

/// Normalized dropfile data structure across both modern KDL and legacy formats.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Dropfile {
    /// Source format of the dropfile.
    pub format: DropfileFormat,
    /// Player handle / alias used to bind a game seat.
    pub alias: Option<String>,
    /// Fallback alias (e.g. DOOR.SYS real name when handle is absent or binds no seat).
    pub fallback_alias: Option<String>,
    /// True if the alias is the user's real name rather than a handle.
    pub alias_is_real_name: bool,
    /// Session time limit in minutes.
    pub timeout_minutes: Option<u32>,
    /// Terminal width in columns.
    pub screen_columns: Option<usize>,
    /// Terminal height in rows.
    pub screen_rows: Option<usize>,
    /// Connection transport type.
    pub connection_type: Option<DoorConnectionType>,
    /// Socket descriptor or port if passed.
    pub socket_descriptor: Option<u64>,
    /// In-game event callback webhook URL (from DOOR.KDL).
    pub event_webhook: Option<String>,
    /// In-game event callback bearer token secret (from DOOR.KDL).
    pub callback_secret: Option<String>,
    /// Input device hint (keyboard, touchscreen, hybrid).
    pub input_device: InputDevice,
    /// Encoding hint (cp437, utf-8).
    pub encoding: String,
    /// Caller platform ID (e.g. Discord snowflake, Matrix MXID).
    pub platform_id: Option<String>,
}

impl From<DoorKdl> for Dropfile {
    fn from(kdl: DoorKdl) -> Self {
        let connection_type = match &kdl.transport.mode {
            TransportMode::LoopbackTcp { .. } => Some(DoorConnectionType::TelnetSocket),
            TransportMode::UnixSocket { .. } => Some(DoorConnectionType::TelnetSocket),
            TransportMode::Stdio => Some(DoorConnectionType::Local),
        };

        let socket_descriptor = match &kdl.transport.mode {
            TransportMode::LoopbackTcp { port, .. } => Some(*port as u64),
            _ => None,
        };

        let event_webhook = kdl.callbacks.as_ref().and_then(|c| c.event_webhook.clone());
        let callback_secret = kdl.callbacks.as_ref().and_then(|c| c.secret.clone());

        Dropfile {
            format: DropfileFormat::DoorKdl,
            alias: Some(kdl.caller.handle.clone()),
            fallback_alias: kdl.caller.display_name,
            alias_is_real_name: false,
            timeout_minutes: kdl.session.time_limit_seconds.map(|s| s / 60),
            screen_columns: Some(kdl.terminal.cols as usize),
            screen_rows: Some(kdl.terminal.rows as usize),
            connection_type,
            socket_descriptor,
            event_webhook,
            callback_secret,
            input_device: kdl.terminal.input_device,
            encoding: kdl.terminal.encoding,
            platform_id: Some(kdl.caller.platform_id),
        }
    }
}

impl Dropfile {
    /// Auto-detect the dropfile format from the filename/extension and parse it.
    ///
    /// - `.kdl` or `door.kdl` -> modern [`DoorKdl`] parser
    /// - `DOOR32.SYS` -> legacy DOOR32 parser
    /// - `DOOR.SYS` -> legacy DOOR.SYS parser
    /// - `CHAIN.TXT` -> legacy WWIV CHAIN.TXT parser
    pub fn load_autodetect(path: &Path) -> Result<Self, DropfileError> {
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_ascii_uppercase();

        if filename.ends_with(".KDL") || filename == "DOOR.KDL" {
            let kdl = DoorKdl::load_from_file(path)?;
            return Ok(Dropfile::from(kdl));
        }

        let bytes = std::fs::read(path)?;
        let content = match String::from_utf8(bytes.clone()) {
            Ok(text) => text,
            Err(_) => cp437::cp437_to_string(&bytes),
        };
        let lines = legacy::split_lines(&content);

        match filename.as_str() {
            "DOOR32.SYS" => Ok(legacy::parse_door32(&lines)),
            "DOOR.SYS" => Ok(legacy::parse_door_sys(&lines)),
            "CHAIN.TXT" => Ok(legacy::parse_chain_txt(&lines)),
            other => Err(DropfileError::UnknownFormat(other.to_string())),
        }
    }
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

        let unified = Dropfile::from(parsed);
        assert_eq!(unified.alias.as_deref(), Some("starmaster"));
        assert_eq!(unified.timeout_minutes, Some(30));
        assert_eq!(unified.connection_type, Some(DoorConnectionType::TelnetSocket));
        assert_eq!(unified.socket_descriptor, Some(5042));
    }

    #[test]
    fn parse_door32_sample() {
        let text = "\
2\r\n\
1234\r\n\
38400\r\n\
Mystic BBS\r\n\
1\r\n\
John Doe\r\n\
StarMaster\r\n\
30\r\n\
45\r\n\
1\r\n\
80\r\n\
25\r\n";
        let lines = legacy::split_lines(text);
        let dropfile = legacy::parse_door32(&lines);
        assert_eq!(dropfile.format, DropfileFormat::Door32);
        assert_eq!(dropfile.alias.as_deref(), Some("StarMaster"));
        assert_eq!(dropfile.fallback_alias.as_deref(), Some("John Doe"));
        assert_eq!(dropfile.timeout_minutes, Some(45));
        assert_eq!(dropfile.screen_columns, Some(80));
        assert_eq!(dropfile.screen_rows, Some(25));
        assert_eq!(dropfile.connection_type, Some(DoorConnectionType::TelnetSocket));
        assert_eq!(dropfile.socket_descriptor, Some(1234));
    }

    #[test]
    fn parse_door_sys_sample() {
        let mut lines_vec = vec!["COM1:", "2400", "8", "1", "19200", "Y", "Y", "Y", "Y", "Jane Doe"];
        while lines_vec.len() < 18 {
            lines_vec.push("");
        }
        lines_vec.push("30"); // Line 19 = timeout minutes
        lines_vec.push("Y");  // Line 20 = ANSI
        lines_vec.push("25"); // Line 21 = rows
        while lines_vec.len() < 35 {
            lines_vec.push("");
        }
        lines_vec.push("GalacticAce"); // Line 36 = handle

        let text = lines_vec.join("\r\n");
        let lines = legacy::split_lines(&text);
        let dropfile = legacy::parse_door_sys(&lines);
        assert_eq!(dropfile.format, DropfileFormat::DoorSys);
        assert_eq!(dropfile.alias.as_deref(), Some("GalacticAce"));
        assert_eq!(dropfile.fallback_alias.as_deref(), Some("Jane Doe"));
        assert_eq!(dropfile.timeout_minutes, Some(30));
        assert_eq!(dropfile.screen_rows, Some(25));
    }

    #[test]
    fn autodetect_file_loading() {
        let dir = std::env::temp_dir().join(format!("nc_dropfile_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let door32_path = dir.join("DOOR32.SYS");
        std::fs::write(&door32_path, "2\n5555\n38400\nBBS\n1\nReal\nHandle\n30\n20\n1\n80\n25\n").unwrap();

        let loaded = Dropfile::load_autodetect(&door32_path).expect("autodetect DOOR32");
        assert_eq!(loaded.format, DropfileFormat::Door32);
        assert_eq!(loaded.alias.as_deref(), Some("Handle"));
        assert_eq!(loaded.socket_descriptor, Some(5555));

        let _ = std::fs::remove_dir_all(&dir);
    }
}

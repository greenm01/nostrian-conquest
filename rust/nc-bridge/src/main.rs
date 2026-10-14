use clap::Parser;
use nc_bridge::server::create_router;
use nc_bridge::{BridgeConfig, NodeManager};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{info, warn};

#[derive(Parser, Debug)]
#[command(name = "nc-bridge")]
#[command(about = "WebSocket PTY & Web Door Bridge for Nostrian Conquest", long_about = None)]
struct Cli {
    /// Enable authenticated multi-game community hosting.
    #[arg(long, env = "NC_GAMES_ROOT")]
    games_root: Option<PathBuf>,
    #[arg(long, env = "NC_PUBLIC_ORIGIN")]
    public_origin: Option<String>,
    #[arg(long, env = "NC_DISCORD_CLIENT_ID")]
    discord_client_id: Option<String>,
    #[arg(long, env = "NC_DISCORD_CLIENT_SECRET", hide_env_values = true)]
    discord_client_secret: Option<String>,
    /// OAuth client registered with matrix.org for one-click Matrix sign-in.
    #[arg(long, env = "NC_MATRIX_OAUTH_CLIENT_ID")]
    matrix_oauth_client_id: Option<String>,
    #[arg(long, env = "NC_MATRIX_OAUTH_CLIENT_SECRET", hide_env_values = true)]
    matrix_oauth_client_secret: Option<String>,
    /// Comma-separated relays asked for Nostr profile names at sign-in.
    #[arg(
        long,
        env = "NC_NOSTR_PROFILE_RELAYS",
        value_delimiter = ',',
        default_value = "wss://relay.damus.io,wss://nos.lol,wss://relay.primal.net"
    )]
    nostr_profile_relays: Vec<String>,

    /// Host interface to bind to
    #[arg(long, default_value = "127.0.0.1", env = "NC_BRIDGE_HOST")]
    host: String,

    /// Port to listen on
    #[arg(short, long, default_value_t = 8080, env = "NC_BRIDGE_PORT")]
    port: u16,

    /// Path to the campaign directory containing ncgame.db
    #[arg(short, long, default_value = ".", env = "NC_BRIDGE_CAMPAIGN_DIR")]
    campaign_dir: PathBuf,

    /// Path or name of the nc-door binary executable
    #[arg(long, default_value = "nc-door", env = "NC_BRIDGE_DOOR_BINARY")]
    nc_door: PathBuf,

    /// Maximum concurrent virtual node sessions
    #[arg(long, default_value_t = 16, env = "NC_BRIDGE_MAX_NODES")]
    max_nodes: usize,

    /// Session timeout in minutes
    #[arg(long, default_value_t = 30, env = "NC_BRIDGE_TIMEOUT_MINS")]
    timeout_mins: u32,

    /// Directory for temporary per-node dropfiles
    #[arg(long, default_value = "/tmp/nc-nodes", env = "NC_BRIDGE_NODE_DIR")]
    node_dir: PathBuf,

    /// Directory containing static web assets (index.html, style.css, app.js)
    #[arg(long, default_value = "static", env = "NC_BRIDGE_STATIC_DIR")]
    static_dir: PathBuf,

    /// Optional webhook URL to forward in-game tactical events (e.g. to nc-herald or Discord)
    #[arg(long, env = "NC_BRIDGE_EVENT_FORWARD_URL")]
    event_forward_url: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cli = Cli::parse();

    // Pre-flight directory checks
    if !cli.static_dir.exists() {
        warn!(
            path = %cli.static_dir.display(),
            "Static assets directory not found at configured path; web client may return 404"
        );
    }

    if !cli.campaign_dir.exists() {
        warn!(
            path = %cli.campaign_dir.display(),
            "Campaign directory not found; door sessions may fail if directory does not exist"
        );
    }

    let config = BridgeConfig {
        bind_host: cli.host,
        bind_port: cli.port,
        campaign_dir: cli.campaign_dir,
        nc_door_binary: cli.nc_door,
        max_nodes: cli.max_nodes,
        session_timeout_seconds: cli.timeout_mins.saturating_mul(60),
        node_base_dir: cli.node_dir,
        static_dir: cli.static_dir,
        event_forward_url: cli.event_forward_url,
    };

    let bind_addr = format!("{}:{}", config.bind_host, config.bind_port);
    let manager = Arc::new(NodeManager::new(config.clone()));

    let app = if let Some(root) = cli.games_root {
        nc_bridge::community::router(nc_bridge::community::Config {
            games_root: root,
            nc_door: config.nc_door_binary.clone(),
            static_dir: config.static_dir.clone(),
            origin: cli
                .public_origin
                .ok_or("--public-origin is required")?
                .trim_end_matches('/')
                .to_owned(),
            client_id: cli
                .discord_client_id
                .ok_or("Discord client ID is required")?,
            client_secret: cli
                .discord_client_secret
                .ok_or("Discord client secret is required")?,
            max_sessions: config.max_nodes,
            matrix: match (cli.matrix_oauth_client_id, cli.matrix_oauth_client_secret) {
                (Some(id), Some(secret)) => {
                    Some(nc_bridge::community::MatrixOAuth::matrix_org(id, secret))
                }
                (None, None) => None,
                _ => return Err("set both Matrix OAuth client ID and secret, or neither".into()),
            },
            nostr_relays: cli.nostr_profile_relays,
        })
        .map_err(std::io::Error::other)?
    } else {
        create_router(manager.clone())
    };
    let listener = TcpListener::bind(&bind_addr).await?;

    info!(
        addr = %bind_addr,
        max_nodes = %config.max_nodes,
        static_dir = %config.static_dir.display(),
        "nc-bridge HTTP & WebSocket server listening"
    );

    println!(
        "nc-bridge: listening on http://{} (max {} concurrent nodes)",
        bind_addr, config.max_nodes
    );

    axum::serve(listener, app).await?;
    Ok(())
}

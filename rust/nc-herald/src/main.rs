use clap::{Parser, Subcommand};
use nc_herald::{Herald, HeraldConfig, InGameEvent, PlatformTarget, TurnAnnouncement};
use std::sync::Arc;
use tracing::info;

#[derive(Parser)]
#[command(name = "nc-herald")]
#[command(about = "Discord and chat platform herald dispatcher for Nostrian Conquest", long_about = None)]
struct Cli {
    /// Discord Webhook URL for broadcasts
    #[arg(long, env = "NC_DISCORD_WEBHOOK_URL")]
    discord_webhook: Option<String>,

    /// Discord OAuth Client ID for Activity token verification
    #[arg(long, env = "NC_DISCORD_CLIENT_ID")]
    discord_client_id: Option<String>,

    /// Discord OAuth Client Secret for Activity token verification
    #[arg(long, env = "NC_DISCORD_CLIENT_SECRET")]
    discord_client_secret: Option<String>,

    /// Dry run mode: print embeds to stdout instead of sending HTTP requests
    #[arg(long, default_value_t = false)]
    dry_run: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run as an HTTP listener daemon receiving events from nc-bridge
    Listen {
        /// Host interface to bind to
        #[arg(long, default_value = "127.0.0.1", env = "NC_HERALD_HOST")]
        host: String,

        /// Port to listen on
        #[arg(short, long, default_value_t = 8081, env = "NC_HERALD_PORT")]
        port: u16,
    },

    /// Send a sample test announcement
    Test,

    /// Broadcast a maintenance turn announcement (Galactic Gazette)
    BroadcastTurn {
        #[arg(long, default_value = "Capella Cluster")]
        campaign: String,

        #[arg(long, default_value_t = 3001)]
        year: u32,
    },

    /// Broadcast a real-time tactical event alert
    BroadcastEvent {
        #[arg(long, default_value = "Lord Vorn")]
        player: String,

        #[arg(long, default_value_t = 1)]
        seat: usize,

        #[arg(long, default_value_t = 1)]
        node: u32,

        #[arg(long, default_value = "Orbital bombardment completed at Capella IV.")]
        summary: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cli = Cli::parse();

    let mut targets = Vec::new();
    if let Some(url) = cli.discord_webhook {
        targets.push(PlatformTarget::DiscordWebhook { url });
    }

    // If no webhook URL is provided, force dry-run mode
    let dry_run = cli.dry_run || targets.is_empty();

    let discord_oauth = match (cli.discord_client_id, cli.discord_client_secret) {
        (Some(client_id), Some(client_secret)) => Some(nc_herald::DiscordOAuthConfig {
            client_id,
            client_secret,
        }),
        _ => None,
    };

    let config = HeraldConfig {
        targets,
        dry_run,
        discord_oauth,
    };
    let herald = Herald::new(config);

    match cli.command {
        Commands::Listen { host, port } => {
            let bind_addr = format!("{host}:{port}");
            let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
            let herald_arc = Arc::new(herald);
            let app = nc_herald::create_listener_router(herald_arc);

            info!(addr = %bind_addr, "nc-herald daemon listening for incoming events");
            println!(
                "nc-herald: listening on http://{bind_addr} (POST /events, POST /broadcast, POST /auth/discord/token)"
            );

            axum::serve(listener, app).await?;
        }
        Commands::Test => {
            info!("Broadcasting test turn announcement");
            let announcement = TurnAnnouncement {
                campaign_name: "Test Campaign".to_string(),
                year: 2350,
                headline: "Communications array online. Stardate verified.".to_string(),
                combat_summaries: vec!["Hostile scout intercepted in Sector 12.".to_string()],
                planetary_revolts: vec![],
                standings: vec![
                    ("Sol Federation".to_string(), 150),
                    ("Vorn Hegemony".to_string(), 145),
                ],
                next_turn_seconds: 86400,
            };
            herald.broadcast_turn(&announcement).await?;
        }
        Commands::BroadcastTurn { campaign, year } => {
            info!(campaign = %campaign, year = year, "Broadcasting turn announcement");
            let announcement = TurnAnnouncement {
                campaign_name: campaign,
                year,
                headline: "Turn maintenance executed. Starmaps and sensor reports updated.".to_string(),
                combat_summaries: vec!["All fleet movements and combat engagements resolved.".to_string()],
                planetary_revolts: vec![],
                standings: vec![
                    ("Sol Federation".to_string(), 210),
                    ("Vorn Hegemony".to_string(), 195),
                    ("Centauri League".to_string(), 180),
                ],
                next_turn_seconds: 86400,
            };
            herald.broadcast_turn(&announcement).await?;
        }
        Commands::BroadcastEvent { player, seat, node, summary } => {
            info!(player = %player, "Broadcasting tactical event");
            let event = InGameEvent {
                node,
                player,
                seat,
                event_type: "tactical_alert".to_string(),
                summary,
            };
            herald.dispatch_event(&event).await?;
        }
    }

    Ok(())
}

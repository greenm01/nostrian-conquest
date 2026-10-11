use clap::{Parser, Subcommand};
use nc_herald::{Herald, HeraldConfig, InGameEvent, PlatformTarget, TurnAnnouncement};
use tracing::info;

#[derive(Parser)]
#[command(name = "nc-herald")]
#[command(about = "Discord and chat platform herald dispatcher for Nostrian Conquest", long_about = None)]
struct Cli {
    /// Discord Webhook URL for broadcasts
    #[arg(long, env = "NC_DISCORD_WEBHOOK_URL")]
    discord_webhook: Option<String>,

    /// Dry run mode: print embeds to stdout instead of sending HTTP requests
    #[arg(long, default_value_t = false)]
    dry_run: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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

    let config = HeraldConfig { targets, dry_run };
    let herald = Herald::new(config);

    match cli.command {
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

use clap::Parser;
use std::path::PathBuf;
use tracing::info;

#[derive(Parser, Debug)]
#[command(name = "nc-discord")]
#[command(about = "Interactive Discord bot for Nostrian Conquest", long_about = None)]
struct Cli {
    /// Discord Bot Token
    #[arg(short, long, env = "NC_DISCORD_BOT_TOKEN")]
    token: String,

    /// Campaign directory containing export/ and reports/ directories
    #[arg(short, long, default_value = ".", env = "NC_DISCORD_CAMPAIGN_DIR")]
    campaign_dir: PathBuf,

    /// URL of the nc-bridge web server for /play link generation
    #[arg(long, env = "NC_DISCORD_BRIDGE_URL")]
    bridge_url: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cli = Cli::parse();

    info!(
        campaign_dir = %cli.campaign_dir.display(),
        bridge_url = ?cli.bridge_url,
        "Starting nc-discord bot service"
    );

    println!(
        "nc-discord: starting bot for campaign at '{}'",
        cli.campaign_dir.display()
    );

    nc_discord::run_bot(cli.token, cli.campaign_dir, cli.bridge_url).await?;
    Ok(())
}

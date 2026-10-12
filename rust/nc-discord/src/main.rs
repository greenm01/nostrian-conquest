use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "nc-discord",
    about = "Discord community commands for managed NC games"
)]
struct Cli {
    #[arg(long, env = "NC_DISCORD_BOT_TOKEN", hide_env_values = true)]
    token: String,
    #[arg(long, env = "NC_GAMES_ROOT")]
    games_root: PathBuf,
    #[arg(long, env = "NC_PUBLIC_ORIGIN")]
    web_url: String,
    #[arg(long, env = "NC_DISCORD_GUILD_ID")]
    guild_id: u64,
    #[arg(long, env = "NC_DISCORD_CHANNEL_ID")]
    channel_id: Option<u64>,
}
#[tokio::main]
async fn main() -> Result<(), nc_discord::Error> {
    let cli = Cli::parse();
    nc_discord::run_bot(
        cli.token,
        cli.games_root,
        cli.web_url,
        cli.guild_id,
        cli.channel_id,
    )
    .await
}

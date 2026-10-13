use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "nc-matrix", about = "Matrix sign-in bot for managed NC games")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run the bot.
    Run {
        #[arg(long, env = "NC_MATRIX_HOMESERVER")]
        homeserver: String,
        #[arg(long, env = "NC_MATRIX_ACCESS_TOKEN", hide_env_values = true)]
        access_token: String,
        #[arg(long, env = "NC_MATRIX_ROOM")]
        room: String,
        #[arg(long, env = "NC_GAMES_ROOT")]
        games_root: PathBuf,
        #[arg(long, env = "NC_PUBLIC_ORIGIN")]
        origin: String,
    },
    /// Sign in once with the bot's password and print its access token.
    Login {
        #[arg(long, default_value = "https://matrix-client.matrix.org")]
        homeserver: String,
        /// Bot user ID, such as @nostrian-conquest-bot:matrix.org.
        #[arg(long)]
        user: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), nc_matrix::Error> {
    match Cli::parse().command {
        Command::Run {
            homeserver,
            access_token,
            room,
            games_root,
            origin,
        } => {
            nc_log::init_stderr_logging(nc_log::LogLevel::Info).map_err(|e| e.to_string())?;
            nc_matrix::run(nc_matrix::Config {
                homeserver,
                access_token,
                room,
                games_root,
                origin,
            })
            .await
        }
        Command::Login { homeserver, user } => {
            let password = rpassword::prompt_password(format!("Password for {user}: "))?;
            let token = nc_matrix::login(&homeserver, &user, &password).await?;
            println!("NC_MATRIX_ACCESS_TOKEN={token}");
            Ok(())
        }
    }
}

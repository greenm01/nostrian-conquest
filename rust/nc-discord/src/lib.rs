//! Discord discovery and authenticated, private campaign downloads.
mod announcements;
mod commands;
pub mod excel;

use nc_host::admin::{self, Request};
use serde_json::Value;
use serenity::all::{GatewayIntents, GuildId};
use std::path::PathBuf;

#[derive(Clone)]
pub struct BotData {
    pub games_root: PathBuf,
    pub web_url: String,
    pub guild_id: GuildId,
}
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, BotData, Error>;

pub async fn host(data: &BotData, request: Request) -> Result<Value, Error> {
    let root = data.games_root.clone();
    tokio::task::spawn_blocking(move || admin::execute(&root, request)).await?
}

pub async fn run_bot(
    token: String,
    games_root: PathBuf,
    web_url: String,
    guild: u64,
    channel: Option<u64>,
) -> Result<(), Error> {
    let url = reqwest::Url::parse(&web_url)?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("web URL must be an HTTPS origin".into());
    }
    if guild == 0 {
        return Err("Discord server ID must be nonzero".into());
    }
    let guild_id = GuildId::new(guild);
    let framework=poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![commands::games(),commands::join(),commands::play(),commands::status(),commands::map(),commands::report(),commands::guide()],
            command_check: Some(|ctx| Box::pin(async move {Ok(ctx.guild_id()==Some(ctx.data().guild_id))})),
            on_error: |err| Box::pin(async move {
                if let poise::FrameworkError::Command{ctx,..}=err {
                    let _=ctx.send(poise::CreateReply::default().ephemeral(true).content("Request could not be completed. Check the game ID and your seat, or try again shortly.")).await;
                } else { tracing::warn!("Discord command failed before execution"); }
            }),
            ..Default::default()
        })
        .setup(move |ctx,_,framework| Box::pin(async move {
            poise::builtins::register_in_guild(ctx,&framework.options().commands,guild_id).await?;
            let data=BotData{games_root,web_url:web_url.trim_end_matches('/').into(),guild_id};
            if let Some(channel) = channel {
                let http=ctx.http.clone();
                tokio::spawn(announcements::run(data.clone(),http,channel));
            }
            Ok(data)
        })).build();
    let mut client = serenity::all::ClientBuilder::new(token, GatewayIntents::GUILDS)
        .framework(framework)
        .await?;
    client.start().await?;
    Ok(())
}

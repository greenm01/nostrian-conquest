use crate::{Context, Error, host};
use nc_host::admin::Request;
use poise::CreateReply;
use serde_json::Value;
use serenity::all::{CreateAllowedMentions, CreateAttachment};

fn reply(text: impl Into<String>) -> CreateReply {
    CreateReply::default()
        .ephemeral(true)
        .content(text)
        .allowed_mentions(CreateAllowedMentions::new())
}
async fn catalog(ctx: Context<'_>) -> Result<Vec<Value>, Error> {
    let rows = host(ctx.data(), Request::Catalog).await?;
    Ok(rows.as_array().ok_or("invalid catalog")?.clone())
}
async fn public_game(ctx: Context<'_>, game: &str) -> Result<Value, Error> {
    catalog(ctx)
        .await?
        .into_iter()
        .find(|row| row["game"] == game)
        .ok_or_else(|| "game is not listed".into())
}
fn play_url(ctx: Context<'_>, game: &str) -> String {
    format!("{}/terminal/index.html?game={game}", ctx.data().web_url)
}

/// List recruiting and active games.
#[poise::command(slash_command, ephemeral)]
pub async fn games(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let rows = catalog(ctx).await?;
    if rows.is_empty() {
        ctx.send(reply("No games are currently listed.")).await?;
    }
    for page in rows.chunks(8) {
        let text = page
            .iter()
            .map(|g| {
                format!(
                    "`{}` — {} | {} | {}/{} seats open | year {}",
                    g["game"].as_str().unwrap_or("?"),
                    g["name"].as_str().unwrap_or("?"),
                    g["state"].as_str().unwrap_or("?"),
                    g["open"],
                    g["players"],
                    g["year"]
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        ctx.send(reply(text)).await?;
    }
    Ok(())
}
/// Claim an empire, using an invite code when required.
#[poise::command(slash_command, ephemeral)]
pub async fn join(
    ctx: Context<'_>,
    #[description = "Game ID from /games"] game: String,
    #[description = "Private eight-character code from the host"] code: Option<String>,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    // The same public name the website uses: display name, else username.
    let author = ctx.author();
    let name = author
        .global_name
        .clone()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| author.name.clone());
    let request = match code {
        Some(code) => Request::ClaimInvite {
            game: game.clone(),
            account: nc_host::admin::AccountIdentity::Discord(ctx.author().id.to_string()),
            code,
            display_name: Some(name),
        },
        None => Request::Join {
            game: game.clone(),
            account: nc_host::admin::AccountIdentity::Discord(ctx.author().id.to_string()),
            display_name: Some(name),
        },
    };
    let access = match host(ctx.data(), request).await {
        Ok(access) => access,
        Err(_) => {
            ctx.send(reply("Could not claim a seat. Check the game ID and invite code with the host. A code works once, while recruitment is open. After repeated failed attempts, wait ten minutes before trying again.")).await?;
            return Ok(());
        }
    };
    ctx.send(reply(format!(
        "You have seat {} in `{game}`. Sign in with this Discord account to play: {}",
        access["seat"],
        play_url(ctx, &game)
    )))
    .await?;
    Ok(())
}
/// Create a private, single-use seat code (server owner only).
#[poise::command(slash_command, ephemeral)]
pub async fn invite(
    ctx: Context<'_>,
    #[description = "Game ID from /games"] game: String,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let guild = ctx.data().guild_id.to_partial_guild(ctx.http()).await?;
    if guild.owner_id != ctx.author().id {
        ctx.send(reply("Only the server owner can create invite codes."))
            .await?;
        return Ok(());
    }
    let result = host(
        ctx.data(),
        Request::CreateInvites {
            game: game.clone(),
            count: 1,
        },
    )
    .await?;
    let code = result["invites"][0]["code"]
        .as_str()
        .ok_or("missing invite code")?;
    ctx.send(reply(format!("Private code for `{game}`: **{code}**\nSend it privately to one player. They claim it with `/join game:{game} code:{code}`, then use `/play game:{game}`. This code works once and is shown only here; keep it private."))).await?;
    Ok(())
}
/// Open your game in a browser terminal.
#[poise::command(slash_command, ephemeral)]
pub async fn play(
    ctx: Context<'_>,
    #[description = "Game ID from /games"] game: String,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let access = host(
        ctx.data(),
        Request::Authorize {
            game: game.clone(),
            account: nc_host::admin::AccountIdentity::Discord(ctx.author().id.to_string()),
        },
    )
    .await?;
    if access["can_play"] != true {
        ctx.send(reply("This game is not accepting sessions right now."))
            .await?;
    } else {
        ctx.send(reply(format!(
            "Open your terminal: {}",
            play_url(ctx, &game)
        )))
        .await?;
    }
    Ok(())
}
/// Show the game's actual year and next scheduled turn.
#[poise::command(slash_command, ephemeral)]
pub async fn status(
    ctx: Context<'_>,
    #[description = "Game ID from /games"] game: String,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let g = public_game(ctx, &game).await?;
    let deadline = g["next_due"]
        .as_i64()
        .map(|t| format!("<t:{t}:F> (<t:{t}:R>)"))
        .unwrap_or_else(|| {
            if g["schedule"]["mode"] == "manual" {
                "Manual turns; no automatic deadline".into()
            } else {
                "Waiting for the host to start or resume".into()
            }
        });
    ctx.send(reply(format!(
        "`{game}` | {} | year {}\nSeats open: {}/{}\nNext turn: {deadline}",
        g["state"].as_str().unwrap_or("?"),
        g["year"],
        g["open"],
        g["players"]
    )))
    .await?;
    Ok(())
}
#[derive(Debug, poise::ChoiceParameter, Clone, Copy)]
pub enum MapFormat {
    Zip,
    Text,
    Csv,
    Ods,
    Excel,
}
async fn export(ctx: Context<'_>, game: &str, format: &str) -> Result<Value, Error> {
    host(
        ctx.data(),
        Request::Export {
            game: game.into(),
            account: nc_host::admin::AccountIdentity::Discord(ctx.author().id.to_string()),
            format: format.into(),
        },
    )
    .await
}
/// Download your own empire's map.
#[poise::command(slash_command, ephemeral)]
pub async fn map(
    ctx: Context<'_>,
    #[description = "Game ID from /games"] game: String,
    #[description = "Download format"] format: Option<MapFormat>,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let format = format.unwrap_or(MapFormat::Zip);
    let data = export(
        ctx,
        &game,
        match format {
            MapFormat::Zip => "map-zip",
            MapFormat::Text => "map-txt",
            MapFormat::Csv => "map-csv",
            MapFormat::Ods => "map-ods",
            MapFormat::Excel => "map-xlsx",
        },
    )
    .await?;
    let name = data["filename"].as_str().ok_or("missing filename")?;
    let bytes = nc_host::map_archive::download_bytes(&data)?;
    ctx.send(
        reply(format!(
            "`{game}` · seat {} · year {}",
            data["seat"], data["year"]
        ))
        .attachment(CreateAttachment::bytes(bytes, name)),
    )
    .await?;
    Ok(())
}
/// Download your own turn reports.
#[poise::command(slash_command, ephemeral)]
pub async fn report(
    ctx: Context<'_>,
    #[description = "Game ID from /games"] game: String,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let data = export(ctx, &game, "reports").await?;
    let text = data["contents"].as_str().ok_or("missing reports")?;
    let mut response = reply(format!(
        "`{game}` · seat {} · year {}",
        data["seat"], data["year"]
    ));
    if text.is_empty() {
        response = reply("No reports are currently visible in your inbox.");
    } else {
        response = response.attachment(CreateAttachment::bytes(
            text.as_bytes(),
            data["filename"].as_str().ok_or("missing filename")?,
        ));
    }
    ctx.send(response).await?;
    Ok(())
}
/// Read the quick start guide.
#[poise::command(slash_command, ephemeral)]
pub async fn guide(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(reply("Quick start guide: https://github.com/greenm01/nostrian-conquest/raw/main/docs/manuals/nc_quick_start.pdf\nKeep a printed map or spreadsheet beside your terminal.")).await?;
    Ok(())
}

/// List the bot's commands and how to start playing.
#[poise::command(slash_command, ephemeral)]
pub async fn help(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(reply(HELP)).await?;
    Ok(())
}

const HELP: &str = "**Nostrian Conquest commands**\n\
`/games` — List games and their IDs.\n\
`/join game [code]` — Claim an empire. Private games need a code from the host.\n\
`/invite game` — Create a private seat code (server owner only).\n\
`/play game` — Open your game in a browser.\n\
`/status game` — Check the year and turn deadline.\n\
`/map game` — Download a ZIP with TXT, CSV, ODS and Excel maps. Optional format selects one file.\n\
`/report game` — Download your empire's reports.\n\
`/manuals` — Download the quick start and player manual.\n\
`/guide` — Open the quick start.\n\
`/about` — About the game.\n\
`/help` — Show this list.\n\n\
Start with /games, then /join and /play. Use the same Discord account in your browser. Replies and empire downloads are private to you.";

/// Learn about Nostrian Conquest.
#[poise::command(slash_command, ephemeral)]
pub async fn about(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(reply("**Nostrian Conquest**\nAn asynchronous, turn-based space strategy game inspired by the 1992 BBS game Esterian Conquest. Build an empire, explore the galaxy, and conquer rival worlds through an old-school BBS interface in your browser.\n\nIssue orders on your own schedule; maintenance resolves each game year.\nhttps://nostrian-conquest.com\nUse /help for commands and /manuals for the rules.")).await?;
    Ok(())
}

/// Download the player manuals as PDFs.
#[poise::command(slash_command, ephemeral)]
pub async fn manuals(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(reply("**Player manuals (PDF)**\n[Quick Start](https://github.com/greenm01/nostrian-conquest/raw/main/docs/manuals/nc_quick_start.pdf) — Start here.\n[Player Manual](https://github.com/greenm01/nostrian-conquest/raw/main/docs/manuals/nc_player_manual.pdf) — Full rules and reference.\n\nKeep a printed map or spreadsheet beside your terminal; /map downloads both.")).await?;
    Ok(())
}

//! Interactive Discord bot implementation for Nostrian Conquest using Poise & Serenity.
//!
//! Provides player utility slash commands (`/status`, `/map`, `/report`, `/play`)
//! while strictly preserving the Fog of War through ephemeral responses.

pub mod excel;

use poise::CreateReply;
use serenity::all::CreateAttachment;
use std::path::PathBuf;
use tracing::info;

/// Shared bot state across command handlers.
pub struct BotData {
    pub campaign_dir: PathBuf,
    pub web_bridge_url: Option<String>,
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, BotData, Error>;

/// Display campaign status, current stardate, and turn maintenance countdown.
#[poise::command(slash_command)]
pub async fn status(ctx: Context<'_>) -> Result<(), Error> {
    let campaign_name = ctx
        .data()
        .campaign_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Campaign");

    ctx.send(
        CreateReply::default().embed(
            serenity::all::CreateEmbed::new()
                .title("🪐 Campaign Status")
                .description(format!("**Sector:** *{campaign_name}*\nOrders are currently active."))
                .field("⏱️ Next Maintenance", "Scheduled daily at 04:00 UTC", false)
                .field(
                    "🛰️ Commands",
                    "`/map` — Personal starmap (.xlsx, .csv, .txt)\n`/report` — Turn briefing (ephemeral)\n`/play` — Launch game activity",
                    false,
                )
                .color(0x33cc66),
        ),
    )
    .await?;

    Ok(())
}

/// File format choice for starmap downloads.
#[derive(Debug, poise::ChoiceParameter, Default, Clone, Copy, PartialEq, Eq)]
pub enum MapFormatChoice {
    /// Download Excel workbook (.xlsx with formatted table and ASCII grid)
    #[name = "Excel Workbook (.xlsx) — Formatted & Styled"]
    #[default]
    Excel,
    /// Download all formats (.xlsx, .csv, and .txt)
    #[name = "All Formats (.xlsx, .csv, and .txt)"]
    All,
    /// Download printable ASCII text grid (.txt)
    #[name = "Printable ASCII Grid (.txt)"]
    Text,
    /// Download raw CSV (.csv)
    #[name = "Raw CSV (.csv)"]
    Csv,
}

/// Retrieve your personal starmap with Fog of War applied (only visible to you).
#[poise::command(slash_command, ephemeral)]
pub async fn map(
    ctx: Context<'_>,
    #[description = "Preferred format: Excel (.xlsx), all formats, printable text (.txt), or raw CSV (.csv)"]
    format: Option<MapFormatChoice>,
) -> Result<(), Error> {
    let choice = format.unwrap_or_default();
    let author_id = ctx.author().id.to_string();
    let author_name = ctx.author().name.clone();

    info!(
        user = %author_name,
        user_id = %author_id,
        format = ?choice,
        "Serving ephemeral starmap export"
    );

    let campaign_dir = &ctx.data().campaign_dir;
    let export_dir = campaign_dir.join("export");
    let mut attachments = Vec::new();

    let include_xlsx = matches!(choice, MapFormatChoice::Excel | MapFormatChoice::All);
    let include_csv = matches!(choice, MapFormatChoice::Csv | MapFormatChoice::All);
    let include_txt = matches!(choice, MapFormatChoice::Text | MapFormatChoice::All);

    let mut staged_csv = None;
    let mut staged_txt = None;
    let mut base_filename = "starmap".to_string();

    if export_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&export_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    let upper = name.to_ascii_uppercase();
                    let is_map = upper.starts_with("ECMAP-") || upper.starts_with("STARMAP");

                    if is_map {
                        if upper.ends_with(".CSV") {
                            if let Ok(bytes) = std::fs::read(&path) {
                                base_filename = name.trim_end_matches(".csv").trim_end_matches(".CSV").to_string();
                                staged_csv = Some(bytes);
                            }
                        } else if upper.ends_with(".TXT") {
                            if let Ok(bytes) = std::fs::read(&path) {
                                staged_txt = Some(bytes);
                            }
                        }
                    }
                }
            }
        }
    }

    // Build Excel (.xlsx) if requested and CSV data is available
    if include_xlsx {
        if let Some(csv_bytes) = &staged_csv {
            let csv_str = String::from_utf8_lossy(csv_bytes);
            let txt_str = staged_txt.as_ref().map(|b| String::from_utf8_lossy(b));

            if let Ok(xlsx_bytes) = excel::build_excel_starmap(&csv_str, txt_str.as_deref()) {
                attachments.push(CreateAttachment::bytes(xlsx_bytes, format!("{base_filename}.xlsx")));
            }
        }
    }

    // Include raw CSV if requested
    if include_csv {
        if let Some(csv_bytes) = staged_csv {
            attachments.push(CreateAttachment::bytes(csv_bytes, format!("{base_filename}.csv")));
        }
    }

    // Include printable ASCII TXT if requested
    if include_txt {
        if let Some(txt_bytes) = staged_txt {
            attachments.push(CreateAttachment::bytes(txt_bytes, format!("{base_filename}.txt")));
        }
    }

    let mut reply = CreateReply::default().ephemeral(true);

    if !attachments.is_empty() {
        let msg = format!(
            "🗺️ **Personal Tactical Starmap for {}**\n*Fog of War applied. Attached {} file(s) for your empire.*",
            author_name,
            attachments.len()
        );
        reply = reply.content(msg);
        for att in attachments {
            reply = reply.attachment(att);
        }
    } else {
        reply = reply.content(format!(
            "🗺️ **Tactical Starmap for {}**\n*No exported starmap files currently staged. Complete a reconnaissance scan or run `/status`.*",
            author_name
        ));
    }

    ctx.send(reply).await?;
    Ok(())
}

/// Retrieve your private turn briefing and battle report (only visible to you).
#[poise::command(slash_command, ephemeral)]
pub async fn report(ctx: Context<'_>) -> Result<(), Error> {
    let author_id = ctx.author().id.to_string();
    let author_name = ctx.author().name.clone();

    info!(user = %author_name, user_id = %author_id, "Serving ephemeral player turn report");

    let campaign_dir = &ctx.data().campaign_dir;
    let reports_dir = campaign_dir.join("reports");
    let mut report_attachment = None;

    if reports_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&reports_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("ECREPORT-") && name.ends_with(".TXT") {
                        if let Ok(bytes) = std::fs::read(&path) {
                            report_attachment = Some(CreateAttachment::bytes(bytes, name.to_string()));
                            break;
                        }
                    }
                }
            }
        }
    }

    let mut reply = CreateReply::default().ephemeral(true);

    if let Some(att) = report_attachment {
        reply = reply
            .content(format!("📜 **Classified Turn Briefing for {}**", author_name))
            .attachment(att);
    } else {
        reply = reply.content(format!(
            "📜 **Turn Briefing for {}**\n*No private intelligence briefing staged this cycle. Stand by for next maintenance run.*",
            author_name
        ));
    }

    ctx.send(reply).await?;
    Ok(())
}

/// Launch the Nostrian Conquest Activity or Web Door directly.
#[poise::command(slash_command)]
pub async fn play(ctx: Context<'_>) -> Result<(), Error> {
    let play_url = ctx
        .data()
        .web_bridge_url
        .as_deref()
        .unwrap_or("http://localhost:8080");

    ctx.send(
        CreateReply::default().embed(
            serenity::all::CreateEmbed::new()
                .title("🚀 Launch Nostrian Conquest")
                .description(format!(
                    "Click below to launch your terminal door session directly:\n\n**Web Door Link:** [Open Terminal]({play_url})\n\n*Or click the Rocket icon in any voice channel to play via Discord Activity.*"
                ))
                .color(0x33aaff),
        ),
    )
    .await?;

    Ok(())
}

/// Start the Poise Discord bot framework.
pub async fn run_bot(
    token: String,
    campaign_dir: PathBuf,
    web_bridge_url: Option<String>,
) -> Result<(), Error> {
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![status(), map(), report(), play()],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                info!("Registering slash commands globally with Discord...");
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                info!("Discord slash commands registered successfully.");
                Ok(BotData {
                    campaign_dir,
                    web_bridge_url,
                })
            })
        })
        .build();

    let mut client = serenity::all::ClientBuilder::new(
        token,
        serenity::all::GatewayIntents::non_privileged(),
    )
    .framework(framework)
    .await?;

    info!("Discord bot client connected to gateway. Listening for commands.");
    client.start().await?;
    Ok(())
}

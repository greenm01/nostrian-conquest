use crate::{BotData, host};
use nc_host::admin::Request;
use serenity::all::{ChannelId, CreateAllowedMentions, CreateMessage, Http};
use std::sync::Arc;

pub(crate) async fn run(data: BotData, http: Arc<Http>, channel: u64) {
    if channel == 0 {
        tracing::error!("Announcement channel must be nonzero");
        return;
    }
    let Ok(resolved) = ChannelId::new(channel).to_channel(&http).await else {
        tracing::error!("Cannot access announcement channel");
        return;
    };
    if resolved
        .guild()
        .is_none_or(|channel| channel.guild_id != data.guild_id)
    {
        tracing::error!("Announcement channel belongs to another Discord server");
        return;
    }
    loop {
        if let Ok(events) = host(&data, Request::Announcements).await {
            for event in events.as_array().into_iter().flatten() {
                let (Some(game), Some(id)) = (event["game"].as_str(), event["id"].as_str()) else {
                    continue;
                };
                let payload = &event["payload"];
                let text = match payload["kind"].as_str() {
                    Some("recruiting") => format!(
                        "`{game}` opened recruitment ({} seats). Use /games for current openings and /join to reserve a seat.\n{}",
                        payload["players"], data.web_url
                    ),
                    Some("turn") => format!(
                        "`{game}` — maintenance completed for year {}. Reports are available in the terminal. Scheduled next turn: <t:{}:F>.\n{}",
                        payload["year"], payload["next_due"], data.web_url
                    ),
                    _ => continue,
                };
                let message = CreateMessage::new()
                    .content(text)
                    .allowed_mentions(CreateAllowedMentions::new());
                if ChannelId::new(channel)
                    .send_message(&http, message)
                    .await
                    .is_err()
                {
                    tracing::warn!(game, "Announcement delivery failed; retained for retry");
                    break;
                }
                if host(
                    &data,
                    Request::AcknowledgeAnnouncement {
                        game: game.into(),
                        id: id.into(),
                    },
                )
                .await
                .is_err()
                {
                    tracing::warn!(
                        game,
                        "Announcement sent but receipt failed; retry may duplicate message"
                    );
                    break;
                }
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}

//! Platform herald, webhook dispatcher, and turn broadcaster for Nostrian Conquest.
//!
//! Posts turn announcements (Galactic Gazette), combat events, and status updates
//! to Discord channels, Matrix rooms, and configured webhooks.

use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};

/// Target chat platform destination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlatformTarget {
    DiscordWebhook { url: String },
    MatrixRoom { homeserver: String, room_id: String, token: String },
    GenericWebhook { url: String, secret: Option<String> },
}

/// A published turn announcement summary (Galactic Gazette).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnAnnouncement {
    pub campaign_name: String,
    pub year: u32,
    pub headline: String,
    pub combat_summaries: Vec<String>,
    pub planetary_revolts: Vec<String>,
    pub standings: Vec<(String, u32)>,
    pub next_turn_seconds: u32,
}

impl TurnAnnouncement {
    /// Format announcement as a Markdown bulletin suitable for chat embeds.
    pub fn format_markdown(&self) -> String {
        let mut msg = String::new();
        msg.push_str(&format!("📢 **GALACTIC GAZETTE — YEAR {}**\n\n", self.year));
        msg.push_str(&format!("*{}: {}*\n\n", self.campaign_name, self.headline));

        if !self.combat_summaries.is_empty() {
            msg.push_str("⚔️ **Combat Reports:**\n");
            for cs in &self.combat_summaries {
                msg.push_str(&format!("• {cs}\n"));
            }
            msg.push('\n');
        }

        if !self.planetary_revolts.is_empty() {
            msg.push_str("🪐 **Planetary Status:**\n");
            for pr in &self.planetary_revolts {
                msg.push_str(&format!("• {pr}\n"));
            }
            msg.push('\n');
        }

        if !self.standings.is_empty() {
            msg.push_str("🏆 **Empire Standings:**\n");
            for (rank, (name, pts)) in self.standings.iter().enumerate() {
                msg.push_str(&format!("{}. {} ({} pts)\n", rank + 1, name, pts));
            }
            msg.push('\n');
        }

        let hours = self.next_turn_seconds / 3600;
        let mins = (self.next_turn_seconds % 3600) / 60;
        msg.push_str(&format!("⏱️ *Orders for Year {} are now open. Maintenance in {}h {}m.*", self.year + 1, hours, mins));

        msg
    }

    /// Construct a Discord-compatible Webhook JSON payload with embeds.
    pub fn to_discord_payload(&self) -> serde_json::Value {
        let mut fields = Vec::new();

        if !self.combat_summaries.is_empty() {
            let val = self.combat_summaries.iter().map(|s| format!("• {s}")).collect::<Vec<_>>().join("\n");
            fields.push(serde_json::json!({
                "name": "⚔️ Combat Reports",
                "value": val,
                "inline": false
            }));
        }

        if !self.planetary_revolts.is_empty() {
            let val = self.planetary_revolts.iter().map(|s| format!("• {s}")).collect::<Vec<_>>().join("\n");
            fields.push(serde_json::json!({
                "name": "🪐 Planetary Status",
                "value": val,
                "inline": false
            }));
        }

        if !self.standings.is_empty() {
            let val = self.standings.iter().enumerate()
                .map(|(i, (name, pts))| format!("{}. **{}** — {} pts", i + 1, name, pts))
                .collect::<Vec<_>>()
                .join("\n");
            fields.push(serde_json::json!({
                "name": "🏆 Empire Standings",
                "value": val,
                "inline": false
            }));
        }

        let hours = self.next_turn_seconds / 3600;
        let mins = (self.next_turn_seconds % 3600) / 60;
        let footer_text = format!("Orders for Year {} open. Maintenance in {}h {}m.", self.year + 1, hours, mins);

        // Color: Amber/Orange if combat happened, otherwise Green
        let color = if !self.combat_summaries.is_empty() {
            0xff9900
        } else {
            0x33cc66
        };

        serde_json::json!({
            "username": "Galactic Herald",
            "embeds": [
                {
                    "title": format!("📢 GALACTIC GAZETTE — YEAR {}", self.year),
                    "description": format!("**{}**: *{}*", self.campaign_name, self.headline),
                    "color": color,
                    "fields": fields,
                    "footer": {
                        "text": footer_text
                    }
                }
            ]
        })
    }
}

/// An in-game real-time event received from `nc-bridge` callbacks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InGameEvent {
    pub node: u32,
    pub player: String,
    pub seat: usize,
    pub event_type: String,
    pub summary: String,
}

impl InGameEvent {
    /// Construct a Discord-compatible Webhook JSON payload for tactical alerts.
    pub fn to_discord_payload(&self) -> serde_json::Value {
        serde_json::json!({
            "username": "Galactic Herald",
            "embeds": [
                {
                    "title": "⚡ Tactical Dispatch",
                    "description": format!("**{}** (Seat {}): {}", self.player, self.seat, self.summary),
                    "color": 0x33aaff,
                    "footer": {
                        "text": format!("Virtual Node {}", self.node)
                    }
                }
            ]
        })
    }
}

/// Discord OAuth application configuration for token verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
}

/// Request to exchange a Discord authorization code for a verified user profile.
#[derive(Debug, Clone, Deserialize)]
pub struct DiscordAuthRequest {
    pub code: String,
}

/// Verified Discord user profile information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordUserProfile {
    pub id: String,
    pub username: String,
    pub global_name: Option<String>,
    pub avatar: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DiscordTokenResponse {
    access_token: String,
}

/// Herald dispatcher configuration.
#[derive(Debug, Clone, Default)]
pub struct HeraldConfig {
    pub targets: Vec<PlatformTarget>,
    pub dry_run: bool,
    pub discord_oauth: Option<DiscordOAuthConfig>,
}

/// The Herald dispatcher service.
pub struct Herald {
    config: HeraldConfig,
    client: reqwest::Client,
}

impl Herald {
    pub fn new(config: HeraldConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    /// Broadcast a turn announcement to all configured platform targets.
    pub async fn broadcast_turn(&self, announcement: &TurnAnnouncement) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.config.dry_run {
            let md = announcement.format_markdown();
            println!("[HERALD DRY-RUN]\n{md}");
            let payload = announcement.to_discord_payload();
            println!("[HERALD DISCORD PAYLOAD]\n{}", serde_json::to_string_pretty(&payload)?);
            return Ok(());
        }

        for target in &self.config.targets {
            match target {
                PlatformTarget::DiscordWebhook { url } => {
                    let payload = announcement.to_discord_payload();
                    self.post_discord_webhook(url, &payload).await?;
                }
                PlatformTarget::MatrixRoom { .. } => {
                    debug!("Matrix broadcast not yet configured");
                }
                PlatformTarget::GenericWebhook { url, secret } => {
                    self.post_generic_webhook(url, secret.as_deref(), announcement).await?;
                }
            }
        }

        Ok(())
    }

    /// Dispatch a real-time event alert.
    pub async fn dispatch_event(&self, event: &InGameEvent) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.config.dry_run {
            println!("[HERALD EVENT DRY-RUN] {} (Node {}): {}", event.player, event.node, event.summary);
            let payload = event.to_discord_payload();
            println!("[HERALD DISCORD EVENT PAYLOAD]\n{}", serde_json::to_string_pretty(&payload)?);
            return Ok(());
        }

        for target in &self.config.targets {
            if let PlatformTarget::DiscordWebhook { url } = target {
                let payload = event.to_discord_payload();
                self.post_discord_webhook(url, &payload).await?;
            }
        }

        Ok(())
    }

    async fn post_discord_webhook(&self, url: &str, payload: &serde_json::Value) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Posting webhook dispatch to Discord: {url}");
        let res = self.client.post(url).json(payload).send().await?;
        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            error!("Discord webhook returned error status {status}: {body}");
            return Err(format!("Discord webhook failed with HTTP {status}: {body}").into());
        }
        Ok(())
    }

    async fn post_generic_webhook(&self, url: &str, secret: Option<&str>, payload: &TurnAnnouncement) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Posting generic webhook to {url}");
        let mut req = self.client.post(url).json(payload);
        if let Some(sec) = secret {
            req = req.bearer_auth(sec);
        }
        let res = req.send().await?;
        if !res.status().is_success() {
            return Err(format!("Generic webhook failed with HTTP {}", res.status()).into());
        }
        Ok(())
    }

    /// Exchange a Discord Embedded App SDK authorization code for a verified Discord user profile.
    pub async fn exchange_discord_code(&self, code: &str) -> Result<DiscordUserProfile, Box<dyn std::error::Error + Send + Sync>> {
        let Some(oauth) = &self.config.discord_oauth else {
            return Err("Discord OAuth not configured on this herald instance".into());
        };

        if self.config.dry_run {
            info!(code = %code, "Dry-run: mock Discord OAuth token exchange");
            return Ok(DiscordUserProfile {
                id: "123456789012345678".to_string(),
                username: "mock_commander".to_string(),
                global_name: Some("Mock Commander".to_string()),
                avatar: None,
            });
        }

        let params = [
            ("client_id", oauth.client_id.as_str()),
            ("client_secret", oauth.client_secret.as_str()),
            ("grant_type", "authorization_code"),
            ("code", code),
        ];

        let token_res = self.client
            .post("https://discord.com/api/oauth2/token")
            .form(&params)
            .send()
            .await?;

        if !token_res.status().is_success() {
            let status = token_res.status();
            let body = token_res.text().await.unwrap_or_default();
            return Err(format!("Discord OAuth token exchange failed with HTTP {status}: {body}").into());
        }

        let token_data: DiscordTokenResponse = token_res.json().await?;

        let user_res = self.client
            .get("https://discord.com/api/users/@me")
            .bearer_auth(&token_data.access_token)
            .send()
            .await?;

        if !user_res.status().is_success() {
            let status = user_res.status();
            let body = user_res.text().await.unwrap_or_default();
            return Err(format!("Discord fetch user profile failed with HTTP {status}: {body}").into());
        }

        let profile: DiscordUserProfile = user_res.json().await?;
        Ok(profile)
    }
}

use std::sync::Arc;
use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};

/// Create an Axum router for the Herald listener daemon.
pub fn create_listener_router(herald: Arc<Herald>) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/events", post(receive_event_handler))
        .route("/broadcast", post(receive_broadcast_handler))
        .route("/auth/discord/token", post(exchange_discord_token_handler))
        .with_state(herald)
}

async fn health_handler() -> Response {
    axum::response::Json(serde_json::json!({
        "status": "ok",
        "service": "nc-herald",
    }))
    .into_response()
}

async fn exchange_discord_token_handler(
    State(herald): State<Arc<Herald>>,
    Json(req): Json<DiscordAuthRequest>,
) -> Response {
    match herald.exchange_discord_code(&req.code).await {
        Ok(profile) => (
            StatusCode::OK,
            axum::response::Json(serde_json::to_value(&profile).unwrap_or_default()),
        )
            .into_response(),
        Err(e) => {
            error!("Failed to exchange Discord authorization code: {e}");
            (
                StatusCode::BAD_REQUEST,
                axum::response::Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

async fn receive_event_handler(
    State(herald): State<Arc<Herald>>,
    Json(event): Json<InGameEvent>,
) -> Response {
    info!(player = %event.player, node = event.node, "Received incoming event dispatch");
    match herald.dispatch_event(&event).await {
        Ok(_) => (
            StatusCode::OK,
            axum::response::Json(serde_json::json!({ "status": "dispatched" })),
        )
            .into_response(),
        Err(e) => {
            error!("Failed to dispatch event: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}

async fn receive_broadcast_handler(
    State(herald): State<Arc<Herald>>,
    Json(announcement): Json<TurnAnnouncement>,
) -> Response {
    info!(campaign = %announcement.campaign_name, year = announcement.year, "Received incoming turn broadcast");
    match herald.broadcast_turn(&announcement).await {
        Ok(_) => (
            StatusCode::OK,
            axum::response::Json(serde_json::json!({ "status": "broadcast_sent" })),
        )
            .into_response(),
        Err(e) => {
            error!("Failed to broadcast turn: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discord_payload_formatting() {
        let announcement = TurnAnnouncement {
            campaign_name: "Test Cluster".to_string(),
            year: 2350,
            headline: "Hostilities erupt in sector 4".to_string(),
            combat_summaries: vec!["Fleet clash at Capella IV: 2 Cruisers lost.".to_string()],
            planetary_revolts: vec!["Deneb III declared independence.".to_string()],
            standings: vec![("Sol Federation".to_string(), 300), ("Vorn".to_string(), 250)],
            next_turn_seconds: 3600,
        };

        let payload = announcement.to_discord_payload();
        let embeds = payload["embeds"].as_array().expect("embeds array");
        assert_eq!(embeds.len(), 1);

        let embed = &embeds[0];
        assert_eq!(embed["title"], "📢 GALACTIC GAZETTE — YEAR 2350");
        assert_eq!(embed["color"], 0xff9900); // Combat color

        let fields = embed["fields"].as_array().expect("fields array");
        assert_eq!(fields.len(), 3);
    }

    #[tokio::test]
    async fn mock_discord_oauth_exchange() {
        let config = HeraldConfig {
            dry_run: true,
            discord_oauth: Some(DiscordOAuthConfig {
                client_id: "test_client_id".to_string(),
                client_secret: "test_secret".to_string(),
            }),
            ..HeraldConfig::default()
        };

        let herald = Herald::new(config);
        let profile = herald.exchange_discord_code("test_code_123").await.expect("exchange code in dry-run");
        assert_eq!(profile.username, "mock_commander");
        assert_eq!(profile.id, "123456789012345678");
    }

    #[tokio::test]
    async fn oauth_not_configured_returns_error() {
        let config = HeraldConfig {
            dry_run: true,
            discord_oauth: None,
            ..HeraldConfig::default()
        };

        let herald = Herald::new(config);
        let res = herald.exchange_discord_code("test_code_123").await;
        assert!(res.is_err());
    }
}

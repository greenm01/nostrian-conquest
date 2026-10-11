//! Platform herald, webhook dispatcher, and turn broadcaster for Nostrian Conquest.
//!
//! Posts turn announcements (Galactic Gazette), combat events, and status updates
//! to Discord channels, Matrix rooms, and configured webhooks.

use serde::{Deserialize, Serialize};

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

/// Herald dispatcher configuration.
#[derive(Debug, Clone)]
pub struct HeraldConfig {
    pub targets: Vec<PlatformTarget>,
    pub dry_run: bool,
}

impl Default for HeraldConfig {
    fn default() -> Self {
        Self {
            targets: Vec::new(),
            dry_run: true,
        }
    }
}

/// The Herald dispatcher service.
pub struct Herald {
    config: HeraldConfig,
}

impl Herald {
    pub fn new(config: HeraldConfig) -> Self {
        Self { config }
    }

    /// Broadcast a turn announcement to all configured platform targets.
    pub async fn broadcast_turn(&self, announcement: &TurnAnnouncement) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let md = announcement.format_markdown();
        if self.config.dry_run {
            println!("[HERALD DRY-RUN]\n{md}");
            return Ok(());
        }

        // Future: iterate over self.config.targets and dispatch HTTP POST
        Ok(())
    }

    /// Dispatch a real-time event alert.
    pub async fn dispatch_event(&self, event: &InGameEvent) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.config.dry_run {
            println!("[HERALD EVENT DRY-RUN] {} (Node {}): {}", event.player, event.node, event.summary);
            return Ok(());
        }

        // Future: HTTP POST to targets
        Ok(())
    }
}

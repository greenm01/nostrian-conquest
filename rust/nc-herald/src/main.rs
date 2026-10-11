use nc_herald::{Herald, HeraldConfig, TurnAnnouncement};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let herald = Herald::new(HeraldConfig::default());
    info!("nc-herald dispatcher initialized");

    let demo = TurnAnnouncement {
        campaign_name: "Capella Cluster".to_string(),
        year: 3001,
        headline: "First exploratory fleets depart homeworlds".to_string(),
        combat_summaries: vec!["No hostile contacts recorded this cycle.".to_string()],
        planetary_revolts: vec![],
        standings: vec![
            ("Sol Hegemony".to_string(), 120),
            ("Vorn Syndicate".to_string(), 118),
        ],
        next_turn_seconds: 86400,
    };

    herald.broadcast_turn(&demo).await?;
    Ok(())
}

use std::time::Duration;

use nc_nostr::discovery::select_discovered_game_from_events as shared_select_discovered_game_from_events;
pub use nc_nostr::discovery::{DiscoveredGame, InviteResolution};
use nc_nostr::hosted::PublishedGameDefinition;
use nostr_sdk::{Client, Event, Filter, Keys, Kind};

use crate::connect::resolve::ResolvedTarget;

pub const GAME_DISCOVERY_TIMEOUT_SECS: u64 = 10;

pub async fn discover_game_for_invite(
    player_keys: &Keys,
    target: &ResolvedTarget,
    invite_code: &str,
) -> Result<DiscoveredGame, String> {
    let client = Client::new(player_keys.clone());
    client
        .add_relay(&target.relay_url)
        .await
        .map_err(|e| relay_transport_error(target, "could not add the relay", &e.to_string()))?;
    client.connect().await;

    let timeout = Duration::from_secs(GAME_DISCOVERY_TIMEOUT_SECS);
    let events = client
        .fetch_events(Filter::new().kinds([Kind::Custom(30500)]), timeout)
        .await
        .map_err(|e| {
            relay_transport_error(
                target,
                "could not fetch hosted game definitions from the relay",
                &e.to_string(),
            )
        })?;

    client.disconnect().await;

    let player_pubkey_hex = player_keys.public_key().to_hex();
    select_discovered_game_from_events(
        events.iter(),
        target,
        invite_code,
        Some(player_pubkey_hex.as_str()),
    )
}

pub fn select_discovered_game_from_events<'a>(
    events: impl IntoIterator<Item = &'a Event>,
    target: &ResolvedTarget,
    invite_code: &str,
    player_pubkey_hex: Option<&str>,
) -> Result<DiscoveredGame, String> {
    shared_select_discovered_game_from_events(
        events,
        &target.server_host,
        target.server_port,
        &target.relay_url,
        invite_code,
        player_pubkey_hex,
    )
}

fn relay_transport_error(target: &ResolvedTarget, context: &str, detail: &str) -> String {
    if is_local_dev_relay(&target.relay_url) {
        format!(
            "{context} at {}: make sure your local relay and nc-host serve are running for this game ({detail})",
            target.relay_url
        )
    } else {
        format!("{context}: {detail}")
    }
}

fn is_local_dev_relay(relay_url: &str) -> bool {
    relay_url.starts_with("ws://localhost")
        || relay_url.starts_with("ws://127.")
        || relay_url.starts_with("ws://[::1]")
}

pub fn parse_game_definition(event: &Event) -> Option<PublishedGameDefinition> {
    nc_nostr::hosted::parse_game_definition(event)
}

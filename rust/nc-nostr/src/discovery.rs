use crate::hash::sha256_hex;
use crate::hosted::{PublishedGameDefinition, parse_game_definition};
use nostr_sdk::Event;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InviteResolution {
    FirstJoin,
    SameIdentityRejoin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredGame {
    pub gate_npub: String,
    pub game_id: String,
    pub game_name: String,
    pub ssh_host: String,
    pub ssh_port: u16,
    pub seat: u32,
    pub resolution: InviteResolution,
}

pub fn select_discovered_game_from_events<'a>(
    events: impl IntoIterator<Item = &'a Event>,
    server_host: &str,
    server_port: u16,
    relay_url: &str,
    invite_code: &str,
    player_pubkey_hex: Option<&str>,
) -> Result<DiscoveredGame, String> {
    let invite_words = normalize_invite_words(invite_code);
    let invite_hash = sha256_hex(invite_words.as_bytes());
    let target_host = server_host.to_ascii_lowercase();

    let events_collected: Vec<&Event> = events.into_iter().collect();
    let mut hash_matches = Vec::new();

    for event in &events_collected {
        let Some(game) = parse_game_definition(event) else {
            continue;
        };
        let pending_seat = game
            .slots
            .iter()
            .find(|slot| slot.status == "pending" && slot.invite_code_hash == invite_hash)
            .map(|slot| slot.seat);
        if let Some(seat) = pending_seat {
            hash_matches.push((game, seat, InviteResolution::FirstJoin));
            continue;
        }

        let claimed_same_identity_seat = player_pubkey_hex
            .and_then(|player| {
                game.slots.iter().find(|slot| {
                    slot.status == "claimed"
                        && slot.invite_code_hash == invite_hash
                        && slot.player_npub.as_deref() == Some(player)
                })
            })
            .map(|slot| slot.seat);
        if let Some(seat) = claimed_same_identity_seat {
            hash_matches.push((game, seat, InviteResolution::SameIdentityRejoin));
        }
    }

    if hash_matches.is_empty() {
        let claimed_match_count = events_collected
            .iter()
            .filter_map(|event| parse_game_definition(event))
            .flat_map(|game| game.slots.into_iter())
            .filter(|slot| slot.status == "claimed" && slot.invite_code_hash == invite_hash)
            .count();
        if claimed_match_count > 0 {
            return Err(
                "this invite code has already been claimed by another player; ask your sysop to reissue the seat"
                    .to_string(),
            );
        }
        return Err(no_game_found_error(relay_url, events_collected.len()));
    }

    if hash_matches.len() == 1 {
        let (game, seat, resolution) = hash_matches.remove(0);
        return Ok(discovered_game(game, seat, resolution));
    }

    let exact_matches: Vec<_> = hash_matches
        .into_iter()
        .filter(|(game, _, _)| {
            game.ssh_host.to_ascii_lowercase() == target_host && game.ssh_port == server_port
        })
        .collect();

    match exact_matches.len() {
        1 => {
            let (game, seat, resolution) = exact_matches.into_iter().next().unwrap();
            Ok(discovered_game(game, seat, resolution))
        }
        0 => Err("multiple hosted games matched this invite code on the relay, but none matched this server address; open the game from the picker instead".to_string()),
        _ => Err(
            "multiple hosted games matched this invite code on the relay; open the game from the picker and choose the right one"
                .to_string(),
        ),
    }
}

fn discovered_game(
    game: PublishedGameDefinition,
    seat: u32,
    resolution: InviteResolution,
) -> DiscoveredGame {
    DiscoveredGame {
        gate_npub: game.gate_npub,
        game_id: game.game_id,
        game_name: game.game_name,
        ssh_host: game.ssh_host,
        ssh_port: game.ssh_port,
        seat,
        resolution,
    }
}

fn no_game_found_error(relay_url: &str, event_count: usize) -> String {
    if is_local_dev_relay(relay_url) {
        format!(
            "could not find this hosted game on the local relay at {}; make sure your local relay and nc-host serve are running for this game, then try again",
            relay_url
        )
    } else {
        format!(
            "the relay was reachable, but no pending hosted seat matched this invite ({} public game definition{} checked); check the invite code and relay, and if your sysop recently created or reissued this seat, ask them to republish hosted metadata",
            event_count,
            if event_count == 1 { "" } else { "s" },
        )
    }
}

fn is_local_dev_relay(relay_url: &str) -> bool {
    relay_url.starts_with("ws://localhost")
        || relay_url.starts_with("ws://127.")
        || relay_url.starts_with("ws://[::1]")
}

fn normalize_invite_words(invite_code: &str) -> String {
    invite_code
        .trim()
        .split_once('@')
        .map(|(words, _)| words)
        .unwrap_or(invite_code)
        .trim()
        .to_ascii_lowercase()
}

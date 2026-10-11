//! Hosted state sync: the fog-of-war state one seat sees, queued as a full
//! `30520` snapshot or a `30521` delta against a snapshot that seat was
//! sent before. A state request and the push after maintenance both go
//! through here.

use std::path::Path;

use nc_data::hosted::{self, HostedStore, SeatStatus};
use nc_nostr::state_sync::{
    GameState, build_delta_response_tags, build_state_delta, build_state_response_tags,
};

use crate::game::outbox::enqueue_encrypted_event;
use crate::game::state::build_game_state_payload;

/// Queue the seat's current state for its player and remember the snapshot
/// so the next sync can be a delta. With `previous`, the player gets a
/// delta against it; without, a full snapshot. `request_id` ties a reply to
/// the `30507` it answers.
pub fn enqueue_state_sync(
    store: &HostedStore,
    game_dir: &Path,
    game_id: &str,
    seat_number: u32,
    player_pubkey: &str,
    previous: Option<&GameState>,
    request_id: Option<&str>,
) -> Result<GameState, Box<dyn std::error::Error>> {
    let state = build_game_state_payload(game_dir, game_id, seat_number)?;
    let (kind, content, mut tags) = match previous {
        Some(previous) => {
            let delta = build_state_delta(previous, &state);
            (
                30521,
                serde_json::to_string(&delta)?,
                tag_rows(build_delta_response_tags(&delta)),
            )
        }
        None => (
            30520,
            serde_json::to_string(&state)?,
            tag_rows(build_state_response_tags(&state)),
        ),
    };
    if let Some(request_id) = request_id {
        tags.push(vec!["request-id".to_string(), request_id.to_string()]);
    }
    enqueue_encrypted_event(
        store.connection(),
        game_id,
        player_pubkey,
        kind,
        &content,
        tags,
    )?;
    hosted::save_state_snapshot(store.connection(), game_id, seat_number, &state)?;
    Ok(state)
}

/// After maintenance, send every claimed seat its new state: a delta
/// against the last snapshot it was sent, or a full snapshot when it has
/// none. Returns how many seats were sent.
pub fn publish_state_to_claimed_seats(
    store: &HostedStore,
    game_dir: &Path,
    game_id: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut published = 0;
    for seat in hosted::list_seats(store.connection(), game_id)? {
        if seat.status != SeatStatus::Claimed {
            continue;
        }
        let Some(player_pubkey) = seat.player_pubkey.as_deref() else {
            continue;
        };
        let previous =
            hosted::get_latest_state_snapshot(store.connection(), game_id, seat.seat_number)?;
        enqueue_state_sync(
            store,
            game_dir,
            game_id,
            seat.seat_number,
            player_pubkey,
            previous.as_ref(),
            None,
        )?;
        published += 1;
    }
    Ok(published)
}

fn tag_rows(tags: Vec<(&'static str, String)>) -> Vec<Vec<String>> {
    tags.into_iter()
        .map(|(key, value)| vec![key.to_string(), value])
        .collect()
}

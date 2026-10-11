//! Names stored as classic text: planet and empire names, and the player
//! handle that owns an empire. Each setter maps characters outside classic
//! text to `?` and cuts the name to the length its classic field holds.

use crate::support::classic_text;
use crate::{Planet, Player, PlayerMode};

/// Classic stores a planet name as a `String[20]`.
const PLANET_NAME_MAX_LEN: usize = 20;
/// Classic stores an empire name as a `String[20]`.
const EMPIRE_NAME_MAX_LEN: usize = 20;
/// Classic stores a handle in 25 space-padded bytes.
const HANDLE_MAX_LEN: usize = 25;

fn classic_name(value: &str, max_len: usize) -> String {
    classic_text(value).chars().take(max_len).collect()
}

pub fn set_planet_name(planet: &mut Planet, name: &str) {
    planet.name = classic_name(name, PLANET_NAME_MAX_LEN);
}

pub fn set_empire_name(player: &mut Player, name: &str) {
    player.empire_name = classic_name(name, EMPIRE_NAME_MAX_LEN);
}

pub fn set_player_handle(player: &mut Player, handle: &str) {
    player.handle = classic_name(handle, HANDLE_MAX_LEN);
}

/// Whether `alias` names this player's handle as `set_player_handle`
/// stores it, ignoring case and surrounding spaces.
pub fn player_handle_matches(player: &Player, alias: &str) -> bool {
    player
        .handle
        .trim()
        .eq_ignore_ascii_case(classic_name(alias, HANDLE_MAX_LEN).trim())
}

/// Who holds the empire, for inspection output: a rogue, an unowned slot,
/// or a player's handle and empire name.
pub fn ownership_summary(player: &Player) -> String {
    let name = player.empire_name.trim();
    let handle = player.handle.trim();
    if player.mode == PlayerMode::Rogue {
        format!("rogue label='{name}'")
    } else if name.starts_with("In Civil Disorder") || name == "Unowned" {
        format!("unowned label='{name}'")
    } else if player.mode.code() != 0 || !handle.is_empty() || !name.is_empty() {
        format!("player handle='{handle}' empire='{name}'")
    } else {
        format!("unowned label='{name}'")
    }
}

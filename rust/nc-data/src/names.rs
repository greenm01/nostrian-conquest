//! Names stored as classic text: planet and empire names, and the player
//! handle that owns an empire. Each setter maps characters outside classic
//! text to `?` and cuts the name to the length its classic field holds.

use crate::support::classic_text;
use crate::{CoreGameData, Planet, Player, PlayerMode};

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

/// Whether a planet other than `except` (0-based) already has `name`, as
/// stored and ignoring case: no name is used twice in one game.
pub fn planet_name_in_use(game_data: &CoreGameData, name: &str, except: Option<usize>) -> bool {
    let name = classic_name(name, PLANET_NAME_MAX_LEN);
    game_data.planets.iter().enumerate().any(|(index, planet)| {
        Some(index) != except && planet.name.trim().eq_ignore_ascii_case(name.trim())
    })
}

/// The name a world has until its owner names it.
const UNNAMED: &str = "Not Named Yet";

fn planet_names() -> &'static [String] {
    static NAMES: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    NAMES.get_or_init(|| {
        let document: kdl::KdlDocument = include_str!("../config/planet_names.kdl")
            .parse()
            .expect("planet_names.kdl parses");
        document
            .nodes()
            .iter()
            .flat_map(|list| list.iter_children())
            .filter_map(|node| node.get(0).and_then(|value| value.as_string()))
            .map(str::to_string)
            .collect()
    })
}

/// Name each of `empire`'s worlds still called "Not Named Yet" from the
/// planet name list, never with a name another planet in the game has, as
/// AI seats name their worlds. A world stays unnamed once the list runs out.
pub fn name_unnamed_worlds(game_data: &mut CoreGameData, empire: u8) {
    let names = planet_names();
    for index in 0..game_data.planets.len() {
        let planet = &game_data.planets[index];
        if planet.owner.0 != empire || planet.name.trim() != UNNAMED {
            continue;
        }
        // Start each world at its own place in the list, not all at its head.
        let start = index * 131 % names.len();
        let free = (0..names.len())
            .map(|step| &names[(start + step) % names.len()])
            .find(|name| !planet_name_in_use(game_data, name, None));
        if let Some(name) = free {
            set_planet_name(&mut game_data.planets[index], name);
        }
    }
}

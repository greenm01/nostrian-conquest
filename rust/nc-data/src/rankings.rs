//! The previous-year rankings CONQUEST.DAT keeps for each empire.

use crate::{Conquest, Player};

/// Copies each empire's production and planet count into the previous-year
/// arrays, as ECMAINT does at the start of a pass (0x27A5). Slots without a
/// player record read 0.
pub fn snapshot_previous_year(conquest: &mut Conquest, players: &[Player]) {
    for slot in 0..25 {
        let player = players.get(slot);
        conquest.previous_production[slot] = player.map_or(0, |player| player.production);
        conquest.previous_planet_count[slot] = player.map_or(0, |player| player.planet_count);
    }
}

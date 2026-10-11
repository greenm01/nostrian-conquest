use super::super::{ColonizationEvent, ColonizationResolvedEvent};
use crate::{CoreGameData, Order};

/// Apply colonization events to the in-memory planet and player records.
///
/// When a ColonizeWorld fleet arrives at an unowned planet:
/// - Planet name set to "Not Named Yet"
/// - Planet ownership_status set to 2 (owned)
/// - Planet owner_empire_slot set to colonizing empire
/// - Planet army_count set to 1 (colonist armies)
/// - Planet raw[0x03] set to 0x81 (colonization flag in potential_production high byte)
/// - Player planet_count incremented
/// - Player production_score incremented (legacy PLAYER offset 0x52; confirmed from fixture)
///
/// Confirmed from fleet-scenario fixture: fleet 0 ColonizeWorld arrives at (15,13),
/// planet 13 colonized by empire 1, player 0 record updated.
pub(super) fn process_colonizations(
    game_data: &mut CoreGameData,
    events: &[ColonizationEvent],
) -> Result<Vec<ColonizationResolvedEvent>, Box<dyn std::error::Error>> {
    let mut resolved = Vec::new();
    for event in events {
        let [cx, cy] = event.coords;
        let Some(fleet) = game_data.fleets.get(event.fleet_idx) else {
            resolved.push(ColonizationResolvedEvent::Aborted {
                fleet_idx: event.fleet_idx,
                colonizer_empire_raw: event.owner_empire,
                coords: event.coords,
                stardate_week: None,
            });
            continue;
        };
        let colonization_aborted = fleet.current_location_coords_raw() != event.coords
            || fleet.standing_order_kind() == Order::SeekHome
            || fleet.etac_count() == 0;
        if colonization_aborted {
            resolved.push(ColonizationResolvedEvent::Aborted {
                fleet_idx: event.fleet_idx,
                colonizer_empire_raw: event.owner_empire,
                coords: event.coords,
                stardate_week: None,
            });
            continue;
        }

        let planet_idx = game_data.planets.iter().position(|p| {
            let [px, py] = p.coords_raw();
            px == cx && py == cy
        });

        if let Some(idx) = planet_idx {
            let planet = &mut game_data.planets[idx];
            let is_unowned = planet.owner_empire_slot_raw() == 0;
            if is_unowned {
                planet.set_planet_name("Not Named Yet");
                planet.set_ownership_status_raw(2);
                planet.set_owner_empire_slot_raw(event.owner_empire);
                planet.set_army_count_raw(1);
                planet.set_present_production(1.0);

                let player_idx = (event.owner_empire as usize).saturating_sub(1);
                if player_idx < game_data.players.len() {
                    let current_count = game_data.players[player_idx].planet_count_raw();
                    game_data.players[player_idx]
                        .set_planet_count_raw(current_count.saturating_add(1));

                    let current_score = game_data.players[player_idx].production_score_raw();
                    game_data.players[player_idx]
                        .set_production_score_raw(current_score.saturating_add(1));
                }

                resolved.push(ColonizationResolvedEvent::Succeeded {
                    fleet_idx: event.fleet_idx,
                    planet_idx: idx,
                    colonizer_empire_raw: event.owner_empire,
                    stardate_week: None,
                });
            } else {
                resolved.push(ColonizationResolvedEvent::BlockedByOwner {
                    fleet_idx: event.fleet_idx,
                    planet_idx: idx,
                    colonizer_empire_raw: event.owner_empire,
                    owner_empire_raw: planet.owner_empire_slot_raw(),
                    stardate_week: None,
                });
            }
        }
    }

    Ok(resolved)
}

use super::super::{ColonizationEvent, ColonizationResolvedEvent};
use crate::{CoreGameData, Order};
use nc_data::{Real48, set_planet_name};

/// Production a new colony starts with, or its potential if that is lower.
const COLONY_HEAD_START: u16 = 3;

/// Apply colonization events to the in-memory planet and player records.
///
/// When a ColonizeWorld fleet arrives at an unowned planet:
/// - Planet name set to "Not Named Yet"
/// - Planet ownership_status set to 2 (owned)
/// - Planet owner_empire_slot set to colonizing empire
/// - Planet army_count set to 1 (colonist armies)
/// - Planet production set to the colony head start, `min(3, potential)`;
///   only founding does this, never capture, recapture, or recovery
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
                colonizer_empire: event.owner_empire,
                coords: event.coords,
                stardate_week: None,
            });
            continue;
        };
        let colonization_aborted = fleet.sector.to_array() != event.coords
            || fleet.order == Order::SeekHome
            || fleet.ships.etacs == 0;
        if colonization_aborted {
            resolved.push(ColonizationResolvedEvent::Aborted {
                fleet_idx: event.fleet_idx,
                colonizer_empire: event.owner_empire,
                coords: event.coords,
                stardate_week: None,
            });
            continue;
        }

        let planet_idx = game_data.planets.iter().position(|p| {
            let [px, py] = p.sector.to_array();
            px == cx && py == cy
        });

        if let Some(idx) = planet_idx {
            let planet = &mut game_data.planets[idx];
            let is_unowned = planet.owner.0 == 0;
            if is_unowned {
                set_planet_name(planet, "Not Named Yet");
                planet.years_under_owner = 2;
                planet.owner.0 = event.owner_empire;
                planet.armies = 1;
                let head_start = u16::from(planet.potential_production).min(COLONY_HEAD_START);
                planet.production = Real48::from_f64(f64::from(head_start));

                let player_idx = (event.owner_empire as usize).saturating_sub(1);
                if player_idx < game_data.players.len() {
                    let current_count = game_data.players[player_idx].planet_count;
                    game_data.players[player_idx].planet_count = current_count.saturating_add(1);

                    let current_score = game_data.players[player_idx].production;
                    game_data.players[player_idx].production =
                        current_score.saturating_add(head_start);
                }

                resolved.push(ColonizationResolvedEvent::Succeeded {
                    fleet_idx: event.fleet_idx,
                    planet_idx: idx,
                    colonizer_empire: event.owner_empire,
                    stardate_week: None,
                });
            } else {
                resolved.push(ColonizationResolvedEvent::BlockedByOwner {
                    fleet_idx: event.fleet_idx,
                    planet_idx: idx,
                    colonizer_empire: event.owner_empire,
                    owner_empire: planet.owner.0,
                    stardate_week: None,
                });
            }
        }
    }

    Ok(resolved)
}

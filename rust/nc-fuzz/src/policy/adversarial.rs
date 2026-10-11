//! Boundary and out-of-range turn values. Most are rejected by the
//! submission path; the ones it accepts must still leave a state that
//! maintenance handles.

use nc_data::{
    CoreGameData, DiplomaticRelation, FleetDetachSelection, FleetTurnAction, PlanetTurnAction,
    TurnDiplomacyDirective,
};

use super::actions::Choices;
use super::view::{fleet, map_size, owned_fleets, owned_planets};
use crate::choice::pick_from;
use crate::turn::TurnAction;

const EDGE_U8: [u8; 5] = [0, 1, 10, 101, u8::MAX];
const EDGE_U16: [u16; 4] = [0, 1, 255, u16::MAX];
const NAMES: [&str; 5] = [
    "",
    "X",
    "A Name Far Longer Than Any Planet Record Can Hold",
    "Quote \" Back \\ Slash",
    "Ünïcødé",
];

pub fn edge_action(state: &CoreGameData, empire: u8, choices: Choices) -> Option<TurnAction> {
    let player_count = usize::from(state.conquest.player_count());
    match choices.pick(0, 9) {
        0 => fleet_order(state, empire, choices),
        1 => {
            let record = fleet_record(state, empire, choices)?;
            let value = pick_from(choices, &EDGE_U8)?;
            Some(TurnAction::Fleet(
                record,
                FleetTurnAction::RulesOfEngagement { value },
            ))
        }
        2 => Some(TurnAction::Tax(pick_from(
            choices,
            &[0, 100, 101, u8::MAX],
        )?)),
        3 => {
            let to_empire_raw = pick_from(
                choices,
                &[
                    0,
                    empire,
                    player_count as u8,
                    player_count as u8 + 1,
                    u8::MAX,
                ],
            )?;
            Some(TurnAction::Diplomacy(TurnDiplomacyDirective {
                to_empire_raw,
                relation: DiplomaticRelation::Enemy,
            }))
        }
        4 => {
            let world = planet_record(state, empire, choices)?;
            let points_remaining_raw = pick_from(choices, &[1, 4, 5, 45, 50, u8::MAX])?;
            let kind_raw = choices.pick(0, 11) as u8;
            Some(TurnAction::Planet(
                world,
                PlanetTurnAction::Build {
                    points_remaining_raw,
                    kind_raw,
                },
            ))
        }
        5 => {
            let world = planet_record(state, empire, choices)?;
            let action = match choices.pick(0, 4) {
                0 => PlanetTurnAction::Commission {
                    slot_0_based: pick_from(choices, &[0, 9, 10, 254])?,
                },
                1 => PlanetTurnAction::RemoveBuild {
                    qty: pick_from(choices, &EDGE_U16)?,
                    kind_raw: choices.pick(0, 11) as u8,
                },
                2 => PlanetTurnAction::ClearBuildKind {
                    kind_raw: choices.pick(0, 11) as u8,
                },
                3 => PlanetTurnAction::Rename {
                    name: pick_from(choices, &NAMES)?.to_string(),
                },
                _ => PlanetTurnAction::Scorch,
            };
            Some(TurnAction::Planet(world, action))
        }
        6 => {
            let record = fleet_record(state, empire, choices)?;
            let selection = edge_selection(state, record, choices);
            Some(TurnAction::Fleet(
                record,
                FleetTurnAction::Detach {
                    selection,
                    donor_speed: Some(pick_from(choices, &EDGE_U8)?),
                    new_fleet_roe: pick_from(choices, &EDGE_U8)?,
                },
            ))
        }
        7 => {
            let record = fleet_record(state, empire, choices)?;
            let host = fleet_record(state, empire, choices)?;
            let selection = edge_selection(state, record, choices);
            Some(TurnAction::Fleet(
                record,
                FleetTurnAction::Transfer {
                    host_fleet_record_index_1_based: host,
                    selection,
                },
            ))
        }
        8 => {
            let record = fleet_record(state, empire, choices)?;
            let host = fleet_record(state, empire, choices)?;
            Some(TurnAction::Fleet(
                record,
                FleetTurnAction::Join {
                    host_fleet_record_index_1_based: host,
                },
            ))
        }
        _ => {
            let record = fleet_record(state, empire, choices)?;
            let planet_record_index_1_based = planet_record(state, empire, choices)?;
            let qty = pick_from(choices, &EDGE_U16)?;
            let action = if choices.flip() {
                FleetTurnAction::UnloadArmies {
                    planet_record_index_1_based,
                    qty,
                }
            } else {
                FleetTurnAction::LoadArmies {
                    planet_record_index_1_based,
                    qty,
                }
            };
            Some(TurnAction::Fleet(record, action))
        }
    }
}

fn fleet_order(state: &CoreGameData, empire: u8, choices: Choices) -> Option<TurnAction> {
    let record = fleet_record(state, empire, choices)?;
    let size = map_size(state);
    let edges = [0, 1, size, size + 1, u8::MAX];
    let target = [pick_from(choices, &edges)?, pick_from(choices, &edges)?];
    let max_speed = state
        .fleets
        .get(record.wrapping_sub(1))
        .map_or(0, |fleet| fleet.max_speed());
    let speed = pick_from(
        choices,
        &[0, max_speed, max_speed.saturating_add(1), u8::MAX],
    )?;
    let order_code = choices.pick(0, 16) as u8;
    let aux = [None, Some(0), Some(1), Some(u8::MAX)];
    Some(TurnAction::Fleet(
        record,
        FleetTurnAction::Order {
            speed,
            order_code,
            target: Some(target),
            aux0: pick_from(choices, &aux)?,
            aux1: pick_from(choices, &aux)?,
        },
    ))
}

/// Own fleets, a foreign fleet, record 0, and records just past the end.
fn fleet_record(state: &CoreGameData, empire: u8, choices: Choices) -> Option<usize> {
    let count = state.fleets.len();
    let mut records = owned_fleets(state, empire);
    if let Some(foreign) =
        (1..=count).find(|&record| fleet(state, record).owner_empire_raw() != empire)
    {
        records.push(foreign);
    }
    records.extend([0, count, count + 1, u16::MAX as usize]);
    pick_from(choices, &records)
}

fn planet_record(state: &CoreGameData, empire: u8, choices: Choices) -> Option<usize> {
    let count = state.planets.len();
    let mut records = owned_planets(state, empire);
    records.extend([0, 1, count, count + 1, u16::MAX as usize]);
    pick_from(choices, &records)
}

/// Nothing, everything, or one more than the donor carries.
fn edge_selection(state: &CoreGameData, record: usize, choices: Choices) -> FleetDetachSelection {
    let Some(donor) = state.fleets.get(record.wrapping_sub(1)) else {
        return FleetDetachSelection {
            destroyers: 1,
            ..FleetDetachSelection::default()
        };
    };
    let extra = u16::from(choices.flip());
    let mut selection = FleetDetachSelection::default();
    if choices.flip() {
        selection.battleships = donor.battleship_count() + extra;
        selection.cruisers = donor.cruiser_count() + extra;
        selection.destroyers = donor.destroyer_count() + extra;
        selection.scouts = donor.scout_count().saturating_add(extra);
        selection.etacs = donor.etac_count() + extra;
        selection.full_transports = donor.army_count().min(donor.troop_transport_count()) + extra;
        selection.empty_transports = donor
            .troop_transport_count()
            .saturating_sub(donor.army_count());
    }
    selection
}

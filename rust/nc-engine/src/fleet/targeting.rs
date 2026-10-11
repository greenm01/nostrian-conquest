use std::collections::BTreeMap;
use std::collections::BTreeSet;

use nc_data::{
    CoreGameData, PlanetIntelSnapshot, PlayerStarmapWorld,
    build_player_starmap_projection_from_snapshots, has_any_force,
};

use crate::navigation::sector_distance_sq;
use crate::orders::FleetTargetInputKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnedFleetTarget {
    pub fleet_record_index_1_based: usize,
    pub fleet_number: u16,
    pub coords: [u8; 2],
    pub target_coords: [u8; 2],
    pub order_code: u8,
    pub current_speed: u8,
    pub max_speed: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnedStarbaseTarget {
    pub base_record_index_1_based: usize,
    pub base_id: u16,
    pub coords: [u8; 2],
    pub destination_coords: [u8; 2],
}

pub fn owned_fleet_targets(game_data: &CoreGameData, owner_empire_id: u8) -> Vec<OwnedFleetTarget> {
    let mut rows = game_data
        .fleets
        .iter()
        .enumerate()
        .filter(|(_, fleet)| fleet.owner.0 == owner_empire_id && has_any_force(fleet))
        .map(|(idx, fleet)| OwnedFleetTarget {
            fleet_record_index_1_based: idx + 1,
            fleet_number: fleet.fleet_number.0,
            coords: fleet.sector.to_array(),
            target_coords: fleet.target.to_array(),
            order_code: fleet.order.to_raw(),
            current_speed: fleet.speed,
            max_speed: fleet.max_speed,
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| row.fleet_number);
    rows
}

pub fn owned_starbase_targets(
    game_data: &CoreGameData,
    owner_empire_id: u8,
) -> Vec<OwnedStarbaseTarget> {
    let mut rows = game_data
        .bases
        .iter()
        .enumerate()
        .filter(|(_, base)| base.owner.0 == owner_empire_id && base.base_id.0 != 0)
        .map(|(idx, base)| OwnedStarbaseTarget {
            base_record_index_1_based: idx + 1,
            base_id: base.base_id.0,
            coords: base.sector.to_array(),
            destination_coords: base.target.to_array(),
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| row.base_id);
    rows
}

pub fn default_starbase_target(
    game_data: &CoreGameData,
    owner_empire_id: u8,
    anchor: [u8; 2],
) -> Option<OwnedStarbaseTarget> {
    owned_starbase_targets(game_data, owner_empire_id)
        .into_iter()
        .min_by_key(|row| sector_distance_sq(anchor, row.coords))
}

pub fn default_host_fleet_target(
    game_data: &CoreGameData,
    owner_empire_id: u8,
    anchor: [u8; 2],
    excluded_records: &BTreeSet<usize>,
) -> Option<OwnedFleetTarget> {
    owned_fleet_targets(game_data, owner_empire_id)
        .into_iter()
        .filter(|row| !excluded_records.contains(&row.fleet_record_index_1_based))
        .min_by_key(|row| sector_distance_sq(anchor, row.coords))
}

pub fn recommended_coordinate_target(
    game_data: &CoreGameData,
    snapshots: &BTreeMap<usize, PlanetIntelSnapshot>,
    viewer_empire_id: u8,
    mission_code: u8,
    anchor: [u8; 2],
    selected_records: &BTreeSet<usize>,
) -> Option<[u8; 2]> {
    recommended_coordinate_target_candidates(
        game_data,
        snapshots,
        viewer_empire_id,
        mission_code,
        anchor,
        selected_records,
    )
    .into_iter()
    .next()
}

pub fn recommended_coordinate_target_y_for_entered_x(
    game_data: &CoreGameData,
    snapshots: &BTreeMap<usize, PlanetIntelSnapshot>,
    viewer_empire_id: u8,
    mission_code: u8,
    anchor: [u8; 2],
    selected_records: &BTreeSet<usize>,
    entered_x: &str,
) -> Option<u8> {
    let candidates = recommended_coordinate_target_candidates(
        game_data,
        snapshots,
        viewer_empire_id,
        mission_code,
        anchor,
        selected_records,
    );
    if !fleet_order_target_y_depends_on_entered_x(mission_code) {
        return candidates.into_iter().next().map(|coords| coords[1]);
    }
    let entered_x = entered_x.trim();
    if entered_x.is_empty() {
        return candidates.into_iter().next().map(|coords| coords[1]);
    }
    let target_x = entered_x.parse::<u8>().ok()?;
    candidates
        .into_iter()
        .find(|coords| coords[0] == target_x)
        .map(|coords| coords[1])
}

pub fn target_available_for_mission(
    game_data: &CoreGameData,
    snapshots: &BTreeMap<usize, PlanetIntelSnapshot>,
    viewer_empire_id: u8,
    mission_code: u8,
    anchor: [u8; 2],
    selected_records: &BTreeSet<usize>,
) -> bool {
    match crate::fleet_target_input_kind(Some(mission_code)) {
        FleetTargetInputKind::StarbaseId => {
            default_starbase_target(game_data, viewer_empire_id, anchor).is_some()
        }
        FleetTargetInputKind::FleetId => {
            default_host_fleet_target(game_data, viewer_empire_id, anchor, selected_records)
                .is_some()
        }
        FleetTargetInputKind::Coordinates | FleetTargetInputKind::None => {
            !fleet_mission_requires_preselected_target(mission_code)
                || recommended_coordinate_target(
                    game_data,
                    snapshots,
                    viewer_empire_id,
                    mission_code,
                    anchor,
                    selected_records,
                )
                .is_some()
        }
    }
}

pub fn recommended_coordinate_target_candidates(
    game_data: &CoreGameData,
    snapshots: &BTreeMap<usize, PlanetIntelSnapshot>,
    viewer_empire_id: u8,
    mission_code: u8,
    anchor: [u8; 2],
    selected_records: &BTreeSet<usize>,
) -> Vec<[u8; 2]> {
    match mission_code {
        0 | 1 | 3 => vec![anchor],
        4 => default_starbase_target(game_data, viewer_empire_id, anchor)
            .into_iter()
            .map(|row| row.coords)
            .collect(),
        2 | 15 => owned_worlds_nearest_first(game_data, viewer_empire_id, selected_records),
        5 => filtered_planet_database_targets_from(
            game_data,
            snapshots,
            viewer_empire_id,
            anchor,
            |world| world.known_owner_empire_id == Some(viewer_empire_id),
        ),
        6 | 7 | 8 => filtered_planet_database_targets_from(
            game_data,
            snapshots,
            viewer_empire_id,
            anchor,
            |world| {
                matches!(
                    world.known_owner_empire_id,
                    Some(owner) if owner > 0 && owner != viewer_empire_id
                )
            },
        ),
        9 => view_world_target_candidates_from(
            game_data,
            snapshots,
            viewer_empire_id,
            anchor,
            selected_records,
        ),
        10 => scout_sector_target_candidates_from(
            game_data,
            snapshots,
            viewer_empire_id,
            anchor,
            selected_records,
        ),
        11 => scout_system_target_candidates_from(
            game_data,
            snapshots,
            viewer_empire_id,
            anchor,
            selected_records,
        ),
        12 => colonize_target_candidates_from(
            game_data,
            snapshots,
            viewer_empire_id,
            anchor,
            selected_records,
        ),
        14 => {
            rendezvous_target_candidates_from(game_data, viewer_empire_id, anchor, selected_records)
        }
        _ => Vec::new(),
    }
}

const NO_OWNED_WORLD_MESSAGE: &str = "You do not own any planets. Order aborted.";

/// Targets ECGAME sets without asking, one per fleet: Seek Home always, and
/// Salvage when the empire owns a single world. `Ok(None)` means the player
/// enters coordinates.
pub fn automatic_order_targets(
    game_data: &CoreGameData,
    mission_code: u8,
    fleet_records: &[usize],
) -> Result<Option<Vec<[u8; 2]>>, String> {
    match nc_data::Order::from_raw(mission_code) {
        nc_data::Order::SeekHome => match game_data.seek_home_targets(fleet_records) {
            Ok(targets) => Ok(Some(targets)),
            Err(nc_data::SeekHomeTarget::AlreadyInOnlySystem) => Err(format!(
                "{} fleet is already in your only controlled star system.",
                fleet_title(game_data, fleet_records[0])
            )),
            Err(_) => Err(NO_OWNED_WORLD_MESSAGE.to_string()),
        },
        nc_data::Order::Salvage => {
            let Some(&first) = fleet_records.first() else {
                return Ok(None);
            };
            let owner = game_data.fleets[first - 1].owner.0;
            let owned = game_data
                .planets
                .iter()
                .filter(|planet| planet.owner.0 == owner)
                .count();
            match owned {
                0 => Err(NO_OWNED_WORLD_MESSAGE.to_string()),
                1 => Ok(game_data
                    .nearest_owned_world(first, false)
                    .map(|target| vec![target; fleet_records.len()])),
                _ => Ok(None),
            }
        }
        _ => Ok(None),
    }
}

/// ECGAME's Seek Home question (`0x13d23`), asked with yes as the default.
pub const SEEK_NEXT_CLOSEST_PROMPT: &str =
    "Do you wish this fleet to seek the next closest system?";

/// The question ECGAME asks when one fleet ordered to Seek Home already sits
/// in the system of its nearest owned world and the empire owns another.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeekNextClosestQuestion {
    /// The line shown above the question.
    pub notice: String,
    /// The target on no: the world the fleet is at.
    pub stay: [u8; 2],
    /// The target on yes: the next closest owned world.
    pub next: [u8; 2],
}

/// ECGAME's Seek Home question for this order, or `None` when it does not
/// ask: another mission, a group of fleets, or a fleet away from its nearest
/// world.
pub fn seek_next_closest_question(
    game_data: &CoreGameData,
    mission_code: u8,
    fleet_records: &[usize],
) -> Option<SeekNextClosestQuestion> {
    let [record] = fleet_records else {
        return None;
    };
    if nc_data::Order::from_raw(mission_code) != nc_data::Order::SeekHome {
        return None;
    }
    let nc_data::SeekHomeTarget::NextClosest(next) = game_data.seek_home_target(*record) else {
        return None;
    };
    Some(SeekNextClosestQuestion {
        notice: format!(
            "{} fleet is already in a star system you control.",
            fleet_title(game_data, *record)
        ),
        stay: game_data.nearest_owned_world(*record, false)?,
        next,
    })
}

/// ECGAME's "The 3rd" before "fleet" in its order messages.
fn fleet_title(game_data: &CoreGameData, record: usize) -> String {
    let number = game_data.fleets[record - 1].fleet_number.0;
    format!(
        "The {}",
        super::starbase::ordinal_number(usize::from(number))
    )
}

pub fn fleet_mission_requires_preselected_target(order_code: u8) -> bool {
    matches!(order_code, 4)
}

pub fn fleet_order_target_requires_planet_system(order_code: u8) -> bool {
    matches!(order_code, 2 | 5 | 6 | 7 | 8 | 9 | 11 | 12 | 15)
}

pub fn fleet_order_target_rejects_owned_planet(order_code: u8) -> bool {
    matches!(order_code, 6 | 7 | 8)
}

pub fn fleet_order_target_rejects_owned_scout_target(order_code: u8) -> bool {
    matches!(order_code, 10 | 11)
}

pub fn fleet_order_target_requires_owned_planet(order_code: u8) -> bool {
    matches!(order_code, 2 | 15)
}

pub fn fleet_order_target_y_depends_on_entered_x(order_code: u8) -> bool {
    matches!(
        crate::fleet_target_input_kind(Some(order_code)),
        FleetTargetInputKind::Coordinates
    )
}

fn friendly_colonize_target_claimed_elsewhere(
    game_data: &CoreGameData,
    viewer_empire_id: u8,
    coords: [u8; 2],
    selected_records: &BTreeSet<usize>,
) -> bool {
    game_data
        .conflicting_friendly_colonize_fleet_record(viewer_empire_id, coords, selected_records)
        .is_some()
}

fn friendly_target_claimed_elsewhere(
    game_data: &CoreGameData,
    viewer_empire_id: u8,
    coords: [u8; 2],
    selected_records: &BTreeSet<usize>,
) -> bool {
    game_data.fleets.iter().enumerate().any(|(idx, fleet)| {
        fleet.owner.0 == viewer_empire_id
            && !selected_records.contains(&(idx + 1))
            && fleet.target.to_array() == coords
    })
}

fn friendly_scout_target_claimed_elsewhere(
    game_data: &CoreGameData,
    viewer_empire_id: u8,
    coords: [u8; 2],
    selected_records: &BTreeSet<usize>,
) -> bool {
    game_data.fleets.iter().enumerate().any(|(idx, fleet)| {
        fleet.owner.0 == viewer_empire_id
            && !selected_records.contains(&(idx + 1))
            && fleet.ships.scouts > 0
            && matches!(
                fleet.order,
                nc_data::Order::ScoutSector | nc_data::Order::ScoutSolarSystem
            )
            && fleet.target.to_array() == coords
    })
}

fn view_world_target_candidates_from(
    game_data: &CoreGameData,
    snapshots: &BTreeMap<usize, PlanetIntelSnapshot>,
    viewer_empire_id: u8,
    anchor: [u8; 2],
    selected_records: &BTreeSet<usize>,
) -> Vec<[u8; 2]> {
    let worlds = player_planet_database_worlds(game_data, snapshots, viewer_empire_id);
    let mut coords = worlds
        .iter()
        .into_iter()
        .filter(|world| matches!(world.intel_tier, nc_data::IntelTier::Unknown))
        .map(|world| world.coords)
        .collect::<Vec<_>>();
    if coords.is_empty() {
        coords = worlds
            .into_iter()
            .filter(|world| world.known_owner_empire_id != Some(viewer_empire_id))
            .map(|world| world.coords)
            .collect();
    }
    coords.sort_by_key(|coords| {
        (
            friendly_target_claimed_elsewhere(
                game_data,
                viewer_empire_id,
                *coords,
                selected_records,
            ),
            sector_distance_sq(anchor, *coords),
            *coords,
        )
    });
    coords.dedup();
    coords
}

fn colonize_target_candidates_from(
    game_data: &CoreGameData,
    snapshots: &BTreeMap<usize, PlanetIntelSnapshot>,
    viewer_empire_id: u8,
    anchor: [u8; 2],
    selected_records: &BTreeSet<usize>,
) -> Vec<[u8; 2]> {
    filtered_planet_database_targets_from(game_data, snapshots, viewer_empire_id, anchor, |world| {
        matches!(world.known_owner_empire_id, None | Some(0))
            && !friendly_colonize_target_claimed_elsewhere(
                game_data,
                viewer_empire_id,
                world.coords,
                selected_records,
            )
    })
}

fn scout_sector_target_candidates_from(
    game_data: &CoreGameData,
    snapshots: &BTreeMap<usize, PlanetIntelSnapshot>,
    viewer_empire_id: u8,
    anchor: [u8; 2],
    selected_records: &BTreeSet<usize>,
) -> Vec<[u8; 2]> {
    let mut coords = filtered_planet_database_targets_from(
        game_data,
        snapshots,
        viewer_empire_id,
        anchor,
        |world| {
            world.known_owner_empire_id != Some(viewer_empire_id)
                && !friendly_scout_target_claimed_elsewhere(
                    game_data,
                    viewer_empire_id,
                    world.coords,
                    selected_records,
                )
        },
    );
    if coords.is_empty() && !coords_are_owned_system(game_data, viewer_empire_id, anchor) {
        coords.push(anchor);
    }
    coords
}

fn scout_system_target_candidates_from(
    game_data: &CoreGameData,
    snapshots: &BTreeMap<usize, PlanetIntelSnapshot>,
    viewer_empire_id: u8,
    anchor: [u8; 2],
    selected_records: &BTreeSet<usize>,
) -> Vec<[u8; 2]> {
    filtered_planet_database_targets_from(game_data, snapshots, viewer_empire_id, anchor, |world| {
        world.known_owner_empire_id != Some(viewer_empire_id)
            && !friendly_scout_target_claimed_elsewhere(
                game_data,
                viewer_empire_id,
                world.coords,
                selected_records,
            )
    })
}

fn rendezvous_target_candidates_from(
    game_data: &CoreGameData,
    viewer_empire_id: u8,
    anchor: [u8; 2],
    selected_records: &BTreeSet<usize>,
) -> Vec<[u8; 2]> {
    let mut coords = game_data
        .fleets
        .iter()
        .enumerate()
        .filter(|(idx, fleet)| {
            fleet.owner.0 == viewer_empire_id
                && !selected_records.contains(&(idx + 1))
                && fleet.order == nc_data::Order::RendezvousSector
        })
        .map(|(_, fleet)| fleet.target.to_array())
        .collect::<Vec<_>>();
    sort_unique_coords_by_distance(anchor, &mut coords);
    if coords.is_empty() {
        coords.push(anchor);
    }
    coords
}

/// ECGAME's Seek Home and Salvage defaults: the world nearest the first
/// selected fleet, then the other owned worlds in planet order (the default
/// Y for an entered X is the first owned world in that column).
fn owned_worlds_nearest_first(
    game_data: &CoreGameData,
    viewer_empire_id: u8,
    selected_records: &BTreeSet<usize>,
) -> Vec<[u8; 2]> {
    let mut coords = game_data
        .planets
        .iter()
        .filter(|planet| planet.owner.0 == viewer_empire_id)
        .map(|planet| planet.sector.to_array())
        .collect::<Vec<_>>();
    if let Some(nearest) = selected_records
        .first()
        .and_then(|&record| game_data.nearest_owned_world(record, false))
    {
        coords.retain(|coords| *coords != nearest);
        coords.insert(0, nearest);
    }
    coords
}

fn filtered_planet_database_targets_from<F>(
    game_data: &CoreGameData,
    snapshots: &BTreeMap<usize, PlanetIntelSnapshot>,
    viewer_empire_id: u8,
    anchor: [u8; 2],
    predicate: F,
) -> Vec<[u8; 2]>
where
    F: Fn(&PlayerStarmapWorld) -> bool,
{
    let mut coords = player_planet_database_worlds(game_data, snapshots, viewer_empire_id)
        .into_iter()
        .filter(predicate)
        .map(|world| world.coords)
        .collect::<Vec<_>>();
    sort_unique_coords_by_distance(anchor, &mut coords);
    coords
}

fn player_planet_database_worlds(
    game_data: &CoreGameData,
    snapshots: &BTreeMap<usize, PlanetIntelSnapshot>,
    viewer_empire_id: u8,
) -> Vec<PlayerStarmapWorld> {
    build_player_starmap_projection_from_snapshots(game_data, snapshots, viewer_empire_id).worlds
}

fn coords_are_owned_system(
    game_data: &CoreGameData,
    viewer_empire_id: u8,
    coords: [u8; 2],
) -> bool {
    game_data
        .planets
        .iter()
        .any(|planet| planet.sector.to_array() == coords && planet.owner.0 == viewer_empire_id)
}

fn sort_unique_coords_by_distance(anchor: [u8; 2], coords: &mut Vec<[u8; 2]>) {
    coords.sort_by_key(|coords| sector_distance_sq(anchor, *coords));
    coords.dedup();
}

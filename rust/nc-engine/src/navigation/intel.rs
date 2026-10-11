use std::collections::{BTreeMap, HashSet};

use nc_data::{
    CoreGameData, Order, PlanetIntelSnapshot, build_player_starmap_projection_from_snapshots,
};

use super::VisibleHazardIntel;

pub(super) fn visible_hazard_intel_from_snapshots(
    game_data: &CoreGameData,
    snapshots: &BTreeMap<usize, PlanetIntelSnapshot>,
    viewer_empire: u8,
) -> VisibleHazardIntel {
    let mut intel = VisibleHazardIntel::default();
    let projection =
        build_player_starmap_projection_from_snapshots(game_data, snapshots, viewer_empire);
    let viewer_owned_worlds: HashSet<[u8; 2]> = game_data
        .planets
        .iter()
        .filter(|planet| planet.owner.0 == viewer_empire)
        .map(|planet| planet.sector.to_array())
        .collect();

    for world in projection.worlds {
        let Some(owner_raw) = world.known_owner_empire_id else {
            continue;
        };
        if owner_raw == 0 || owner_raw == viewer_empire {
            continue;
        }

        intel.foreign_worlds.insert(world.coords);

        if world.known_potential_production == Some(100) {
            intel.hostile_homeworlds.insert(world.coords);
        }
    }

    for fleet in &game_data.fleets {
        if fleet.owner.0 == 0 || fleet.owner.0 == viewer_empire {
            continue;
        }
        if fleet.order != Order::GuardBlockadeWorld {
            continue;
        }
        let coords = fleet.sector.to_array();
        if viewer_owned_worlds.contains(&coords) {
            intel.hostile_blockades.insert(coords);
        }
    }

    intel
}

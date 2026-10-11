use std::collections::{BTreeMap, BTreeSet};

use nc_data::{GameStateBuilder, IntelTier, PlanetIntelSnapshot};
use nc_engine::{recommended_coordinate_target, target_available_for_mission};

fn seed_non_owned_world(
    game_data: &mut nc_data::CoreGameData,
    coords: [u8; 2],
    owner_empire_id: u8,
) -> usize {
    let (planet_index, planet) = game_data
        .planets
        .iter_mut()
        .enumerate()
        .find(|(_, planet)| planet.owner_empire_slot_raw() == 0 && planet.coords_raw() == [0, 0])
        .expect("unused planet slot");
    planet.set_coords_raw(coords);
    planet.set_owner_empire_slot_raw(owner_empire_id);
    planet_index + 1
}

fn snapshot_map(
    game_data: &nc_data::CoreGameData,
    viewer_empire_id: u8,
    unknown_planet_record_index: Option<usize>,
) -> BTreeMap<usize, PlanetIntelSnapshot> {
    game_data
        .planets
        .iter()
        .enumerate()
        .filter_map(|(planet_index, planet)| {
            let planet_record_index_1_based = planet_index + 1;
            if planet.owner_empire_slot_raw() == viewer_empire_id {
                return None;
            }
            let (intel_tier, known_owner_empire_id) =
                if Some(planet_record_index_1_based) == unknown_planet_record_index {
                    (IntelTier::Unknown, None)
                } else if planet.coords_raw() == [0, 0] {
                    (IntelTier::Owned, Some(viewer_empire_id))
                } else {
                    (IntelTier::Partial, Some(planet.owner_empire_slot_raw()))
                };
            Some((
                planet_record_index_1_based,
                PlanetIntelSnapshot {
                    planet_record_index_1_based,
                    intel_tier,
                    compat_is_orbit_seed: false,
                    last_intel_year: None,
                    seen_year: None,
                    scout_year: None,
                    known_name: None,
                    known_owner_empire_id,
                    known_potential_production: None,
                    known_armies: None,
                    known_ground_batteries: None,
                    known_starbase_count: None,
                    known_current_production: None,
                    known_stored_points: None,
                    known_docked_summary: None,
                    known_orbit_summary: None,
                    compat_word_1e: None,
                },
            ))
        })
        .collect()
}

#[test]
fn view_world_prefers_unknown_targets_while_any_remain() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_homeworld_coords(vec![[16, 13], [12, 6], [4, 15], [15, 15]])
        .build_initialized_baseline()
        .expect("baseline");
    let unknown_target = seed_non_owned_world(&mut game_data, [8, 8], 0);
    let nearer_known_target = seed_non_owned_world(&mut game_data, [15, 13], 2);
    let snapshots = snapshot_map(&game_data, 1, Some(unknown_target));

    let target =
        recommended_coordinate_target(&game_data, &snapshots, 1, 9, [16, 13], &BTreeSet::new());

    assert_eq!(target, Some([8, 8]));
    assert_ne!(
        target,
        Some(game_data.planets[nearer_known_target - 1].coords_raw())
    );
}

#[test]
fn view_world_falls_back_to_nearest_non_owned_world_when_unknowns_are_exhausted() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_homeworld_coords(vec![[16, 13], [12, 6], [4, 15], [15, 15]])
        .build_initialized_baseline()
        .expect("baseline");
    seed_non_owned_world(&mut game_data, [8, 8], 0);
    seed_non_owned_world(&mut game_data, [15, 13], 2);
    let snapshots = snapshot_map(&game_data, 1, None);

    let target =
        recommended_coordinate_target(&game_data, &snapshots, 1, 9, [16, 13], &BTreeSet::new());

    assert_eq!(target, Some([15, 13]));
    assert!(target_available_for_mission(
        &game_data,
        &snapshots,
        1,
        9,
        [16, 13],
        &BTreeSet::new(),
    ));
}

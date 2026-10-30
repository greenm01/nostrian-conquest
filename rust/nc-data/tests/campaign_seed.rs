use std::path::PathBuf;

use nc_compat::load_classic_core_files;
use nc_data::{Coords, CoreGameData, derive_campaign_seed_from_runtime};

fn init_fixture() -> CoreGameData {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/ecutil-init/v1.5");
    load_classic_core_files(&dir).expect("fixture should load")
}

// The seed hashes the typed game state in a fixed field order. A change to
// that order or to how a field feeds the hash moves every derived seed, so
// it shows up here first.
#[test]
fn derived_seed_is_pinned_for_the_init_fixture() {
    assert_eq!(
        derive_campaign_seed_from_runtime(&init_fixture(), &[], &[]),
        12_718_367_860_779_055_136
    );
}

#[test]
fn derived_seed_changes_with_a_single_field() {
    let game_data = init_fixture();
    let mut changed = game_data.clone();
    changed.fleets[0].rules_of_engagement += 1;

    assert_ne!(
        derive_campaign_seed_from_runtime(&game_data, &[], &[]),
        derive_campaign_seed_from_runtime(&changed, &[], &[])
    );
}

// Crippled hulls are part of the state, but a fleet with none adds nothing
// (the pinned seed above), and equal counts in two classes hash apart.
#[test]
fn derived_seed_sees_crippled_hulls_by_class() {
    let game_data = init_fixture();
    let seed = |game_data: &CoreGameData| derive_campaign_seed_from_runtime(game_data, &[], &[]);
    let mut destroyers = game_data.clone();
    destroyers.fleets[0].crippled.destroyers.fresh = 1;
    let mut cruisers = game_data.clone();
    cruisers.fleets[0].crippled.cruisers.fresh = 1;
    let mut later = destroyers.clone();
    later.fleets[0].crippled.destroyers.fresh = 0;
    later.fleets[0].crippled.destroyers.turns[1] = 1;

    let seeds = [
        seed(&game_data),
        seed(&destroyers),
        seed(&cruisers),
        seed(&later),
    ];
    for (i, a) in seeds.iter().enumerate() {
        for b in &seeds[i + 1..] {
            assert_ne!(a, b);
        }
    }
}

// Sighting memory is part of the state, but none adds nothing (the pinned
// seed above), and the viewer, the place, the layer, and the opponent each
// move the seed.
#[test]
fn derived_seed_sees_sighting_memory() {
    let game_data = init_fixture();
    let seed = |game_data: &CoreGameData| derive_campaign_seed_from_runtime(game_data, &[], &[]);
    let remembering = |viewer, sector: [u8; 2], layer, opponent| {
        let mut changed = game_data.clone();
        changed
            .sightings
            .note(viewer, Coords::from_array(sector), layer, opponent);
        changed
    };

    let seeds = [
        seed(&game_data),
        seed(&remembering(1, [4, 5], 1, 2)),
        seed(&remembering(2, [4, 5], 1, 1)),
        seed(&remembering(1, [5, 4], 1, 2)),
        seed(&remembering(1, [4, 5], 0, 2)),
        seed(&remembering(1, [4, 5], 1, 3)),
    ];
    for (i, a) in seeds.iter().enumerate() {
        for b in &seeds[i + 1..] {
            assert_ne!(a, b);
        }
    }

    let mut forgotten = remembering(1, [4, 5], 1, 2);
    forgotten.sightings.forget(1, Coords::from_array([4, 5]));
    assert_eq!(seed(&forgotten), seed(&game_data));
}

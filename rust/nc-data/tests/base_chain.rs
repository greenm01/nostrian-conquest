//! Each empire's starbases form a chain sorted by id: PLAYER.DAT 0x44 names
//! the first record and 0x46 the last, and each record links its
//! neighbours. ECGAME's Starbase Command Move cancels the builds that rely
//! on the moved base. See docs/dev/ec-starbase-spec.md.

use nc_compat::load_classic_core_files;
use nc_data::{
    Coords, CoreGameData, ProductionItemKind, Real48, Starbase, present_production_points,
    set_starbase_at_rest,
};
use std::path::Path;

fn starbase_fixture() -> CoreGameData {
    let dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/ecmaint-starbase-pre/v1.5");
    load_classic_core_files(&dir).expect("fixture loads")
}

/// A base of empire 1 at rest in orbit at `coords`, not yet chained.
fn push_base(game_data: &mut CoreGameData, coords: [u8; 2]) -> usize {
    let mut base = Starbase::default();
    base.owner.0 = 1;
    base.sector = Coords::from_array(coords);
    set_starbase_at_rest(&mut base);
    base.target = Coords::from_array(coords);
    base.target_depth = 1;
    game_data.bases.push(base);
    game_data.bases.len() - 1
}

fn chain_ids(game_data: &CoreGameData) -> Vec<u16> {
    game_data
        .base_chain(1)
        .expect("chain loads")
        .iter()
        .map(|&idx| game_data.bases[idx].base_id.0)
        .collect()
}

#[test]
fn the_fixture_chain_is_one_base_with_no_tail_word() {
    let game_data = starbase_fixture();
    assert_eq!(game_data.base_chain(1), Ok(vec![0]));
    assert_eq!(game_data.players[0].bases_first.0, 1);
    // ECGAME leaves 0x46 at 0; ECMAINT sets it to the tail after a run.
    assert_eq!(game_data.players[0].bases_last.0, 0);
    assert!(game_data.base_chain_errors().is_empty());
    assert!(game_data.unchained_base_errors().is_empty());
}

#[test]
fn a_new_base_takes_the_lowest_free_id_in_id_order() {
    let mut game_data = starbase_fixture();
    let second = push_base(&mut game_data, [4, 4]);
    assert_eq!(game_data.link_new_base(second), 2);
    let third = push_base(&mut game_data, [5, 5]);
    assert_eq!(game_data.link_new_base(third), 3);
    assert_eq!(chain_ids(&game_data), [1, 2, 3]);

    // Losing Starbase 2 frees its id; the next base fills the gap and sits
    // between 1 and 3 in the chain.
    game_data.unlink_base(second);
    game_data.bases[second] = Starbase::default();
    assert_eq!(chain_ids(&game_data), [1, 3]);
    let fourth = push_base(&mut game_data, [6, 6]);
    assert_eq!(game_data.link_new_base(fourth), 2);
    assert_eq!(chain_ids(&game_data), [1, 2, 3]);
    let fourth_base = &game_data.bases[fourth];
    assert_eq!(fourth_base.record.0, fourth as u16 + 1);
    assert_eq!(fourth_base.prev.0, 1);
    assert_eq!(fourth_base.next.0, third as u16 + 1);
    assert_eq!(game_data.players[0].bases_first.0, 1);
    assert_eq!(game_data.players[0].bases_last.0, third as u16 + 1);
}

#[test]
fn unlinking_the_head_or_tail_moves_the_player_words() {
    let mut game_data = starbase_fixture();
    let second = push_base(&mut game_data, [4, 4]);
    game_data.link_new_base(second);

    game_data.unlink_base(0);
    assert_eq!(game_data.players[0].bases_first.0, 2);
    assert_eq!(game_data.bases[second].prev.0, 0);

    game_data.unlink_base(second);
    assert_eq!(game_data.players[0].bases_first.0, 0);
    assert_eq!(game_data.players[0].bases_last.0, 0);
}

#[test]
fn commissioning_a_starbase_links_it_under_the_lowest_free_id() {
    let mut game_data = starbase_fixture();
    let planet_idx = game_data
        .planets
        .iter()
        .position(|planet| planet.sector.to_array() == [16, 13])
        .expect("home world");
    let planet = &mut game_data.planets[planet_idx];
    let slot = nc_data::stardock_slot_for_kind(9).expect("starbase slot");
    planet.stardock[slot] = 1;

    game_data
        .commission_planet_stardock_slot(1, planet_idx + 1, slot)
        .expect("commission");

    assert_eq!(chain_ids(&game_data), [1, 2]);
    assert_eq!(game_data.bases[0].next.0, 2);
    assert_eq!(game_data.players[0].bases_last.0, 2);
}

#[test]
fn chains_ecmaint_cannot_load_fail_preflight() {
    let mut game_data = starbase_fixture();
    // A one-base chain that links to itself (`0x05 = 1`) loops forever.
    game_data.bases[0].next.0 = 1;
    assert_eq!(
        game_data.base_chain_errors(),
        ["PLAYER[1] base chain loops back to BASES record 1"]
    );
    assert!(!game_data.ecmaint_structural_preflight_errors().is_empty());

    // A live base no chain reaches.
    let mut game_data = starbase_fixture();
    game_data.players[0].bases_first.0 = 0;
    assert_eq!(
        game_data.unchained_base_errors(),
        ["BASES[1] (empire 1, Starbase 1) is not in its owner's chain"]
    );
}

/// The starbase fixture's home world with one cruiser in its build queue.
fn home_with_queue(production_exceeded: bool) -> (CoreGameData, usize) {
    let mut game_data = starbase_fixture();
    let planet_idx = game_data
        .planets
        .iter()
        .position(|planet| planet.sector.to_array() == [16, 13])
        .expect("home world");
    let production = present_production_points(&game_data.planets[planet_idx]);
    let planet = &mut game_data.planets[planet_idx];
    planet.build_queue[0].kind = ProductionItemKind::Cruiser;
    // A cruiser costs 15 points.
    let count = if production_exceeded {
        u32::from(production) / 15 + 1
    } else {
        1
    };
    planet.build_queue[0].points_owed = count * 15;
    (game_data, planet_idx)
}

#[test]
fn moving_a_base_cancels_the_builds_that_rely_on_it() {
    let (mut game_data, planet_idx) = home_with_queue(true);
    game_data
        .set_starbase_destination(1, 1, [18, 13])
        .expect("move");
    let planet = &game_data.planets[planet_idx];
    assert_eq!(planet.build_queue[0].points_owed, 0);
    assert_eq!(planet.build_queue[0].kind.code(), 0);
}

#[test]
fn builds_within_the_worlds_production_survive_a_move() {
    let (mut game_data, planet_idx) = home_with_queue(false);
    game_data
        .set_starbase_destination(1, 1, [18, 13])
        .expect("move");
    assert_eq!(game_data.planets[planet_idx].build_queue[0].points_owed, 15);
}

#[test]
fn another_base_at_rest_in_orbit_keeps_the_builds() {
    let (mut game_data, planet_idx) = home_with_queue(true);
    let second = push_base(&mut game_data, [16, 13]);
    game_data.link_new_base(second);
    game_data
        .set_starbase_destination(1, 1, [18, 13])
        .expect("move");
    assert_ne!(game_data.planets[planet_idx].build_queue[0].points_owed, 0);
}

#[test]
fn a_base_not_fully_in_orbit_cancels_nothing() {
    let (mut game_data, planet_idx) = home_with_queue(true);
    game_data.bases[0].depth = Real48::from_bytes([0x80, 0, 0, 0, 0, 0x7f]);
    game_data
        .set_starbase_destination(1, 1, [18, 13])
        .expect("move");
    assert_ne!(game_data.planets[planet_idx].build_queue[0].points_owed, 0);
}

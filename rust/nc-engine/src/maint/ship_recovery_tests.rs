//! The starbase snapshot, tick by tick: who qualifies when the ticks begin,
//! and what strikes a fleet off. Inline because the watch is the engine's
//! private bookkeeping; the pass-level results are in `tests/combat/recovery.rs`.

use std::path::Path;

use nc_compat::load_classic_core_files;
use nc_data::{
    Coords, CoreGameData, CrippledHulls, Order, Real48, Starbase, set_fleet_at_rest,
    set_starbase_at_rest,
};

use super::{AssistWatch, recover_ships};

const WORLD: [u8; 2] = [15, 13];
/// Empire 2's world in the fixture sits at `WORLD`; this fleet is its own.
const FLEET: usize = 4;

fn hulls(turns_1: u32, turns_2: u32, fresh: u32) -> CrippledHulls {
    CrippledHulls {
        turns: [turns_1, turns_2],
        fresh,
    }
}

/// Empire 2 owns the world at `WORLD`, rests one fleet of five cruisers in
/// its orbit with the given crippled hulls, and keeps one starbase there.
fn scene(crippled: CrippledHulls) -> CoreGameData {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join("ecmaint-post")
        .join("v1.5");
    let mut game_data = load_classic_core_files(&dir).expect("fixture should load");
    game_data.bases.clear();
    let planet = game_data
        .planets
        .iter_mut()
        .find(|planet| planet.sector.to_array() == WORLD)
        .expect("the fixture's world");
    planet.owner.0 = 2;
    let fleet = &mut game_data.fleets[FLEET];
    fleet.owner.0 = 2;
    fleet.sector = Coords::from_array(WORLD);
    fleet.order = Order::HoldPosition;
    fleet.target = fleet.sector;
    fleet.speed = 0;
    fleet.ships.cruisers = 5;
    fleet.ships.destroyers = 0;
    fleet.ships.battleships = 0;
    fleet.ships.scouts = 0;
    fleet.ships.etacs = 0;
    nc_data::set_troop_transports(fleet, 0);
    set_fleet_at_rest(fleet);
    fleet.crippled.cruisers = crippled;
    add_base(&mut game_data, 1);
    game_data
}

fn add_base(game_data: &mut CoreGameData, id: u16) -> usize {
    let mut base = Starbase::default();
    base.base_id.0 = id;
    base.owner.0 = 2;
    base.sector = Coords::from_array(WORLD);
    base.target = base.sector;
    base.target_depth = 1;
    set_starbase_at_rest(&mut base);
    game_data.bases.push(base);
    game_data.bases.len() - 1
}

/// What the recovery step reports for the fleet: hulls recovered, and how
/// many of them came back early.
fn recovered(game_data: &mut CoreGameData, watch: &AssistWatch) -> (u32, u32) {
    let events = recover_ships(game_data, watch);
    events
        .iter()
        .map(|event| (event.hulls, event.early))
        .fold((0, 0), |sum, (hulls, early)| (sum.0 + hulls, sum.1 + early))
}

#[test]
fn only_resting_fleets_with_cripples_beside_their_own_base_qualify() {
    let game_data = scene(hulls(0, 2, 0));
    assert_eq!(AssistWatch::begin(&game_data).entries.len(), 1);

    let nothing_crippled = scene(CrippledHulls::default());
    assert!(AssistWatch::begin(&nothing_crippled).entries.is_empty());

    // Hulls crippled in the pass that is ending are not in the snapshot.
    let mut fresh = scene(hulls(0, 0, 2));
    let watch = AssistWatch::begin(&fresh);
    assert_eq!(recovered(&mut fresh, &watch), (0, 0));

    let mut deep_space = scene(hulls(0, 2, 0));
    deep_space.fleets[FLEET].depth = Real48::ZERO;
    assert!(AssistWatch::begin(&deep_space).entries.is_empty());

    let mut foreign_base = scene(hulls(0, 2, 0));
    foreign_base.bases[0].owner.0 = 3;
    assert!(AssistWatch::begin(&foreign_base).entries.is_empty());

    let mut other_world = scene(hulls(0, 2, 0));
    let planet = other_world
        .planets
        .iter_mut()
        .find(|planet| planet.sector.to_array() == WORLD)
        .expect("the world");
    planet.owner.0 = 3;
    assert!(AssistWatch::begin(&other_world).entries.is_empty());

    let mut moving_base = scene(hulls(0, 2, 0));
    moving_base.bases[0].moving = true;
    assert!(AssistWatch::begin(&moving_base).entries.is_empty());
}

#[test]
fn a_fleet_that_steps_away_and_back_is_struck_off_for_the_pass() {
    let mut game_data = scene(hulls(0, 2, 0));
    let mut watch = AssistWatch::begin(&game_data);
    watch.observe(&game_data);
    assert_eq!(watch.entries.len(), 1);

    let rest = game_data.fleets[FLEET].depth;
    game_data.fleets[FLEET].depth = Real48::ZERO;
    watch.observe(&game_data);
    assert!(watch.entries.is_empty());

    // It is back in orbit by the end, but it was not there all the time.
    game_data.fleets[FLEET].depth = rest;
    watch.observe(&game_data);
    assert_eq!(recovered(&mut game_data, &watch), (0, 0));
    assert_eq!(game_data.fleets[FLEET].crippled.cruisers, hulls(2, 0, 0));
}

#[test]
fn losing_the_world_or_every_base_strikes_a_fleet_off() {
    let mut game_data = scene(hulls(0, 2, 0));
    let mut watch = AssistWatch::begin(&game_data);
    game_data.planets.iter_mut().for_each(|planet| {
        if planet.sector.to_array() == WORLD {
            planet.owner.0 = 1;
        }
    });
    watch.observe(&game_data);
    assert!(watch.entries.is_empty());

    let mut game_data = scene(hulls(0, 2, 0));
    let second = add_base(&mut game_data, 2);
    let mut watch = AssistWatch::begin(&game_data);
    // One of two bases destroyed: the other is still the same base from the
    // start, so the fleet stays qualified.
    game_data.bases[second] = Starbase::default();
    watch.observe(&game_data);
    assert_eq!(watch.entries.len(), 1);
    // The first base moves off: none of the start's bases is left.
    game_data.bases[0].moving = true;
    watch.observe(&game_data);
    assert!(watch.entries.is_empty());
}

#[test]
fn a_base_that_arrives_during_the_pass_does_not_qualify_anyone() {
    let mut game_data = scene(hulls(0, 2, 0));
    game_data.bases.clear();
    let mut watch = AssistWatch::begin(&game_data);
    add_base(&mut game_data, 1);
    watch.observe(&game_data);
    assert_eq!(recovered(&mut game_data, &watch), (0, 0));
}

#[test]
fn losses_and_merges_narrow_the_assisted_hulls_in_order() {
    // Five cruisers rest at the base, three of them crippled with two turns
    // left. Two more crippled hulls with two turns left merge in, then
    // finishing fire takes two hulls of that kind.
    let mut game_data = scene(hulls(0, 3, 0));
    let mut watch = AssistWatch::begin(&game_data);
    game_data.fleets[FLEET].crippled.cruisers.turns[1] += 2;
    watch.observe(&game_data);
    game_data.fleets[FLEET].crippled.cruisers.take(2);
    watch.observe(&game_data);
    // Five hulls remain, and the three assisted ones are still among them
    // because the lost hulls were the unassisted ones.
    assert_eq!(watch.entries[0].assisted.cruisers, hulls(0, 3, 0));
    // Losing two more reaches the assisted hulls.
    game_data.fleets[FLEET].crippled.cruisers.take(2);
    watch.observe(&game_data);
    assert_eq!(watch.entries[0].assisted.cruisers, hulls(0, 1, 0));
    assert_eq!(recovered(&mut game_data, &watch), (1, 1));
    assert_eq!(game_data.fleets[FLEET].crippled.cruisers, hulls(0, 0, 0));
}

#[test]
fn a_loss_before_a_merge_does_not_let_the_newcomer_take_the_places() {
    // Three assisted hulls rest at the base. Two are finished, then two
    // newcomers with the same turns left merge in: the cohort is back to
    // three hulls but only one of them was assisted.
    let mut game_data = scene(hulls(0, 3, 0));
    let mut watch = AssistWatch::begin(&game_data);
    game_data.fleets[FLEET].crippled.cruisers.take(2);
    watch.observe(&game_data);
    game_data.fleets[FLEET].crippled.cruisers.turns[1] += 2;
    watch.observe(&game_data);
    assert_eq!(recovered(&mut game_data, &watch), (1, 1));
    assert_eq!(game_data.fleets[FLEET].crippled.cruisers, hulls(2, 0, 0));
}

//! Where a fleet goes when ROE makes it abort a mission at a blocked
//! world: classic's nearest friendly world (`24a3:0876`), or the fallback
//! of the decision that aborted it when its empire owns no other world.

use nc_data::{CoreGameData, MissionAbortReason, MissionOutcome, Order, Real48};

use super::support::{
    Fleet, NO_SHIPS, Ships, WORLD, add_starbase, board_with_world, ca, declared, put, years,
};

/// Empire 1's home world, next door to `WORLD`.
const HOME_PLANET_IDX: usize = 14;
/// From `WORLD`, four sectors by the Manhattan count and 4.0 as the fleet
/// flies. It comes first in PLANETS.DAT.
const BY_MANHATTAN: [u8; 2] = [11, 13];
/// Five sectors by the Manhattan count and 3.61 as the fleet flies.
const BY_TRAVEL: [u8; 2] = [12, 11];

/// `board_with_world` with empire 1's home world given to empire 3. With
/// `friendly_worlds`, empire 1 owns `BY_MANHATTAN` and `BY_TRAVEL`
/// instead; without, it owns nothing.
fn board(friendly_worlds: bool) -> CoreGameData {
    let mut game_data = board_with_world();
    let planets = &mut game_data.planets.records;
    planets[HOME_PLANET_IDX].set_owner_empire_slot_raw(3);
    if friendly_worlds {
        planets[7].set_coords_raw(BY_MANHATTAN);
        planets[7].set_owner_empire_slot_raw(1);
        assert_eq!(planets[10].coords_raw(), BY_TRAVEL);
        planets[10].set_owner_empire_slot_raw(1);
    }
    game_data
}

/// Empire 1's fleet record 0, one cruiser on Bombard `WORLD` with `roe`,
/// dives from the sector's deep space at speed 1: it is in the system from
/// tick 26 and reaches orbit, and strikes, only at the year's end.
fn bomber(game_data: &mut CoreGameData, roe: u8) {
    let fleet = Fleet::on_station(1, WORLD, false, Order::BombardWorld, ca(1))
        .heading(Order::BombardWorld, WORLD, true, 1)
        .roe(roe)
        .max_speed(1);
    put(game_data, 0, fleet);
}

/// Empire 2's starbase in orbit of `WORLD`. A bomber on ROE 1 that sights
/// it will not take it on and aborts, and the abort is reported.
fn base_blocks(friendly_worlds: bool) -> CoreGameData {
    let mut game_data = board(friendly_worlds);
    add_starbase(&mut game_data, 2, 1, WORLD, true);
    bomber(&mut game_data, 1);
    game_data
}

/// Empire 2's guard in orbit of `WORLD`, on ROE 0 like everyone else here,
/// and an empire 1 blockader on station beside it that identifies it for
/// its empire. The bomber, on ROE 0, re-checks the guard it now knows and
/// aborts without a report.
fn guard_blocks(friendly_worlds: bool) -> CoreGameData {
    let mut game_data = board(friendly_worlds);
    let on_station =
        |owner, ships| Fleet::on_station(owner, WORLD, true, Order::GuardBlockadeWorld, ships);
    put(
        &mut game_data,
        4,
        on_station(2, Ships { bb: 10, ..NO_SHIPS }),
    );
    put(&mut game_data, 1, on_station(1, ca(1)));
    bomber(&mut game_data, 0);
    game_data
}

/// The tick of fleet 0's reported abort before a blocked orbit, if any.
fn reported_abort(events: &nc_data::MaintenanceEvents) -> Option<u8> {
    events
        .mission_events
        .iter()
        .find(|event| {
            event.fleet_idx == 0
                && event.outcome == MissionOutcome::Aborted
                && event.abort_reason == Some(MissionAbortReason::OrbitBlocked)
        })
        .map(|event| event.stardate_week.expect("the abort is dated"))
}

#[test]
fn a_fleet_turned_back_by_a_base_heads_home_by_travel_distance() {
    let mut sighted = 0;
    for (game_data, events) in years(&base_blocks(true), &declared(&[]), 0..32) {
        let Some(tick) = reported_abort(&events) else {
            continue;
        };
        // Tick 52 is the orbital gate's abort; earlier ones are the
        // bomber's own sighting of the base on its way down.
        sighted += usize::from(tick < 52);
        let fleet = &game_data.fleets.records[0];
        assert_eq!(fleet.standing_order_kind(), Order::SeekHome);
        assert_eq!(fleet.standing_order_target_coords_raw(), BY_TRAVEL);
        assert_eq!(fleet.world_target_flag_raw(), 1);
        assert_eq!(fleet.current_speed(), 1);
    }
    assert!(sighted > 0);
}

#[test]
fn a_fleet_turned_back_by_a_base_with_no_world_to_seek_stops_where_it_is() {
    let mut sighted = 0;
    for (game_data, events) in years(&base_blocks(false), &declared(&[]), 0..32) {
        if !reported_abort(&events).is_some_and(|tick| tick < 52) {
            continue;
        }
        sighted += 1;
        let fleet = &game_data.fleets.records[0];
        assert_eq!(fleet.standing_order_kind(), Order::HoldPosition);
        assert_eq!(fleet.current_speed(), 0);
        assert_eq!(fleet.current_location_coords_raw(), WORLD);
        assert!(
            fleet.depth() >= Real48::HALF && fleet.depth() < Real48::ONE,
            "it stops in the system, short of orbit"
        );
    }
    assert!(sighted > 0);
}

#[test]
fn a_fleet_turned_back_by_a_guard_heads_home_by_travel_distance() {
    let mut rechecked = 0;
    for (game_data, events) in years(&guard_blocks(true), &declared(&[]), 0..32) {
        let fleet = &game_data.fleets.records[0];
        // The orbital gate's abort is reported; the re-check's is not.
        if fleet.standing_order_kind() != Order::SeekHome || reported_abort(&events).is_some() {
            continue;
        }
        rechecked += 1;
        assert_eq!(fleet.standing_order_target_coords_raw(), BY_TRAVEL);
        assert_eq!(fleet.world_target_flag_raw(), 1);
    }
    assert!(rechecked > 0);
}

#[test]
fn a_fleet_turned_back_by_a_guard_with_no_world_to_seek_climbs_out() {
    let mut climbing = 0;
    for (game_data, _) in years(&guard_blocks(false), &declared(&[]), 0..32) {
        let fleet = &game_data.fleets.records[0];
        if fleet.standing_order_kind() != Order::MoveOnly {
            continue;
        }
        climbing += 1;
        assert_eq!(fleet.standing_order_target_coords_raw(), WORLD);
        assert_eq!(fleet.world_target_flag_raw(), 0, "out to deep space");
        assert_eq!(fleet.current_speed(), 1);
    }
    assert!(climbing > 0);
}

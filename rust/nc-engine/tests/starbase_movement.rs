//! Classic starbase movement: a base steps at speed 1, carries its escort,
//! needs a friendly world beneath it, and its guards follow it
//! (`docs/dev/ec-starbase-spec.md`). The fixtures and the probe bytes below
//! come from classic ECMAINT.

use nc_compat::classic::{
    encode_fleet, encode_player, serialize_bases_dat, serialize_fleets_dat, serialize_player_dat,
};
use nc_compat::load_classic_core_files;
use nc_data::{
    Coords, CoreGameData, DiplomacyOverride, DiplomaticRelation, FleetMergeEvent,
    LostWorldResponse, MaintenanceEvents, Mission, MissionOutcome, Order, Real48, StarbaseReport,
    set_mission_target, starship_count,
};
use nc_engine::{run_maintenance_turn, run_maintenance_turn_with_context_and_seed};
use std::path::Path;

const HOME: [u8; 2] = [16, 13];

fn load(name: &str) -> CoreGameData {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
        .join("v1.5");
    load_classic_core_files(&dir).expect("fixture loads")
}

fn hex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).expect("hex"))
        .collect()
}

fn year(game_data: &mut CoreGameData) -> MaintenanceEvents {
    run_maintenance_turn(game_data).expect("maintenance runs")
}

/// Make fleet `idx` (empire 1's records are 0-3) a guard of Starbase 1 at
/// rest at `coords` and depth `depth`.
fn guard(game_data: &mut CoreGameData, idx: usize, coords: [u8; 2], depth: Real48) {
    let fleet = &mut game_data.fleets[idx];
    fleet.sector = Coords::from_array(coords);
    fleet.x_fraction = Real48::HALF;
    fleet.y_fraction = Real48::HALF;
    fleet.depth = depth;
    fleet.order = Order::GuardStarbase;
    fleet.target = Coords::from_array(coords);
    set_mission_target(fleet, u8::from(depth.round() == 1), 1);
    fleet.speed = 0;
}

fn guard_merges(events: &MaintenanceEvents) -> Vec<&FleetMergeEvent> {
    events
        .fleet_merge_events
        .iter()
        .filter(|event| event.kind == Mission::GuardStarbase)
        .collect()
}

#[test]
fn a_base_ordered_to_deep_space_climbs_out_with_its_escort() {
    let mut game_data = load("ecmaint-starbase-move-pre");
    let events = year(&mut game_data);
    let expected = load("ecmaint-starbase-move-post");
    assert_eq!(
        serialize_bases_dat(&game_data.bases),
        serialize_bases_dat(&expected.bases)
    );
    assert_eq!(
        serialize_fleets_dat(&game_data.fleets),
        serialize_fleets_dat(&expected.fleets)
    );
    assert_eq!(
        serialize_player_dat(&game_data.players),
        serialize_player_dat(&expected.players)
    );
    assert!(events.starbase_report_events.is_empty());
}

/// Classic's next two years of the move probe: BASES.DAT, fleet 1, and
/// PLAYER.DAT 0x40..0x48. The base covers one sector a year, stops just
/// short of the sector centre, and arrives on entering (18,13).
#[test]
fn the_moving_base_matches_classic_year_by_year() {
    let classic = [
        (
            "0100010001000001000000110d7f00fbffff7f80000000000000000000000001120d00",
            "0100010200010000000300110d7f00fbffff7f80000000000000000000000004110d0001000600000100000000000000010000000000",
        ),
        (
            "0100010001000001000000120d7f00f6ffff7f80000000000000000000000000120d00",
            "0100010200010000000300120d7f00f6ffff7f80000000000000000000000004120d0001000600000100000000000000010000000000",
        ),
    ];
    let mut game_data = load("ecmaint-starbase-move-pre");
    year(&mut game_data);
    for (index, (base, escort)) in classic.iter().enumerate() {
        let events = year(&mut game_data);
        let label = format!("year {}", index + 2);
        assert_eq!(
            serialize_bases_dat(&game_data.bases),
            hex(base),
            "{label} BASES.DAT"
        );
        assert_eq!(
            &encode_fleet(&game_data.fleets[0])[..],
            &hex(escort)[..],
            "{label} fleet 1"
        );
        assert_eq!(
            encode_player(&game_data.players[0])[0x40..0x48],
            hex("0100040001000100")[..],
            "{label} PLAYER.DAT"
        );
        let arrived = events.starbase_report_events.iter().any(|event| {
            event.report == StarbaseReport::Arrived { orbit: false } && event.coords == [18, 13]
        });
        assert_eq!(arrived, index == 1, "{label} arrival report");
    }
}

/// The planet under the base changed hands. Classic lifts the base out of
/// orbit and carries its escort. Rust also has empire 1, now without a
/// world, lose fleet 4 to civil disorder (a Rust rule classic does not
/// share), so only the base, its escort, and the chain words are compared.
#[test]
fn a_base_whose_world_changes_hands_lifts_off_with_its_escort() {
    let mut game_data = load("ecmaint-starbase-lift-pre");
    let events = year(&mut game_data);
    let expected = load("ecmaint-starbase-lift-post");
    assert_eq!(
        serialize_bases_dat(&game_data.bases),
        serialize_bases_dat(&expected.bases)
    );
    assert_eq!(game_data.fleets[0], expected.fleets[0]);
    assert_eq!(
        encode_player(&game_data.players[0])[0x44..0x48],
        encode_player(&expected.players[0])[0x44..0x48]
    );

    let reports: Vec<_> = events
        .starbase_report_events
        .iter()
        .map(|event| (event.stardate_week, event.report))
        .collect();
    assert_eq!(
        reports,
        [
            (
                Some(1),
                StarbaseReport::LostWorld {
                    planet_idx: Some(14),
                    system: HOME,
                    response: LostWorldResponse::Leaving,
                }
            ),
            (Some(52), StarbaseReport::Arrived { orbit: false }),
        ]
    );
}

#[test]
fn a_base_bound_for_a_lost_world_holds_where_it_is() {
    let mut game_data = load("ecmaint-starbase-pre");
    let target = [4, 13];
    let base = &mut game_data.bases[0];
    base.moving = true;
    base.target = Coords::from_array(target);
    base.target_depth = 1;

    let events = year(&mut game_data);

    // It climbed one tick, then held: depth snapped back to its layer.
    let base = &game_data.bases[0];
    assert_eq!(base.sector.to_array(), HOME);
    assert_eq!(base.target.to_array(), HOME);
    assert_eq!(base.target_depth, 1);
    assert_eq!(base.depth, Real48::ONE);
    assert!(!base.moving);
    assert!(matches!(
        events.starbase_report_events[..],
        [nc_data::StarbaseReportEvent {
            stardate_week: Some(1),
            report: StarbaseReport::LostWorld {
                system: [4, 13],
                response: LostWorldResponse::Holding,
                ..
            },
            ..
        }]
    ));
    // The escort rode the first tick's climb and stays there.
    let escort = &game_data.fleets[0];
    assert!(escort.depth < Real48::ONE);
    assert_eq!(escort.target_depth, 1);
}

#[test]
fn guards_elsewhere_follow_the_base_to_its_new_layer() {
    let mut game_data = load("ecmaint-starbase-move-pre");
    // Far enough away not to reach the base this year.
    guard(&mut game_data, 2, [2, 13], Real48::ZERO);
    set_mission_target(&mut game_data.fleets[2], 1, 1);

    year(&mut game_data);

    let fleet = &game_data.fleets[2];
    assert_eq!(fleet.order, Order::GuardStarbase);
    assert_eq!(fleet.target.to_array(), HOME);
    assert_eq!(fleet.target_depth, 0, "the base's new layer");
    assert!(fleet.speed > 0);
}

#[test]
fn a_guard_waiting_where_the_base_arrives_joins_its_escort() {
    let mut game_data = load("ecmaint-starbase-move-pre");
    // Fleet 2 waits in deep space in the base's sector.
    guard(&mut game_data, 1, HOME, Real48::ZERO);
    let escort_ships = starship_count(&game_data.fleets[0]);
    let waiting_ships = starship_count(&game_data.fleets[1]);

    let events = year(&mut game_data);

    let escort = &game_data.fleets[0];
    assert_eq!(escort.fleet_number.0, 1);
    assert_eq!(starship_count(escort), escort_ships + waiting_ships);
    assert_eq!(
        game_data
            .fleets
            .iter()
            .filter(|fleet| fleet.owner.0 == 1)
            .count(),
        3
    );
    // The merge as the base passes is silent.
    assert!(guard_merges(&events).is_empty());
}

#[test]
fn an_arriving_guard_merges_into_the_escort() {
    let mut game_data = load("ecmaint-starbase-move-pre");
    // Fleet 2 waits in orbit; the base's climb retargets it to deep space,
    // and it arrives when it climbs into the base's layer.
    guard(&mut game_data, 1, HOME, Real48::ONE);
    let ships = starship_count(&game_data.fleets[0]) + starship_count(&game_data.fleets[1]);

    let events = year(&mut game_data);

    assert_eq!(starship_count(&game_data.fleets[0]), ships);
    let merges = guard_merges(&events);
    assert_eq!(merges.len(), 1);
    assert_eq!(merges[0].absorbed_fleet_number, 2);
    assert_eq!(merges[0].host_fleet_number, 1);
}

#[test]
fn a_guard_that_would_pass_3000_ships_gives_up() {
    let mut game_data = load("ecmaint-starbase-move-pre");
    game_data.fleets[0].ships.destroyers = 2999;
    guard(&mut game_data, 1, HOME, Real48::ONE);

    let events = year(&mut game_data);

    let fleet = &game_data.fleets[1];
    assert_eq!(fleet.order, Order::HoldPosition);
    assert_eq!(fleet.speed, 0);
    assert!(events.mission_events.iter().any(|event| {
        event.fleet_idx == 1
            && event.kind == Mission::GuardStarbase
            && event.outcome == MissionOutcome::Failed
    }));
    assert!(guard_merges(&events).is_empty());
}

#[test]
fn an_arriving_guard_with_no_partner_becomes_the_escort() {
    let mut game_data = load("ecmaint-starbase-lift-post");
    // Isolate guard movement: the fixture's planetless civil-disorder empire
    // would now become a rogue and disband its base before movement.
    game_data.players[0].mode = nc_data::PlayerMode::Human;
    // The base is in deep space at home. Fleet 2 comes from the next
    // sector with the world-target flag ECGAME writes for the order.
    guard(&mut game_data, 1, [15, 13], Real48::ZERO);
    let fleet = &mut game_data.fleets[1];
    fleet.target = Coords::from_array(HOME);
    set_mission_target(fleet, 1, 1);
    // Fleet 1, the old escort, leaves.
    game_data.fleets[0].order = Order::HoldPosition;

    year(&mut game_data);

    let base = &game_data.bases[0];
    let fleet = &game_data.fleets[1];
    assert_eq!(base.escort.0, fleet.fleet_number.0);
    assert_eq!(fleet.depth, base.depth);
    assert_eq!(fleet.speed, 0);
}

fn alien_patrol_at_home(game_data: &mut CoreGameData) {
    let fleet = &mut game_data.fleets[4];
    assert_eq!(fleet.owner.0, 2);
    fleet.sector = Coords::from_array(HOME);
    fleet.depth = Real48::ZERO;
    fleet.order = Order::PatrolSector;
    fleet.target = Coords::from_array(HOME);
    set_mission_target(fleet, 0, 0);
    fleet.speed = 0;
    fleet.rules_of_engagement = 10;
}

fn empire_2_declares_empire_1() -> Vec<DiplomacyOverride> {
    vec![DiplomacyOverride {
        from_empire: 2,
        to_empire: 1,
        relation: DiplomaticRelation::Enemy,
    }]
}

/// A deep-space patrol can only meet the base, and its escort, once they
/// leave orbit: the unit table follows the base's layer.
#[test]
fn a_deep_space_patrol_meets_a_base_only_after_it_leaves_orbit() {
    let diplomacy = empire_2_declares_empire_1();
    let mut weeks = Vec::new();
    for (scenario, lifts) in [
        ("ecmaint-starbase-lift-pre", true),
        ("ecmaint-starbase-pre", false),
    ] {
        for seed in 0..20 {
            let mut game_data = load(scenario);
            alien_patrol_at_home(&mut game_data);
            let events =
                run_maintenance_turn_with_context_and_seed(&mut game_data, seed, &[], &diplomacy)
                    .expect("maintenance runs");
            let battles: Vec<u8> = events
                .fleet_battle_events
                .iter()
                .filter(|event| event.coords == HOME)
                .filter_map(|event| event.stardate_week)
                .collect();
            if lifts {
                weeks.extend(battles);
            } else {
                assert!(battles.is_empty(), "seed {seed}: the base stayed in orbit");
            }
        }
    }
    assert!(!weeks.is_empty(), "the patrol never met the lifting base");
    // The base leaves the system layer on tick 27 (depth below 0.5).
    assert!(weeks.iter().all(|week| *week >= 27), "{weeks:?}");
}

#[test]
fn a_base_under_way_does_not_scan() {
    let mut game_data = load("ecmaint-starbase-move-pre");
    // An alien fleet holds in orbit at home all year.
    let fleet = &mut game_data.fleets[4];
    fleet.sector = Coords::from_array(HOME);
    fleet.order = Order::HoldPosition;
    fleet.target = Coords::from_array(HOME);
    for seed in 0..10 {
        let mut game_data = game_data.clone();
        let events = run_maintenance_turn_with_context_and_seed(&mut game_data, seed, &[], &[])
            .expect("maintenance runs");
        assert!(
            !events
                .scout_contact_events
                .iter()
                .any(|event| matches!(event.source, nc_data::ContactReportSource::Starbase { .. }))
        );
    }
}

fn guard_arrivals(events: &MaintenanceEvents) -> usize {
    events
        .mission_events
        .iter()
        .filter(|event| {
            event.kind == Mission::GuardStarbase && event.outcome == MissionOutcome::Arrived
        })
        .count()
}

/// Classic keeps the `0x22 = 1` ECGAME writes, so a guard of a deep-space
/// base re-arrives every tick. Rust gives the escort its base's layer on
/// arrival, and it arrives once (t040).
#[test]
fn a_guard_arrives_at_its_base_once() {
    let mut game_data = load("ecmaint-starbase-lift-post");
    // Isolate guard movement: the fixture's planetless civil-disorder empire
    // would now become a rogue and disband its base before movement.
    game_data.players[0].mode = nc_data::PlayerMode::Human;
    guard(&mut game_data, 1, [15, 13], Real48::ZERO);
    let fleet = &mut game_data.fleets[1];
    fleet.target = Coords::from_array(HOME);
    set_mission_target(fleet, 1, 1);
    game_data.fleets[0].order = Order::HoldPosition;

    assert_eq!(guard_arrivals(&year(&mut game_data)), 1);
    let fleet = &game_data.fleets[1];
    assert_eq!(fleet.target_depth, 0, "the base's layer");
    assert_eq!(guard_arrivals(&year(&mut game_data)), 0);
}

/// A guard at rest in orbit under its deep-space base never moves in
/// classic. Rust climbs it to the base, where it merges into the escort
/// (t040).
#[test]
fn a_guard_in_orbit_climbs_to_its_deep_space_base() {
    let mut game_data = load("ecmaint-starbase-lift-post");
    // Isolate guard movement: the fixture's planetless civil-disorder empire
    // would now become a rogue and disband its base before movement.
    game_data.players[0].mode = nc_data::PlayerMode::Human;
    guard(&mut game_data, 1, HOME, Real48::ONE);
    let ships = starship_count(&game_data.fleets[0]) + starship_count(&game_data.fleets[1]);

    let events = year(&mut game_data);

    assert_eq!(starship_count(&game_data.fleets[0]), ships);
    let merges = guard_merges(&events);
    assert_eq!(merges.len(), 1);
    assert_eq!(merges[0].absorbed_fleet_number, 2);
    assert_eq!(merges[0].host_fleet_number, 1);
}

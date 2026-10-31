//! Planetary assault: when a fleet strikes, and what each planetary hit
//! costs.

use nc_data::{
    Coords, CoreGameData, CrippledHulls, MaintenanceEvents, Mission, MissionAbortReason,
    MissionOutcome, Order, Real48, has_any_force, set_fleet_at_rest,
};
use nc_engine::{
    run_maintenance_turn, run_maintenance_turn_with_context,
    run_maintenance_turn_with_context_and_seed,
};

use super::support::{
    SYSTEM, SYSTEM_PLANET_IDX, Ships, diplomacy, empty_board, mark_arrived, place_fleet,
    place_starbase,
};

/// Empire 1 fleet record 0 sits in orbit of `SYSTEM` (owned by empire 2) on
/// an assault order that executes this turn.
fn assault_on_system(order: Order, ships: Ships, armies: u8, batteries: u8) -> CoreGameData {
    let mut game_data = empty_board(armies, batteries);
    place_fleet(&mut game_data, 0, 1, SYSTEM, order, SYSTEM, ships, 10);
    mark_arrived(&mut game_data, 0);
    game_data
}

/// The tick a fleet diving from deep space at speed 3 reaches orbit.
const DIVE_TICK: u8 = 18;

/// Fleet record `idx` of `owner` in deep space over `SYSTEM`, diving toward
/// orbit on `order` at speed 3: it shares the system layer from tick 9 and
/// reaches depth 1.0 on `DIVE_TICK`.
fn diving_on_system(
    game_data: &mut CoreGameData,
    idx: usize,
    owner: u8,
    order: Order,
    ships: Ships,
    roe: u8,
) {
    place_fleet(game_data, idx, owner, SYSTEM, order, SYSTEM, ships, roe);
    let fleet = &mut game_data.fleets[idx];
    fleet.target_depth = 1;
    set_fleet_at_rest(fleet);
    fleet.depth = Real48::ZERO;
    fleet.max_speed = 3;
    fleet.speed = 3;
}

/// Fleet record `idx` of `owner` at rest in orbit of `SYSTEM` on Hold with
/// ROE 0: it never starts a fight.
fn holding_orbit(game_data: &mut CoreGameData, idx: usize, owner: u8, ships: Ships) {
    place_fleet(
        game_data,
        idx,
        owner,
        SYSTEM,
        Order::HoldPosition,
        SYSTEM,
        ships,
        0,
    );
    set_fleet_at_rest(&mut game_data.fleets[idx]);
}

/// Empire 1's only world in the fixture, at (16,13), next door to `SYSTEM`.
const HOME_PLANET_IDX: usize = 14;
/// Where `far_home` moves it: a fleet seeking it from `SYSTEM` is still
/// under way at year end.
const HOME: [u8; 2] = [3, 13];

fn far_home(game_data: &mut CoreGameData) {
    game_data.planets[HOME_PLANET_IDX].sector = Coords::from_array(HOME);
}

fn cruiser() -> Ships {
    Ships {
        ca: 1,
        ..Ships::default()
    }
}

fn four_battleships() -> Ships {
    Ships {
        bb: 4,
        ..Ships::default()
    }
}

/// A year among neutral empires.
fn run_neutral(game_data: &mut CoreGameData, seed: u64) -> MaintenanceEvents {
    run_maintenance_turn_with_context_and_seed(game_data, seed, &[], &diplomacy(&[]))
        .expect("maintenance should succeed")
}

/// Fleet 0's mission was aborted at the orbit it could not get past.
fn repelled(events: &MaintenanceEvents, kind: Mission) -> bool {
    events.mission_events.iter().any(|event| {
        event.fleet_idx == 0
            && event.kind == kind
            && event.outcome == MissionOutcome::Aborted
            && matches!(
                event.abort_reason,
                Some(MissionAbortReason::OrbitBlocked | MissionAbortReason::RoeDeclined(_))
            )
    })
}

#[test]
fn one_cruiser_cannot_clear_ten_batteries_in_one_round() {
    let mut game_data = assault_on_system(Order::BombardWorld, cruiser(), 0, 10);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let bombard = &events.bombard_events[0];
    assert_eq!(bombard.defender_batteries_initial, 10);
    assert!(
        bombard.defender_battery_losses <= 1,
        "each battery costs 20 hits; a lone cruiser deals at most 31 per round: {}",
        bombard.defender_battery_losses
    );
}

#[test]
fn stardock_ships_absorb_their_own_defense_strength() {
    let mut game_data = assault_on_system(Order::BombardWorld, cruiser(), 0, 0);
    let planet = &mut game_data.planets[SYSTEM_PLANET_IDX];
    planet.stardock.battleships = 1;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let bombard = &events.bombard_events[0];
    assert_eq!(
        bombard.stardock_items_destroyed, 0,
        "a docked battleship (DS 100) outlasts a lone cruiser's per-round hits"
    );
    assert_eq!(game_data.planets[SYSTEM_PLANET_IDX].stardock.battleships, 1);
}

#[test]
fn breakthrough_hits_reach_industry_on_the_ten_times_scale() {
    let mut game_data = assault_on_system(
        Order::BombardWorld,
        Ships {
            dd: 1,
            ..Ships::default()
        },
        0,
        0,
    );
    let planet = &mut game_data.planets[SYSTEM_PLANET_IDX];
    planet.production = Real48::from_f64(100.0);
    planet.stored_goods = Real48::from_f64(10000.0);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let bombard = &events.bombard_events[0];
    assert!(bombard.breakthrough);
    assert!(
        bombard.factories_destroyed <= 9,
        "a destroyer deals at most 30 hits a round, nine factories a year on the 10x scale: {}",
        bombard.factories_destroyed
    );
    assert!(bombard.stored_goods_destroyed <= 225);
}

#[test]
fn invasion_softening_compares_firepower_against_army_strength() {
    let mut game_data = assault_on_system(
        Order::InvadeWorld,
        Ships {
            ca: 1,
            tt: 40,
            armies: 40,
            ..Ships::default()
        },
        20,
        0,
    );

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let assault = &events.assault_report_events[0];
    assert_eq!(assault.kind, Mission::InvadeWorld);
    assert!(
        assault.defender_army_losses_softening <= 3,
        "one cruiser (bombard AS 30) against 20 armies (AS 200) is disadvantaged \
         and each army costs 10 hits: {}",
        assault.defender_army_losses_softening
    );
}

#[test]
fn bombard_fleet_is_not_folded_into_a_same_turn_invasion() {
    let mut game_data = assault_on_system(
        Order::InvadeWorld,
        Ships {
            ca: 4,
            tt: 10,
            armies: 10,
            ..Ships::default()
        },
        1,
        0,
    );
    place_fleet(
        &mut game_data,
        1,
        1,
        SYSTEM,
        Order::BombardWorld,
        SYSTEM,
        Ships {
            dd: 2,
            tt: 3,
            armies: 3,
            ..Ships::default()
        },
        10,
    );
    mark_arrived(&mut game_data, 1);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let invasion = &events.assault_report_events[0];
    assert_eq!(invasion.kind, Mission::InvadeWorld);
    assert_eq!(
        invasion.attacker_loaded_armies_initial, 10,
        "only the invading fleet lands its armies"
    );
    let bombard_fleet = &game_data.fleets[1];
    assert_eq!(bombard_fleet.ships.loaded_transports, 3);
    // Both strike on tick 1 in the year's unit order: the bomber either
    // bombarded first or found the world already its own. Once the world is
    // its own it guards it, as classic does; either way it keeps its armies,
    // in orbit.
    assert_eq!(game_data.planets[SYSTEM_PLANET_IDX].owner.0, 1);
    assert_eq!(bombard_fleet.order, Order::GuardBlockadeWorld);
    assert_eq!(bombard_fleet.depth, nc_data::Real48::ONE);
    assert_eq!(game_data.fleets[0].ships.loaded_transports, 0);
}

#[test]
fn a_bombard_fleet_in_orbit_at_the_start_of_the_year_strikes_on_tick_one() {
    let mut game_data = assault_on_system(Order::BombardWorld, cruiser(), 0, 0);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(events.bombard_events[0].stardate_week, Some(1));
    let mission = events
        .mission_events
        .iter()
        .find(|event| {
            event.kind == Mission::BombardWorld && event.outcome == MissionOutcome::Succeeded
        })
        .expect("the bombardment is reported");
    assert_eq!(mission.stardate_week, Some(1));
}

#[test]
fn a_bombardment_resolves_in_its_tick_before_relief_arrives() {
    // Empire 1's destroyer bombards from orbit on tick 1. Empire 2's relief
    // squadron dives in from deep space, shares the orbit from tick 9, and
    // engages the bomber through the encounter loop, too late to spare the
    // world or to weaken the bombardment.
    let mut game_data = assault_on_system(
        Order::BombardWorld,
        Ships {
            dd: 1,
            ..Ships::default()
        },
        0,
        0,
    );
    diving_on_system(
        &mut game_data,
        4,
        2,
        Order::MoveOnly,
        Ships {
            bb: 5,
            ..Ships::default()
        },
        10,
    );

    let events = run_maintenance_turn_with_context(&mut game_data, &[], &diplomacy(&[(1, 2)]))
        .expect("maintenance should succeed");

    let bombard = &events.bombard_events[0];
    assert_eq!(bombard.stardate_week, Some(1));
    assert_eq!(bombard.attacker_initial.destroyers, 1);
    assert_eq!(bombard.attacker_losses.destroyers, 0);
    let battle = events
        .fleet_battle_events
        .iter()
        .find(|event| event.reporting_empire == 2)
        .expect("the relief squadron engages the bomber");
    assert!(
        battle.stardate_week > bombard.stardate_week,
        "the battle comes after the strike: {:?}",
        battle.stardate_week
    );
    let bomber = &game_data.fleets[0];
    assert!(
        !(has_any_force(bomber) && bomber.order == Order::BombardWorld),
        "five battleships end the bombardment"
    );
}

#[test]
fn assault_fleets_of_different_classes_strike_in_tick_order() {
    // Empire 1's cruiser bombards on tick 1; its transports blitz when they
    // reach orbit on tick 18. Neither waits for the other, and the blitz
    // does not go first for being a blitz.
    let mut game_data = assault_on_system(Order::BombardWorld, cruiser(), 1, 0);
    diving_on_system(
        &mut game_data,
        1,
        1,
        Order::BlitzWorld,
        Ships {
            tt: 20,
            armies: 20,
            ..Ships::default()
        },
        10,
    );

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(events.bombard_events.len(), 1);
    assert_eq!(events.bombard_events[0].stardate_week, Some(1));
    let blitz = &events.assault_report_events[0];
    assert_eq!(blitz.kind, Mission::BlitzWorld);
    assert_eq!(blitz.stardate_week, Some(DIVE_TICK));
    assert_eq!(
        events.ownership_change_events[0].stardate_week,
        Some(DIVE_TICK)
    );
    assert_eq!(game_data.planets[SYSTEM_PLANET_IDX].owner.0, 1);
    // From the tick after the capture the bomber finds the world its own
    // and guards it, as classic does.
    assert_eq!(game_data.fleets[0].order, Order::GuardBlockadeWorld);
    assert_eq!(game_data.fleets[1].order, Order::HoldPosition);
}

#[test]
fn a_world_captured_mid_year_is_defended_by_its_new_owner() {
    // Empire 1's invasion takes `SYSTEM` from empire 2 on tick 1. Empire 3's
    // cruiser reaches orbit on tick 18 and bombards empire 1's world.
    let mut game_data = assault_on_system(
        Order::InvadeWorld,
        Ships {
            dd: 1,
            tt: 20,
            armies: 20,
            ..Ships::default()
        },
        1,
        0,
    );
    game_data.fleets[0].rules_of_engagement = 0;
    // The invasion fleet holding the orbit it took is no starbase: the
    // bomber passes the gate (ADR bd8trz18).
    diving_on_system(
        &mut game_data,
        8,
        3,
        Order::BombardWorld,
        Ships {
            ca: 2,
            ..Ships::default()
        },
        10,
    );

    let events = run_maintenance_turn_with_context(&mut game_data, &[], &diplomacy(&[]))
        .expect("maintenance should succeed");

    let capture = &events.ownership_change_events[0];
    assert_eq!(capture.stardate_week, Some(1));
    assert_eq!(capture.new_owner_empire, 1);
    let bombard = &events.bombard_events[0];
    assert_eq!(bombard.stardate_week, Some(DIVE_TICK));
    assert_eq!(bombard.attacker_empire, 3);
    assert_eq!(bombard.defender_empire, 1);
    assert_eq!(game_data.planets[SYSTEM_PLANET_IDX].owner.0, 1);
}

/// Landing empties the crippled loaded transports first, each cohort moving
/// to the empty counter with its hull. Four loaded transports, two of them
/// crippled, take an undefended world: all four land, and the pass's
/// recovery step then counts the cripples down as empty transports.
#[test]
fn landing_moves_the_crippled_transports_cohorts_to_the_empty_counter() {
    let mut game_data = assault_on_system(
        Order::InvadeWorld,
        Ships {
            dd: 1,
            tt: 4,
            armies: 4,
            ..Ships::default()
        },
        0,
        0,
    );
    game_data.fleets[0].crippled.loaded_transports = CrippledHulls {
        turns: [1, 1],
        fresh: 0,
    };

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(events.ownership_change_events[0].new_owner_empire, 1);
    let fleet = &game_data.fleets[0];
    assert_eq!(
        (fleet.ships.loaded_transports, fleet.ships.empty_transports),
        (0, 4)
    );
    assert!(fleet.crippled.loaded_transports.is_empty());
    assert_eq!(
        fleet.crippled.empty_transports,
        CrippledHulls {
            turns: [1, 0],
            fresh: 0,
        }
    );
}

#[test]
fn landing_fire_pays_transport_defense_strength_on_the_ten_times_scale() {
    // Twenty loaded transports blitz a world with one battery and no garrison.
    // The battery fires 90 AS at the descending transports at a third of the
    // ratio CRT (t202: fleeting targets in the atmosphere), Overwhelming
    // column: 30..=75 hits, each transport costs 2x its DS of 10, and a
    // critical sinks one more.
    let ships = Ships {
        tt: 20,
        armies: 20,
        ..Ships::default()
    };
    let mut game_data = assault_on_system(Order::BlitzWorld, ships, 0, 1);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let blitz = &events.assault_report_events[0];
    assert_eq!(blitz.kind, Mission::BlitzWorld);
    let transport_losses = blitz.attacker_ship_losses.transports;
    assert!(
        (1..=4).contains(&transport_losses),
        "one battery should sink 1 to 4 transports, not one per battery: {transport_losses}"
    );
    assert_eq!(blitz.transport_army_losses, transport_losses);
}

// The orbital gate (t054): what the world's owner has in orbit fights an
// assault at its strike tick, sighted or not.

#[test]
fn an_unsighted_starbase_fights_an_invasion_before_it_lands() {
    // Empire 2's starbase protects `SYSTEM`. Empire 1's invasion force is
    // at rest in orbit, so its strike test passes on tick 1 after a single
    // scan with a 1.7% chance of sighting the base. The gate fights the
    // base regardless: four battleships destroy it, and the landing follows
    // in the same tick.
    let mut game_data = assault_on_system(
        Order::InvadeWorld,
        Ships {
            bb: 4,
            tt: 8,
            armies: 8,
            ..Ships::default()
        },
        1,
        0,
    );
    place_starbase(&mut game_data, 2, SYSTEM);

    for seed in 0..8u64 {
        let mut year = game_data.clone();
        let events = run_neutral(&mut year, seed);

        let battle = events
            .fleet_battle_events
            .iter()
            .find(|event| event.reporting_empire == 1)
            .unwrap_or_else(|| panic!("the base fights the invader, seed {seed}"));
        assert_eq!(battle.stardate_week, Some(1), "seed {seed}");
        assert_eq!(battle.enemy_initial_starbases, 1);
        assert!(battle.held_field);
        assert!(year.bases.iter().all(|base| base.base_id.0 == 0));
        let assault = &events.assault_report_events[0];
        assert_eq!(assault.kind, Mission::InvadeWorld);
        assert_eq!(assault.stardate_week, Some(1), "seed {seed}");
        assert_eq!(events.ownership_change_events[0].stardate_week, Some(1));
        assert_eq!(year.planets[SYSTEM_PLANET_IDX].owner.0, 1);
    }
}

#[test]
fn an_owners_fleet_in_orbit_does_not_block_the_strike() {
    // Empire 2's destroyer holds in orbit on ROE 0 and never starts a fight,
    // and empire 1's cruisers, neutral to it, ignore it. Only a starbase
    // blocks the strike (ADR bd8trz18): the bombardment goes ahead in tick 1
    // without a shot at the destroyer.
    let mut game_data = assault_on_system(
        Order::BombardWorld,
        Ships {
            ca: 3,
            ..Ships::default()
        },
        0,
        0,
    );
    holding_orbit(
        &mut game_data,
        4,
        2,
        Ships {
            dd: 1,
            ..Ships::default()
        },
    );

    let events = run_neutral(&mut game_data, 0);

    assert!(events.fleet_battle_events.is_empty());
    assert_eq!(game_data.fleets[4].ships.destroyers, 1);
    assert_eq!(events.bombard_events[0].stardate_week, Some(1));
}

#[test]
fn a_starbase_and_its_guard_fight_the_bomber_at_the_gate_and_nobody_else_does() {
    // Empire 2's base has a guard destroyer beside it and a second destroyer
    // holding in orbit. The gate pairs the bomber with the base and its
    // guard, as classic's base battle does; the holding destroyer stays out.
    let mut game_data = assault_on_system(Order::BombardWorld, four_battleships(), 0, 0);
    place_starbase(&mut game_data, 2, SYSTEM);
    for idx in [4, 5] {
        holding_orbit(
            &mut game_data,
            idx,
            2,
            Ships {
                dd: 1,
                ..Ships::default()
            },
        );
    }
    game_data.fleets[4].order = Order::GuardStarbase;
    game_data.fleets[4].mission_target = 1;

    let events = run_neutral(&mut game_data, 0);

    let gate = events
        .fleet_battle_events
        .iter()
        .find(|event| event.reporting_empire == 1 && event.stardate_week == Some(1))
        .expect("the base and its guard fight the bomber");
    assert_eq!(gate.enemy_initial_starbases, 1);
    assert_eq!(gate.enemy_initial.destroyers, 1, "the guard only");
    assert_eq!(game_data.fleets[5].ships.destroyers, 1);
}

#[test]
fn a_defender_in_deep_space_does_not_fight_at_the_strike_tick() {
    // The same destroyer in the sector's deep space is in another layer:
    // the bombardment goes ahead without a shot.
    let mut game_data = assault_on_system(
        Order::BombardWorld,
        Ships {
            ca: 3,
            ..Ships::default()
        },
        0,
        0,
    );
    holding_orbit(
        &mut game_data,
        4,
        2,
        Ships {
            dd: 1,
            ..Ships::default()
        },
    );
    game_data.fleets[4].depth = Real48::ZERO;

    let events = run_neutral(&mut game_data, 0);

    assert!(events.fleet_battle_events.is_empty());
    assert_eq!(game_data.fleets[4].ships.destroyers, 1);
    assert_eq!(events.bombard_events[0].stardate_week, Some(1));
}

#[test]
fn an_attacker_whose_roe_forbids_the_fight_seeks_home_without_a_shot() {
    // ROE 2 wants four-to-one odds; one cruiser (7) against a starbase (30)
    // has none. The bomber aborts at the gate, as it would on sighting the
    // base, and heads for home at full speed.
    let mut game_data = assault_on_system(Order::BombardWorld, cruiser(), 0, 0);
    game_data.fleets[0].rules_of_engagement = 2;
    place_starbase(&mut game_data, 2, SYSTEM);
    far_home(&mut game_data);

    let events = run_neutral(&mut game_data, 0);

    assert!(events.fleet_battle_events.is_empty());
    assert!(events.bombard_events.is_empty());
    assert!(
        repelled(&events, Mission::BombardWorld),
        "{:?}",
        events.mission_events
    );
    let fleet = &game_data.fleets[0];
    assert_eq!(fleet.ships.cruisers, 1);
    assert_eq!(fleet.order, Order::SeekHome);
    assert_eq!(fleet.target.to_array(), HOME);
    assert_eq!(fleet.target_depth, 1);
}

#[test]
fn the_gate_weighs_a_crippled_base_at_half_and_a_crippled_attacker_at_half() {
    // ROE 6 wants even odds. Three cruisers (90) fall short of a base (100)
    // but not of a crippled one (50); four (120) pass the whole base, but
    // with two of them crippled (90) they do not.
    let gate = |cruisers: u16, attacker_cripples: u32, base_crippled: bool| {
        let mut game_data = assault_on_system(
            Order::BombardWorld,
            Ships {
                ca: cruisers,
                ..Ships::default()
            },
            0,
            0,
        );
        game_data.fleets[0].rules_of_engagement = 6;
        game_data.fleets[0].crippled.cruisers.turns[1] = attacker_cripples;
        place_starbase(&mut game_data, 2, SYSTEM);
        if base_crippled {
            game_data.bases[0].crippled.turns[1] = 1;
        }
        far_home(&mut game_data);
        let events = run_neutral(&mut game_data, 0);
        let refused =
            repelled(&events, Mission::BombardWorld) && events.fleet_battle_events.is_empty();
        let fought = !events.fleet_battle_events.is_empty();
        assert_ne!(refused, fought, "the attacker either fights or turns back");
        fought
    };
    assert!(!gate(3, 0, false), "90 against 100");
    assert!(gate(3, 0, true), "90 against 50");
    assert!(gate(4, 0, false), "120 against 100");
    assert!(!gate(4, 2, false), "90 against 100");
}

#[test]
fn a_beaten_attacker_retreats_repelled_by_the_orbital_defence() {
    // The base and the destroyer beside it kill the bomber's only warship in
    // round 1 and spend the battle's 64 rounds on its scouts. NC's battle
    // leaves a beaten side alive only at that cap. The survivors seek home
    // and the report says the orbital defence repelled them.
    let mut game_data = assault_on_system(
        Order::BombardWorld,
        Ships {
            dd: 1,
            sc: 1200,
            ..Ships::default()
        },
        0,
        0,
    );
    place_starbase(&mut game_data, 2, SYSTEM);
    holding_orbit(
        &mut game_data,
        4,
        2,
        Ships {
            dd: 1,
            ..Ships::default()
        },
    );
    far_home(&mut game_data);

    for seed in 0..4u64 {
        let mut year = game_data.clone();
        let events = run_neutral(&mut year, seed);

        let battle = events
            .fleet_battle_events
            .iter()
            .find(|event| event.reporting_empire == 1)
            .unwrap_or_else(|| panic!("the orbit's defenders fight the bomber, seed {seed}"));
        assert_eq!(battle.stardate_week, Some(1), "seed {seed}");
        assert!(!battle.held_field, "seed {seed}");
        assert!(events.bombard_events.is_empty(), "seed {seed}");
        assert!(repelled(&events, Mission::BombardWorld), "seed {seed}");
        let fleet = &year.fleets[0];
        assert_eq!(fleet.ships.destroyers, 0, "seed {seed}");
        assert!(fleet.ships.scouts > 0, "seed {seed}");
        assert_eq!(fleet.order, Order::SeekHome, "seed {seed}");
        assert_eq!(fleet.target.to_array(), HOME);
    }
}

#[test]
fn a_transport_only_blitz_passes_any_orbit_but_a_starbases() {
    // Loaded transports alone can fight nobody. They land past the owner's
    // fleets in orbit, warships or not, since only a starbase blocks the
    // strike (ADR bd8trz18); his starbase turns them for home.
    let loaded = Ships {
        tt: 10,
        armies: 10,
        ..Ships::default()
    };
    let scouts = Ships {
        sc: 2,
        ..Ships::default()
    };
    let destroyer = Ships {
        dd: 1,
        ..Ships::default()
    };
    let blitz = || {
        let mut game_data = assault_on_system(Order::BlitzWorld, loaded, 1, 0);
        game_data.fleets[0].rules_of_engagement = 0;
        far_home(&mut game_data);
        game_data
    };

    for (name, mut game_data) in [
        ("empty orbit", blitz()),
        ("scouts in orbit", {
            let mut game_data = blitz();
            holding_orbit(&mut game_data, 4, 2, scouts);
            game_data
        }),
        ("destroyer in orbit", {
            let mut game_data = blitz();
            holding_orbit(&mut game_data, 4, 2, destroyer);
            game_data
        }),
    ] {
        let events = run_neutral(&mut game_data, 0);
        assert!(events.fleet_battle_events.is_empty(), "{name}");
        assert_eq!(
            events.assault_report_events[0].kind,
            Mission::BlitzWorld,
            "{name}"
        );
        assert_eq!(
            events.assault_report_events[0].stardate_week,
            Some(1),
            "{name}"
        );
        assert_eq!(game_data.planets[SYSTEM_PLANET_IDX].owner.0, 1, "{name}");
    }

    for (name, mut game_data) in [("starbase in orbit", {
        let mut game_data = blitz();
        place_starbase(&mut game_data, 2, SYSTEM);
        game_data
    })] {
        let events = run_neutral(&mut game_data, 0);
        assert!(events.fleet_battle_events.is_empty(), "{name}");
        assert!(events.assault_report_events.is_empty(), "{name}");
        assert!(
            repelled(&events, Mission::BlitzWorld),
            "{name}: {:?}",
            events.mission_events
        );
        let fleet = &game_data.fleets[0];
        assert_eq!(
            fleet.ships.loaded_transports, 10,
            "{name}: the armies stay aboard"
        );
        assert_eq!(fleet.order, Order::SeekHome, "{name}");
        assert_eq!(fleet.target.to_array(), HOME, "{name}");
        assert_eq!(game_data.planets[SYSTEM_PLANET_IDX].owner.0, 2, "{name}");
    }
}

#[test]
fn an_empires_bombarding_fleets_strike_together_after_one_gate_battle() {
    // Two bombardment fleets of empire 1 are at rest in orbit over the
    // starbase's world. Whichever the year's unit order runs first destroys
    // the base at its gate, then bombards with the other as one force
    // (t202), in the same tick; the other does not strike again.
    let mut game_data = assault_on_system(Order::BombardWorld, four_battleships(), 0, 0);
    place_fleet(
        &mut game_data,
        1,
        1,
        SYSTEM,
        Order::BombardWorld,
        SYSTEM,
        four_battleships(),
        10,
    );
    mark_arrived(&mut game_data, 1);
    place_starbase(&mut game_data, 2, SYSTEM);

    for seed in 0..4u64 {
        let mut year = game_data.clone();
        let events = run_neutral(&mut year, seed);

        assert_eq!(
            events
                .fleet_battle_events
                .iter()
                .filter(|event| event.reporting_empire == 1)
                .count(),
            1,
            "one battle with the base, seed {seed}"
        );
        assert_eq!(events.bombard_events.len(), 1, "seed {seed}");
        assert_eq!(
            events.bombard_events[0].attacker_initial.battleships, 8,
            "both fleets bombard, seed {seed}"
        );
        assert!(
            events
                .bombard_events
                .iter()
                .all(|event| event.stardate_week == Some(1))
        );
        assert!(year.bases.iter().all(|base| base.base_id.0 == 0));
    }
}

#[test]
fn a_bombardment_with_more_starbases_than_fleets_finds_its_companions() {
    // The joint-bombardment scan walks every unit; a starbase unit's index
    // is a base record, which can lie past the last fleet record (t202).
    let mut game_data = assault_on_system(Order::BombardWorld, four_battleships(), 0, 2);
    let home = game_data
        .planets
        .iter()
        .find(|planet| planet.owner.0 == 1)
        .expect("empire 1 has a world")
        .sector
        .to_array();
    while game_data.bases.len() <= game_data.fleets.len() {
        place_starbase(&mut game_data, 1, home);
        let id = game_data.bases.len() as u16;
        let base = game_data.bases.last_mut().expect("just placed");
        base.base_id.0 = id;
        base.record.0 = id;
    }
    let events = run_neutral(&mut game_data, 0);
    assert_eq!(events.bombard_events.len(), 1);
}

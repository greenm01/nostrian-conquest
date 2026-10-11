//! What a captured world keeps.
//!
//! Inline rather than in tests/ so a strike runs by itself, without the
//! yearly economy pass that would change the numbers before the fleet
//! arrives. Every capture here is certain: the defenders have no armies.

use std::path::Path;

use nc_compat::load_classic_core_files;
use nc_data::{
    BuildSlot, Coords, CoreGameData, EmpireId, EmpireUnitSummary, InheritedAssets, MissionOutcome,
    Order, Planet, ProductionItemKind, Real48, set_armies_aboard, set_fleet_at_rest,
    set_troop_transports,
};

use super::assault::{AssaultEvents, resolve_strike};

const SYSTEM: [u8; 2] = [15, 13];
const SYSTEM_PLANET_IDX: usize = 13;

/// A world of empire 2 as `world` describes it, and empire 1's fleet record
/// 0 in orbit of it on `order` with a destroyer escort and `landing` loaded
/// transports.
fn strike_board(order: Order, landing: u16, world: Planet) -> CoreGameData {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join("ecmaint-post")
        .join("v1.5");
    let mut game_data = load_classic_core_files(&dir).expect("fixture should load");
    game_data.bases.clear();
    game_data.planets[SYSTEM_PLANET_IDX] = Planet {
        sector: Coords::from_array(SYSTEM),
        name: "TargetPrime".to_string(),
        owner: EmpireId(2),
        years_under_owner: 2,
        ..world
    };
    let fleet = &mut game_data.fleets[0];
    fleet.owner.0 = 1;
    fleet.sector = Coords::from_array(SYSTEM);
    fleet.order = order;
    fleet.target = Coords::from_array(SYSTEM);
    fleet.speed = 3;
    fleet.ships.destroyers = 1;
    fleet.ships.cruisers = 0;
    fleet.ships.battleships = 0;
    fleet.ships.scouts = 0;
    fleet.ships.etacs = 0;
    set_troop_transports(fleet, landing);
    set_armies_aboard(fleet, landing);
    fleet.rules_of_engagement = 10;
    fleet.target_depth = 1;
    set_fleet_at_rest(fleet);
    game_data
}

/// A world of production 100 with goods, a dock, and a queue to inherit.
fn held_world() -> Planet {
    let mut planet = Planet {
        potential_production: 100,
        production: Real48::from_int(100),
        stored_goods: Real48::from_int(400),
        ..Planet::default()
    };
    planet.stardock.destroyers = 3;
    planet.stardock.battleships = 1;
    planet.build_queue[0] = BuildSlot {
        points_owed: 60,
        kind: ProductionItemKind::Cruiser,
    };
    planet
}

const DOCKED: EmpireUnitSummary = EmpireUnitSummary {
    destroyers: 3,
    battleships: 1,
    cruisers: 0,
    scouts: 0,
    transports: 0,
    etacs: 0,
    starbases: 0,
    armies: 0,
    ground_batteries: 0,
};

fn production(game_data: &CoreGameData) -> f64 {
    game_data.planets[SYSTEM_PLANET_IDX].production.to_f64()
}

fn goods(game_data: &CoreGameData) -> f64 {
    game_data.planets[SYSTEM_PLANET_IDX].stored_goods.to_f64()
}

/// The inherited assets on the one capture report `events` hold.
fn inherited(events: &AssaultEvents) -> InheritedAssets {
    let [report] = events.assault_report_events.as_slice() else {
        panic!(
            "one assault report expected: {:?}",
            events.assault_report_events
        );
    };
    assert_eq!(report.outcome, MissionOutcome::Succeeded);
    report
        .inherited
        .expect("a capture reports what it inherited")
}

#[test]
fn a_world_that_falls_passes_on_what_survived() {
    for (order, landing) in [(Order::InvadeWorld, 5), (Order::BlitzWorld, 5)] {
        let mut game_data = strike_board(order, landing, held_world());
        let events = resolve_strike(&mut game_data, 0, 0);

        let planet = &game_data.planets[SYSTEM_PLANET_IDX];
        assert_eq!(planet.owner.0, 1, "{order:?}");
        assert_eq!(planet.armies, 5, "{order:?}");
        // No capture bonus, no loss: goods, production, dock, and queue as
        // they stood.
        assert_eq!(production(&game_data), 100.0, "{order:?}");
        assert_eq!(goods(&game_data), 400.0, "{order:?}");
        assert_eq!(planet.stardock.destroyers, 3, "{order:?}");
        assert_eq!(planet.stardock.battleships, 1, "{order:?}");
        assert_eq!(planet.build_queue[0].points_owed, 60, "{order:?}");
        assert_eq!(planet.build_queue[0].kind, ProductionItemKind::Cruiser);
        assert!(!planet.scorch_earth);
        assert_eq!(
            inherited(&events),
            InheritedAssets {
                stored_goods: 400,
                docked: DOCKED,
                queued_work: 60,
                scorched: false,
            },
            "{order:?}"
        );
    }
}

#[test]
fn a_capture_does_not_floor_production_or_forgive_debt() {
    let world = Planet {
        production: Real48::from_f64(0.4),
        stored_goods: Real48::from_int(-40),
        ..held_world()
    };
    let mut game_data = strike_board(Order::BlitzWorld, 5, world);
    let events = resolve_strike(&mut game_data, 0, 0);

    assert_eq!(game_data.planets[SYSTEM_PLANET_IDX].owner.0, 1);
    assert!((production(&game_data) - 0.4).abs() < 1e-4);
    assert_eq!(goods(&game_data), -40.0);
    // Debt is not an asset.
    assert_eq!(inherited(&events).stored_goods, 0);
}

#[test]
fn a_scorched_world_that_falls_is_halved_and_its_dock_emptied() {
    for order in [Order::InvadeWorld, Order::BlitzWorld] {
        let world = Planet {
            scorch_earth: true,
            ..held_world()
        };
        let mut game_data = strike_board(order, 5, world);
        let events = resolve_strike(&mut game_data, 0, 0);

        let planet = &game_data.planets[SYSTEM_PLANET_IDX];
        assert_eq!(planet.owner.0, 1, "{order:?}");
        assert_eq!(production(&game_data), 50.0, "{order:?}");
        assert_eq!(goods(&game_data), 200.0, "{order:?}");
        assert_eq!(planet.stardock.to_counts(), [0; 8], "{order:?}");
        assert!(
            !planet.scorch_earth,
            "{order:?}: the capture lifts the order"
        );
        // The queue is not the defenders' to burn: the work survives.
        assert_eq!(planet.build_queue[0].points_owed, 60, "{order:?}");
        assert_eq!(
            inherited(&events),
            InheritedAssets {
                stored_goods: 200,
                docked: EmpireUnitSummary::default(),
                queued_work: 60,
                scorched: true,
            },
            "{order:?}"
        );
    }
}

#[test]
fn a_scorched_capture_floors_production_at_one_and_leaves_debt_alone() {
    let scorched = |production: f64, potential: u8, goods: i32| Planet {
        production: Real48::from_f64(production),
        potential_production: potential,
        stored_goods: Real48::from_int(goods),
        scorch_earth: true,
        ..held_world()
    };

    // 1.5 halves to 0.75 and the floor lifts it to 1.0.
    let mut game_data = strike_board(Order::BlitzWorld, 5, scorched(1.5, 100, 400));
    resolve_strike(&mut game_data, 0, 0);
    assert_eq!(production(&game_data), 1.0);

    // A world with no potential stays at production 0.
    let mut game_data = strike_board(Order::BlitzWorld, 5, scorched(0.0, 0, 400));
    resolve_strike(&mut game_data, 0, 0);
    assert_eq!(production(&game_data), 0.0);

    // Debt is neither halved nor forgiven.
    let mut game_data = strike_board(Order::BlitzWorld, 5, scorched(100.0, 100, -40));
    resolve_strike(&mut game_data, 0, 0);
    assert_eq!(goods(&game_data), -40.0);
    assert_eq!(production(&game_data), 50.0);
}

#[test]
fn a_failed_assault_leaves_a_scorched_world_as_it_was() {
    // Five defending armies kill the one landing army before it fires.
    let world = Planet {
        armies: 5,
        scorch_earth: true,
        ..held_world()
    };
    let mut game_data = strike_board(Order::BlitzWorld, 1, world);
    let events = resolve_strike(&mut game_data, 0, 0);

    let planet = &game_data.planets[SYSTEM_PLANET_IDX];
    assert_eq!(planet.owner.0, 2);
    assert!(
        planet.scorch_earth,
        "the order stands until the world falls"
    );
    assert_eq!(production(&game_data), 100.0);
    assert_eq!(goods(&game_data), 400.0);
    assert_eq!(planet.stardock.destroyers, 3);
    assert_eq!(events.assault_report_events[0].inherited, None);
}

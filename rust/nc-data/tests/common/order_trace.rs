use nc_data::{
    Coords, CoreGameData, Fleet, GameStateBuilder, MaintenanceEvents, Mission, MissionOutcome,
    Order, PlayerMode, Starbase, recompute_max_speed, set_armies_aboard, set_fleet_at_rest,
    set_mission_target, set_planet_name, set_starbase_at_rest, set_troop_transports,
};
use nc_engine::run_maintenance_turn;

pub const TRACE_START: [u8; 2] = [8, 8];
pub const TRACE_TARGET: [u8; 2] = [11, 8];
pub const TRACE_SPEED: u8 = 3;
pub const TARGET_PLANET_IDX: usize = 4;

#[derive(Clone, Copy)]
pub struct FleetTurnExpectation {
    pub turn: usize,
    pub coords: [u8; 2],
    pub target: [u8; 2],
    pub order: Order,
    pub speed: u8,
    pub ready_flag: Option<u8>,
}

pub struct TraceCase {
    pub name: &'static str,
    pub setup: fn() -> CoreGameData,
    pub fleet_idx: usize,
    pub turns_to_run: usize,
    pub expectations: &'static [FleetTurnExpectation],
    pub before_turn: fn(usize, &mut CoreGameData),
    pub check: fn(&TraceRun),
}

pub struct TraceRun {
    pub states: Vec<CoreGameData>,
    pub events: Vec<MaintenanceEvents>,
}

#[derive(Clone, Copy)]
pub enum TargetWorldOwner {
    Unowned,
    Friendly,
    Foreign,
}

pub fn no_turn_mutation(_: usize, _: &mut CoreGameData) {}

pub fn run_trace_case(case: TraceCase) {
    let mut game_data = (case.setup)();
    let mut states = vec![game_data.clone()];
    let mut events = Vec::new();

    for turn in 1..=case.turns_to_run {
        (case.before_turn)(turn, &mut game_data);
        let turn_events = run_maintenance_turn(&mut game_data)
            .unwrap_or_else(|err| panic!("{} maintenance failed: {err}", case.name));
        events.push(turn_events);
        states.push(game_data.clone());
    }

    for expected in case.expectations {
        assert_fleet_state(case.name, &states[expected.turn], case.fleet_idx, *expected);
    }

    (case.check)(&TraceRun { states, events });
}

pub fn assert_mission_event(
    events: &MaintenanceEvents,
    fleet_idx: usize,
    kind: Mission,
    outcome: MissionOutcome,
) {
    assert!(
        events.mission_events.iter().any(|event| {
            event.fleet_idx == fleet_idx && event.kind == kind && event.outcome == outcome
        }),
        "missing mission event {:?} {:?} for fleet {}",
        kind,
        outcome,
        fleet_idx
    );
}

fn assert_fleet_state(
    name: &str,
    state: &CoreGameData,
    fleet_idx: usize,
    expected: FleetTurnExpectation,
) {
    assert!(
        fleet_idx < state.fleets.len(),
        "{name} turn {} expected fleet index {} but only {} fleet(s) remain",
        expected.turn,
        fleet_idx,
        state.fleets.len()
    );
    let fleet = &state.fleets[fleet_idx];
    assert_eq!(
        fleet.sector.to_array(),
        expected.coords,
        "{name} turn {} coords",
        expected.turn
    );
    assert_eq!(
        fleet.target.to_array(),
        expected.target,
        "{name} turn {} target",
        expected.turn
    );
    assert_eq!(
        fleet.order, expected.order,
        "{name} turn {} order",
        expected.turn
    );
    assert_eq!(
        fleet.speed, expected.speed,
        "{name} turn {} speed",
        expected.turn
    );
    if let Some(ready_flag) = expected.ready_flag {
        assert_eq!(
            fleet.depth.to_bytes()[0],
            ready_flag,
            "{name} turn {} ready flag",
            expected.turn
        );
    }
}

pub fn build_move_only_trace() -> CoreGameData {
    let mut game_data = build_trace_baseline();
    let fleet = &mut game_data.fleets[0];
    configure_scout_fleet(fleet);
    arm_new_order(
        fleet,
        Order::MoveOnly,
        TRACE_START,
        TRACE_TARGET,
        TRACE_SPEED,
    );
    game_data
}

pub fn build_colonize_world_trace() -> CoreGameData {
    let mut game_data = build_world_trace(TargetWorldOwner::Unowned);
    let fleet = &mut game_data.fleets[0];
    configure_colonizer_fleet(fleet);
    arm_new_order(
        fleet,
        Order::ColonizeWorld,
        TRACE_START,
        TRACE_TARGET,
        TRACE_SPEED,
    );
    game_data
}

pub fn build_view_world_trace() -> CoreGameData {
    let mut game_data = build_world_trace(TargetWorldOwner::Foreign);
    let fleet = &mut game_data.fleets[0];
    configure_cruiser_fleet(fleet);
    arm_new_order(
        fleet,
        Order::ViewWorld,
        TRACE_START,
        TRACE_TARGET,
        TRACE_SPEED,
    );
    game_data
}

pub fn build_seek_home_trace() -> CoreGameData {
    let mut game_data = build_world_trace(TargetWorldOwner::Friendly);
    let fleet = &mut game_data.fleets[0];
    configure_cruiser_fleet(fleet);
    arm_new_order(
        fleet,
        Order::SeekHome,
        TRACE_START,
        TRACE_TARGET,
        TRACE_SPEED,
    );
    game_data
}

pub fn build_salvage_trace() -> CoreGameData {
    let mut game_data = build_world_trace(TargetWorldOwner::Friendly);
    let fleet = &mut game_data.fleets[0];
    configure_salvage_fleet(fleet);
    arm_new_order(
        fleet,
        Order::Salvage,
        TRACE_START,
        TRACE_TARGET,
        TRACE_SPEED,
    );
    game_data
}

pub fn build_scout_sector_trace() -> CoreGameData {
    let mut game_data = build_world_trace(TargetWorldOwner::Foreign);
    let fleet = &mut game_data.fleets[0];
    configure_scout_fleet(fleet);
    arm_new_order(
        fleet,
        Order::ScoutSector,
        TRACE_START,
        TRACE_TARGET,
        TRACE_SPEED,
    );
    game_data
}

pub fn build_scout_system_trace() -> CoreGameData {
    let mut game_data = build_world_trace(TargetWorldOwner::Foreign);
    let fleet = &mut game_data.fleets[0];
    configure_scout_fleet(fleet);
    arm_new_order(
        fleet,
        Order::ScoutSolarSystem,
        TRACE_START,
        TRACE_TARGET,
        TRACE_SPEED,
    );
    game_data
}

pub fn build_patrol_sector_trace() -> CoreGameData {
    let mut game_data = build_trace_baseline();
    let fleet = &mut game_data.fleets[0];
    configure_cruiser_fleet(fleet);
    arm_new_order(
        fleet,
        Order::PatrolSector,
        TRACE_START,
        TRACE_TARGET,
        TRACE_SPEED,
    );
    game_data
}

pub fn build_guard_starbase_trace() -> CoreGameData {
    let mut game_data = build_trace_baseline();
    let mut base = Starbase::default();
    base.base_id.0 = 1;
    base.owner.0 = 1;
    base.record.0 = 1;
    base.sector = Coords::from_array(TRACE_TARGET);
    // At rest in orbit, as ECGAME leaves a base that is not moving.
    set_starbase_at_rest(&mut base);
    base.target = Coords::from_array(TRACE_TARGET);
    base.target_depth = 1;
    game_data.bases = vec![base];
    // The base heads player 1's chain.
    game_data.players[0].bases_first.0 = 1;

    let fleet = &mut game_data.fleets[0];
    configure_cruiser_fleet(fleet);
    arm_new_order(
        fleet,
        Order::GuardStarbase,
        TRACE_START,
        TRACE_TARGET,
        TRACE_SPEED,
    );
    set_mission_target(fleet, 1, 1);
    game_data
}

pub fn build_guard_blockade_world_trace() -> CoreGameData {
    let mut game_data = build_world_trace(TargetWorldOwner::Foreign);
    let fleet = &mut game_data.fleets[0];
    configure_cruiser_fleet(fleet);
    arm_new_order(
        fleet,
        Order::GuardBlockadeWorld,
        TRACE_START,
        TRACE_TARGET,
        TRACE_SPEED,
    );
    game_data
}

pub fn build_bombard_world_trace() -> CoreGameData {
    let mut game_data = build_world_trace(TargetWorldOwner::Foreign);
    let fleet = &mut game_data.fleets[0];
    configure_bombard_fleet(fleet);
    arm_new_order(
        fleet,
        Order::BombardWorld,
        TRACE_START,
        TRACE_TARGET,
        TRACE_SPEED,
    );
    game_data
}

pub fn build_invade_world_trace() -> CoreGameData {
    let mut game_data = build_world_trace(TargetWorldOwner::Foreign);
    {
        let target = &mut game_data.planets[TARGET_PLANET_IDX];
        target.ground_batteries = 1;
        target.armies = 1;
    }
    let fleet = &mut game_data.fleets[0];
    configure_assault_fleet(fleet);
    arm_new_order(
        fleet,
        Order::InvadeWorld,
        TRACE_START,
        TRACE_TARGET,
        TRACE_SPEED,
    );
    game_data
}

pub fn build_blitz_world_trace() -> CoreGameData {
    let mut game_data = build_world_trace(TargetWorldOwner::Foreign);
    {
        let target = &mut game_data.planets[TARGET_PLANET_IDX];
        // No batteries: the landing is unopposed, so the outcome does not hang
        // on the year's combat rolls.
        target.ground_batteries = 0;
        target.armies = 1;
    }
    let fleet = &mut game_data.fleets[0];
    configure_blitz_fleet(fleet);
    arm_new_order(
        fleet,
        Order::BlitzWorld,
        TRACE_START,
        TRACE_TARGET,
        TRACE_SPEED,
    );
    game_data
}

pub fn build_join_chase_trace() -> CoreGameData {
    let mut game_data = build_trace_baseline();

    let host = &mut game_data.fleets[0];
    configure_cruiser_fleet(host);
    arm_new_order(host, Order::MoveOnly, [10, 10], [14, 10], TRACE_SPEED);

    let host_id = game_data.fleets[0].record.0;
    let joiner = &mut game_data.fleets[1];
    configure_cruiser_fleet(joiner);
    joiner.owner.0 = 1;
    arm_new_order(
        joiner,
        Order::JoinAnotherFleet,
        [4, 10],
        [10, 10],
        TRACE_SPEED,
    );
    joiner.mission_target = host_id;

    game_data
}

pub fn build_join_merge_trace() -> CoreGameData {
    let mut game_data = build_trace_baseline();
    game_data.players[0].mode = PlayerMode::Rogue;
    let coords = [10, 10];

    let host = &mut game_data.fleets[0];
    configure_cruiser_fleet(host);
    arm_stationary_order(host, Order::HoldPosition, coords);

    let host_id = game_data.fleets[0].record.0;
    let joiner = &mut game_data.fleets[1];
    configure_cruiser_fleet(joiner);
    joiner.owner.0 = 1;
    arm_stationary_order(joiner, Order::JoinAnotherFleet, coords);
    joiner.mission_target = host_id;

    game_data
}

pub fn build_rendezvous_travel_trace() -> CoreGameData {
    let mut game_data = build_trace_baseline();
    let fleet = &mut game_data.fleets[0];
    configure_cruiser_fleet(fleet);
    arm_new_order(
        fleet,
        Order::RendezvousSector,
        TRACE_START,
        TRACE_TARGET,
        TRACE_SPEED,
    );
    game_data
}

pub fn build_rendezvous_merge_trace() -> CoreGameData {
    let mut game_data = build_trace_baseline();
    game_data.players[0].mode = PlayerMode::Rogue;

    let slow = &mut game_data.fleets[0];
    configure_cruiser_fleet(slow);
    slow.max_speed = 6;
    arm_stationary_order(slow, Order::RendezvousSector, TRACE_TARGET);

    let fast = &mut game_data.fleets[1];
    configure_etac_fleet(fast);
    fast.owner.0 = 1;
    fast.max_speed = 3;
    arm_stationary_order(fast, Order::RendezvousSector, TRACE_TARGET);

    game_data
}

pub fn configured_delayed_hostile_arrival_state(
    order: Order,
    ships: (u16, u16, u16, u16, u16, u16, u16),
) -> (CoreGameData, usize, [u8; 2]) {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");
    let target_coords = [25, 25];
    let target_idx = 4;

    let target_world = &mut game_data.planets[target_idx];
    target_world.sector = Coords::from_array(target_coords);
    target_world.owner.0 = 2;
    target_world.years_under_owner = 2;
    set_planet_name(target_world, "Target");
    target_world.armies = 10;
    target_world.ground_batteries = 4;

    let fleet = &mut game_data.fleets[0];
    fleet.ships.battleships = ships.0;
    fleet.ships.cruisers = ships.1;
    fleet.ships.destroyers = ships.2;
    set_troop_transports(fleet, ships.3);
    set_armies_aboard(fleet, ships.4);
    fleet.ships.etacs = ships.5;
    fleet.ships.scouts = ships.6;
    recompute_max_speed(fleet);
    fleet.sector = Coords::new(24, 25);
    fleet.speed = 3;
    fleet.order = order;
    fleet.target = Coords::from_array(target_coords);
    set_mission_target(fleet, 1, 0);
    set_fleet_at_rest(fleet);

    (game_data, target_idx, target_coords)
}

pub fn build_trace_baseline() -> CoreGameData {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");

    // The traces follow one fleet's orders; autopilot would give a fleet left
    // on Hold new ones.
    game_data.players[0].autopilot = false;
    game_data.bases = vec![];
    for fleet in game_data.fleets.iter_mut().skip(1) {
        clear_fleet_for_probe(fleet);
        fleet.owner.0 = 0;
        arm_stationary_order(fleet, Order::HoldPosition, [1, 1]);
    }

    game_data
}

pub fn build_world_trace(owner: TargetWorldOwner) -> CoreGameData {
    let mut game_data = build_trace_baseline();
    let target = &mut game_data.planets[TARGET_PLANET_IDX];
    target.sector = Coords::from_array(TRACE_TARGET);
    set_planet_name(target, "Target");
    target.ground_batteries = 4;
    target.armies = 10;
    match owner {
        TargetWorldOwner::Unowned => {
            target.owner.0 = 0;
            target.years_under_owner = 0;
        }
        TargetWorldOwner::Friendly => {
            target.owner.0 = 1;
            target.years_under_owner = 2;
        }
        TargetWorldOwner::Foreign => {
            target.owner.0 = 2;
            target.years_under_owner = 2;
        }
    }
    game_data
}

pub fn clear_fleet_for_probe(fleet: &mut Fleet) {
    fleet.ships.battleships = 0;
    fleet.ships.cruisers = 0;
    fleet.ships.destroyers = 0;
    set_troop_transports(fleet, 0);
    set_armies_aboard(fleet, 0);
    fleet.ships.etacs = 0;
    fleet.ships.scouts = 0;
    fleet.rules_of_engagement = 0;
    recompute_max_speed(fleet);
}

pub fn configure_cruiser_fleet(fleet: &mut Fleet) {
    clear_fleet_for_probe(fleet);
    fleet.ships.cruisers = 1;
    recompute_max_speed(fleet);
}

pub fn configure_scout_fleet(fleet: &mut Fleet) {
    clear_fleet_for_probe(fleet);
    fleet.ships.scouts = 1;
    recompute_max_speed(fleet);
}

pub fn configure_colonizer_fleet(fleet: &mut Fleet) {
    clear_fleet_for_probe(fleet);
    fleet.ships.etacs = 3;
    recompute_max_speed(fleet);
}

pub fn configure_salvage_fleet(fleet: &mut Fleet) {
    clear_fleet_for_probe(fleet);
    fleet.ships.destroyers = 1;
    fleet.ships.cruisers = 1;
    recompute_max_speed(fleet);
}

pub fn configure_bombard_fleet(fleet: &mut Fleet) {
    clear_fleet_for_probe(fleet);
    fleet.ships.destroyers = 1;
    recompute_max_speed(fleet);
}

pub fn configure_assault_fleet(fleet: &mut Fleet) {
    clear_fleet_for_probe(fleet);
    fleet.ships.battleships = 4;
    fleet.ships.cruisers = 4;
    fleet.ships.destroyers = 4;
    set_troop_transports(fleet, 6);
    set_armies_aboard(fleet, 24);
    recompute_max_speed(fleet);
}

pub fn configure_blitz_fleet(fleet: &mut Fleet) {
    clear_fleet_for_probe(fleet);
    fleet.ships.destroyers = 1;
    set_troop_transports(fleet, 10);
    set_armies_aboard(fleet, 10);
    recompute_max_speed(fleet);
}

pub fn configure_etac_fleet(fleet: &mut Fleet) {
    clear_fleet_for_probe(fleet);
    fleet.ships.etacs = 1;
    recompute_max_speed(fleet);
}

pub fn arm_new_order(fleet: &mut Fleet, order: Order, start: [u8; 2], target: [u8; 2], speed: u8) {
    fleet.sector = Coords::from_array(start);
    fleet.order = order;
    fleet.target = Coords::from_array(target);
    fleet.speed = speed;
    fleet.target_depth = u8::from(order.targets_world());
    set_fleet_at_rest(fleet);
}

pub fn arm_stationary_order(fleet: &mut Fleet, order: Order, coords: [u8; 2]) {
    fleet.sector = Coords::from_array(coords);
    fleet.order = order;
    fleet.target = Coords::from_array(coords);
    fleet.speed = 0;
    fleet.target_depth = u8::from(order.targets_world());
    set_fleet_at_rest(fleet);
}

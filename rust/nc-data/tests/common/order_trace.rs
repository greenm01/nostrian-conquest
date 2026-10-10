use nc_data::{
    BaseDat, BaseRecord, CoreGameData, FleetRecord, GameStateBuilder, MaintenanceEvents, Mission,
    MissionOutcome, Order,
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
        fleet_idx < state.fleets.records.len(),
        "{name} turn {} expected fleet index {} but only {} fleet(s) remain",
        expected.turn,
        fleet_idx,
        state.fleets.records.len()
    );
    let fleet = &state.fleets.records[fleet_idx];
    assert_eq!(
        fleet.current_location_coords_raw(),
        expected.coords,
        "{name} turn {} coords",
        expected.turn
    );
    assert_eq!(
        fleet.standing_order_target_coords_raw(),
        expected.target,
        "{name} turn {} target",
        expected.turn
    );
    assert_eq!(
        fleet.standing_order_kind(),
        expected.order,
        "{name} turn {} order",
        expected.turn
    );
    assert_eq!(
        fleet.current_speed(),
        expected.speed,
        "{name} turn {} speed",
        expected.turn
    );
    if let Some(ready_flag) = expected.ready_flag {
        assert_eq!(
            fleet.transit_ready_flag_raw(),
            ready_flag,
            "{name} turn {} ready flag",
            expected.turn
        );
    }
}

pub fn build_move_only_trace() -> CoreGameData {
    let mut game_data = build_trace_baseline();
    let fleet = &mut game_data.fleets.records[0];
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
    let fleet = &mut game_data.fleets.records[0];
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
    let fleet = &mut game_data.fleets.records[0];
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
    let fleet = &mut game_data.fleets.records[0];
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
    let fleet = &mut game_data.fleets.records[0];
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
    let fleet = &mut game_data.fleets.records[0];
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
    let fleet = &mut game_data.fleets.records[0];
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
    let fleet = &mut game_data.fleets.records[0];
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
    let mut base = BaseRecord::new_zeroed();
    base.set_base_id_raw(1);
    base.set_owner_empire_raw(1);
    base.set_record_number_raw(1);
    base.set_coords_raw(TRACE_TARGET);
    // At rest in orbit, as ECGAME leaves a base that is not moving.
    base.set_at_rest_motion();
    base.set_target_coords_raw(TRACE_TARGET);
    base.set_target_depth_raw(1);
    game_data.bases = BaseDat {
        records: vec![base],
    };
    // The base heads player 1's chain.
    game_data.player.records[0].set_first_base_record_raw(1);

    let fleet = &mut game_data.fleets.records[0];
    configure_cruiser_fleet(fleet);
    arm_new_order(
        fleet,
        Order::GuardStarbase,
        TRACE_START,
        TRACE_TARGET,
        TRACE_SPEED,
    );
    fleet.set_mission_aux_bytes([1, 1]);
    game_data
}

pub fn build_guard_blockade_world_trace() -> CoreGameData {
    let mut game_data = build_world_trace(TargetWorldOwner::Foreign);
    let fleet = &mut game_data.fleets.records[0];
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
    let fleet = &mut game_data.fleets.records[0];
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
        let target = &mut game_data.planets.records[TARGET_PLANET_IDX];
        target.set_ground_batteries_raw(1);
        target.set_army_count_raw(1);
    }
    let fleet = &mut game_data.fleets.records[0];
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
        let target = &mut game_data.planets.records[TARGET_PLANET_IDX];
        // No batteries: the landing is unopposed, so the outcome does not hang
        // on the year's combat rolls.
        target.set_ground_batteries_raw(0);
        target.set_army_count_raw(1);
    }
    let fleet = &mut game_data.fleets.records[0];
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

    let host = &mut game_data.fleets.records[0];
    configure_cruiser_fleet(host);
    arm_new_order(host, Order::MoveOnly, [10, 10], [14, 10], TRACE_SPEED);

    let host_id = game_data.fleets.records[0].fleet_id_word_raw();
    let joiner = &mut game_data.fleets.records[1];
    configure_cruiser_fleet(joiner);
    joiner.set_owner_empire_raw(1);
    arm_new_order(
        joiner,
        Order::JoinAnotherFleet,
        [4, 10],
        [10, 10],
        TRACE_SPEED,
    );
    joiner.set_join_host_fleet_id_raw(host_id);

    game_data
}

pub fn build_join_merge_trace() -> CoreGameData {
    let mut game_data = build_trace_baseline();
    game_data.player.records[0].raw[0x00] = 0xff;
    let coords = [10, 10];

    let host = &mut game_data.fleets.records[0];
    configure_cruiser_fleet(host);
    arm_stationary_order(host, Order::HoldPosition, coords);

    let host_id = game_data.fleets.records[0].fleet_id_word_raw();
    let joiner = &mut game_data.fleets.records[1];
    configure_cruiser_fleet(joiner);
    joiner.set_owner_empire_raw(1);
    arm_stationary_order(joiner, Order::JoinAnotherFleet, coords);
    joiner.set_join_host_fleet_id_raw(host_id);

    game_data
}

pub fn build_rendezvous_travel_trace() -> CoreGameData {
    let mut game_data = build_trace_baseline();
    let fleet = &mut game_data.fleets.records[0];
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
    game_data.player.records[0].raw[0x00] = 0xff;

    let slow = &mut game_data.fleets.records[0];
    configure_cruiser_fleet(slow);
    slow.set_max_speed(6);
    arm_stationary_order(slow, Order::RendezvousSector, TRACE_TARGET);

    let fast = &mut game_data.fleets.records[1];
    configure_etac_fleet(fast);
    fast.set_owner_empire_raw(1);
    fast.set_max_speed(3);
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

    let target_world = &mut game_data.planets.records[target_idx];
    target_world.set_coords_raw(target_coords);
    target_world.set_owner_empire_slot_raw(2);
    target_world.set_ownership_status_raw(2);
    target_world.set_planet_name("Target");
    target_world.set_army_count_raw(10);
    target_world.set_ground_batteries_raw(4);

    let fleet = &mut game_data.fleets.records[0];
    fleet.set_battleship_count(ships.0);
    fleet.set_cruiser_count(ships.1);
    fleet.set_destroyer_count(ships.2);
    fleet.set_troop_transport_count(ships.3);
    fleet.set_army_count(ships.4);
    fleet.set_etac_count(ships.5);
    fleet.set_scout_count(ships.6 as u16);
    fleet.recompute_max_speed_from_composition();
    fleet.set_current_location_coords_raw([24, 25]);
    fleet.set_current_speed(3);
    fleet.set_standing_order_kind(order);
    fleet.set_standing_order_target_coords_raw(target_coords);
    fleet.set_mission_aux_bytes([1, 0]);
    fleet.set_at_rest_motion();

    (game_data, target_idx, target_coords)
}

pub fn build_trace_baseline() -> CoreGameData {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");

    game_data.bases = BaseDat { records: vec![] };
    for fleet in game_data.fleets.records.iter_mut().skip(1) {
        clear_fleet_for_probe(fleet);
        fleet.set_owner_empire_raw(0);
        arm_stationary_order(fleet, Order::HoldPosition, [1, 1]);
    }

    game_data
}

pub fn build_world_trace(owner: TargetWorldOwner) -> CoreGameData {
    let mut game_data = build_trace_baseline();
    let target = &mut game_data.planets.records[TARGET_PLANET_IDX];
    target.set_coords_raw(TRACE_TARGET);
    target.set_planet_name("Target");
    target.set_ground_batteries_raw(4);
    target.set_army_count_raw(10);
    match owner {
        TargetWorldOwner::Unowned => {
            target.set_owner_empire_slot_raw(0);
            target.set_ownership_status_raw(0);
        }
        TargetWorldOwner::Friendly => {
            target.set_owner_empire_slot_raw(1);
            target.set_ownership_status_raw(2);
        }
        TargetWorldOwner::Foreign => {
            target.set_owner_empire_slot_raw(2);
            target.set_ownership_status_raw(2);
        }
    }
    game_data
}

pub fn clear_fleet_for_probe(fleet: &mut FleetRecord) {
    fleet.set_battleship_count(0);
    fleet.set_cruiser_count(0);
    fleet.set_destroyer_count(0);
    fleet.set_troop_transport_count(0);
    fleet.set_army_count(0);
    fleet.set_etac_count(0);
    fleet.set_scout_count(0);
    fleet.set_rules_of_engagement(0);
    fleet.recompute_max_speed_from_composition();
}

pub fn configure_cruiser_fleet(fleet: &mut FleetRecord) {
    clear_fleet_for_probe(fleet);
    fleet.set_cruiser_count(1);
    fleet.recompute_max_speed_from_composition();
}

pub fn configure_scout_fleet(fleet: &mut FleetRecord) {
    clear_fleet_for_probe(fleet);
    fleet.set_scout_count(1);
    fleet.recompute_max_speed_from_composition();
}

pub fn configure_colonizer_fleet(fleet: &mut FleetRecord) {
    clear_fleet_for_probe(fleet);
    fleet.set_etac_count(3);
    fleet.recompute_max_speed_from_composition();
}

pub fn configure_salvage_fleet(fleet: &mut FleetRecord) {
    clear_fleet_for_probe(fleet);
    fleet.set_destroyer_count(1);
    fleet.set_cruiser_count(1);
    fleet.recompute_max_speed_from_composition();
}

pub fn configure_bombard_fleet(fleet: &mut FleetRecord) {
    clear_fleet_for_probe(fleet);
    fleet.set_destroyer_count(1);
    fleet.recompute_max_speed_from_composition();
}

pub fn configure_assault_fleet(fleet: &mut FleetRecord) {
    clear_fleet_for_probe(fleet);
    fleet.set_battleship_count(4);
    fleet.set_cruiser_count(4);
    fleet.set_destroyer_count(4);
    fleet.set_troop_transport_count(6);
    fleet.set_army_count(24);
    fleet.recompute_max_speed_from_composition();
}

pub fn configure_blitz_fleet(fleet: &mut FleetRecord) {
    clear_fleet_for_probe(fleet);
    fleet.set_destroyer_count(1);
    fleet.set_troop_transport_count(10);
    fleet.set_army_count(10);
    fleet.recompute_max_speed_from_composition();
}

pub fn configure_etac_fleet(fleet: &mut FleetRecord) {
    clear_fleet_for_probe(fleet);
    fleet.set_etac_count(1);
    fleet.recompute_max_speed_from_composition();
}

pub fn arm_new_order(
    fleet: &mut FleetRecord,
    order: Order,
    start: [u8; 2],
    target: [u8; 2],
    speed: u8,
) {
    fleet.set_current_location_coords_raw(start);
    fleet.set_standing_order_kind(order);
    fleet.set_standing_order_target_coords_raw(target);
    fleet.set_current_speed(speed);
    fleet.set_mission_target(u8::from(order.targets_world()), fleet.mission_id_raw());
    fleet.set_at_rest_motion();
}

pub fn arm_stationary_order(fleet: &mut FleetRecord, order: Order, coords: [u8; 2]) {
    fleet.set_current_location_coords_raw(coords);
    fleet.set_standing_order_kind(order);
    fleet.set_standing_order_target_coords_raw(coords);
    fleet.set_current_speed(0);
    fleet.set_mission_target(u8::from(order.targets_world()), fleet.mission_id_raw());
    fleet.set_at_rest_motion();
}

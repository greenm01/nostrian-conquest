use nc_compat::load_classic_core_files;
use nc_data::{
    Coords, CoreGameData, DiplomacyOverride, DiplomaticRelation, EmpireId, Order, Planet, Real48,
    set_armies_aboard, set_fleet_at_rest, set_starbase_at_rest, set_troop_transports,
};
use std::path::Path;

/// Owned world at (15,13) used as the defended system in these scenarios.
pub const SYSTEM: [u8; 2] = [15, 13];
pub const SYSTEM_PLANET_IDX: usize = 13;
/// Empty deep-space sector.
pub const DEEP_SPACE: [u8; 2] = [8, 8];

#[derive(Clone, Copy, Default)]
pub struct Ships {
    pub dd: u16,
    pub ca: u16,
    pub bb: u16,
    pub sc: u16,
    pub tt: u16,
    pub armies: u16,
}

/// Fixture with no starbases and the world at `SYSTEM` owned by empire 2
/// with the given ground defenses. The fixture's other fleets stay at their
/// home worlds, away from `SYSTEM` and `DEEP_SPACE`, so fleet record indices
/// remain stable through maintenance.
pub fn empty_board(armies: u8, batteries: u8) -> CoreGameData {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join("ecmaint-post")
        .join("v1.5");
    let mut game_data = load_classic_core_files(&dir).expect("fixture should load");
    game_data.bases.clear();
    game_data.planets[SYSTEM_PLANET_IDX] = Planet {
        sector: Coords::from_array(SYSTEM),
        potential_production: 100,
        production: Real48::from_int(100),
        stored_goods: Real48::from_int(66),
        name: "TargetPrime".to_string(),
        armies: u16::from(armies),
        ground_batteries: u16::from(batteries),
        years_under_owner: 2,
        owner: EmpireId(2),
        ..Planet::default()
    };
    game_data
}

/// Configure fleet record `idx` as a stationary fleet of `owner` at `coords`.
#[allow(clippy::too_many_arguments)]
pub fn place_fleet(
    game_data: &mut CoreGameData,
    idx: usize,
    owner: u8,
    coords: [u8; 2],
    order: Order,
    target: [u8; 2],
    ships: Ships,
    roe: u8,
) {
    let fleet = &mut game_data.fleets[idx];
    fleet.owner.0 = owner;
    fleet.sector = Coords::from_array(coords);
    fleet.order = order;
    fleet.target = Coords::from_array(target);
    fleet.speed = 0;
    fleet.ships.destroyers = ships.dd;
    fleet.ships.cruisers = ships.ca;
    fleet.ships.battleships = ships.bb;
    fleet.ships.scouts = ships.sc;
    set_troop_transports(fleet, ships.tt);
    set_armies_aboard(fleet, ships.armies);
    fleet.ships.etacs = 0;
    fleet.rules_of_engagement = roe;
    if target != coords {
        // A contact stalled in transit: with no maximum speed the dispatcher
        // prelude has no speed to restore, so it ends the year here.
        fleet.max_speed = 0;
    }
}

/// A starbase of `owner` at rest in orbit (depth 1.0) over `coords`.
pub fn place_starbase(game_data: &mut CoreGameData, owner: u8, coords: [u8; 2]) {
    let mut base = nc_data::Starbase::default();
    base.base_id.0 = 1;
    base.record.0 = 1;
    base.sector = Coords::from_array(coords);
    base.target = Coords::from_array(coords);
    base.owner.0 = owner;
    set_starbase_at_rest(&mut base);
    base.target_depth = 1;
    game_data.bases.push(base);
}

/// Settle a fleet that already sits at its hostile-mission target at rest in
/// orbit (depth 1.0), so its strike test passes on the first tick.
pub fn mark_arrived(game_data: &mut CoreGameData, idx: usize) {
    let fleet = &mut game_data.fleets[idx];
    fleet.speed = 3;
    fleet.target_depth = 1;
    set_fleet_at_rest(fleet);
}

/// Explicit diplomacy among empires 1-4: the listed pairs are mutual enemies,
/// everyone else is neutral.
pub fn diplomacy(enemy_pairs: &[(u8, u8)]) -> Vec<DiplomacyOverride> {
    let mut overrides = Vec::new();
    for from in 1..=4u8 {
        for to in 1..=4u8 {
            if from == to {
                continue;
            }
            let enemy = enemy_pairs
                .iter()
                .any(|&(a, b)| (a, b) == (from, to) || (b, a) == (from, to));
            overrides.push(DiplomacyOverride {
                from_empire_raw: from,
                to_empire_raw: to,
                relation: if enemy {
                    DiplomaticRelation::Enemy
                } else {
                    DiplomaticRelation::Neutral
                },
            });
        }
    }
    overrides
}

//! The move fixture as the 2026-10-10 classic probes (t064, t069) used it:
//! empire 1 is a human empire with its home world at (16,13) and a lone
//! cruiser as fleet 1; (12,11) and (12,12) are unowned worlds, (18,15) and
//! (2,16) further ones, and (13,13) an empty sector.

use nc_compat::load_classic_core_files;
use nc_data::{
    Coords, CoreGameData, MaintenanceEvents, Order, PlayerMode, Real48, Starbase,
    recompute_max_speed, set_armies_aboard, set_planet_name, set_troop_transports,
};
use nc_engine::{build_results_report_blocks, run_maintenance_turn};
use std::path::Path;

pub const HOME: [u8; 2] = [16, 13];
pub const NEAR_WORLD: [u8; 2] = [12, 12];
pub const OTHER_NEAR_WORLD: [u8; 2] = [12, 11];
pub const FAR_WORLD: [u8; 2] = [18, 15];
pub const REMOTE_WORLD: [u8; 2] = [2, 16];
pub const EMPTY_SECTOR: [u8; 2] = [13, 13];

/// Empire 1 is a human empire with the home world and a cruiser fleet 1
/// (the fixture's other fleets stay at home on Guard/Blockade).
pub fn scenario() -> CoreGameData {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/ecmaint-move-pre/v1.5");
    let mut game_data = load_classic_core_files(&dir).expect("fixture should load");
    game_data.players[0].mode = PlayerMode::Human;
    game_data.players[0].autopilot = false;
    let fleet = &mut game_data.fleets[0];
    set_ships(fleet, 0, 1, 0, 0, 0);
    game_data
}

pub fn set_ships(
    fleet: &mut nc_data::Fleet,
    battleships: u16,
    cruisers: u16,
    destroyers: u16,
    scouts: u16,
    etacs: u16,
) {
    fleet.ships.battleships = battleships;
    fleet.ships.cruisers = cruisers;
    fleet.ships.destroyers = destroyers;
    fleet.ships.scouts = scouts;
    fleet.ships.etacs = etacs;
    set_troop_transports(fleet, 0);
    set_armies_aboard(fleet, 0);
    recompute_max_speed(fleet);
}

pub fn planet_at(game_data: &mut CoreGameData, sector: [u8; 2]) -> &mut nc_data::Planet {
    game_data
        .planets
        .iter_mut()
        .find(|planet| planet.sector.to_array() == sector)
        .expect("the fixture has a world here")
}

pub fn give_world(game_data: &mut CoreGameData, sector: [u8; 2], empire: u8) {
    let planet = planet_at(game_data, sector);
    planet.owner.0 = empire;
    planet.years_under_owner = 2;
}

pub fn set_production(game_data: &mut CoreGameData, sector: [u8; 2], production: i32) {
    planet_at(game_data, sector).production = Real48::from_int(production);
}

/// Fleet 1 at the center of `sector` at `depth`, on `order` toward `target`
/// with a world target.
pub fn place_fleet(
    game_data: &mut CoreGameData,
    sector: [u8; 2],
    depth: Real48,
    order: Order,
    target: [u8; 2],
    speed: u8,
) {
    let fleet = &mut game_data.fleets[0];
    fleet.sector = Coords::from_array(sector);
    fleet.x_fraction = Real48::HALF;
    fleet.y_fraction = Real48::HALF;
    fleet.depth = depth;
    fleet.order = order;
    fleet.target = Coords::from_array(target);
    fleet.target_depth = 1;
    fleet.speed = speed;
}

pub fn run(game_data: &mut CoreGameData) -> MaintenanceEvents {
    run_maintenance_turn(game_data).expect("maintenance should succeed")
}

/// Empire 1's results for the year, as one line of text.
pub fn report_text(game_data: &CoreGameData, events: &MaintenanceEvents) -> String {
    build_results_report_blocks(game_data, events)
        .iter()
        .filter(|row| row.viewer_empire_id == 1)
        .map(|row| row.decoded_text.replace('\n', " "))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn name_world(game_data: &mut CoreGameData, sector: [u8; 2], name: &str) {
    set_planet_name(planet_at(game_data, sector), name);
}

pub fn depth(game_data: &CoreGameData) -> f64 {
    game_data.fleets[0].depth.to_f64()
}

/// A starbase of `empire` at the centre of `sector` at `depth`, resting there
/// with target depth `target_depth` and `escort` as its escort word. Its
/// base id is the lowest one the empire has not used; the returned index is
/// its record's.
pub fn add_base(
    game_data: &mut CoreGameData,
    empire: u8,
    sector: [u8; 2],
    depth: Real48,
    target_depth: u8,
    escort: u16,
) -> usize {
    let mut base = Starbase::default();
    base.base_id.0 = 1 + game_data
        .bases
        .iter()
        .filter(|other| other.owner.0 == empire)
        .count() as u16;
    base.owner.0 = empire;
    base.sector = Coords::from_array(sector);
    base.x_fraction = Real48::HALF;
    base.y_fraction = Real48::HALF;
    base.depth = depth;
    base.target = base.sector;
    base.target_depth = target_depth;
    base.escort.0 = escort;
    game_data.bases.push(base);
    let index = game_data.bases.len() - 1;
    game_data.bases[index].record.0 = (index + 1) as u16;
    game_data.relink_base_chain(empire);
    game_data.sync_last_base_records();
    index
}

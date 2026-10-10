use nc_compat::load_classic_core_files;
use nc_data::{CoreGameData, DiplomacyOverride, DiplomaticRelation, Order};
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
    game_data.bases.records.clear();
    game_data.planets.records[SYSTEM_PLANET_IDX].set_as_owned_target_world(
        SYSTEM,
        [0x64, 0x87],
        [0x00, 0x00, 0x00, 0x00, 0x48, 0x87],
        0x04,
        0x0b,
        *b"TargetPrimeet",
        [0x05, 0x1d, 0x0b, 0x11, 0x25, 0x1c, 0x05],
        armies,
        batteries,
        2,
        2,
    );
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
    let fleet = &mut game_data.fleets.records[idx];
    fleet.set_owner_empire_raw(owner);
    fleet.set_current_location_coords_raw(coords);
    fleet.set_standing_order_kind(order);
    fleet.set_standing_order_target_coords_raw(target);
    fleet.set_current_speed(0);
    fleet.set_destroyer_count(ships.dd);
    fleet.set_cruiser_count(ships.ca);
    fleet.set_battleship_count(ships.bb);
    fleet.set_scout_count(ships.sc);
    fleet.set_troop_transport_count(ships.tt);
    fleet.set_army_count(ships.armies);
    fleet.set_etac_count(0);
    fleet.set_rules_of_engagement(roe);
    if target != coords {
        // A contact stalled in transit: with no maximum speed the dispatcher
        // prelude has no speed to restore, so it ends the year here.
        fleet.set_max_speed(0);
    }
}

/// A starbase of `owner` at rest in orbit (depth 1.0) over `coords`.
pub fn place_starbase(game_data: &mut CoreGameData, owner: u8, coords: [u8; 2]) {
    let mut base = nc_data::BaseRecord::new_zeroed();
    base.set_base_id_raw(1);
    base.set_record_number_raw(1);
    base.set_coords_raw(coords);
    base.set_target_coords_raw(coords);
    base.set_owner_empire_raw(owner);
    base.set_at_rest_motion();
    base.set_target_depth_raw(1);
    game_data.bases.records.push(base);
}

/// Settle a fleet that already sits at its hostile-mission target at rest in
/// orbit (depth 1.0), so its strike test passes on the first tick.
pub fn mark_arrived(game_data: &mut CoreGameData, idx: usize) {
    let fleet = &mut game_data.fleets.records[idx];
    fleet.set_current_speed(3);
    fleet.set_mission_target(1, fleet.mission_id_raw());
    fleet.set_at_rest_motion();
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

//! Scenarios for the encounter tests: the four-empire `ecmaint-post`
//! fixture with fleets placed by hand, run over many seeds.

use nc_compat::load_classic_core_files;
use nc_data::{
    CoreGameData, DiplomacyOverride, DiplomaticRelation, MaintenanceEvents, Order, Real48,
};
use nc_engine::run_maintenance_turn_with_context_and_seed;
use std::path::Path;

#[derive(Clone, Copy, Default)]
pub struct Ships {
    pub bb: u16,
    pub ca: u16,
    pub dd: u16,
    pub sc: u8,
    pub tt: u16,
    pub et: u16,
}

pub const NO_SHIPS: Ships = Ships {
    bb: 0,
    ca: 0,
    dd: 0,
    sc: 0,
    tt: 0,
    et: 0,
};

pub fn ca(n: u16) -> Ships {
    Ships { ca: n, ..NO_SHIPS }
}

/// One fleet, placed at the sector centre in deep space (`in_system`
/// false, depth 0) or at rest in orbit (depth 1).
#[derive(Clone, Copy)]
pub struct Fleet {
    pub owner: u8,
    pub at: [u8; 2],
    pub in_system: bool,
    pub order: Order,
    pub target: [u8; 2],
    /// The order's target depth: 1 for a world, 0 for deep space.
    pub target_in_system: bool,
    pub ships: Ships,
    pub roe: u8,
    pub speed: u8,
    pub max_speed: u8,
    /// Mission id byte (`0x23`), the guarded starbase for Guard Starbase.
    pub mission_id: u8,
}

impl Fleet {
    /// A fleet stopped on its order at `at`.
    pub fn on_station(owner: u8, at: [u8; 2], in_system: bool, order: Order, ships: Ships) -> Self {
        Self {
            owner,
            at,
            in_system,
            order,
            target: at,
            target_in_system: in_system,
            ships,
            roe: 0,
            speed: 0,
            max_speed: 3,
            mission_id: 0,
        }
    }

    pub fn roe(mut self, roe: u8) -> Self {
        self.roe = roe;
        self
    }

    pub fn max_speed(mut self, max_speed: u8) -> Self {
        self.max_speed = max_speed;
        self
    }

    /// Ordered to `target` at `speed`.
    pub fn heading(
        mut self,
        order: Order,
        target: [u8; 2],
        target_in_system: bool,
        speed: u8,
    ) -> Self {
        self.order = order;
        self.target = target;
        self.target_in_system = target_in_system;
        self.speed = speed;
        self
    }

    pub fn guarding(mut self, base_id: u8) -> Self {
        self.mission_id = base_id;
        self
    }
}

/// The fixture: four empires, each with four Guard/Blockade fleets at its
/// home world (empire 1 at (16,13), 2 at (4,13), 3 at (6,5), 4 at (13,5)),
/// no starbases. Year 3001.
pub fn board() -> CoreGameData {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join("ecmaint-post")
        .join("v1.5");
    let mut game_data = load_classic_core_files(&dir).expect("fixture should load");
    game_data.bases.clear();
    game_data
}

/// The world at (15,13), owned by empire 2.
pub const WORLD: [u8; 2] = [15, 13];

/// `board()` with `WORLD` given to empire 2.
pub fn board_with_world() -> CoreGameData {
    let mut game_data = board();
    game_data.planets[13].set_as_owned_target_world(
        WORLD,
        [0x64, 0x87],
        [0x00, 0x00, 0x00, 0x00, 0x48, 0x87],
        0x04,
        0x0b,
        *b"TargetPrimeet",
        [0x05, 0x1d, 0x0b, 0x11, 0x25, 0x1c, 0x05],
        4,
        0,
        2,
        2,
    );
    game_data
}

/// Replace fleet record `idx` (owned by `fleet.owner` in the fixture:
/// records 0-3 empire 1, 4-7 empire 2, 8-11 empire 3, 12-15 empire 4).
pub fn put(game_data: &mut CoreGameData, idx: usize, fleet: Fleet) {
    let record = &mut game_data.fleets[idx];
    assert_eq!(record.owner_empire_raw(), fleet.owner, "fleet {idx} owner");
    record.set_current_location_coords_raw(fleet.at);
    record.set_x_fraction(Real48::HALF);
    record.set_y_fraction(Real48::HALF);
    record.set_depth(if fleet.in_system {
        Real48::ONE
    } else {
        Real48::ZERO
    });
    record.set_standing_order_kind(fleet.order);
    record.set_standing_order_target_coords_raw(fleet.target);
    record.set_mission_aux_bytes([u8::from(fleet.target_in_system), fleet.mission_id]);
    record.set_battleship_count(fleet.ships.bb);
    record.set_cruiser_count(fleet.ships.ca);
    record.set_destroyer_count(fleet.ships.dd);
    record.set_scout_count(u16::from(fleet.ships.sc));
    record.set_troop_transport_count(fleet.ships.tt);
    record.set_army_count(0);
    record.set_etac_count(fleet.ships.et);
    record.set_rules_of_engagement(fleet.roe);
    record.set_max_speed(fleet.max_speed);
    record.set_current_speed(fleet.speed);
}

/// A starbase of `owner` at `coords`, in orbit (depth 1) or deep space.
pub fn add_starbase(
    game_data: &mut CoreGameData,
    owner: u8,
    id: u8,
    coords: [u8; 2],
    in_system: bool,
) {
    let mut base = nc_data::Starbase::default();
    base.set_base_id_raw(u16::from(id));
    base.set_record_number_raw(1);
    base.set_coords_raw(coords);
    base.set_target_coords_raw(coords);
    base.set_owner_empire_raw(owner);
    base.set_at_rest_motion();
    if !in_system {
        base.set_depth(nc_data::Real48::ZERO);
    }
    base.set_target_depth_raw(u8::from(in_system));
    game_data.bases.push(base);
}

/// Diplomacy among empires 1-4: `(from, to)` means `from` has declared `to`
/// an enemy. Everyone else is neutral.
pub fn declared(enemies: &[(u8, u8)]) -> Vec<DiplomacyOverride> {
    let mut overrides = Vec::new();
    for from in 1..=4u8 {
        for to in (1..=4u8).filter(|to| *to != from) {
            overrides.push(DiplomacyOverride {
                from_empire_raw: from,
                to_empire_raw: to,
                relation: if enemies.contains(&(from, to)) {
                    DiplomaticRelation::Enemy
                } else {
                    DiplomaticRelation::Neutral
                },
            });
        }
    }
    overrides
}

/// Run one year of `scenario` under each of `seeds` seeds.
pub fn years<'a>(
    scenario: &'a CoreGameData,
    diplomacy: &'a [DiplomacyOverride],
    seeds: std::ops::Range<u64>,
) -> impl Iterator<Item = (CoreGameData, MaintenanceEvents)> + 'a {
    seeds.map(move |seed| {
        let mut game_data = scenario.clone();
        let events =
            run_maintenance_turn_with_context_and_seed(&mut game_data, seed, &[], diplomacy)
                .expect("maintenance should succeed");
        (game_data, events)
    })
}

/// The tick `viewer` first sighted an alien fleet at `coords`, if it did.
pub fn first_contact(events: &MaintenanceEvents, viewer: u8, coords: [u8; 2]) -> Option<u8> {
    events
        .scout_contact_events
        .iter()
        .filter(|event| event.viewer_empire_raw == viewer && event.coords == coords)
        .filter_map(|event| event.stardate_week)
        .min()
}

/// Whether `empire` fought a battle at `coords`.
pub fn fought(events: &MaintenanceEvents, empire: u8, coords: [u8; 2]) -> bool {
    attacks(events, empire, coords) > 0
        || events
            .fleet_battle_events
            .iter()
            .any(|event| event.reporting_empire_raw == empire && event.coords == coords)
}

/// Battles `empire` started at `coords`.
pub fn attacks(events: &MaintenanceEvents, empire: u8, coords: [u8; 2]) -> usize {
    events
        .fleet_battle_events
        .iter()
        .filter(|event| {
            event.reporting_empire_raw == empire
                && event.coords == coords
                && event.perspective
                    == nc_data::maintenance_types::FleetBattlePerspective::Intercepted
        })
        .count()
}

/// Binomial check: `hits` of `trials` against probability `p`, within
/// `sigmas` standard deviations (plus one count of slack).
pub fn assert_rate(label: &str, hits: usize, trials: usize, p: f64, sigmas: f64) {
    let expected = p * trials as f64;
    let sd = (trials as f64 * p * (1.0 - p)).sqrt();
    assert!(
        (hits as f64 - expected).abs() <= sigmas * sd + 1.0,
        "{label}: {hits} of {trials}, expected {expected:.1} (p = {p:.4}, sd {sd:.1})"
    );
}

/// Chance of at least one success in `ticks` tries at `percent` each.
pub fn within(percent: f64, ticks: u32) -> f64 {
    1.0 - (1.0 - percent / 100.0).powi(ticks as i32)
}

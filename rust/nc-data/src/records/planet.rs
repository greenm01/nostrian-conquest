use crate::PLANET_RECORD_SIZE;
use crate::classic::{decode_planet, encode_planet};
use crate::model::{Coords, Planet};
use crate::real48::Real48;
use crate::support::classic_text;

use crate::model::ProductionItemKind;

/// The stardock is 8 u32 counts at `+0x38`, one per item type, in this
/// order: destroyer, cruiser, battleship, scout, troop transport, ETAC,
/// starbase, IPBM (`docs/dev/ec-battle-spec.md`).
pub const STARDOCK_SLOT_COUNT: usize = 8;
/// The build kind each stardock slot holds. Rust builds no IPBMs, so that
/// slot has no kind.
const STARDOCK_SLOT_KINDS: [u8; STARDOCK_SLOT_COUNT] = [1, 2, 3, 4, 5, 6, 9, 0];
/// Classic stores a planet name as a `String[20]`.
const PLANET_NAME_MAX_LEN: usize = 20;

/// The stardock slot that holds items of build kind `kind_raw`, if any.
pub fn stardock_slot_for_kind(kind_raw: u8) -> Option<usize> {
    (kind_raw != 0)
        .then(|| {
            STARDOCK_SLOT_KINDS
                .iter()
                .position(|&kind| kind == kind_raw)
        })
        .flatten()
}

impl Planet {
    pub fn from_raw(raw: [u8; PLANET_RECORD_SIZE]) -> Self {
        decode_planet(&raw)
    }

    /// The classic PLANETS.DAT encoding of this record.
    pub fn to_bytes(&self) -> [u8; PLANET_RECORD_SIZE] {
        encode_planet(self)
    }

    /// Sector x, y and potential production, PLANETS.DAT 0x00..0x02.
    pub fn header_bytes(&self) -> [u8; 3] {
        [self.sector.x, self.sector.y, self.potential_production]
    }

    pub fn coords_raw(&self) -> [u8; 2] {
        self.sector.to_array()
    }

    pub fn set_coords_raw(&mut self, coords: [u8; 2]) {
        self.sector = Coords::from_array(coords);
    }

    pub fn string_len(&self) -> u8 {
        self.name.chars().count() as u8
    }

    pub fn status_or_name_bytes(&self) -> &[u8] {
        self.name.as_bytes()
    }

    pub fn potential_production_points(&self) -> u16 {
        u16::from(self.potential_production)
    }

    pub fn set_potential_production_points(&mut self, value: u8) {
        self.potential_production = value;
    }

    /// Present production ("factories").
    pub fn present_production(&self) -> f64 {
        self.production.to_f64()
    }

    pub fn set_present_production(&mut self, value: f64) {
        self.production = Real48::from_f64(value);
    }

    /// Present production rounded to whole points.
    pub fn present_production_points(&self) -> Option<u16> {
        Some(
            self.present_production()
                .round()
                .clamp(0.0, f64::from(u16::MAX)) as u16,
        )
    }

    pub fn set_present_production_points(&mut self, value: u16) -> bool {
        self.set_present_production(f64::from(value));
        true
    }

    /// Stored goods (production points).
    pub fn stored_goods(&self) -> f64 {
        self.stored_goods.to_f64()
    }

    pub fn set_stored_goods(&mut self, value: f64) {
        self.stored_goods = Real48::from_f64(value);
    }

    /// Stored goods rounded to whole points.
    pub fn stored_production_points(&self) -> u32 {
        self.stored_goods().round().clamp(0.0, f64::from(u32::MAX)) as u32
    }

    pub fn set_stored_production_points(&mut self, value: u32) {
        self.set_stored_goods(f64::from(value));
    }

    /// The Scorch-Earth order: on capture, production and goods are halved
    /// and the stardock emptied.
    pub fn scorch_earth_raw(&self) -> u8 {
        u8::from(self.scorch_earth)
    }

    pub fn set_scorch_earth_raw(&mut self, value: u8) {
        self.scorch_earth = value != 0;
    }

    pub fn potential_production_points_current_known(&self) -> u16 {
        self.potential_production_points()
    }

    pub fn present_production_points_current_known(&self) -> Option<u16> {
        self.present_production_points()
    }

    /// Stores `name` as classic text, cut to the 20 characters a planet
    /// name holds.
    pub fn set_planet_name(&mut self, name: &str) {
        self.name = classic_text(name)
            .chars()
            .take(PLANET_NAME_MAX_LEN)
            .collect();
    }

    pub fn planet_name(&self) -> String {
        self.name.clone()
    }

    pub fn set_status_or_name_summary_raw(&mut self, value: &str) {
        self.set_planet_name(value);
    }

    pub fn set_status_or_name_prefix_raw(&mut self, value: &str) {
        self.set_planet_name(value);
    }

    /// Production points still owed in build slot `slot`.
    // t039: remove in phase 5
    pub fn build_count_raw(&self, slot: usize) -> u8 {
        self.build_queue[slot].points_owed
    }

    /// The unit kind in build slot `slot` as its byte; 0 for an empty slot.
    // t039: remove in phase 5
    pub fn build_kind_raw(&self, slot: usize) -> u8 {
        self.build_queue[slot].kind.to_raw()
    }

    // t039: remove in phase 5
    pub fn set_build_count_raw(&mut self, slot: usize, value: u8) {
        self.build_queue[slot].points_owed = value;
    }

    // t039: remove in phase 5
    pub fn set_build_kind_raw(&mut self, slot: usize, value: u8) {
        self.build_queue[slot].kind = ProductionItemKind::from_raw(value);
    }

    pub fn stardock_count_raw(&self, slot: usize) -> u16 {
        self.stardock_count(slot).min(u32::from(u16::MAX)) as u16
    }

    pub fn stardock_count(&self, slot: usize) -> u32 {
        self.stardock.to_counts()[slot]
    }

    /// The build kind in `slot`, or 0 when the slot is empty.
    pub fn stardock_kind_raw(&self, slot: usize) -> u8 {
        if self.stardock_count(slot) == 0 {
            0
        } else {
            STARDOCK_SLOT_KINDS[slot]
        }
    }

    pub fn stardock_item_kind_current_known(&self, slot: usize) -> ProductionItemKind {
        ProductionItemKind::from_raw(self.stardock_kind_raw(slot))
    }

    pub fn set_stardock_count_raw(&mut self, slot: usize, value: u16) {
        self.set_stardock_count(slot, u32::from(value));
    }

    pub fn set_stardock_count(&mut self, slot: usize, value: u32) {
        let mut counts = self.stardock.to_counts();
        counts[slot] = value;
        self.stardock = crate::model::Stardock::from_counts(counts);
    }

    /// Seeds this record as an owned world from the byte pieces older
    /// scenario code passes: `potential_production` is the potential and
    /// the production real's exponent, `factories` the production
    /// mantissa and the goods exponent, `goods_high_byte` the goods
    /// mantissa's top byte. `name_suffix_raw` was the stale name tail,
    /// which the model does not carry.
    // t039: remove in phase 5
    #[allow(clippy::too_many_arguments)]
    pub fn set_as_owned_target_world(
        &mut self,
        coords: [u8; 2],
        potential_production: [u8; 2],
        factories: [u8; 6],
        goods_high_byte: u8,
        name_len: u8,
        name_buffer: [u8; 13],
        _name_suffix_raw: [u8; 7],
        army_count: u8,
        ground_batteries: u8,
        ownership_status: u8,
        owner_empire_slot: u8,
    ) {
        let name_len = usize::from(name_len).min(name_buffer.len());
        *self = Planet {
            sector: Coords::from_array(coords),
            potential_production: potential_production[0],
            production: Real48::from_bytes([
                potential_production[1],
                factories[0],
                factories[1],
                factories[2],
                factories[3],
                factories[4],
            ]),
            stored_goods: Real48::from_bytes([factories[5], 0, 0, 0, 0, goods_high_byte]),
            name: String::from_utf8_lossy(&name_buffer[..name_len]).into_owned(),
            armies: u16::from(army_count),
            ground_batteries: u16::from(ground_batteries),
            years_under_owner: ownership_status,
            owner: crate::model::EmpireId(owner_empire_slot),
            ..Planet::default()
        };
    }

    pub fn owner_empire_slot_raw(&self) -> u8 {
        self.owner.0
    }
    pub fn set_owner_empire_slot_raw(&mut self, value: u8) {
        self.owner.0 = value;
    }

    /// Armies on the surface, as far as a byte holds them.
    pub fn army_count_raw(&self) -> u8 {
        self.armies.min(u16::from(u8::MAX)) as u8
    }

    pub fn set_army_count_raw(&mut self, value: u8) {
        self.armies = u16::from(value);
    }

    /// Ground batteries, as far as a byte holds them.
    pub fn ground_batteries_raw(&self) -> u8 {
        self.ground_batteries.min(u16::from(u8::MAX)) as u8
    }

    pub fn set_ground_batteries_raw(&mut self, value: u8) {
        self.ground_batteries = u16::from(value);
    }

    /// Years under the current owner, capped at 2: 0 just captured, 1
    /// recently captured, 2 fully functional.
    pub fn ownership_status_raw(&self) -> u8 {
        self.years_under_owner
    }
    pub fn set_ownership_status_raw(&mut self, value: u8) {
        self.years_under_owner = value;
    }

    pub fn status_or_name_summary(&self) -> String {
        self.name.trim().to_string()
    }

    pub fn is_named_homeworld_seed(&self) -> bool {
        self.status_or_name_summary() == "Not Named Yet"
    }

    /// Present production in `64 <= P < 128`, the band ECUTIL's homeworld
    /// seed (100) falls in; older code read this as the production real's
    /// exponent byte, `0x87`.
    pub fn has_homeworld_seed_production(&self) -> bool {
        (64.0..128.0).contains(&self.present_production())
    }

    pub fn is_homeworld_seed_ignoring_name(&self) -> bool {
        self.ownership_status_raw() == 2
            && self.owner_empire_slot_raw() != 0
            && self.has_homeworld_seed_production()
    }

    pub fn derived_summary(&self) -> String {
        let [x, y] = self.coords_raw();
        let text = self.status_or_name_summary();
        let mut parts = vec![format!("({},{}): {}", x, y, text)];
        if self.is_named_homeworld_seed() {
            parts.push("likely_homeworld_seed".to_string());
        }
        if self.build_count_raw(0) != 0 || self.build_kind_raw(0) != 0 {
            parts.push(format!(
                "build_raw={:02x}/{:02x}",
                self.build_count_raw(0),
                self.build_kind_raw(0)
            ));
        }
        if self.owner_empire_slot_raw() != 0 {
            parts.push(format!(
                "owner_slot={} owner_status={:02x}",
                self.owner_empire_slot_raw(),
                self.ownership_status_raw()
            ));
        }
        if self.army_count_raw() != 0 {
            parts.push(format!("armies={}", self.army_count_raw()));
        }
        if self.ground_batteries_raw() != 0 {
            parts.push(format!("batteries={}", self.ground_batteries_raw()));
        }
        parts.join(" | ")
    }
}

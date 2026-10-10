use crate::PLANET_RECORD_SIZE;
use crate::support::{ParseError, classic_text, copy_array, decode_real48, encode_real48};

pub const STARDOCK_SLOT_COUNT: usize = 10;
const STARDOCK_COUNT_BASE_OFFSET: usize = 0x38;
const STARDOCK_KIND_BASE_OFFSET: usize = 0x4C;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductionItemKind {
    Destroyer,
    Cruiser,
    Battleship,
    Scout,
    Transport,
    Etac,
    GroundBattery,
    Army,
    Starbase,
    Unknown(u8),
}

impl ProductionItemKind {
    pub fn from_raw(raw: u8) -> Self {
        match raw {
            1 => Self::Destroyer,
            2 => Self::Cruiser,
            3 => Self::Battleship,
            4 => Self::Scout,
            5 => Self::Transport,
            6 => Self::Etac,
            7 => Self::GroundBattery,
            8 => Self::Army,
            9 => Self::Starbase,
            other => Self::Unknown(other),
        }
    }

    pub fn requires_stardock(self) -> bool {
        matches!(
            self,
            Self::Destroyer
                | Self::Cruiser
                | Self::Battleship
                | Self::Scout
                | Self::Transport
                | Self::Etac
                | Self::Starbase
        )
    }

    pub fn build_cost(self) -> Option<u32> {
        match self {
            Self::Destroyer => Some(5),
            Self::Cruiser => Some(15),
            Self::Battleship => Some(45),
            Self::Scout => Some(15),
            Self::Transport => Some(5),
            Self::Etac => Some(20),
            Self::GroundBattery => Some(20),
            Self::Army => Some(2),
            Self::Starbase => Some(50),
            Self::Unknown(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanetRecord {
    pub raw: [u8; PLANET_RECORD_SIZE],
}

impl PlanetRecord {
    pub fn new_zeroed() -> Self {
        Self {
            raw: [0; PLANET_RECORD_SIZE],
        }
    }

    pub fn header_bytes(&self) -> &[u8] {
        &self.raw[..3]
    }

    pub fn coords_raw(&self) -> [u8; 2] {
        [self.raw[0], self.raw[1]]
    }

    pub fn set_coords_raw(&mut self, coords: [u8; 2]) {
        self.raw[0] = coords[0];
        self.raw[1] = coords[1];
    }

    pub fn header_value_raw(&self) -> u8 {
        self.raw[2]
    }

    pub fn set_header_value_raw(&mut self, value: u8) {
        self.raw[2] = value;
    }

    pub fn string_len(&self) -> u8 {
        self.raw[0x0F]
    }

    pub fn status_or_name_bytes(&self) -> &[u8] {
        &self.raw[0x10..=0x1C]
    }

    pub fn potential_production_raw(&self) -> [u8; 2] {
        [self.raw[0x02], self.raw[0x03]]
    }
    pub fn set_potential_production_raw(&mut self, value: [u8; 2]) {
        self.raw[0x02] = value[0];
        self.raw[0x03] = value[1];
    }

    pub fn potential_production_high_byte_raw(&self) -> u8 {
        self.raw[0x03]
    }

    pub fn set_potential_production_high_byte_raw(&mut self, value: u8) {
        self.raw[0x03] = value;
    }

    pub fn factories_raw(&self) -> [u8; 6] {
        copy_array(&self.raw[0x04..0x0A])
    }
    pub fn set_factories_raw(&mut self, value: [u8; 6]) {
        self.raw[0x04..0x0A].copy_from_slice(&value);
    }

    pub fn potential_production_points(&self) -> u16 {
        self.raw[0x02] as u16
    }

    fn decoded_present_production_points(&self) -> Option<u16> {
        decode_real48(self.factories_raw()).map(|value| value.round().max(0.0) as u16)
    }

    pub fn present_production_points(&self) -> Option<u16> {
        let decoded = self.decoded_present_production_points()?;
        // Only an uninitialized "Not Named Yet" seed (classic seeds store 50)
        // reads at potential; named homeworlds report their real production.
        if self.is_homeworld_seed_ignoring_name() && self.is_named_homeworld_seed() {
            Some(decoded.max(self.potential_production_points()))
        } else {
            Some(decoded)
        }
    }

    pub fn set_present_production_points(&mut self, value: u16) -> bool {
        match encode_real48(f64::from(value)) {
            Some(encoded) => {
                self.set_factories_raw(encoded);
                true
            }
            None => false,
        }
    }

    pub fn factories_word_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x08], self.raw[0x09]])
    }
    pub fn set_factories_word_raw(&mut self, value: u16) {
        self.raw[0x08..0x0A].copy_from_slice(&value.to_le_bytes());
    }

    pub fn stored_goods_raw(&self) -> u32 {
        u32::from_le_bytes(copy_array(&self.raw[0x0A..0x0E]))
    }
    pub fn set_stored_goods_raw(&mut self, value: u32) {
        self.raw[0x0A..0x0E].copy_from_slice(&value.to_le_bytes());
    }

    pub fn stored_production_points(&self) -> u32 {
        self.stored_goods_raw()
    }

    pub fn set_stored_production_points(&mut self, value: u32) {
        self.set_stored_goods_raw(value);
    }

    pub fn potential_production_points_current_known(&self) -> u16 {
        self.potential_production_points()
    }

    pub fn present_production_points_current_known(&self) -> Option<u16> {
        self.present_production_points()
    }

    pub fn economy_marker_raw(&self) -> u8 {
        self.raw[0x0E]
    }
    pub fn set_economy_marker_raw(&mut self, value: u8) {
        self.raw[0x0E] = value;
    }

    pub fn set_planet_name(&mut self, name: &str) {
        // Naming a "Not Named Yet" seed ends its potential floor, so store the
        // floored production first; a rename never changes production.
        if self.is_homeworld_seed_ignoring_name() && self.is_named_homeworld_seed() {
            if let Some(present) = self.present_production_points()
                && Some(present) != self.decoded_present_production_points()
            {
                let _ = self.set_present_production_points(present);
            }
        }
        let bytes = name.as_bytes();
        let mut len = bytes.len().min(13);
        while !name.is_char_boundary(len) {
            len -= 1;
        }
        self.raw[0x0F] = len as u8;
        self.raw[0x10..0x1D].fill(0);
        self.raw[0x10..0x10 + len].copy_from_slice(&bytes[..len]);
    }

    /// Get the planet name as a String.
    /// Uses Pascal-style encoding: length at 0x0F, content at 0x10-0x1C.
    pub fn planet_name(&self) -> String {
        let len = self.raw[0x0F] as usize;
        if len == 0 || len > 13 {
            return String::new();
        }
        let name_bytes = &self.raw[0x10..0x10 + len];
        String::from_utf8_lossy(name_bytes).to_string()
    }

    pub fn set_planet_name_buffer(&mut self, len: u8, buffer: &[u8; 13]) {
        self.raw[0x0F] = len;
        self.raw[0x10..0x1D].copy_from_slice(buffer);
    }

    pub fn set_status_or_name_summary_raw(&mut self, value: &str) {
        let value = classic_text(value);
        let bytes = value.as_bytes();
        let len = bytes.len().min(13);
        self.raw[0x0F] = len as u8;
        self.raw[0x10..0x1D].fill(0);
        self.raw[0x10..0x10 + len].copy_from_slice(&bytes[..len]);
    }

    pub fn set_status_or_name_prefix_raw(&mut self, value: &str) {
        let bytes = value.as_bytes();
        let len = bytes.len().min(13);
        self.raw[0x0F] = len as u8;
        self.raw[0x10..0x10 + len].copy_from_slice(&bytes[..len]);
    }

    pub fn build_count_raw(&self, slot: usize) -> u8 {
        self.raw[0x24 + slot]
    }

    pub fn build_kind_raw(&self, slot: usize) -> u8 {
        self.raw[0x2E + slot]
    }

    pub fn set_build_count_raw(&mut self, slot: usize, value: u8) {
        self.raw[0x24 + slot] = value;
    }

    pub fn set_build_kind_raw(&mut self, slot: usize, value: u8) {
        self.raw[0x2E + slot] = value;
    }

    pub fn stardock_count_raw(&self, slot: usize) -> u16 {
        let offset = STARDOCK_COUNT_BASE_OFFSET + slot * 2;
        u16::from_le_bytes([self.raw[offset], self.raw[offset + 1]])
    }

    pub fn stardock_kind_raw(&self, slot: usize) -> u8 {
        self.raw[STARDOCK_KIND_BASE_OFFSET + slot]
    }

    pub fn stardock_item_kind_current_known(&self, slot: usize) -> ProductionItemKind {
        ProductionItemKind::from_raw(self.stardock_kind_raw(slot))
    }

    pub fn set_stardock_count_raw(&mut self, slot: usize, value: u16) {
        let offset = STARDOCK_COUNT_BASE_OFFSET + slot * 2;
        let bytes = value.to_le_bytes();
        self.raw[offset] = bytes[0];
        self.raw[offset + 1] = bytes[1];
    }

    pub fn set_stardock_kind_raw(&mut self, slot: usize, value: u8) {
        self.raw[STARDOCK_KIND_BASE_OFFSET + slot] = value;
    }

    /// Set the 7 raw bytes at [0x1d..0x24] — the region between the end of the
    /// name buffer and the first build-count slot.  These bytes are not decoded
    /// but are preserved verbatim in fixture-derived target worlds.
    pub fn set_name_suffix_raw(&mut self, value: [u8; 7]) {
        self.raw[0x1D..0x24].copy_from_slice(&value);
    }

    /// Seed this record as an owned target world from explicit field values.
    ///
    /// Sets:
    /// - coords [0x00..0x01]
    /// - potential_production [0x02..0x03]
    /// - factories (6 bytes) [0x04..0x09]
    /// - economy marker byte [0x0e]
    /// - planet name (len + 13-byte buffer) [0x0f..0x1c]
    /// - name_suffix_raw (7 bytes) [0x1d..0x23]
    /// - army_count [0x58], ground_batteries [0x5a]
    /// - ownership_status [0x5c], owner_empire_slot [0x5d]
    ///
    /// All other bytes are zeroed first.  The caller is responsible for
    /// passing fixture-accurate values for the opaque fields (potential_production,
    /// factories, name_buffer, name_suffix_raw) when exact fixture matching is required.
    pub fn set_as_owned_target_world(
        &mut self,
        coords: [u8; 2],
        potential_production: [u8; 2],
        factories: [u8; 6],
        economy_marker: u8,
        name_len: u8,
        name_buffer: [u8; 13],
        name_suffix_raw: [u8; 7],
        army_count: u8,
        ground_batteries: u8,
        ownership_status: u8,
        owner_empire_slot: u8,
    ) {
        self.raw = [0u8; PLANET_RECORD_SIZE];
        self.raw[0x00] = coords[0];
        self.raw[0x01] = coords[1];
        self.set_potential_production_raw(potential_production);
        self.set_factories_raw(factories);
        self.raw[0x0E] = economy_marker;
        self.set_planet_name_buffer(name_len, &name_buffer);
        self.set_name_suffix_raw(name_suffix_raw);
        self.raw[0x58] = army_count;
        self.raw[0x5A] = ground_batteries;
        self.raw[0x5C] = ownership_status;
        self.raw[0x5D] = owner_empire_slot;
    }

    pub fn population_raw(&self) -> [u8; 6] {
        copy_array(&self.raw[0x52..0x58])
    }
    pub fn set_population_raw(&mut self, value: [u8; 6]) {
        self.raw[0x52..0x58].copy_from_slice(&value);
    }

    pub fn owner_empire_slot_raw(&self) -> u8 {
        self.raw[0x5D]
    }
    pub fn set_owner_empire_slot_raw(&mut self, value: u8) {
        self.raw[0x5D] = value;
    }

    /// Army count at offset 0x58.
    /// 0x8E (142) for Dust Bowl, 0x0A (10) for other homeworlds, 0 for unowned.
    pub fn army_count_raw(&self) -> u8 {
        self.raw[0x58]
    }

    pub fn set_army_count_raw(&mut self, value: u8) {
        self.raw[0x58] = value;
    }

    /// Ground batteries at offset 0x5A.
    /// 0x0F (15) for Dust Bowl, 0x04 (4) for other homeworlds, 0 for unowned.
    pub fn ground_batteries_raw(&self) -> u8 {
        self.raw[0x5A]
    }

    pub fn set_ground_batteries_raw(&mut self, value: u8) {
        self.raw[0x5A] = value;
    }

    pub fn ownership_status_raw(&self) -> u8 {
        self.raw[0x5C]
    }
    pub fn set_ownership_status_raw(&mut self, value: u8) {
        self.raw[0x5C] = value;
    }

    /// Conversion countdown for conquered planets. While > 0, the planet does
    /// not produce revenue or growth for the new owner. Decremented each turn.
    pub fn conversion_countdown_raw(&self) -> u8 {
        self.raw[0x5E]
    }
    pub fn set_conversion_countdown_raw(&mut self, value: u8) {
        self.raw[0x5E] = value;
    }

    pub fn status_or_name_summary(&self) -> String {
        let len = self.string_len() as usize;
        let text = &self.status_or_name_bytes()[..len.min(self.status_or_name_bytes().len())];
        String::from_utf8_lossy(text)
            .trim_matches(char::from(0))
            .trim()
            .to_string()
    }

    pub fn is_named_homeworld_seed(&self) -> bool {
        self.status_or_name_summary() == "Not Named Yet"
    }

    pub fn is_homeworld_seed_ignoring_name(&self) -> bool {
        self.ownership_status_raw() == 2
            && self.owner_empire_slot_raw() != 0
            && self.raw[0x03] == 0x87
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanetDat {
    pub records: Vec<PlanetRecord>,
}

impl PlanetDat {
    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        if data.len() % PLANET_RECORD_SIZE != 0 {
            return Err(ParseError::WrongRecordMultiple {
                file_type: "PLANETS.DAT",
                record_size: PLANET_RECORD_SIZE,
                actual: data.len(),
            });
        }
        Ok(Self {
            records: data
                .chunks_exact(PLANET_RECORD_SIZE)
                .map(|chunk| PlanetRecord {
                    raw: copy_array(chunk),
                })
                .collect(),
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.records
            .iter()
            .flat_map(|record| record.raw)
            .collect::<Vec<_>>()
    }
}

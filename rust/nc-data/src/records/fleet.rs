use crate::FLEET_RECORD_SIZE;
use crate::real48::Real48;
use crate::support::{ParseError, copy_array};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FleetRecord {
    pub raw: [u8; FLEET_RECORD_SIZE],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    HoldPosition,
    MoveOnly,
    SeekHome,
    PatrolSector,
    GuardStarbase,
    GuardBlockadeWorld,
    BombardWorld,
    InvadeWorld,
    BlitzWorld,
    ViewWorld,
    ScoutSector,
    ScoutSolarSystem,
    ColonizeWorld,
    JoinAnotherFleet,
    RendezvousSector,
    Salvage,
    Unknown(u8),
}

impl Order {
    pub fn from_raw(raw: u8) -> Self {
        match raw {
            0 => Self::HoldPosition,
            1 => Self::MoveOnly,
            2 => Self::SeekHome,
            3 => Self::PatrolSector,
            4 => Self::GuardStarbase,
            5 => Self::GuardBlockadeWorld,
            6 => Self::BombardWorld,
            7 => Self::InvadeWorld,
            8 => Self::BlitzWorld,
            9 => Self::ViewWorld,
            10 => Self::ScoutSector,
            11 => Self::ScoutSolarSystem,
            12 => Self::ColonizeWorld,
            13 => Self::JoinAnotherFleet,
            14 => Self::RendezvousSector,
            15 => Self::Salvage,
            other => Self::Unknown(other),
        }
    }

    pub fn to_raw(self) -> u8 {
        match self {
            Self::HoldPosition => 0,
            Self::MoveOnly => 1,
            Self::SeekHome => 2,
            Self::PatrolSector => 3,
            Self::GuardStarbase => 4,
            Self::GuardBlockadeWorld => 5,
            Self::BombardWorld => 6,
            Self::InvadeWorld => 7,
            Self::BlitzWorld => 8,
            Self::ViewWorld => 9,
            Self::ScoutSector => 10,
            Self::ScoutSolarSystem => 11,
            Self::ColonizeWorld => 12,
            Self::JoinAnotherFleet => 13,
            Self::RendezvousSector => 14,
            Self::Salvage => 15,
            Self::Unknown(raw) => raw,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::HoldPosition => "hold",
            Self::MoveOnly => "move",
            Self::SeekHome => "seek_home",
            Self::PatrolSector => "patrol",
            Self::GuardStarbase => "guard_starbase",
            Self::GuardBlockadeWorld => "guard_blockade",
            Self::BombardWorld => "bombard",
            Self::InvadeWorld => "invade",
            Self::BlitzWorld => "blitz",
            Self::ViewWorld => "view",
            Self::ScoutSector => "scout_sector",
            Self::ScoutSolarSystem => "scout_system",
            Self::ColonizeWorld => "colonize",
            Self::JoinAnotherFleet => "join_fleet",
            Self::RendezvousSector => "rendezvous",
            Self::Salvage => "salvage",
            Self::Unknown(_) => "unknown",
        }
    }

    /// ECGAME writes `0x22` as 1 for every order whose target is a world
    /// or fleet and 0 for orders that target a bare sector.
    pub fn targets_world(self) -> bool {
        !matches!(
            self,
            Self::MoveOnly
                | Self::PatrolSector
                | Self::ScoutSector
                | Self::RendezvousSector
                | Self::Unknown(_)
        )
    }

    pub fn display_label(self) -> &'static str {
        match self {
            Self::HoldPosition => "Hold position",
            Self::MoveOnly => "Move fleet",
            Self::SeekHome => "Seek home",
            Self::PatrolSector => "Patrol sector",
            Self::GuardStarbase => "Guard starbase",
            Self::GuardBlockadeWorld => "Guard/blockade world",
            Self::BombardWorld => "Bombard world",
            Self::InvadeWorld => "Invade world",
            Self::BlitzWorld => "Blitz world",
            Self::ViewWorld => "View world",
            Self::ScoutSector => "Scout sector",
            Self::ScoutSolarSystem => "Scout solar system",
            Self::ColonizeWorld => "Colonize world",
            Self::JoinAnotherFleet => "Join another fleet",
            Self::RendezvousSector => "Rendezvous at sector",
            Self::Salvage => "Salvage",
            Self::Unknown(_) => "Unknown order",
        }
    }
}

impl FleetRecord {
    pub fn new_zeroed() -> Self {
        Self {
            raw: [0; FLEET_RECORD_SIZE],
        }
    }

    pub fn local_slot_word_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x00], self.raw[0x01]])
    }
    pub fn set_local_slot_word_raw(&mut self, value: u16) {
        self.raw[0x00..0x02].copy_from_slice(&value.to_le_bytes());
    }
    pub fn local_slot(&self) -> u8 {
        self.raw[0x00]
    }
    pub fn owner_empire_raw(&self) -> u8 {
        self.raw[0x02]
    }
    pub fn set_owner_empire_raw(&mut self, value: u8) {
        self.raw[0x02] = value;
    }
    pub fn next_fleet_link_word_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x03], self.raw[0x04]])
    }
    pub fn set_next_fleet_link_word_raw(&mut self, value: u16) {
        self.raw[0x03..0x05].copy_from_slice(&value.to_le_bytes());
    }
    pub fn next_fleet_id(&self) -> u8 {
        self.raw[0x03]
    }

    pub fn total_starships(&self) -> u32 {
        u32::from(self.battleship_count())
            + u32::from(self.cruiser_count())
            + u32::from(self.destroyer_count())
            + u32::from(self.troop_transport_count())
            + u32::from(self.scout_count())
            + u32::from(self.etac_count())
    }

    pub fn has_any_combat_ships(&self) -> bool {
        self.destroyer_count() > 0 || self.cruiser_count() > 0 || self.battleship_count() > 0
    }

    pub fn is_support_only(&self) -> bool {
        !self.has_any_combat_ships() && self.total_starships() > 0
    }
    pub fn fleet_id_word_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x05], self.raw[0x06]])
    }
    pub fn set_fleet_id_word_raw(&mut self, value: u16) {
        self.raw[0x05..0x07].copy_from_slice(&value.to_le_bytes());
    }
    pub fn fleet_id(&self) -> u8 {
        self.raw[0x05]
    }
    pub fn previous_fleet_id(&self) -> u8 {
        self.raw[0x07]
    }
    pub fn set_previous_fleet_id(&mut self, value: u8) {
        self.raw[0x07] = value;
    }
    /// Armies loaded onto this fleet for an invasion order (raw[0x08]).
    /// Empirically confirmed from the invade scenario fixture.
    /// No docs reference yet; keep as raw until further RE confirms semantics.
    pub fn invasion_army_count_raw(&self) -> u8 {
        self.raw[0x08]
    }
    pub fn set_invasion_army_count_raw(&mut self, value: u8) {
        self.raw[0x08] = value;
    }
    pub fn max_speed(&self) -> u8 {
        self.raw[0x09]
    }
    pub fn set_max_speed(&mut self, value: u8) {
        self.raw[0x09] = value;
    }
    pub fn current_speed(&self) -> u8 {
        self.raw[0x0A]
    }
    pub fn set_current_speed(&mut self, value: u8) {
        self.raw[0x0A] = value;
    }
    pub fn current_location_coords_raw(&self) -> [u8; 2] {
        [self.raw[0x0B], self.raw[0x0C]]
    }
    pub fn set_current_location_coords_raw(&mut self, coords: [u8; 2]) {
        self.raw[0x0B] = coords[0];
        self.raw[0x0C] = coords[1];
    }

    /// ECGAME's recompute of `0x09`: start at 7 and take the minimum over the
    /// ship types present (ETAC 3, BB 4, CA 5, transports 5, DD 6, scouts 6).
    pub fn recompute_max_speed_from_composition(&mut self) {
        let mut max_speed = 7;
        for (count, speed) in [
            (self.etac_count(), 3),
            (self.battleship_count(), 4),
            (self.cruiser_count(), 5),
            (self.troop_transport_count(), 5),
            (self.destroyer_count(), 6),
            (self.scout_count(), 6),
        ] {
            if count > 0 {
                max_speed = max_speed.min(speed);
            }
        }
        self.set_max_speed(max_speed);
        if self.current_speed() > max_speed {
            self.set_current_speed(max_speed);
        }
    }

    pub fn has_any_force(&self) -> bool {
        self.scout_count() > 0
            || self.battleship_count() > 0
            || self.cruiser_count() > 0
            || self.destroyer_count() > 0
            || self.troop_transport_count() > 0
            || self.army_count() > 0
            || self.etac_count() > 0
    }

    pub fn mission_param_bytes(&self) -> &[u8] {
        &self.raw[0x1F..=0x21]
    }
    pub fn standing_order_code_raw(&self) -> u8 {
        self.raw[0x1F]
    }
    pub fn set_standing_order_code_raw(&mut self, value: u8) {
        self.raw[0x1F] = value;
    }
    pub fn set_standing_order_kind(&mut self, value: Order) {
        self.set_standing_order_code_raw(value.to_raw());
    }
    pub fn standing_order_kind(&self) -> Order {
        Order::from_raw(self.standing_order_code_raw())
    }
    pub fn standing_order_target_coords_raw(&self) -> [u8; 2] {
        [self.raw[0x20], self.raw[0x21]]
    }
    pub fn set_standing_order_target_coords_raw(&mut self, coords: [u8; 2]) {
        self.raw[0x20] = coords[0];
        self.raw[0x21] = coords[1];
    }
    pub fn mission_aux_bytes(&self) -> [u8; 2] {
        [self.raw[0x22], self.raw[0x23]]
    }
    pub fn world_target_flag_raw(&self) -> u8 {
        self.raw[0x22]
    }
    /// 0x23..=0x24: mission target id, a word: the Join host fleet id or
    /// the guarded starbase number.
    pub fn mission_id_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x23], self.raw[0x24]])
    }
    pub fn set_mission_id_raw(&mut self, value: u16) {
        self.raw[0x23..0x25].copy_from_slice(&value.to_le_bytes());
    }
    pub fn join_host_fleet_id_raw(&self) -> u16 {
        self.mission_id_raw()
    }
    pub fn set_join_host_fleet_id_raw(&mut self, value: u16) {
        self.set_mission_id_raw(value);
    }
    pub fn guard_starbase_index_raw(&self) -> u16 {
        self.mission_id_raw()
    }
    /// Writes the world-target flag (0x22) and the mission id word
    /// (0x23..=0x24) together, as ECGAME does when it sets an order.
    pub fn set_mission_target(&mut self, world_target_flag: u8, mission_id: u16) {
        self.raw[0x22] = world_target_flag;
        self.set_mission_id_raw(mission_id);
    }
    /// Raw bytes 0x22 (world-target flag) and 0x23 (mission id low byte);
    /// 0x24 is left alone.
    pub fn set_mission_aux_bytes(&mut self, value: [u8; 2]) {
        self.raw[0x22] = value[0];
        self.raw[0x23] = value[1];
    }
    pub fn tuple_a_payload_raw(&self) -> [u8; 5] {
        copy_array(&self.raw[0x0D..0x12])
    }
    pub fn set_tuple_a_payload_raw(&mut self, payload: [u8; 5]) {
        self.raw[0x0D..0x12].copy_from_slice(&payload);
    }

    pub fn tuple_b_payload_raw(&self) -> [u8; 5] {
        copy_array(&self.raw[0x13..0x18])
    }
    pub fn set_tuple_b_payload_raw(&mut self, payload: [u8; 5]) {
        self.raw[0x13..0x18].copy_from_slice(&payload);
    }
    pub fn tuple_c_payload_raw(&self) -> [u8; 5] {
        copy_array(&self.raw[0x19..0x1E])
    }
    pub fn set_tuple_c_payload_raw(&mut self, payload: [u8; 5]) {
        self.raw[0x19..0x1E].copy_from_slice(&payload);
    }

    /// 0x0D..=0x12: X offset inside the current sector, a 6-byte real
    /// (0.5 is the sector centre). Position X = `0x0B + x_fraction`.
    pub fn x_fraction(&self) -> Real48 {
        Real48::from_bytes(copy_array(&self.raw[0x0D..=0x12]))
    }
    pub fn set_x_fraction(&mut self, value: Real48) {
        self.raw[0x0D..=0x12].copy_from_slice(&value.to_bytes());
    }
    /// 0x13..=0x18: Y offset inside the current sector, a 6-byte real.
    pub fn y_fraction(&self) -> Real48 {
        Real48::from_bytes(copy_array(&self.raw[0x13..=0x18]))
    }
    pub fn set_y_fraction(&mut self, value: Real48) {
        self.raw[0x13..=0x18].copy_from_slice(&value.to_bytes());
    }
    /// 0x19..=0x1E: depth into the target system, a 6-byte real: 0.0 in
    /// deep space, 1.0 at rest in orbit.
    pub fn depth(&self) -> Real48 {
        Real48::from_bytes(copy_array(&self.raw[0x19..=0x1E]))
    }
    pub fn set_depth(&mut self, value: Real48) {
        self.raw[0x19..=0x1E].copy_from_slice(&value.to_bytes());
    }
    /// At rest in orbit at the sector centre: offsets 0.5, depth 1.0.
    pub fn set_at_rest_motion(&mut self) {
        self.set_x_fraction(Real48::HALF);
        self.set_y_fraction(Real48::HALF);
        self.set_depth(Real48::ONE);
    }
    /// The exponent byte of the depth real (0x80 while 0.5 <= depth < 1.0).
    pub fn transit_ready_flag_raw(&self) -> u8 {
        self.raw[0x19]
    }

    pub fn standing_order_summary(&self) -> String {
        let [x, y] = self.standing_order_target_coords_raw();
        match self.standing_order_kind() {
            Order::HoldPosition => "Hold position".to_string(),
            Order::MoveOnly => format!("Move fleet to Sector ({x},{y})"),
            Order::SeekHome => "Seek home".to_string(),
            Order::PatrolSector => format!("Patrol Sector ({x},{y})"),
            Order::GuardStarbase => format!("Guard starbase at Sector ({x},{y})"),
            Order::GuardBlockadeWorld => {
                format!("Guard/blockade world in System ({x},{y})")
            }
            Order::BombardWorld => format!("Bombard world in System ({x},{y})"),
            Order::InvadeWorld => format!("Invade world in System ({x},{y})"),
            Order::BlitzWorld => format!("Blitz world in System ({x},{y})"),
            Order::ViewWorld => format!("View world in System ({x},{y})"),
            Order::ScoutSector => format!("Scout Sector ({x},{y})"),
            Order::ScoutSolarSystem => format!("Scout solar system ({x},{y})"),
            Order::ColonizeWorld => format!("Colonize world in System ({x},{y})"),
            Order::JoinAnotherFleet => "Join another fleet".to_string(),
            Order::RendezvousSector => format!("Rendezvous at Sector ({x},{y})"),
            Order::Salvage => format!("Salvage at Sector ({x},{y})"),
            Order::Unknown(code) => {
                format!("Unknown order {code} target ({x},{y})")
            }
        }
    }

    pub fn rules_of_engagement(&self) -> u8 {
        self.raw[0x25]
    }
    pub fn set_rules_of_engagement(&mut self, value: u8) {
        self.raw[0x25] = value;
    }
    pub fn battleship_count(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x26], self.raw[0x27]])
    }
    pub fn set_battleship_count(&mut self, value: u16) {
        self.raw[0x26..0x28].copy_from_slice(&value.to_le_bytes());
    }
    pub fn cruiser_count(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x28], self.raw[0x29]])
    }
    pub fn set_cruiser_count(&mut self, value: u16) {
        self.raw[0x28..0x2A].copy_from_slice(&value.to_le_bytes());
    }
    pub fn destroyer_count(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x2A], self.raw[0x2B]])
    }
    pub fn set_destroyer_count(&mut self, value: u16) {
        self.raw[0x2A..0x2C].copy_from_slice(&value.to_le_bytes());
    }
    /// 0x2C: troop transports without an army.
    pub fn empty_transport_count(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x2C], self.raw[0x2D]])
    }
    pub fn set_empty_transport_count(&mut self, value: u16) {
        self.raw[0x2C..0x2E].copy_from_slice(&value.to_le_bytes());
    }
    /// 0x2E: troop transports carrying one army each.
    pub fn loaded_transport_count(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x2E], self.raw[0x2F]])
    }
    pub fn set_loaded_transport_count(&mut self, value: u16) {
        self.raw[0x2E..0x30].copy_from_slice(&value.to_le_bytes());
    }
    /// All troop transports, empty and loaded.
    pub fn troop_transport_count(&self) -> u16 {
        self.empty_transport_count()
            .saturating_add(self.loaded_transport_count())
    }
    /// Sets the transport total, keeping loaded transports (up to the new
    /// total) and making the rest empty.
    pub fn set_troop_transport_count(&mut self, total: u16) {
        let loaded = self.loaded_transport_count().min(total);
        self.set_loaded_transport_count(loaded);
        self.set_empty_transport_count(total - loaded);
    }
    /// Armies aboard: one per loaded transport.
    pub fn army_count(&self) -> u16 {
        self.loaded_transport_count()
    }
    /// Loads or unloads armies, moving transports between the empty and
    /// loaded counts. The transport total grows if it cannot hold `armies`.
    pub fn set_army_count(&mut self, armies: u16) {
        let total = self.troop_transport_count();
        self.set_loaded_transport_count(armies);
        self.set_empty_transport_count(total.saturating_sub(armies));
    }
    pub fn etac_count(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x30], self.raw[0x31]])
    }
    pub fn set_etac_count(&mut self, value: u16) {
        self.raw[0x30..0x32].copy_from_slice(&value.to_le_bytes());
    }
    pub fn scout_count(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x32], self.raw[0x33]])
    }
    pub fn set_scout_count(&mut self, value: u16) {
        self.raw[0x32..0x34].copy_from_slice(&value.to_le_bytes());
    }
    /// 0x34..=0x35: a word Rust does not interpret. ECMAINT's encounter code
    /// writes fleet strength here and its Join merge adds it; Rust keeps it
    /// byte for byte.
    pub fn strength_cache_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x34], self.raw[0x35]])
    }
    pub fn set_strength_cache_raw(&mut self, value: u16) {
        self.raw[0x34..0x36].copy_from_slice(&value.to_le_bytes());
    }

    fn counted_ship_composition_parts(&self) -> Vec<String> {
        [
            ("SC", self.scout_count()),
            ("BB", self.battleship_count()),
            ("CA", self.cruiser_count()),
            ("DD", self.destroyer_count()),
            ("TT", self.troop_transport_count()),
            ("AR", self.army_count()),
            ("ET", self.etac_count()),
        ]
        .into_iter()
        .filter_map(|(label, count)| (count > 0).then(|| format!("{label}={count}")))
        .collect()
    }

    fn fleet_list_ship_composition_tokens(&self) -> Vec<String> {
        fn token(label: &str, count: u16) -> Option<String> {
            match count {
                0 => None,
                1 => Some(label.to_string()),
                _ => Some(format!("{count}{label}")),
            }
        }

        [
            token("SC", self.scout_count()),
            token("BB", self.battleship_count()),
            token("CA", self.cruiser_count()),
            token("DD", self.destroyer_count()),
            token("TT*", self.loaded_transport_count()),
            token("TT", self.empty_transport_count()),
            token("ET", self.etac_count()),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    pub fn ship_composition_summary(&self) -> String {
        let parts = self.counted_ship_composition_parts();
        assert!(!parts.is_empty(), "empty fleet record is not a fleet");
        parts.join(" ")
    }

    pub fn ship_composition_table_summary(&self) -> String {
        let parts = self.fleet_list_ship_composition_tokens();
        assert!(!parts.is_empty(), "empty fleet record is not a fleet");
        parts.join(" ")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FleetDat {
    pub records: Vec<FleetRecord>,
}

impl FleetDat {
    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        if data.len() % FLEET_RECORD_SIZE != 0 {
            return Err(ParseError::WrongRecordMultiple {
                file_type: "FLEETS.DAT",
                record_size: FLEET_RECORD_SIZE,
                actual: data.len(),
            });
        }
        Ok(Self {
            records: data
                .chunks_exact(FLEET_RECORD_SIZE)
                .map(|chunk| FleetRecord {
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

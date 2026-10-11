use crate::FLEET_RECORD_SIZE;
use crate::classic::{decode_fleet, encode_fleet};
use crate::model::{Coords, Fleet};
use crate::real48::Real48;

use crate::model::Order;

impl Fleet {
    pub fn from_raw(raw: [u8; FLEET_RECORD_SIZE]) -> Self {
        decode_fleet(&raw)
    }

    /// The classic FLEETS.DAT encoding of this record.
    pub fn to_bytes(&self) -> [u8; FLEET_RECORD_SIZE] {
        encode_fleet(self)
    }

    pub fn local_slot_word_raw(&self) -> u16 {
        self.fleet_number.0
    }
    pub fn set_local_slot_word_raw(&mut self, value: u16) {
        self.fleet_number.0 = value;
    }
    pub fn local_slot(&self) -> u8 {
        self.fleet_number.0 as u8
    }
    pub fn owner_empire_raw(&self) -> u8 {
        self.owner.0
    }
    pub fn set_owner_empire_raw(&mut self, value: u8) {
        self.owner.0 = value;
    }
    pub fn next_fleet_link_word_raw(&self) -> u16 {
        self.next.0
    }
    pub fn set_next_fleet_link_word_raw(&mut self, value: u16) {
        self.next.0 = value;
    }
    pub fn next_fleet_id(&self) -> u16 {
        self.next_fleet_link_word_raw()
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
        self.record.0
    }
    pub fn set_fleet_id_word_raw(&mut self, value: u16) {
        self.record.0 = value;
    }
    pub fn fleet_id(&self) -> u16 {
        self.fleet_id_word_raw()
    }
    /// The previous fleet in the owner's chain.
    pub fn previous_fleet_id(&self) -> u16 {
        self.prev.0
    }
    pub fn set_previous_fleet_id(&mut self, value: u16) {
        self.prev.0 = value;
    }
    pub fn max_speed(&self) -> u8 {
        self.max_speed
    }
    pub fn set_max_speed(&mut self, value: u8) {
        self.max_speed = value;
    }
    pub fn current_speed(&self) -> u8 {
        self.speed
    }
    pub fn set_current_speed(&mut self, value: u8) {
        self.speed = value;
    }
    pub fn current_location_coords_raw(&self) -> [u8; 2] {
        self.sector.to_array()
    }
    pub fn set_current_location_coords_raw(&mut self, coords: [u8; 2]) {
        self.sector = Coords::from_array(coords);
    }

    /// ECGAME's recompute of the maximum speed: start at 7 and take the
    /// minimum over the ship types present (ETAC 3, BB 4, CA 5, transports
    /// 5, DD 6, scouts 6).
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

    /// The order code and target sector, as FLEETS.DAT 0x1F..0x21 hold them.
    pub fn mission_param_bytes(&self) -> [u8; 3] {
        [self.order.to_raw(), self.target.x, self.target.y]
    }
    pub fn standing_order_code_raw(&self) -> u8 {
        self.order.to_raw()
    }
    pub fn set_standing_order_code_raw(&mut self, value: u8) {
        self.order = Order::from_raw(value);
    }
    pub fn set_standing_order_kind(&mut self, value: Order) {
        self.order = value;
    }
    pub fn standing_order_kind(&self) -> Order {
        self.order
    }
    pub fn standing_order_target_coords_raw(&self) -> [u8; 2] {
        self.target.to_array()
    }
    pub fn set_standing_order_target_coords_raw(&mut self, coords: [u8; 2]) {
        self.target = Coords::from_array(coords);
    }
    /// The target depth and the low byte of the mission target, as
    /// FLEETS.DAT 0x22..0x23 hold them.
    pub fn mission_aux_bytes(&self) -> [u8; 2] {
        [self.target_depth, self.mission_target as u8]
    }
    pub fn world_target_flag_raw(&self) -> u8 {
        self.target_depth
    }
    /// 0x23..=0x24: mission target id, a word: the Join host's fleet number
    /// or the guarded starbase number.
    pub fn mission_id_raw(&self) -> u16 {
        self.mission_target
    }
    pub fn set_mission_id_raw(&mut self, value: u16) {
        self.mission_target = value;
    }
    /// The Join host's fleet number within the owner's empire (the host's
    /// 0x00 word), not its record id. ECMAINT's merge and ECGAME's
    /// renumbering both match 0x23 against 0x00.
    pub fn join_host_fleet_id_raw(&self) -> u16 {
        self.mission_id_raw()
    }
    pub fn set_join_host_fleet_id_raw(&mut self, value: u16) {
        self.set_mission_id_raw(value);
    }
    pub fn guard_starbase_index_raw(&self) -> u16 {
        self.mission_id_raw()
    }
    /// Writes the target depth and the mission target together, as ECGAME
    /// does when it sets an order.
    pub fn set_mission_target(&mut self, world_target_flag: u8, mission_id: u16) {
        self.target_depth = world_target_flag;
        self.mission_target = mission_id;
    }
    /// Writes the target depth and the low byte of the mission target; the
    /// high byte is left alone.
    pub fn set_mission_aux_bytes(&mut self, value: [u8; 2]) {
        self.target_depth = value[0];
        self.mission_target = (self.mission_target & 0xFF00) | u16::from(value[1]);
    }

    /// X offset inside the current sector (0.5 is the sector centre).
    /// Position X = sector x + `x_fraction`.
    pub fn x_fraction(&self) -> Real48 {
        self.x_fraction
    }
    pub fn set_x_fraction(&mut self, value: Real48) {
        self.x_fraction = value;
    }
    /// Y offset inside the current sector.
    pub fn y_fraction(&self) -> Real48 {
        self.y_fraction
    }
    pub fn set_y_fraction(&mut self, value: Real48) {
        self.y_fraction = value;
    }
    /// Depth into the target system: 0.0 in deep space, 1.0 at rest in
    /// orbit.
    pub fn depth(&self) -> Real48 {
        self.depth
    }
    pub fn set_depth(&mut self, value: Real48) {
        self.depth = value;
    }
    /// At rest in orbit at the sector centre: offsets 0.5, depth 1.0.
    pub fn set_at_rest_motion(&mut self) {
        self.set_x_fraction(Real48::HALF);
        self.set_y_fraction(Real48::HALF);
        self.set_depth(Real48::ONE);
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
        self.rules_of_engagement
    }
    pub fn set_rules_of_engagement(&mut self, value: u8) {
        self.rules_of_engagement = value;
    }
    pub fn battleship_count(&self) -> u16 {
        self.ships.battleships
    }
    pub fn set_battleship_count(&mut self, value: u16) {
        self.ships.battleships = value;
    }
    pub fn cruiser_count(&self) -> u16 {
        self.ships.cruisers
    }
    pub fn set_cruiser_count(&mut self, value: u16) {
        self.ships.cruisers = value;
    }
    pub fn destroyer_count(&self) -> u16 {
        self.ships.destroyers
    }
    pub fn set_destroyer_count(&mut self, value: u16) {
        self.ships.destroyers = value;
    }
    /// Troop transports without an army.
    pub fn empty_transport_count(&self) -> u16 {
        self.ships.empty_transports
    }
    pub fn set_empty_transport_count(&mut self, value: u16) {
        self.ships.empty_transports = value;
    }
    /// Troop transports carrying one army each.
    pub fn loaded_transport_count(&self) -> u16 {
        self.ships.loaded_transports
    }
    pub fn set_loaded_transport_count(&mut self, value: u16) {
        self.ships.loaded_transports = value;
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
        self.ships.etacs
    }
    pub fn set_etac_count(&mut self, value: u16) {
        self.ships.etacs = value;
    }
    pub fn scout_count(&self) -> u16 {
        self.ships.scouts
    }
    pub fn set_scout_count(&mut self, value: u16) {
        self.ships.scouts = value;
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

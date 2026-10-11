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
        crate::fleet_ships::starship_count(self)
    }

    pub fn has_any_combat_ships(&self) -> bool {
        crate::fleet_ships::has_combat_ships(self)
    }

    pub fn is_support_only(&self) -> bool {
        crate::fleet_ships::is_support_only(self)
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
        crate::fleet_ships::recompute_max_speed(self)
    }

    pub fn has_any_force(&self) -> bool {
        crate::fleet_ships::has_any_force(self)
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
        crate::fleet_orders::set_mission_target(self, world_target_flag, mission_id)
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
        crate::motion::set_fleet_at_rest(self)
    }

    pub fn standing_order_summary(&self) -> String {
        crate::fleet_orders::order_summary(self)
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
        crate::fleet_ships::troop_transports(self)
    }
    /// Sets the transport total, keeping loaded transports (up to the new
    /// total) and making the rest empty.
    pub fn set_troop_transport_count(&mut self, total: u16) {
        crate::fleet_ships::set_troop_transports(self, total)
    }
    /// Armies aboard: one per loaded transport.
    pub fn army_count(&self) -> u16 {
        self.loaded_transport_count()
    }
    /// Loads or unloads armies, moving transports between the empty and
    /// loaded counts. The transport total grows if it cannot hold `armies`.
    pub fn set_army_count(&mut self, armies: u16) {
        crate::fleet_ships::set_armies_aboard(self, armies)
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

    pub fn ship_composition_summary(&self) -> String {
        crate::fleet_ships::ship_composition_summary(self)
    }

    pub fn ship_composition_table_summary(&self) -> String {
        crate::fleet_ships::ship_composition_table_summary(self)
    }
}

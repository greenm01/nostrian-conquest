use crate::{Conquest, CoreGameData, Fleet, GameStateMutationError, Ipbm, Planet, Player, Setup};
use crate::{
    Coords, EmpireId, PlayerMode, Real48, set_armies_aboard, set_empire_name, set_fleet_at_rest,
    set_mission_target, set_player_handle, set_troop_transports,
};

const DEFAULT_EMPIRE_TAX_RATE: u8 = 50;

/// Builder for creating arbitrary ECMAINT-compliant gamestate directories.
///
/// This builder allows constructing gamestates from scratch with configurable
/// parameters, ensuring all cross-file linkage rules are satisfied.
///
/// # Example
/// ```
/// use nc_data::GameStateBuilder;
///
/// let gamestate = GameStateBuilder::new()
///     .with_player_count(4)
///     .with_year(3001)
///     .build_initialized_baseline()
///     .unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct GameStateBuilder {
    player_count: u8,
    game_year: u16,
    homeworld_coords: Vec<[u8; 2]>,
    fleet_orders: Vec<FleetOrderSpec>,
    planet_builds: Vec<PlanetBuildSpec>,
    guard_starbase_orders: Vec<GuardStarbaseSpec>,
    ipbm_count: u16,
}

/// Specification for a fleet order.
#[derive(Debug, Clone)]
pub struct FleetOrderSpec {
    pub fleet_index_1_based: usize,
    pub speed: u8,
    pub order_code: u8,
    pub target: [u8; 2],
    pub aux: [u8; 2],
}

/// Specification for a planet build order.
#[derive(Debug, Clone)]
pub struct PlanetBuildSpec {
    pub planet_index_1_based: usize,
    pub slot: u8,
    pub kind: u8,
}

/// Specification for a guard starbase order.
#[derive(Debug, Clone)]
pub struct GuardStarbaseSpec {
    pub player_index_1_based: usize,
    pub fleet_index_1_based: usize,
    pub target: [u8; 2],
    pub base_id: u16,
}

impl Default for GameStateBuilder {
    fn default() -> Self {
        Self {
            player_count: 4,
            game_year: 3001,
            homeworld_coords: vec![
                [16, 13], // Player 1 homeworld
                [30, 6],  // Player 2 homeworld
                [2, 25],  // Player 3 homeworld
                [26, 26], // Player 4 homeworld
            ],
            fleet_orders: Vec::new(),
            planet_builds: Vec::new(),
            guard_starbase_orders: Vec::new(),
            ipbm_count: 0,
        }
    }
}

impl GameStateBuilder {
    /// Generate a name buffer for a player's homeworld.
    /// "Player N HW", cut to the 11 characters of "Player 1 HW".
    fn homeworld_name_for_player(player_idx: usize) -> String {
        format!("Player {} HW", player_idx + 1)
            .chars()
            .take(11)
            .collect()
    }

    /// Create a new builder with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the number of players.
    pub fn with_player_count(mut self, count: u8) -> Self {
        self.player_count = count.clamp(1, 25);
        // Adjust homeworld coords to match player count
        while self.homeworld_coords.len() < self.player_count as usize {
            self.homeworld_coords.push([0, 0]);
        }
        self.homeworld_coords.truncate(self.player_count as usize);
        self
    }

    /// Set the game year.
    pub fn with_year(mut self, year: u16) -> Self {
        self.game_year = year;
        self
    }

    /// Set homeworld coordinates for all players.
    pub fn with_homeworld_coords(mut self, coords: Vec<[u8; 2]>) -> Self {
        self.homeworld_coords = coords;
        self.player_count = self.homeworld_coords.len().clamp(1, 25) as u8;
        self
    }

    /// Add a fleet order.
    pub fn with_fleet_order(
        mut self,
        fleet_index_1_based: usize,
        speed: u8,
        order_code: u8,
        target: [u8; 2],
        aux: [u8; 2],
    ) -> Self {
        self.fleet_orders.push(FleetOrderSpec {
            fleet_index_1_based,
            speed,
            order_code,
            target,
            aux,
        });
        self
    }

    /// Add a planet build order.
    pub fn with_planet_build(mut self, planet_index_1_based: usize, slot: u8, kind: u8) -> Self {
        self.planet_builds.push(PlanetBuildSpec {
            planet_index_1_based,
            slot,
            kind,
        });
        self
    }

    /// Add a guard starbase order.
    pub fn with_guard_starbase(
        mut self,
        player_index_1_based: usize,
        fleet_index_1_based: usize,
        target: [u8; 2],
        base_id: u16,
    ) -> Self {
        self.guard_starbase_orders.push(GuardStarbaseSpec {
            player_index_1_based,
            fleet_index_1_based,
            target,
            base_id,
        });
        self
    }

    /// Set IPBM count (for all players - currently supports count=0 best).
    pub fn with_ipbm_count(mut self, count: u16) -> Self {
        self.ipbm_count = count;
        self
    }

    /// Build an initialized baseline gamestate.
    ///
    /// This creates a clean post-maint state with:
    /// - Proper fleet blocks for each player
    /// - Homeworld planets configured
    /// - Setup and Conquest headers set
    /// - Empty auxiliary files (BASES.DAT, IPBM.DAT)
    pub fn build_initialized_baseline(&self) -> Result<CoreGameData, GameStateMutationError> {
        let player_records = (0..self.player_count as usize)
            .map(|_| Player::default())
            .collect();
        let planet_records = (0..planet_record_count_for_players(self.player_count))
            .map(|_| Planet::default())
            .collect();

        let mut data = CoreGameData {
            players: player_records,
            planets: planet_records,
            fleets: vec![],
            bases: vec![],
            ipbms: vec![],
            setup: Setup::default(),
            conquest: Conquest::default(),
        };

        // Configure conquest header
        data.conquest.game_year = self.game_year;
        data.conquest.player_count = self.player_count;

        // Configure setup header
        data.setup.version_tag = *b"EC151";
        data.setup.com_irq = [4, 3, 4, 3];
        data.setup.com_hw_flow_control = [true; 4];
        data.setup.snoop = true;
        data.setup.max_key_gap_minutes = 10;
        data.setup.remote_timeout = true;
        data.setup.local_timeout = false;

        // Configure player records
        for (idx, player) in data.players.iter_mut().enumerate() {
            player.mode = PlayerMode::Human;
            player.tax_rate = DEFAULT_EMPIRE_TAX_RATE;
            player.ipbms_first.0 = self.ipbm_count;
            player.autopilot = idx == 0;
        }

        // Configure homeworld planets
        for (player_idx, coords) in self.homeworld_coords.iter().enumerate() {
            if let Some(planet) = data.planets.get_mut(player_idx) {
                *planet = seeded_homeworld(
                    *coords,
                    Self::homeworld_name_for_player(player_idx),
                    (player_idx + 1) as u8,
                );
            }
            // Pre-joined empires get the treasury joining would have set.
            data.seed_first_turn_treasury(player_idx + 1, DEFAULT_EMPIRE_TAX_RATE);
        }

        // Build fleet blocks
        self.build_fleet_blocks(&mut data)?;

        // Setup empty auxiliary state
        data.bases = vec![];
        if self.ipbm_count > 0 {
            data.ipbms = (0..self.ipbm_count).map(|_| Ipbm::default()).collect();
        }

        // Apply any additional orders
        self.apply_orders(&mut data)?;

        Ok(data)
    }

    /// Build a fresh joinable new-game baseline for `ECGAME`.
    ///
    /// This preserves the pre-join homeworld seed semantics from the
    /// `ECUTIL`-initialized baseline:
    /// - player slots remain unjoined/in-civil-disorder
    /// - homeworld seeds remain `Not Named Yet`
    /// - fleet blocks already exist at the seeded homeworld coords
    pub fn build_joinable_new_game_baseline(&self) -> Result<CoreGameData, GameStateMutationError> {
        let player_records = (0..self.player_count as usize)
            .map(|_| Player::default())
            .collect();
        let planet_records = (0..planet_record_count_for_players(self.player_count))
            .map(|_| Planet::default())
            .collect();

        let mut data = CoreGameData {
            players: player_records,
            planets: planet_records,
            fleets: vec![],
            bases: vec![],
            ipbms: vec![],
            setup: Setup::default(),
            conquest: Conquest::default(),
        };

        // ECUTIL F1 enables every day and writes 100 and 1 to all 25
        // ranking slots.
        data.conquest.game_year = self.game_year;
        data.conquest.player_count = self.player_count;
        data.conquest.maintenance_days.0 = [true; 7];
        data.conquest.previous_production = [100; 25];
        data.conquest.previous_planet_count = [1; 25];

        data.setup.version_tag = *b"EC151";
        data.setup.com_irq = [4, 3, 4, 3];
        data.setup.com_hw_flow_control = [true; 4];
        data.setup.snoop = true;
        data.setup.max_key_gap_minutes = 10;
        data.setup.remote_timeout = true;
        data.setup.local_timeout = false;

        for (idx, player) in data.players.iter_mut().enumerate() {
            let homeworld_planet_index_1_based = idx + 1;
            let fleet_start = idx * 4 + 1;
            let fleet_end = fleet_start + 3;
            seed_unjoined_player_slot(
                player,
                fleet_start as u16,
                fleet_end as u16,
                homeworld_planet_index_1_based as u8,
                self.ipbm_count,
            );
        }

        for (player_idx, coords) in self.homeworld_coords.iter().enumerate() {
            if let Some(planet) = data.planets.get_mut(player_idx) {
                seed_unjoined_homeworld_seed(planet, *coords, (player_idx + 1) as u8);
            }
        }

        self.build_fleet_blocks(&mut data)?;
        self.apply_orders(&mut data)?;

        Ok(data)
    }

    /// Build fleet blocks for all players.
    fn build_fleet_blocks(&self, data: &mut CoreGameData) -> Result<(), GameStateMutationError> {
        let player_count = self.player_count as usize;
        let expected_fleet_count = player_count * 4;

        let mut records = Vec::with_capacity(expected_fleet_count);

        for block_idx in 0..player_count {
            let coords = self
                .homeworld_coords
                .get(block_idx)
                .copied()
                .unwrap_or([0, 0]);

            for slot_idx in 0..4 {
                let fleet_record_index_1_based = block_idx * 4 + slot_idx + 1;
                let mut record = Fleet::default();
                let fleet_id = fleet_record_index_1_based as u16;
                let local_slot = (slot_idx + 1) as u16;
                let owner_empire = (block_idx + 1) as u8;
                let prev = if slot_idx == 0 { 0 } else { fleet_id - 1 };
                let next = if slot_idx == 3 { 0 } else { fleet_id + 1 };

                record.fleet_number.0 = local_slot;
                record.owner.0 = owner_empire;
                record.next.0 = next;
                record.record.0 = fleet_id;
                record.prev.0 = prev;
                record.max_speed = if slot_idx < 2 { 3 } else { 6 };
                record.speed = 0;
                record.sector = Coords::from_array(coords);
                set_fleet_at_rest(&mut record);
                record.order = crate::Order::GuardBlockadeWorld;
                record.target = Coords::from_array(coords);
                set_mission_target(&mut record, 1, 0);
                record.ships.scouts = 0;
                record.rules_of_engagement = 6;
                record.ships.battleships = 0;
                record.ships.cruisers = if slot_idx < 2 { 1 } else { 0 };
                record.ships.destroyers = if slot_idx < 2 { 0 } else { 1 };
                set_troop_transports(&mut record, 0);
                set_armies_aboard(&mut record, 0);
                record.ships.etacs = if slot_idx < 2 { 1 } else { 0 };

                records.push(record);
            }
        }

        data.fleets = records;
        Ok(())
    }

    /// Apply all configured orders to the gamestate.
    fn apply_orders(&self, data: &mut CoreGameData) -> Result<(), GameStateMutationError> {
        // Apply fleet orders
        for order in &self.fleet_orders {
            ensure_planet_target_for_order(data, order.order_code, order.target);
            data.set_fleet_order(
                order.fleet_index_1_based,
                order.speed,
                order.order_code,
                order.target,
                Some(order.aux[0]),
                Some(order.aux[1]),
            )?;
        }

        // Apply planet builds
        for build in &self.planet_builds {
            data.set_planet_build(build.planet_index_1_based, build.slot, build.kind)?;
        }

        // Apply guard starbase orders
        for guard in &self.guard_starbase_orders {
            data.set_guard_starbase(
                guard.player_index_1_based,
                guard.fleet_index_1_based,
                guard.target,
                guard.base_id,
                guard.player_index_1_based as u8,
            )?;
        }

        Ok(())
    }
}

/// Reset a mid-game player slot back to its pre-join (civil disorder) baseline.
///
/// Used when a Sandbox game ejects an MIA player to open the seat for a new player.
/// All of the following are reset:
/// - Player record → civil disorder, cleared handle, autopilot on, starter fleet chain
/// - Homeworld planet → "Not Named Yet" with starting stats (coords preserved)
/// - All other planets owned by this empire → released to neutral (coords preserved)
/// - Starter fleet block (4 records) → rebuilt at homeworld coords
/// - Any extra fleets owned by this empire → zeroed (culled at next maintenance)
pub fn reset_player_slot_to_baseline(
    data: &mut CoreGameData,
    player_index_1_based: usize,
) -> Result<(), GameStateMutationError> {
    if player_index_1_based == 0 || player_index_1_based > data.players.len() {
        return Err(GameStateMutationError::MissingPlayerRecord {
            index_1_based: player_index_1_based,
        });
    }

    let player_idx = player_index_1_based - 1;
    let fleet_start = player_idx * 4 + 1;
    let fleet_end = fleet_start + 3;
    let homeworld_planet_index_1_based = (player_idx + 1) as u8;

    // Capture homeworld coords before touching planet records.
    let homeworld_coords = data
        .planets
        .get(player_idx)
        .map(|p| p.sector.to_array())
        .unwrap_or([0, 0]);

    // Reset the player record.
    let ipbm_count = data
        .players
        .get(player_idx)
        .map(|p| p.ipbms_first.0)
        .unwrap_or(0);
    if let Some(player) = data.players.get_mut(player_idx) {
        seed_unjoined_player_slot(
            player,
            fleet_start as u16,
            fleet_end as u16,
            homeworld_planet_index_1_based,
            ipbm_count,
        );
    }

    // Reset the homeworld planet.
    if let Some(planet) = data.planets.get_mut(player_idx) {
        seed_unjoined_homeworld_seed(planet, homeworld_coords, player_index_1_based as u8);
    }

    // Release all other planets owned by this empire to neutral.
    for (planet_idx, planet) in data.planets.iter_mut().enumerate() {
        if planet_idx == player_idx {
            continue; // homeworld already handled
        }
        if planet.owner.0 == player_index_1_based as u8 {
            planet.owner.0 = 0;
            planet.armies = 0;
            planet.ground_batteries = 0;
            planet.years_under_owner = 0;
        }
    }

    // Zero all fleet records beyond the starter block that belong to this empire.
    let empire_id = player_index_1_based as u8;
    for (fleet_idx, fleet) in data.fleets.iter_mut().enumerate() {
        let fleet_id = fleet_idx + 1; // 1-based
        let in_starter_block = fleet_id >= fleet_start && fleet_id <= fleet_end;
        if !in_starter_block && fleet.owner.0 == empire_id {
            *fleet = Fleet::default();
        }
    }

    // Rebuild the 4-slot starter fleet block at the homeworld coords.
    for slot_idx in 0..4 {
        let fleet_record_index_1_based = fleet_start + slot_idx;
        let fleet_array_idx = fleet_record_index_1_based - 1;
        if let Some(record) = data.fleets.get_mut(fleet_array_idx) {
            let fleet_id = fleet_record_index_1_based as u16;
            let local_slot = (slot_idx + 1) as u16;
            let prev = if slot_idx == 0 { 0 } else { fleet_id - 1 };
            let next = if slot_idx == 3 { 0 } else { fleet_id + 1 };

            *record = Fleet::default();
            record.fleet_number.0 = local_slot;
            record.owner.0 = empire_id;
            record.next.0 = next;
            record.record.0 = fleet_id;
            record.prev.0 = prev;
            record.max_speed = if slot_idx < 2 { 3 } else { 6 };
            record.speed = 0;
            record.sector = Coords::from_array(homeworld_coords);
            set_fleet_at_rest(record);
            record.order = crate::Order::GuardBlockadeWorld;
            record.target = Coords::from_array(homeworld_coords);
            set_mission_target(record, 1, 0);
            record.ships.scouts = 0;
            record.rules_of_engagement = 6;
            record.ships.battleships = 0;
            record.ships.cruisers = if slot_idx < 2 { 1 } else { 0 };
            record.ships.destroyers = if slot_idx < 2 { 0 } else { 1 };
            set_troop_transports(record, 0);
            set_armies_aboard(record, 0);
            record.ships.etacs = if slot_idx < 2 { 1 } else { 0 };
        }
    }

    Ok(())
}

fn fleet_order_requires_planet_target(order_code: u8) -> bool {
    matches!(order_code, 5 | 6 | 7 | 8 | 9 | 11 | 12 | 15)
}

fn ensure_planet_target_for_order(data: &mut CoreGameData, order_code: u8, coords: [u8; 2]) {
    if !fleet_order_requires_planet_target(order_code)
        || data
            .planets
            .iter()
            .any(|planet| planet.sector.to_array() == coords)
    {
        return;
    }

    if let Some(target) = data
        .planets
        .iter_mut()
        .find(|planet| planet.owner.0 == 0 && planet.sector.to_array() == [0, 0])
    {
        target.sector = Coords::from_array(coords);
    }
}

fn planet_record_count_for_players(player_count: u8) -> usize {
    (player_count as usize) * 5
}

fn seed_unjoined_player_slot(
    player: &mut Player,
    fleet_start: u16,
    fleet_end: u16,
    homeworld_planet_index_1_based: u8,
    ipbm_count: u16,
) {
    *player = Player::default();
    player.mode = PlayerMode::CivilDisorder;
    set_player_handle(player, "");
    // ECUTIL F1 writes 24 lines per screen.
    player.page_lines = 24;
    set_empire_name(player, "In Civil Disorder");
    player.fleets_first.0 = fleet_start;
    player.fleets_last.0 = fleet_end;
    player.homeworld = homeworld_planet_index_1_based;
    player.planet_count = 0x01;
    player.production = 100;
    player.tax_rate = DEFAULT_EMPIRE_TAX_RATE;
    player.ipbms_first.0 = ipbm_count;
    player.autopilot = true;
}

fn seed_unjoined_homeworld_seed(planet: &mut Planet, coords: [u8; 2], owner_empire_slot: u8) {
    *planet = seeded_homeworld(coords, "Not Named Yet".to_string(), owner_empire_slot);
}

/// An owned homeworld as ECUTIL seeds one: potential and present production
/// 100, 35 stored goods, 10 armies, 4 ground batteries, fully owned.
fn seeded_homeworld(coords: [u8; 2], name: String, owner_empire_slot: u8) -> Planet {
    Planet {
        sector: Coords::from_array(coords),
        potential_production: 100,
        production: Real48::from_int(100),
        stored_goods: Real48::from_int(35),
        name,
        armies: 10,
        ground_batteries: 4,
        years_under_owner: 2,
        owner: EmpireId(owner_empire_slot),
        ..Planet::default()
    }
}

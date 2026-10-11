use nc_data::classic::{parse_conquest_dat, parse_setup_dat};
use nc_data::{Conquest, CoreGameData, GameStateBuilder, Planet, Player, Setup, Starbase};

pub fn zeroed_setup() -> Setup {
    parse_setup_dat(&vec![0; nc_data::SETUP_DAT_SIZE]).expect("zeroed setup should parse")
}

pub fn zeroed_conquest() -> Conquest {
    parse_conquest_dat(&vec![0; nc_data::CONQUEST_DAT_SIZE]).expect("zeroed conquest should parse")
}

pub fn configured_conquest(player_count: u8) -> Conquest {
    let mut conquest = zeroed_conquest();
    conquest.set_game_year(3000);
    conquest.set_player_count(player_count);
    conquest.set_maintenance_schedule_enabled([true; 7]);
    conquest
}

pub fn player_with_empire_name(name: &str, tax_rate: u8, stored_points: u16) -> Player {
    let mut record = Player::default();
    record.set_owner_empire_raw(1);
    record.set_tax_rate_raw(tax_rate);
    record.set_last_run_year_raw(stored_points);
    record.set_controlled_empire_name_raw(name);
    record
}

pub fn owned_planet(
    owner_empire_slot: u8,
    potential_production: u8,
    present_production: f64,
    stored_points: u32,
    armies: u8,
    batteries: u8,
) -> Planet {
    let mut record = Planet::default();
    record.set_owner_empire_slot_raw(owner_empire_slot);
    // A regular world, not one just captured.
    record.set_ownership_status_raw(2);
    record.set_potential_production_points(potential_production);
    record.set_present_production(present_production);
    record.set_stored_production_points(stored_points);
    record.set_army_count_raw(armies);
    record.set_ground_batteries_raw(batteries);
    record
}

pub fn owned_planet_with_present_production(
    owner_empire_slot: u8,
    potential_production: u8,
    present_production: u16,
    stored_points: u32,
    armies: u8,
    batteries: u8,
) -> Planet {
    owned_planet(
        owner_empire_slot,
        potential_production,
        f64::from(present_production),
        stored_points,
        armies,
        batteries,
    )
}

pub fn owned_homeworld_seed(
    owner_empire_slot: u8,
    potential_production: u8,
    present_production: f64,
    armies: u8,
    batteries: u8,
) -> Planet {
    let mut record = owned_planet(
        owner_empire_slot,
        potential_production,
        present_production,
        0,
        armies,
        batteries,
    );
    record.set_ownership_status_raw(2);
    record.set_planet_name("Not Named Yet");
    record
}

pub fn single_planet_game(player: Player, planet: Planet) -> CoreGameData {
    CoreGameData {
        players: vec![player],
        planets: vec![planet],
        fleets: vec![],
        bases: vec![],
        ipbms: vec![],
        setup: zeroed_setup(),
        conquest: configured_conquest(1),
    }
}

pub fn commissioned_starbase(owner_empire_raw: u8, coords: [u8; 2]) -> Starbase {
    let mut base = Starbase::default();
    base.set_base_id_raw(1);
    base.set_owner_empire_raw(owner_empire_raw);
    base.set_coords_raw(coords);
    // At rest in orbit, as a commissioned base starts.
    base.set_at_rest_motion();
    base.set_target_coords_raw(coords);
    base.set_target_depth_raw(1);
    base
}

pub fn joinable_single_player_game() -> CoreGameData {
    GameStateBuilder::new()
        .with_player_count(1)
        .build_joinable_new_game_baseline()
        .expect("joinable baseline should build")
}

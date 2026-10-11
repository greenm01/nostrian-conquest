use nc_data::classic::{parse_conquest_dat, parse_setup_dat};
use nc_data::{
    Conquest, Coords, CoreGameData, GameStateBuilder, Planet, Player, PlayerMode, Real48, Setup,
    Starbase, set_empire_name, set_planet_name, set_starbase_at_rest,
};

pub fn zeroed_setup() -> Setup {
    parse_setup_dat(&vec![0; nc_data::SETUP_DAT_SIZE]).expect("zeroed setup should parse")
}

pub fn zeroed_conquest() -> Conquest {
    parse_conquest_dat(&vec![0; nc_data::CONQUEST_DAT_SIZE]).expect("zeroed conquest should parse")
}

pub fn configured_conquest(player_count: u8) -> Conquest {
    let mut conquest = zeroed_conquest();
    conquest.game_year = 3000;
    conquest.player_count = player_count;
    conquest.maintenance_days.0 = [true; 7];
    conquest
}

pub fn player_with_empire_name(name: &str, tax_rate: u8, stored_points: u16) -> Player {
    let mut record = Player {
        mode: PlayerMode::Human,
        tax_rate,
        last_year_on: stored_points,
        ..Player::default()
    };
    set_empire_name(&mut record, name);
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
    record.owner.0 = owner_empire_slot;
    // A regular world, not one just captured.
    record.years_under_owner = 2;
    record.potential_production = potential_production;
    record.production = Real48::from_f64(present_production);
    record.stored_goods = Real48::from_f64(f64::from(stored_points));
    record.armies = u16::from(armies);
    record.ground_batteries = u16::from(batteries);
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
    record.years_under_owner = 2;
    set_planet_name(&mut record, "Not Named Yet");
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
    base.base_id.0 = 1;
    base.owner.0 = owner_empire_raw;
    base.sector = Coords::from_array(coords);
    // At rest in orbit, as a commissioned base starts.
    set_starbase_at_rest(&mut base);
    base.target = Coords::from_array(coords);
    base.target_depth = 1;
    base
}

pub fn joinable_single_player_game() -> CoreGameData {
    GameStateBuilder::new()
        .with_player_count(1)
        .build_joinable_new_game_baseline()
        .expect("joinable baseline should build")
}

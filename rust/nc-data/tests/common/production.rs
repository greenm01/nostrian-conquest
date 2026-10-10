use nc_data::{
    BaseDat, BaseRecord, ConquestDat, CoreGameData, FleetDat, GameStateBuilder, IpbmDat, PlanetDat,
    PlanetRecord, PlayerDat, PlayerRecord, SetupDat, encode_real48,
};

pub fn zeroed_setup() -> SetupDat {
    SetupDat::parse(&vec![0; nc_data::SETUP_DAT_SIZE]).expect("zeroed setup should parse")
}

pub fn zeroed_conquest() -> ConquestDat {
    ConquestDat::parse(&vec![0; nc_data::CONQUEST_DAT_SIZE]).expect("zeroed conquest should parse")
}

pub fn configured_conquest(player_count: u8) -> ConquestDat {
    let mut conquest = zeroed_conquest();
    conquest.set_game_year(3000);
    conquest.set_player_count(player_count);
    conquest.set_maintenance_schedule_enabled([true; 7]);
    conquest
}

pub fn player_with_empire_name(name: &str, tax_rate: u8, stored_points: u16) -> PlayerRecord {
    let mut record = PlayerRecord::new_zeroed();
    record.set_owner_empire_raw(1);
    record.set_tax_rate_raw(tax_rate);
    record.raw[0x4E..0x50].copy_from_slice(&stored_points.to_le_bytes());
    let bytes = name.as_bytes();
    let len = bytes.len().min(19);
    record.raw[27] = len as u8;
    record.raw[28..28 + len].copy_from_slice(&bytes[..len]);
    record
}

pub fn owned_planet(
    owner_empire_slot: u8,
    potential_production: u8,
    factories_real48: [u8; 6],
    stored_points: u32,
    armies: u8,
    batteries: u8,
) -> PlanetRecord {
    let mut record = PlanetRecord::new_zeroed();
    record.set_owner_empire_slot_raw(owner_empire_slot);
    record.set_potential_production_raw([potential_production, 0]);
    record.set_factories_raw(factories_real48);
    record.set_stored_goods_raw(stored_points);
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
) -> PlanetRecord {
    owned_planet(
        owner_empire_slot,
        potential_production,
        encode_real48(f64::from(present_production)).expect("present production should encode"),
        stored_points,
        armies,
        batteries,
    )
}

pub fn owned_homeworld_seed(
    owner_empire_slot: u8,
    potential_production: u8,
    factories_real48: [u8; 6],
    armies: u8,
    batteries: u8,
) -> PlanetRecord {
    let mut record = owned_planet(
        owner_empire_slot,
        potential_production,
        factories_real48,
        0,
        armies,
        batteries,
    );
    record.set_potential_production_raw([potential_production, 0x87]);
    record.set_ownership_status_raw(2);
    record.set_planet_name("Not Named Yet");
    record
}

pub fn single_planet_game(player: PlayerRecord, planet: PlanetRecord) -> CoreGameData {
    CoreGameData {
        player: PlayerDat {
            records: vec![player],
        },
        planets: PlanetDat {
            records: vec![planet],
        },
        fleets: FleetDat { records: vec![] },
        bases: BaseDat { records: vec![] },
        ipbm: IpbmDat { records: vec![] },
        setup: zeroed_setup(),
        conquest: configured_conquest(1),
    }
}

pub fn commissioned_starbase(owner_empire_raw: u8, coords: [u8; 2]) -> BaseRecord {
    let mut base = BaseRecord::new_zeroed();
    base.set_active_flag_raw(1);
    base.set_owner_empire_raw(owner_empire_raw);
    base.set_coords_raw(coords);
    base
}

pub fn joinable_single_player_game() -> CoreGameData {
    GameStateBuilder::new()
        .with_player_count(1)
        .build_joinable_new_game_baseline()
        .expect("joinable baseline should build")
}

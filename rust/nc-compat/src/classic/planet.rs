use super::PLANET_RECORD_SIZE;
use nc_classic::ParseError;
use nc_data::{BuildSlot, Coords, EmpireId, Planet, ProductionItemKind, Real48, Stardock};

use super::{
    pascal_string, put_pascal_string, put_real, put_u16, put_u32, real_at, u16_at, u32_at,
};

/// The unit classic's queue slot holds, in classic's order: 1 destroyer,
/// 2 cruiser, 3 battleship, 4 scout, 5 transport, 6 ETAC, 7 starbase,
/// 8 reserved, 9 army, 10 ground battery. Rust numbers its kinds differently,
/// so the two map by name.
const CLASSIC_QUEUE_KINDS: [Option<ProductionItemKind>; 10] = [
    Some(ProductionItemKind::Destroyer),
    Some(ProductionItemKind::Cruiser),
    Some(ProductionItemKind::Battleship),
    Some(ProductionItemKind::Scout),
    Some(ProductionItemKind::Transport),
    Some(ProductionItemKind::Etac),
    Some(ProductionItemKind::Starbase),
    None,
    Some(ProductionItemKind::Army),
    Some(ProductionItemKind::GroundBattery),
];

/// Decode a record. A classic queue holds counts of units, which neither
/// side has paid for yet, so a count `n` of a kind becomes `n * cost` points
/// owed and the treasury stays as it is. Orders in the reserved slot 8 (only
/// a file edit fills it) have no Rust unit and are refused, naming the
/// planet.
pub fn decode_planet(bytes: &[u8; PLANET_RECORD_SIZE]) -> Result<Planet, ParseError> {
    let mut build_queue = [BuildSlot::EMPTY; 10];
    let mut next_slot = 0;
    for (index, kind) in CLASSIC_QUEUE_KINDS.iter().enumerate() {
        let units = u16_at(bytes, 0x24 + 2 * index);
        if units == 0 {
            continue;
        }
        let Some(kind) = *kind else {
            return Err(ParseError::ReservedBuildSlot {
                planet: pascal_string(bytes, 0x0F, 20),
                sector: [bytes[0x00], bytes[0x01]],
                units,
            });
        };
        build_queue[next_slot] = BuildSlot {
            points_owed: u32::from(units) * kind.build_cost().expect("queue kinds have a cost"),
            kind,
        };
        next_slot += 1;
    }
    let mut dock = [0u32; 8];
    for (slot, count) in dock.iter_mut().enumerate() {
        *count = u32_at(bytes, 0x38 + 4 * slot);
    }
    Ok(Planet {
        sector: Coords::new(bytes[0x00], bytes[0x01]),
        potential_production: bytes[0x02],
        production: real_at(bytes, 0x03),
        stored_goods: real_at(bytes, 0x09),
        name: pascal_string(bytes, 0x0F, 20),
        build_queue,
        stardock: Stardock::from_counts(dock),
        armies: u16_at(bytes, 0x58),
        ground_batteries: u16_at(bytes, 0x5A),
        years_under_owner: bytes[0x5C],
        owner: EmpireId(bytes[0x5D]),
        scorch_earth: bytes[0x60] != 0,
        // PLANETS.DAT has no crippled batteries; a classic world has none.
        crippled_batteries: 0,
    })
}

/// Encode a record. Each kind's owed points become `ceil(points / cost)`
/// units, since ECMAINT charges whole units. A slot that is part paid (Rust's
/// pass pays by the point) has `ceil(p / c) * c - p` of its last unit paid
/// already; that much is credited to the stored goods, so ECMAINT's charge
/// leaves the treasury where Rust's own pass would. Queue slots whose kind
/// has no classic unit are not written.
pub fn encode_planet(planet: &Planet) -> [u8; PLANET_RECORD_SIZE] {
    let mut bytes = [0u8; PLANET_RECORD_SIZE];
    bytes[0x00] = planet.sector.x;
    bytes[0x01] = planet.sector.y;
    bytes[0x02] = planet.potential_production;
    put_real(&mut bytes, 0x03, planet.production);
    let mut units = [0u32; 10];
    let mut paid_ahead = 0u32;
    for entry in &planet.build_queue {
        let Some(index) = CLASSIC_QUEUE_KINDS
            .iter()
            .position(|kind| *kind == Some(entry.kind))
        else {
            continue;
        };
        let cost = entry.kind.build_cost().expect("queue kinds have a cost");
        let owed_units = entry.points_owed.div_ceil(cost);
        units[index] += owed_units;
        paid_ahead += owed_units * cost - entry.points_owed;
    }
    let stored_goods = if paid_ahead == 0 {
        planet.stored_goods
    } else {
        Real48::from_f64(planet.stored_goods.to_f64() + f64::from(paid_ahead))
    };
    put_real(&mut bytes, 0x09, stored_goods);
    put_pascal_string(&mut bytes, 0x0F, 20, &planet.name);
    for (index, count) in units.into_iter().enumerate() {
        put_u16(
            &mut bytes,
            0x24 + 2 * index,
            u16::try_from(count).unwrap_or(u16::MAX),
        );
    }
    for (slot, count) in planet.stardock.to_counts().into_iter().enumerate() {
        put_u32(&mut bytes, 0x38 + 4 * slot, count);
    }
    put_u16(&mut bytes, 0x58, planet.armies);
    put_u16(&mut bytes, 0x5A, planet.ground_batteries);
    bytes[0x5C] = planet.years_under_owner;
    bytes[0x5D] = planet.owner.0;
    bytes[0x60] = u8::from(planet.scorch_earth);
    bytes
}

pub fn parse_planets_dat(data: &[u8]) -> Result<Vec<Planet>, ParseError> {
    if !data.len().is_multiple_of(PLANET_RECORD_SIZE) {
        return Err(ParseError::WrongRecordMultiple {
            file_type: "PLANETS.DAT",
            record_size: PLANET_RECORD_SIZE,
            actual: data.len(),
        });
    }
    data.as_chunks::<PLANET_RECORD_SIZE>()
        .0
        .iter()
        .map(decode_planet)
        .collect()
}

pub fn serialize_planets_dat(planets: &[Planet]) -> Vec<u8> {
    planets.iter().flat_map(encode_planet).collect()
}

use super::PLANET_RECORD_SIZE;
use nc_classic::ParseError;
use nc_data::{BuildSlot, Coords, EmpireId, Planet, ProductionItemKind, Stardock};

use super::{
    pascal_string, put_pascal_string, put_real, put_u16, put_u32, real_at, u16_at, u32_at,
};

pub fn decode_planet(bytes: &[u8; PLANET_RECORD_SIZE]) -> Planet {
    // Rust's slots, not classic's counts by kind; see `Planet::build_queue`.
    let build_queue = std::array::from_fn(|slot| BuildSlot {
        points_owed: bytes[0x24 + slot],
        kind: ProductionItemKind::from_code(bytes[0x2E + slot]),
    });
    let mut dock = [0u32; 8];
    for (slot, count) in dock.iter_mut().enumerate() {
        *count = u32_at(bytes, 0x38 + 4 * slot);
    }
    Planet {
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
    }
}

pub fn encode_planet(planet: &Planet) -> [u8; PLANET_RECORD_SIZE] {
    let mut bytes = [0u8; PLANET_RECORD_SIZE];
    bytes[0x00] = planet.sector.x;
    bytes[0x01] = planet.sector.y;
    bytes[0x02] = planet.potential_production;
    put_real(&mut bytes, 0x03, planet.production);
    put_real(&mut bytes, 0x09, planet.stored_goods);
    put_pascal_string(&mut bytes, 0x0F, 20, &planet.name);
    for (slot, entry) in planet.build_queue.iter().enumerate() {
        bytes[0x24 + slot] = entry.points_owed;
        bytes[0x2E + slot] = entry.kind.code();
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
    Ok(data
        .as_chunks::<PLANET_RECORD_SIZE>()
        .0
        .iter()
        .map(decode_planet)
        .collect())
}

pub fn serialize_planets_dat(planets: &[Planet]) -> Vec<u8> {
    planets.iter().flat_map(encode_planet).collect()
}

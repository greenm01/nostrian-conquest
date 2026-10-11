use super::FLEET_RECORD_SIZE;
use nc_classic::ParseError;
use nc_data::{Coords, EmpireId, Fleet, FleetId, Order, RecordNumber, ShipCounts};

use super::{put_real, put_u16, real_at, u16_at};

pub fn decode_fleet(bytes: &[u8; FLEET_RECORD_SIZE]) -> Fleet {
    Fleet {
        fleet_number: FleetId(u16_at(bytes, 0x00)),
        owner: EmpireId(bytes[0x02]),
        next: RecordNumber(u16_at(bytes, 0x03)),
        record: RecordNumber(u16_at(bytes, 0x05)),
        prev: RecordNumber(u16_at(bytes, 0x07)),
        max_speed: bytes[0x09],
        speed: bytes[0x0A],
        sector: Coords::new(bytes[0x0B], bytes[0x0C]),
        x_fraction: real_at(bytes, 0x0D),
        y_fraction: real_at(bytes, 0x13),
        depth: real_at(bytes, 0x19),
        order: Order::from_code(bytes[0x1F]),
        target: Coords::new(bytes[0x20], bytes[0x21]),
        target_depth: bytes[0x22],
        mission_target: u16_at(bytes, 0x23),
        rules_of_engagement: bytes[0x25],
        ships: ShipCounts {
            battleships: u16_at(bytes, 0x26),
            cruisers: u16_at(bytes, 0x28),
            destroyers: u16_at(bytes, 0x2A),
            empty_transports: u16_at(bytes, 0x2C),
            loaded_transports: u16_at(bytes, 0x2E),
            etacs: u16_at(bytes, 0x30),
            scouts: u16_at(bytes, 0x32),
        },
    }
}

/// The dead strength word at 0x34 encodes as zero.
pub fn encode_fleet(fleet: &Fleet) -> [u8; FLEET_RECORD_SIZE] {
    let mut bytes = [0u8; FLEET_RECORD_SIZE];
    put_u16(&mut bytes, 0x00, fleet.fleet_number.0);
    bytes[0x02] = fleet.owner.0;
    put_u16(&mut bytes, 0x03, fleet.next.0);
    put_u16(&mut bytes, 0x05, fleet.record.0);
    put_u16(&mut bytes, 0x07, fleet.prev.0);
    bytes[0x09] = fleet.max_speed;
    bytes[0x0A] = fleet.speed;
    bytes[0x0B] = fleet.sector.x;
    bytes[0x0C] = fleet.sector.y;
    put_real(&mut bytes, 0x0D, fleet.x_fraction);
    put_real(&mut bytes, 0x13, fleet.y_fraction);
    put_real(&mut bytes, 0x19, fleet.depth);
    bytes[0x1F] = fleet.order.code();
    bytes[0x20] = fleet.target.x;
    bytes[0x21] = fleet.target.y;
    bytes[0x22] = fleet.target_depth;
    put_u16(&mut bytes, 0x23, fleet.mission_target);
    bytes[0x25] = fleet.rules_of_engagement;
    put_u16(&mut bytes, 0x26, fleet.ships.battleships);
    put_u16(&mut bytes, 0x28, fleet.ships.cruisers);
    put_u16(&mut bytes, 0x2A, fleet.ships.destroyers);
    put_u16(&mut bytes, 0x2C, fleet.ships.empty_transports);
    put_u16(&mut bytes, 0x2E, fleet.ships.loaded_transports);
    put_u16(&mut bytes, 0x30, fleet.ships.etacs);
    put_u16(&mut bytes, 0x32, fleet.ships.scouts);
    bytes
}

pub fn parse_fleets_dat(data: &[u8]) -> Result<Vec<Fleet>, ParseError> {
    if !data.len().is_multiple_of(FLEET_RECORD_SIZE) {
        return Err(ParseError::WrongRecordMultiple {
            file_type: "FLEETS.DAT",
            record_size: FLEET_RECORD_SIZE,
            actual: data.len(),
        });
    }
    Ok(data
        .as_chunks::<FLEET_RECORD_SIZE>()
        .0
        .iter()
        .map(decode_fleet)
        .collect())
}

pub fn serialize_fleets_dat(fleets: &[Fleet]) -> Vec<u8> {
    fleets.iter().flat_map(encode_fleet).collect()
}

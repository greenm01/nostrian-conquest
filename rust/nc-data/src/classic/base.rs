use crate::BASE_RECORD_SIZE;
use crate::model::{Coords, EmpireId, FleetId, RecordNumber, Starbase, StarbaseId};
use crate::support::ParseError;

use super::{put_real, put_u16, real_at, u16_at};

pub fn decode_base(bytes: &[u8; BASE_RECORD_SIZE]) -> Starbase {
    Starbase {
        escort: FleetId(u16_at(bytes, 0x00)),
        base_id: StarbaseId(u16_at(bytes, 0x02)),
        owner: EmpireId(bytes[0x04]),
        next: RecordNumber(u16_at(bytes, 0x05)),
        record: RecordNumber(u16_at(bytes, 0x07)),
        prev: RecordNumber(u16_at(bytes, 0x09)),
        sector: Coords::new(bytes[0x0B], bytes[0x0C]),
        x_fraction: real_at(bytes, 0x0D),
        y_fraction: real_at(bytes, 0x13),
        depth: real_at(bytes, 0x19),
        moving: bytes[0x1F] != 0,
        target: Coords::new(bytes[0x20], bytes[0x21]),
        target_depth: bytes[0x22],
    }
}

pub fn encode_base(base: &Starbase) -> [u8; BASE_RECORD_SIZE] {
    let mut bytes = [0u8; BASE_RECORD_SIZE];
    put_u16(&mut bytes, 0x00, base.escort.0);
    put_u16(&mut bytes, 0x02, base.base_id.0);
    bytes[0x04] = base.owner.0;
    put_u16(&mut bytes, 0x05, base.next.0);
    put_u16(&mut bytes, 0x07, base.record.0);
    put_u16(&mut bytes, 0x09, base.prev.0);
    bytes[0x0B] = base.sector.x;
    bytes[0x0C] = base.sector.y;
    put_real(&mut bytes, 0x0D, base.x_fraction);
    put_real(&mut bytes, 0x13, base.y_fraction);
    put_real(&mut bytes, 0x19, base.depth);
    bytes[0x1F] = u8::from(base.moving);
    bytes[0x20] = base.target.x;
    bytes[0x21] = base.target.y;
    bytes[0x22] = base.target_depth;
    bytes
}

pub fn parse_bases_dat(data: &[u8]) -> Result<Vec<Starbase>, ParseError> {
    if !data.len().is_multiple_of(BASE_RECORD_SIZE) {
        return Err(ParseError::WrongRecordMultiple {
            file_type: "BASES.DAT",
            record_size: BASE_RECORD_SIZE,
            actual: data.len(),
        });
    }
    Ok(data
        .as_chunks::<BASE_RECORD_SIZE>()
        .0
        .iter()
        .map(decode_base)
        .collect())
}

pub fn serialize_bases_dat(bases: &[Starbase]) -> Vec<u8> {
    bases.iter().flat_map(encode_base).collect()
}

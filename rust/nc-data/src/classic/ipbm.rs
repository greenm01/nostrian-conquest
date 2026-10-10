use crate::IPBM_RECORD_SIZE;
use crate::model::{Coords, EmpireId, Ipbm, RecordNumber};
use crate::support::ParseError;

use super::{put_real, put_u16, real_at, u16_at};

pub fn decode_ipbm(bytes: &[u8; IPBM_RECORD_SIZE]) -> Ipbm {
    Ipbm {
        ipbm_number: u16_at(bytes, 0x00),
        owner: EmpireId(bytes[0x02]),
        next: RecordNumber(u16_at(bytes, 0x03)),
        record: RecordNumber(u16_at(bytes, 0x05)),
        prev: RecordNumber(u16_at(bytes, 0x07)),
        sector: Coords::new(bytes[0x09], bytes[0x0A]),
        x_fraction: real_at(bytes, 0x0B),
        y_fraction: real_at(bytes, 0x11),
        depth: real_at(bytes, 0x17),
        target: Coords::new(bytes[0x1D], bytes[0x1E]),
        target_depth: bytes[0x1F],
    }
}

pub fn encode_ipbm(ipbm: &Ipbm) -> [u8; IPBM_RECORD_SIZE] {
    let mut bytes = [0u8; IPBM_RECORD_SIZE];
    put_u16(&mut bytes, 0x00, ipbm.ipbm_number);
    bytes[0x02] = ipbm.owner.0;
    put_u16(&mut bytes, 0x03, ipbm.next.0);
    put_u16(&mut bytes, 0x05, ipbm.record.0);
    put_u16(&mut bytes, 0x07, ipbm.prev.0);
    bytes[0x09] = ipbm.sector.x;
    bytes[0x0A] = ipbm.sector.y;
    put_real(&mut bytes, 0x0B, ipbm.x_fraction);
    put_real(&mut bytes, 0x11, ipbm.y_fraction);
    put_real(&mut bytes, 0x17, ipbm.depth);
    bytes[0x1D] = ipbm.target.x;
    bytes[0x1E] = ipbm.target.y;
    bytes[0x1F] = ipbm.target_depth;
    bytes
}

pub fn parse_ipbm_dat(data: &[u8]) -> Result<Vec<Ipbm>, ParseError> {
    if !data.len().is_multiple_of(IPBM_RECORD_SIZE) {
        return Err(ParseError::WrongRecordMultiple {
            file_type: "IPBM.DAT",
            record_size: IPBM_RECORD_SIZE,
            actual: data.len(),
        });
    }
    Ok(data
        .as_chunks::<IPBM_RECORD_SIZE>()
        .0
        .iter()
        .map(decode_ipbm)
        .collect())
}

pub fn serialize_ipbm_dat(ipbms: &[Ipbm]) -> Vec<u8> {
    ipbms.iter().flat_map(encode_ipbm).collect()
}

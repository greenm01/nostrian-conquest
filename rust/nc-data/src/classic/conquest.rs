use crate::CONQUEST_DAT_SIZE;
use crate::model::{Conquest, MaintenanceDays};
use crate::support::{ParseError, copy_array, expect_size};

use super::{put_u16, u16_at};

pub fn decode_conquest(bytes: &[u8; CONQUEST_DAT_SIZE]) -> Conquest {
    let mut days = [false; 7];
    for (day, &byte) in days.iter_mut().zip(&bytes[3..10]) {
        *day = byte != 0;
    }
    let mut previous_production = [0u16; 25];
    for (slot, word) in previous_production.iter_mut().enumerate() {
        *word = u16_at(bytes, 0x0A + 2 * slot);
    }
    Conquest {
        game_year: u16_at(bytes, 0x00),
        player_count: bytes[0x02],
        maintenance_days: MaintenanceDays(days),
        previous_production,
        previous_planet_count: copy_array(&bytes[0x3C..0x55]),
    }
}

/// Bytes 0x55 onward, which no program reads, encode as zeros.
pub fn encode_conquest(conquest: &Conquest) -> [u8; CONQUEST_DAT_SIZE] {
    let mut bytes = [0u8; CONQUEST_DAT_SIZE];
    put_u16(&mut bytes, 0x00, conquest.game_year);
    bytes[0x02] = conquest.player_count;
    for (slot, enabled) in bytes[3..10].iter_mut().zip(conquest.maintenance_days.0) {
        *slot = u8::from(enabled);
    }
    for (slot, word) in conquest.previous_production.into_iter().enumerate() {
        put_u16(&mut bytes, 0x0A + 2 * slot, word);
    }
    bytes[0x3C..0x55].copy_from_slice(&conquest.previous_planet_count);
    bytes
}

pub fn parse_conquest_dat(data: &[u8]) -> Result<Conquest, ParseError> {
    expect_size(data, CONQUEST_DAT_SIZE, "CONQUEST.DAT")?;
    Ok(decode_conquest(&copy_array(data)))
}

pub fn serialize_conquest_dat(conquest: &Conquest) -> Vec<u8> {
    encode_conquest(conquest).to_vec()
}

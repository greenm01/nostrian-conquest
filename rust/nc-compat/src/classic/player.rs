use super::PLAYER_RECORD_SIZE;
use nc_classic::ParseError;
use nc_data::{DiplomaticRelation, Player, PlayerMode, RecordNumber};

use super::{
    i32_at, padded_text, pascal_string, put_i32, put_padded_text, put_pascal_string, put_u16,
    u16_at,
};

pub fn decode_player(bytes: &[u8; PLAYER_RECORD_SIZE]) -> Player {
    let mut diplomacy = [DiplomaticRelation::Neutral; 25];
    for (slot, &byte) in diplomacy.iter_mut().zip(&bytes[0x54..0x6D]) {
        *slot = DiplomaticRelation::from_raw(byte);
    }
    Player {
        mode: PlayerMode::from_raw(bytes[0x00]),
        handle: padded_text(&bytes[0x01..0x1A]),
        page_lines: bytes[0x1A],
        empire_name: pascal_string(bytes, 0x1B, 20),
        messages_first: i32_at(bytes, 0x30),
        messages_last: i32_at(bytes, 0x34),
        results_first: i32_at(bytes, 0x38),
        results_last: i32_at(bytes, 0x3C),
        fleets_first: RecordNumber(u16_at(bytes, 0x40)),
        fleets_last: RecordNumber(u16_at(bytes, 0x42)),
        bases_first: RecordNumber(u16_at(bytes, 0x44)),
        bases_last: RecordNumber(u16_at(bytes, 0x46)),
        ipbms_first: RecordNumber(u16_at(bytes, 0x48)),
        ipbms_last: RecordNumber(u16_at(bytes, 0x4A)),
        homeworld: bytes[0x4C],
        last_year_on: u16_at(bytes, 0x4E),
        planet_count: bytes[0x50],
        tax_rate: bytes[0x51],
        production: u16_at(bytes, 0x52),
        diplomacy,
        autopilot: bytes[0x6D] != 0,
    }
}

pub fn encode_player(player: &Player) -> [u8; PLAYER_RECORD_SIZE] {
    let mut bytes = [0u8; PLAYER_RECORD_SIZE];
    bytes[0x00] = player.mode.to_raw();
    put_padded_text(&mut bytes[0x01..0x1A], &player.handle);
    bytes[0x1A] = player.page_lines;
    put_pascal_string(&mut bytes, 0x1B, 20, &player.empire_name);
    put_i32(&mut bytes, 0x30, player.messages_first);
    put_i32(&mut bytes, 0x34, player.messages_last);
    put_i32(&mut bytes, 0x38, player.results_first);
    put_i32(&mut bytes, 0x3C, player.results_last);
    put_u16(&mut bytes, 0x40, player.fleets_first.0);
    put_u16(&mut bytes, 0x42, player.fleets_last.0);
    put_u16(&mut bytes, 0x44, player.bases_first.0);
    put_u16(&mut bytes, 0x46, player.bases_last.0);
    put_u16(&mut bytes, 0x48, player.ipbms_first.0);
    put_u16(&mut bytes, 0x4A, player.ipbms_last.0);
    bytes[0x4C] = player.homeworld;
    // ECUTIL writes the homeworld twice; no program reads the copy.
    bytes[0x4D] = player.homeworld;
    put_u16(&mut bytes, 0x4E, player.last_year_on);
    bytes[0x50] = player.planet_count;
    bytes[0x51] = player.tax_rate;
    put_u16(&mut bytes, 0x52, player.production);
    for (slot, relation) in bytes[0x54..0x6D].iter_mut().zip(player.diplomacy) {
        *slot = relation.to_raw();
    }
    bytes[0x6D] = u8::from(player.autopilot);
    bytes
}

pub fn parse_player_dat(data: &[u8]) -> Result<Vec<Player>, ParseError> {
    if !data.len().is_multiple_of(PLAYER_RECORD_SIZE) {
        return Err(ParseError::WrongRecordMultiple {
            file_type: "PLAYER.DAT",
            record_size: PLAYER_RECORD_SIZE,
            actual: data.len(),
        });
    }
    Ok(data
        .as_chunks::<PLAYER_RECORD_SIZE>()
        .0
        .iter()
        .map(decode_player)
        .collect())
}

pub fn serialize_player_dat(players: &[Player]) -> Vec<u8> {
    players.iter().flat_map(encode_player).collect()
}

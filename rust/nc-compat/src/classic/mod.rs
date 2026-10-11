//! Codecs between the typed model in `nc_data` and the classic `.DAT`
//! byte layouts mapped in `docs/dev/ec-file-formats.md`, with the record
//! and file sizes. Every byte a classic program
//! reads round-trips; the bytes none reads (see [`ignored_bytes`]) are not
//! carried, and `encode` writes zeros or the documented constant there.

mod base;
mod conquest;
mod fleet;
mod ipbm;
mod planet;
mod player;
mod setup;

use std::ops::Range;

pub const PLAYER_RECORD_SIZE: usize = 110;
pub const PLANET_RECORD_SIZE: usize = 97;
pub const FLEET_RECORD_SIZE: usize = 54;
pub const INITIALIZED_FLEET_RECORD_COUNT: usize = 16;
pub const INITIALIZED_FLEETS_DAT_SIZE: usize = FLEET_RECORD_SIZE * INITIALIZED_FLEET_RECORD_COUNT;
pub const BASE_RECORD_SIZE: usize = 35;
pub const IPBM_RECORD_SIZE: usize = 32;
pub const SETUP_DAT_SIZE: usize = 522;
pub const CONQUEST_DAT_SIZE: usize = 2085;

use nc_classic::copy_array;
use nc_data::Real48;

pub use base::{decode_base, encode_base, parse_bases_dat, serialize_bases_dat};
pub use conquest::{decode_conquest, encode_conquest, parse_conquest_dat, serialize_conquest_dat};
pub use fleet::{decode_fleet, encode_fleet, parse_fleets_dat, serialize_fleets_dat};
pub use ipbm::{decode_ipbm, encode_ipbm, parse_ipbm_dat, serialize_ipbm_dat};
pub use planet::{decode_planet, encode_planet, parse_planets_dat, serialize_planets_dat};
pub use player::{decode_player, encode_player, parse_player_dat, serialize_player_dat};
pub use setup::{decode_setup, encode_setup, parse_setup_dat, serialize_setup_dat};

/// The seven core classic data files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassicFile {
    Player,
    Planets,
    Fleets,
    Bases,
    Ipbm,
    Setup,
    Conquest,
}

impl ClassicFile {
    pub const ALL: [Self; 7] = [
        Self::Player,
        Self::Planets,
        Self::Fleets,
        Self::Bases,
        Self::Ipbm,
        Self::Setup,
        Self::Conquest,
    ];

    pub fn from_file_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|file| file.file_name().eq_ignore_ascii_case(name))
    }

    pub const fn file_name(self) -> &'static str {
        match self {
            Self::Player => "PLAYER.DAT",
            Self::Planets => "PLANETS.DAT",
            Self::Fleets => "FLEETS.DAT",
            Self::Bases => "BASES.DAT",
            Self::Ipbm => "IPBM.DAT",
            Self::Setup => "SETUP.DAT",
            Self::Conquest => "CONQUEST.DAT",
        }
    }
}

/// The byte ranges of `data` that no classic program reads: planet and
/// empire name tails past the length byte, diplomacy slots and CONQUEST
/// ranking slots past the player count, the FLEETS 0x34 word, the PLAYER
/// 0x4D homeworld copy, PLANETS 0x5E and 0x5F, SETUP 13..511, and
/// CONQUEST 0x55 onward. The model does not carry them; a byte comparison
/// against a classic file should skip them.
pub fn ignored_bytes(file: ClassicFile, data: &[u8]) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    match file {
        ClassicFile::Player => {
            let player_count = data.len() / PLAYER_RECORD_SIZE;
            for base in (0..data.len()).step_by(PLAYER_RECORD_SIZE) {
                let name_len = usize::from(data[base + 0x1B]).min(20);
                ranges.push(base + 0x1C + name_len..base + 0x30);
                ranges.push(base + 0x4D..base + 0x4E);
                ranges.push(base + 0x54 + player_count.min(25)..base + 0x6D);
            }
        }
        ClassicFile::Planets => {
            for base in (0..data.len()).step_by(PLANET_RECORD_SIZE) {
                let name_len = usize::from(data[base + 0x0F]).min(20);
                ranges.push(base + 0x10 + name_len..base + 0x24);
                ranges.push(base + 0x5E..base + 0x60);
            }
        }
        ClassicFile::Fleets => {
            for base in (0..data.len()).step_by(FLEET_RECORD_SIZE) {
                ranges.push(base + 0x34..base + 0x36);
            }
        }
        ClassicFile::Bases | ClassicFile::Ipbm => {}
        ClassicFile::Setup => ranges.push(13..512),
        ClassicFile::Conquest => {
            let player_count = usize::from(data[2]).min(25);
            ranges.push(0x0A + 2 * player_count..0x3C);
            ranges.push(0x3C + player_count..0x55);
            ranges.push(0x55..data.len());
        }
    }
    ranges.retain(|range| !range.is_empty());
    ranges
}

/// `data` with every ignored byte set to zero, for byte comparisons.
pub fn without_ignored_bytes(file: ClassicFile, data: &[u8]) -> Vec<u8> {
    let mut masked = data.to_vec();
    for range in ignored_bytes(file, data) {
        masked[range].fill(0);
    }
    masked
}

fn u16_at(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn put_u16(bytes: &mut [u8], at: usize, value: u16) {
    bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
}

fn i32_at(bytes: &[u8], at: usize) -> i32 {
    i32::from_le_bytes(copy_array(&bytes[at..at + 4]))
}

fn put_i32(bytes: &mut [u8], at: usize, value: i32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(copy_array(&bytes[at..at + 4]))
}

fn put_u32(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn real_at(bytes: &[u8], at: usize) -> Real48 {
    Real48::from_bytes(copy_array(&bytes[at..at + 6]))
}

fn put_real(bytes: &mut [u8], at: usize, value: Real48) {
    bytes[at..at + 6].copy_from_slice(&value.to_bytes());
}

/// Classic text is one byte per character; bytes above ASCII map to the
/// Latin-1 character of the same code so they survive a round trip.
fn text_from_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|&byte| char::from(byte)).collect()
}

fn text_byte(ch: char) -> u8 {
    u8::try_from(u32::from(ch)).unwrap_or(b'?')
}

/// A Pascal `String[max]`: the length byte at `at`, then the characters.
fn pascal_string(bytes: &[u8], at: usize, max: usize) -> String {
    let len = usize::from(bytes[at]).min(max);
    text_from_bytes(&bytes[at + 1..at + 1 + len])
}

/// Stores `value` as a Pascal `String[max]`, cut to `max` characters, and
/// zeroes the rest of the slot.
fn put_pascal_string(bytes: &mut [u8], at: usize, max: usize, value: &str) {
    let text: Vec<u8> = value.chars().take(max).map(text_byte).collect();
    bytes[at] = text.len() as u8;
    bytes[at + 1..at + 1 + max].fill(0);
    bytes[at + 1..at + 1 + text.len()].copy_from_slice(&text);
}

/// A fixed-width field padded with spaces, as classic stores a handle.
fn padded_text(bytes: &[u8]) -> String {
    text_from_bytes(bytes)
        .trim_end_matches([' ', '\0'])
        .to_string()
}

fn put_padded_text(bytes: &mut [u8], value: &str) {
    bytes.fill(b' ');
    for (slot, ch) in bytes.iter_mut().zip(value.chars()) {
        *slot = text_byte(ch);
    }
}

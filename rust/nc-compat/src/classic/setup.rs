use super::SETUP_DAT_SIZE;
use nc_classic::{ParseError, copy_array, expect_size};
use nc_data::Setup;

use super::{put_u16, u16_at};

pub fn decode_setup(bytes: &[u8; SETUP_DAT_SIZE]) -> Setup {
    Setup {
        version_tag: copy_array(&bytes[0..5]),
        com_irq: copy_array(&bytes[5..9]),
        com_hw_flow_control: [
            bytes[9] != 0,
            bytes[10] != 0,
            bytes[11] != 0,
            bytes[12] != 0,
        ],
        snoop: bytes[512] != 0,
        max_key_gap_minutes: u16_at(bytes, 513),
        remote_timeout: bytes[515] != 0,
        local_timeout: bytes[516] != 0,
        minimum_time_minutes: bytes[517],
        purge_after_turns: u16_at(bytes, 518),
        autopilot_after_turns: u16_at(bytes, 520),
    }
}

/// Bytes 13..511, which no program reads, encode as zeros.
pub fn encode_setup(setup: &Setup) -> [u8; SETUP_DAT_SIZE] {
    let mut bytes = [0u8; SETUP_DAT_SIZE];
    bytes[0..5].copy_from_slice(&setup.version_tag);
    bytes[5..9].copy_from_slice(&setup.com_irq);
    for (slot, enabled) in bytes[9..13].iter_mut().zip(setup.com_hw_flow_control) {
        *slot = u8::from(enabled);
    }
    bytes[512] = u8::from(setup.snoop);
    put_u16(&mut bytes, 513, setup.max_key_gap_minutes);
    bytes[515] = u8::from(setup.remote_timeout);
    bytes[516] = u8::from(setup.local_timeout);
    bytes[517] = setup.minimum_time_minutes;
    put_u16(&mut bytes, 518, setup.purge_after_turns);
    put_u16(&mut bytes, 520, setup.autopilot_after_turns);
    bytes
}

pub fn parse_setup_dat(data: &[u8]) -> Result<Setup, ParseError> {
    expect_size(data, SETUP_DAT_SIZE, "SETUP.DAT")?;
    Ok(decode_setup(&copy_array(data)))
}

pub fn serialize_setup_dat(setup: &Setup) -> Vec<u8> {
    encode_setup(setup).to_vec()
}

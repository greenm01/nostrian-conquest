use crate::records::fleet::FleetRecord;

const EXACT_POSITION_MAGIC: u8 = 0x42;
const EXACT_POSITION_SCALE: f64 = 256.0;

pub fn decode_exact_position(fleet: &FleetRecord) -> Option<[f64; 2]> {
    let payload = fleet.extended_tuple_c_payload_raw();
    if payload[5] != EXACT_POSITION_MAGIC {
        return None;
    }

    let x_fixed = u16::from_le_bytes([payload[1], payload[2]]);
    let y_fixed = u16::from_le_bytes([payload[3], payload[4]]);
    Some([
        f64::from(x_fixed) / EXACT_POSITION_SCALE,
        f64::from(y_fixed) / EXACT_POSITION_SCALE,
    ])
}

pub fn store_exact_position(fleet: &mut FleetRecord, exact: [f64; 2]) {
    let x_fixed = encode_exact_coord(exact[0]);
    let y_fixed = encode_exact_coord(exact[1]);
    let mut payload = fleet.extended_tuple_c_payload_raw();
    payload[1..3].copy_from_slice(&x_fixed.to_le_bytes());
    payload[3..5].copy_from_slice(&y_fixed.to_le_bytes());
    payload[5] = EXACT_POSITION_MAGIC;
    fleet.set_extended_tuple_c_payload_raw(payload);
}

/// The exact position as `store_exact_position` keeps it between turns.
pub fn round_trip_exact_position(exact: [f64; 2]) -> [f64; 2] {
    exact.map(|value| f64::from(encode_exact_coord(value)) / EXACT_POSITION_SCALE)
}

pub fn clear_exact_position(fleet: &mut FleetRecord) {
    let mut payload = fleet.extended_tuple_c_payload_raw();
    payload[1..].fill(0);
    fleet.set_extended_tuple_c_payload_raw(payload);
}

pub fn reset_motion_state_for_new_orders(fleet: &mut FleetRecord) {
    let mut payload = fleet.tuple_a_payload_raw();
    payload[0] = 0x80;
    payload[2] = 0x00;
    fleet.set_tuple_a_payload_raw(payload);
    clear_exact_position(fleet);
}

pub fn reset_motion_state_for_stationary_arrival(fleet: &mut FleetRecord) {
    fleet.set_tuple_a_payload_raw([0x80, 0x00, 0x00, 0x00, 0x00]);
    clear_exact_position(fleet);
}

fn encode_exact_coord(value: f64) -> u16 {
    (value * EXACT_POSITION_SCALE)
        .round()
        .clamp(0.0, f64::from(u16::MAX)) as u16
}

//! The 6-byte real arithmetic must match ECMAINT's runtime bit for bit; the
//! expected bytes come from the classic runtime routines and fleet traces.

use nc_data::Real48;

fn int(value: i32) -> Real48 {
    Real48::from_int(value)
}

#[test]
fn encodes_classic_constants() {
    assert_eq!(Real48::HALF.to_bytes(), [0x80, 0, 0, 0, 0, 0]);
    assert_eq!(Real48::ONE.to_bytes(), [0x81, 0, 0, 0, 0, 0]);
    assert_eq!(int(52).to_bytes(), [0x86, 0, 0, 0, 0, 0x50]);
    assert_eq!(int(-3).to_bytes(), [0x82, 0, 0, 0, 0, 0xc0]);
    assert_eq!(Real48::ZERO.to_f64(), 0.0);
    assert_eq!((int(13) + Real48::HALF).to_f64(), 13.5);
}

#[test]
fn arithmetic_rounds_like_the_classic_runtime() {
    assert_eq!(
        (int(3) / int(52)).to_bytes(),
        [0x7c, 0x4f, 0xec, 0xc4, 0x4e, 0x6c]
    );
    assert_eq!(
        (int(1) / int(10)).to_bytes(),
        [0x7d, 0xcd, 0xcc, 0xcc, 0xcc, 0x4c]
    );
    assert_eq!(
        (int(1) / int(3) * (int(2) / int(7))).to_bytes(),
        [0x7d, 0x0c, 0xc3, 0x30, 0x0c, 0x43]
    );
    assert_eq!(
        int(2).sqrt().to_bytes(),
        [0x81, 0xfa, 0x33, 0xf3, 0x04, 0x35]
    );
    assert_eq!(
        int(18).sqrt().to_bytes(),
        [0x83, 0xfc, 0x66, 0xb6, 0xc3, 0x07]
    );
}

#[test]
fn a_full_climb_leaves_a_residue_and_a_full_dive_falls_short() {
    // A speed-1 fleet climbing out of orbit for 52 ticks ends just above 0
    // (classic tuple c `5b 00 00 00 00 40`), and 52 ticks of diving end just
    // below 1.0, so the exact `Z == 1.0` arrival test does not fire.
    let tick = int(1) / int(52);
    let mut climb = Real48::ONE;
    let mut dive = Real48::ZERO;
    for _ in 0..52 {
        climb = climb - tick;
        dive = dive + tick;
    }
    assert_eq!(climb.to_bytes(), [0x5b, 0, 0, 0, 0, 0x40]);
    assert_eq!(dive.to_bytes(), [0x80, 0xfa, 0xff, 0xff, 0xff, 0x7f]);
    assert!(dive < Real48::ONE);
    assert_eq!(dive.round(), 1);
}

#[test]
fn trunc_round_and_ordering() {
    let x = int(13) + int(9) / int(10);
    assert_eq!(x.trunc(), 13);
    assert_eq!(x.round(), 14);
    assert_eq!(Real48::HALF.round(), 1);
    assert_eq!((int(-3) / int(2)).round(), -2);
    assert!(Real48::HALF < Real48::ONE);
    assert!(int(-1) < Real48::ZERO);
    assert_eq!(x - int(13) + int(13), x);
}

//! Year-by-year fleet motion against classic ECMAINT.
//!
//! Each case replays one fleet from a 2026-10-10 classic mission sweep
//! (`tools/ncmaint_mission_sweep.py`, missions 1-15) and
//! checks every classic year the Rust engine must reproduce: the fleet's
//! speed, order, and FLEETS.DAT bytes 0x0B..=0x1E (sector, X/Y offsets, and
//! depth, all 6-byte reals). Hostile outcomes are random in classic and Rust
//! fights with NC combat, so those cases check the classic strike year and
//! then Rust's post-strike state: at rest in orbit at the target with
//! speed 0, still on Bombard, or holding after Invade or Blitz.

use nc_data::{BaseRecord, CoreGameData, FleetRecord, MaintenanceEvents, Mission};
use nc_engine::run_maintenance_turn;
use std::path::Path;

const HOME: [u8; 2] = [16, 13];

/// What classic shows for the traced fleet at the end of one year.
enum Year {
    /// Speed, order, and bytes 0x0B..=0x1E.
    State(u8, u8, &'static str),
    /// The fleet merged into its join host or was salvaged.
    Absorbed,
    /// The hostile strike year.
    Strike,
}

impl Year {
    const fn state(speed: u8, order: u8, motion: &'static str) -> Self {
        Self::State(speed, order, motion)
    }
}

struct Case {
    name: &'static str,
    /// The fleet's classic FLEETS.DAT record before the first year.
    start: &'static str,
    years: &'static [Year],
}

fn hex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).expect("hex"))
        .collect()
}

fn load_fixture(name: &str) -> CoreGameData {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
        .join("v1.5");
    CoreGameData::load(&dir).unwrap_or_else(|err| panic!("fixture {name}: {err}"))
}

/// The sweep's scenario with the traced fleet as fleet record 1.
fn scenario(case: &Case) -> CoreGameData {
    let start = hex(case.start);
    let mission = start[0x1f];
    let mut game_data = if matches!(mission, 6..=8) {
        let mut game_data = load_fixture("ecmaint-bombard-pre");
        // The scenario's own attacker is parked at home, out of the way.
        let parked = &mut game_data.fleets.records[2];
        parked.set_scout_count(0);
        parked.set_battleship_count(0);
        parked.set_cruiser_count(1);
        parked.set_destroyer_count(0);
        parked.set_troop_transport_count(0);
        parked.set_army_count(0);
        parked.set_etac_count(0);
        parked.recompute_max_speed_from_composition();
        parked.set_current_speed(0);
        parked.set_standing_order_code_raw(5);
        parked.set_standing_order_target_coords_raw(HOME);
        parked.set_mission_aux_bytes([1, 0]);
        game_data
    } else {
        load_fixture("ecmaint-move-pre")
    };
    if mission == 4 {
        // `nc-cli guard-starbase-onebase`: Starbase 1 at rest in orbit at
        // the guard's target, escorted by fleet 1.
        let base_at = [start[0x20], start[0x21]];
        let mut base = BaseRecord::new_zeroed();
        base.set_escort_fleet_slot_raw(1);
        base.set_base_id_raw(1);
        base.set_record_number_raw(1);
        base.set_coords_raw(base_at);
        base.set_at_rest_motion();
        base.set_target_coords_raw(base_at);
        base.set_target_depth_raw(1);
        base.set_owner_empire_raw(1);
        game_data.bases.records = vec![base];
        game_data.player.records[0].set_first_base_record_raw(1);
    }
    let mut fleet = FleetRecord::from_raw(start[..].try_into().expect("a fleet record"));
    if matches!(mission, 10 | 11) {
        // Classic counts the lone scout at 0x32, Rust at 0x24.
        fleet.set_scout_count(1);
    }
    game_data.fleets.records[0] = fleet;
    game_data
}

fn empire_one_fleets(game_data: &CoreGameData) -> usize {
    game_data
        .fleets
        .records
        .iter()
        .filter(|fleet| fleet.owner_empire_raw() == 1)
        .count()
}

fn struck(events: &MaintenanceEvents, mission: u8) -> bool {
    match mission {
        6 => events
            .bombard_events
            .iter()
            .any(|event| event.attacker_fleet_number == Some(1) && event.attacker_empire_raw == 1),
        7 | 8 => events.assault_report_events.iter().any(|event| {
            event.attacker_empire_raw == 1
                && event.kind
                    == if mission == 7 {
                        Mission::InvadeWorld
                    } else {
                        Mission::BlitzWorld
                    }
        }),
        _ => unreachable!(),
    }
}

fn run_cases(cases: &[Case]) {
    for case in cases {
        let mut game_data = scenario(case);
        let mission = hex(case.start)[0x1f];
        for (index, expected) in case.years.iter().enumerate() {
            let year = index + 1;
            let fleets_before = empire_one_fleets(&game_data);
            let events = run_maintenance_turn(&mut game_data)
                .unwrap_or_else(|err| panic!("{} year {year}: {err}", case.name));
            let fleet = &game_data.fleets.records[0];
            match expected {
                Year::State(speed, order, motion) => {
                    assert_eq!(
                        fleet.to_bytes()[0x0b..=0x1e],
                        hex(motion)[..],
                        "{} year {year} motion",
                        case.name
                    );
                    assert_eq!(
                        fleet.current_speed(),
                        *speed,
                        "{} year {year} speed",
                        case.name
                    );
                    assert_eq!(
                        fleet.standing_order_code_raw(),
                        *order,
                        "{} year {year} order",
                        case.name
                    );
                    if matches!(mission, 6..=8) {
                        assert!(
                            !struck(&events, mission),
                            "{} year {year} early strike",
                            case.name
                        );
                    }
                }
                Year::Absorbed => assert_eq!(
                    empire_one_fleets(&game_data),
                    fleets_before - 1,
                    "{} year {year} absorbed",
                    case.name
                ),
                Year::Strike => {
                    assert!(struck(&events, mission), "{} year {year} strike", case.name);
                    let target = fleet.standing_order_target_coords_raw();
                    let at_rest = format!(
                        "{:02x}{:02x}800000000000800000000000810000000000",
                        target[0], target[1]
                    );
                    assert_eq!(
                        fleet.to_bytes()[0x0b..=0x1e],
                        hex(&at_rest)[..],
                        "{} year {year} in orbit",
                        case.name
                    );
                    assert_eq!(fleet.current_speed(), 0, "{} year {year} speed", case.name);
                    assert_eq!(
                        fleet.standing_order_code_raw(),
                        if mission == 6 { 6 } else { 0 },
                        "{} year {year} order",
                        case.name
                    );
                }
            }
        }
    }
}

#[test]
fn sector_orders_stop_on_entering_the_target_sector() {
    run_cases(SECTOR_ORDER_CASES);
}

#[test]
fn guard_world_settles_in_the_classic_year_with_classic_bytes() {
    run_cases(GUARD_CASES);
}

#[test]
fn seek_home_guard_starbase_and_colonize_resolve_as_classic() {
    run_cases(SETTLING_CASES);
}

/// Mission 4 with the base over an unowned world (`--deep-base`): the base
/// climbs to deep space in the first year, its guards' world-target flag
/// drops to 0, and a guard arrives on entering the base's sector.
#[test]
fn guard_starbase_follows_a_base_that_left_orbit_as_classic() {
    run_cases(DEEP_BASE_CASES);
}

#[test]
fn view_and_scout_system_resolve_as_classic() {
    run_cases(OBSERVER_CASES);
}

#[test]
fn join_and_salvage_absorb_the_fleet_in_the_classic_year() {
    run_cases(ABSORBED_CASES);
}

#[test]
fn hostile_orders_strike_in_the_classic_year() {
    run_cases(HOSTILE_CASES);
}

const GUARD_CASES: &[Case] = &[
    Case {
        name: "m05-v1-d1",
        start: "01000102000100000005010f0c80000000000080000000000081000000000005100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(1, 5, "0f0c8000000000008000000000005b0000000040"),
            Year::state(1, 5, "100d7e00cccc13547e00cccc1354000000000000"),
            Year::state(1, 5, "100d800000000000800000000000801d9619f615"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m05-v1-d3",
        start: "01000102000100000005010d0a80000000000080000000000081000000000005100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(1, 5, "0d0a8000000000008000000000005b0000000040"),
            Year::state(1, 5, "0e0b7e00cccc13547e00cccc1354000000000000"),
            Year::state(1, 5, "0e0b800066e6096a800066e6096a000000000000"),
            Year::state(1, 5, "0f0c800099d90e1f800099d90e1f000000000000"),
            Year::state(1, 5, "100d7f00989927287f0098992728000000000000"),
            Year::state(1, 5, "100d800000000000800000000000804cc24ce241"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m05-v1-d6",
        start: "01000102000100000005010a0780000000000080000000000081000000000005100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(1, 5, "0a078000000000008000000000005b0000000040"),
            Year::state(1, 5, "0b087e00cccc13547e80d0cc1354000000000000"),
            Year::state(1, 5, "0b08800066e6096a802067e6096a000000000000"),
            Year::state(1, 5, "0c09800099d90e1f80209ad90e1f000000000000"),
            Year::state(1, 5, "0d0a7f00989927287f409a992728000000000000"),
            Year::state(1, 5, "0e0b7c00f0ff8b117c0002008c11000000000000"),
            Year::state(1, 5, "0e0b800032b31d3e802033b31d3e000000000000"),
            Year::state(1, 5, "0f0c7f00ca4c45667f40cc4c4566000000000000"),
            Year::state(1, 5, "100d7e0061669e207e8064669e20000000000000"),
            Year::state(1, 5, "100d80000000000080000000000080df8599c403"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m05-v1-d8",
        start: "0100010200010000000501080580000000000080000000000081000000000005100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(1, 5, "08058000000000008000000000005b0000000040"),
            Year::state(1, 5, "09067e00cccc13547e60d2cc1354000000000000"),
            Year::state(1, 5, "0906800066e6096a802069e6096a000000000000"),
            Year::state(1, 5, "0a07800099d90e1f80209cd90e1f000000000000"),
            Year::state(1, 5, "0b087f00989927287f409e992728000000000000"),
            Year::state(1, 5, "0c097c00f0ff8b117c0022008c11000000000000"),
            Year::state(1, 5, "0c09800032b31d3e802035b31d3e000000000000"),
            Year::state(1, 5, "0d0a7f00ca4c45667f40d04c4566000000000000"),
            Year::state(1, 5, "0e0b7e0060669e207e806c669e20000000000000"),
            Year::state(1, 5, "0e0b8020cb8c2c5d8020ce8c2c5d000000000000"),
            Year::state(1, 5, "0f0c8070ff7f3112802001803112000000000000"),
            Year::state(1, 5, "100d7f8066e66c0e7f4068e66c0e000000000000"),
            Year::state(1, 5, "100d8000000000008000000000008004b5ccb02f"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m05-v1-s1",
        start: "01000102000100000005010f0d80000000000080000000000081000000000005100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(1, 5, "0f0d8000000000008000000000005b0000000040"),
            Year::state(1, 5, "100d7f00feffff7f800000000000000000000000"),
            Year::state(1, 5, "100d80000000000080000000000080fafeffff7f"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m05-v1-s3",
        start: "01000102000100000005010d0d80000000000080000000000081000000000005100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(1, 5, "0d0d8000000000008000000000005b0000000040"),
            Year::state(1, 5, "0e0d80b000000000800000000000000000000000"),
            Year::state(1, 5, "0f0d807001000000800000000000000000000000"),
            Year::state(1, 5, "100d80000000000080000000000060000000500c"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m05-v2-d1",
        start: "01000102000100000005020f0c80000000000080000000000081000000000005100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(2, 5, "100d7e80d0cc13547e80d1cc1354000000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m05-v2-d3",
        start: "01000102000100000005020d0a80000000000080000000000081000000000005100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(2, 5, "0e0b7e80d2cc13547e80d2cc1354000000000000"),
            Year::state(2, 5, "0f0c80e09dd90e1f80e09dd90e1f000000000000"),
            Year::state(2, 5, "100d80000000000080000000000080d7ca4ce241"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m05-v2-d6",
        start: "01000102000100000005020a0780000000000080000000000081000000000005100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(2, 5, "0b087e80d2cc13547e40d0cc1354000000000000"),
            Year::state(2, 5, "0c0980e09dd90e1f80509dd90e1f000000000000"),
            Year::state(2, 5, "0e0b7c0072008c117c0069008c11000000000000"),
            Year::state(2, 5, "0f0c7f80e04c45667fa0df4c4566000000000000"),
            Year::state(2, 5, "100d80000000000080000000000080279699c403"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m05-v2-d8",
        start: "0100010200010000000502080580000000000080000000000081000000000005100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(2, 5, "09067e80d2cc13547e40cfcc1354000000000000"),
            Year::state(2, 5, "0a0780e09dd90e1f80709bd90e1f000000000000"),
            Year::state(2, 5, "0c097c0065008c117c0044008c11000000000000"),
            Year::state(2, 5, "0d0a7fe0dd4c45667f00db4c4566000000000000"),
            Year::state(2, 5, "0e0b8080d78c2c5d80c0d68c2c5d000000000000"),
            Year::state(2, 5, "100d7f407fe66c0e7fa07fe66c0e000000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m05-v2-s1",
        start: "01000102000100000005020f0d80000000000080000000000081000000000005100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(2, 5, "100d80000000000080000000000060000000a038"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m05-v2-s3",
        start: "01000102000100000005020d0d80000000000080000000000081000000000005100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(2, 5, "0e0d80c000000000800000000000000000000000"),
            Year::state(2, 5, "100d80000000000080000000000062000000280e"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m05-v3-d1",
        start: "01000102000100000005030f0c80000000000080000000000081000000000005100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(3, 5, "100d80000000000080000000000080499819f615"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m05-v3-d3",
        start: "01000102000100000005030d0a80000000000080000000000081000000000005100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(3, 5, "0e0b803068e6096a803068e6096a000000000000"),
            Year::state(3, 5, "100d80000000000080000000000080c4c84ce241"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m05-v3-d6",
        start: "01000102000100000005030a0780000000000080000000000081000000000005100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(3, 5, "0b08803068e6096a802068e6096a000000000000"),
            Year::state(3, 5, "0e0b7c0047008c117c0046008c11000000000000"),
            Year::state(3, 5, "100d7e0082669e207e8082669e20000000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m05-v3-d8",
        start: "0100010200010000000503080580000000000080000000000081000000000005100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(3, 5, "0906803068e6096a802868e6096a000000000000"),
            Year::state(3, 5, "0c097c0047008c117c0046008c11000000000000"),
            Year::state(3, 5, "0e0b7ec082669e207e8082669e20000000000000"),
            Year::state(3, 5, "100d7f8079e66c0e7fc079e66c0e000000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m05-v3-s1",
        start: "01000102000100000005030f0d80000000000080000000000081000000000005100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(3, 5, "100d80000000000080000000000080b9ffffff7f"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m05-v3-s3",
        start: "01000102000100000005030d0d80000000000080000000000081000000000005100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(3, 5, "0f0d7fc0feffff7f800000000000000000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m05-v4-d1",
        start: "01000102000100000005040f0c80000000000080000000000081000000000005100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m05-v4-d3",
        start: "01000102000100000005040d0a80000000000080000000000081000000000005100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(4, 5, "0f0c80709bd90e1f80709bd90e1f000000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m05-v4-d6",
        start: "01000102000100000005040a0780000000000080000000000081000000000005100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(4, 5, "0c0980709bd90e1f80709bd90e1f000000000000"),
            Year::state(4, 5, "0f0c7f60d54c45667f60d54c4566000000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m05-v4-d8",
        start: "0100010200010000000504080580000000000080000000000081000000000005100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(4, 5, "0a0780709bd90e1f80709bd90e1f000000000000"),
            Year::state(4, 5, "0d0a7f60d54c45667f60d54c4566000000000000"),
            Year::state(4, 5, "100d7f4074e66c0e7fe073e66c0e000000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m05-v4-s1",
        start: "01000102000100000005040f0d80000000000080000000000081000000000005100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m05-v4-s3",
        start: "01000102000100000005040d0d80000000000080000000000081000000000005100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(4, 5, "100d8000000000008000000000005e0000000045"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m05-v5-d1",
        start: "01000102000100000005050f0c80000000000080000000000081000000000005100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m05-v5-d3",
        start: "01000102000100000005050d0a80000000000080000000000081000000000005100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(5, 5, "100d7f80a19927287f60a1992728000000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m05-v5-d6",
        start: "01000102000100000005050a0780000000000080000000000081000000000005100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(5, 5, "0d0a7f60a19927287f60a1992728000000000000"),
            Year::state(5, 5, "100d80000000000080000000000080739299c403"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m05-v5-d8",
        start: "0100010200010000000505080580000000000080000000000081000000000005100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(5, 5, "0b087f00a09927287f00a0992728000000000000"),
            Year::state(5, 5, "0e0b80b0d48c2c5d80b0d48c2c5d000000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m05-v5-s1",
        start: "01000102000100000005050f0d80000000000080000000000081000000000005100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m05-v5-s3",
        start: "01000102000100000005050d0d80000000000080000000000081000000000005100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
            Year::state(0, 5, "100d800000000000800000000000810000000000"),
        ],
    },
];

const SETTLING_CASES: &[Case] = &[
    Case {
        name: "m02-v1-d3",
        start: "01000102000100000005010d0a80000000000080000000000081000000000002100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(1, 2, "0d0a8000000000008000000000005b0000000040"),
            Year::state(1, 2, "0e0b7e00cccc13547e00cccc1354000000000000"),
            Year::state(1, 2, "0e0b800066e6096a800066e6096a000000000000"),
            Year::state(1, 2, "0f0c800099d90e1f800099d90e1f000000000000"),
            Year::state(1, 2, "100d7f00989927287f0098992728000000000000"),
            Year::state(0, 0, "100d800000000000800000000000804cc24ce241"),
            Year::state(0, 0, "100d800000000000800000000000804cc24ce241"),
        ],
    },
    Case {
        name: "m02-v1-d6",
        start: "01000102000100000005010a0780000000000080000000000081000000000002100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(1, 2, "0a078000000000008000000000005b0000000040"),
            Year::state(1, 2, "0b087e00cccc13547e80d0cc1354000000000000"),
            Year::state(1, 2, "0b08800066e6096a802067e6096a000000000000"),
            Year::state(1, 2, "0c09800099d90e1f80209ad90e1f000000000000"),
            Year::state(1, 2, "0d0a7f00989927287f409a992728000000000000"),
            Year::state(1, 2, "0e0b7c00f0ff8b117c0002008c11000000000000"),
            Year::state(1, 2, "0e0b800032b31d3e802033b31d3e000000000000"),
            Year::state(1, 2, "0f0c7f00ca4c45667f40cc4c4566000000000000"),
            Year::state(1, 2, "100d7e0061669e207e8064669e20000000000000"),
            Year::state(0, 0, "100d80000000000080000000000080df8599c403"),
        ],
    },
    Case {
        name: "m02-v1-s3",
        start: "01000102000100000005010d0d80000000000080000000000081000000000002100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(1, 2, "0d0d8000000000008000000000005b0000000040"),
            Year::state(1, 2, "0e0d80b000000000800000000000000000000000"),
            Year::state(1, 2, "0f0d807001000000800000000000000000000000"),
            Year::state(1, 2, "100d80000000000080000000000060000000500c"),
            Year::state(0, 0, "100d800000000000800000000000810000000000"),
            Year::state(0, 0, "100d800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m02-v3-d3",
        start: "01000102000100000005030d0a80000000000080000000000081000000000002100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(3, 2, "0e0b803068e6096a803068e6096a000000000000"),
            Year::state(0, 0, "100d80000000000080000000000080c4c84ce241"),
            Year::state(0, 0, "100d80000000000080000000000080c4c84ce241"),
        ],
    },
    Case {
        name: "m02-v3-d6",
        start: "01000102000100000005030a0780000000000080000000000081000000000002100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(3, 2, "0b08803068e6096a802068e6096a000000000000"),
            Year::state(3, 2, "0e0b7c0047008c117c0046008c11000000000000"),
            Year::state(3, 2, "100d7e0082669e207e8082669e20000000000000"),
            Year::state(0, 0, "100d800000000000800000000000810000000000"),
            Year::state(0, 0, "100d800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m02-v3-s3",
        start: "01000102000100000005030d0d80000000000080000000000081000000000002100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(3, 2, "0f0d7fc0feffff7f800000000000000000000000"),
            Year::state(0, 0, "100d800000000000800000000000810000000000"),
            Year::state(0, 0, "100d800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m04-v1-d3",
        start: "01000102000100000005010d0a80000000000080000000000081000000000004100d0101000600000100000000000000000000000000",
        years: &[
            Year::state(1, 4, "0d0a8000000000008000000000005b0000000040"),
            Year::state(1, 4, "0e0b7e00cccc13547e00cccc1354000000000000"),
            Year::state(1, 4, "0e0b800066e6096a800066e6096a000000000000"),
            Year::state(1, 4, "0f0c800099d90e1f800099d90e1f000000000000"),
            Year::state(1, 4, "100d7f00989927287f0098992728000000000000"),
            Year::state(0, 4, "100d800000000000800000000000810000000000"),
            Year::state(0, 4, "100d800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m04-v1-d6",
        start: "01000102000100000005010a0780000000000080000000000081000000000004100d0101000600000100000000000000000000000000",
        years: &[
            Year::state(1, 4, "0a078000000000008000000000005b0000000040"),
            Year::state(1, 4, "0b087e00cccc13547e80d0cc1354000000000000"),
            Year::state(1, 4, "0b08800066e6096a802067e6096a000000000000"),
            Year::state(1, 4, "0c09800099d90e1f80209ad90e1f000000000000"),
            Year::state(1, 4, "0d0a7f00989927287f409a992728000000000000"),
            Year::state(1, 4, "0e0b7c00f0ff8b117c0002008c11000000000000"),
            Year::state(1, 4, "0e0b800032b31d3e802033b31d3e000000000000"),
            Year::state(1, 4, "0f0c7f00ca4c45667f40cc4c4566000000000000"),
            Year::state(1, 4, "100d7e0061669e207e8064669e20000000000000"),
            Year::state(0, 4, "100d800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m04-v1-s3",
        start: "01000102000100000005010d0d80000000000080000000000081000000000004100d0101000600000100000000000000000000000000",
        years: &[
            Year::state(1, 4, "0d0d8000000000008000000000005b0000000040"),
            Year::state(1, 4, "0e0d80b000000000800000000000000000000000"),
            Year::state(1, 4, "0f0d807001000000800000000000000000000000"),
            Year::state(1, 4, "100d80000000000080000000000060000000500c"),
            Year::state(0, 4, "100d800000000000800000000000810000000000"),
            Year::state(0, 4, "100d800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m04-v3-d3",
        start: "01000102000100000005030d0a80000000000080000000000081000000000004100d0101000600000100000000000000000000000000",
        years: &[
            Year::state(3, 4, "0e0b803068e6096a803068e6096a000000000000"),
            Year::state(0, 4, "100d800000000000800000000000810000000000"),
            Year::state(0, 4, "100d800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m04-v3-d6",
        start: "01000102000100000005030a0780000000000080000000000081000000000004100d0101000600000100000000000000000000000000",
        years: &[
            Year::state(3, 4, "0b08803068e6096a802068e6096a000000000000"),
            Year::state(3, 4, "0e0b7c0047008c117c0046008c11000000000000"),
            Year::state(3, 4, "100d7e0082669e207e8082669e20000000000000"),
            Year::state(0, 4, "100d800000000000800000000000810000000000"),
            Year::state(0, 4, "100d800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m04-v3-s3",
        start: "01000102000100000005030d0d80000000000080000000000081000000000004100d0101000600000100000000000000000000000000",
        years: &[
            Year::state(3, 4, "0f0d7fc0feffff7f800000000000000000000000"),
            Year::state(0, 4, "100d800000000000800000000000810000000000"),
            Year::state(0, 4, "100d800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m12-v1-d3",
        start: "010001020001000000030109098000000000008000000000008100000000000c0c0c0100000600000100000000000000010000000000",
        years: &[
            Year::state(1, 12, "09098000000000008000000000005b0000000040"),
            Year::state(1, 12, "0a0a7e00cccc13547e00cccc1354000000000000"),
            Year::state(1, 12, "0a0a800066e6096a800066e6096a000000000000"),
            Year::state(1, 12, "0b0b800099d90e1f800099d90e1f000000000000"),
            Year::state(1, 12, "0c0c7f00989927287f0098992728000000000000"),
            Year::state(0, 0, "0c0c800000000000800000000000804cc24ce241"),
            Year::state(0, 0, "0c0c800000000000800000000000804cc24ce241"),
        ],
    },
    Case {
        name: "m12-v1-d6",
        start: "010001020001000000030106068000000000008000000000008100000000000c0c0c0100000600000100000000000000010000000000",
        years: &[
            Year::state(1, 12, "06068000000000008000000000005b0000000040"),
            Year::state(1, 12, "07077e60d2cc13547e60d2cc1354000000000000"),
            Year::state(1, 12, "0707803869e6096a803869e6096a000000000000"),
            Year::state(1, 12, "080880709cd90e1f80709cd90e1f000000000000"),
            Year::state(1, 12, "09097fe09e9927287fe09e992728000000000000"),
            Year::state(1, 12, "0a0a7c0027008c117c0027008c11000000000000"),
            Year::state(1, 12, "0a0a807035b31d3e807035b31d3e000000000000"),
            Year::state(1, 12, "0b0b7fe0d04c45667fe0d04c4566000000000000"),
            Year::state(1, 12, "0c0c7ec06d669e207ec06d669e20000000000000"),
            Year::state(0, 0, "0c0c80000000000080000000000080738999c403"),
        ],
    },
    Case {
        name: "m12-v1-s3",
        start: "0100010200010000000301090c8000000000008000000000008100000000000c0c0c0100000600000100000000000000010000000000",
        years: &[
            Year::state(1, 12, "090c8000000000008000000000005b0000000040"),
            Year::state(1, 12, "0a0c80b000000000800000000000000000000000"),
            Year::state(1, 12, "0b0c807001000000800000000000000000000000"),
            Year::state(1, 12, "0c0c80000000000080000000000062000000140b"),
            Year::state(0, 0, "0c0c800000000000800000000000810000000000"),
            Year::state(0, 0, "0c0c800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m12-v3-d3",
        start: "010001020001000000030309098000000000008000000000008100000000000c0c0c0100000600000100000000000000010000000000",
        years: &[
            Year::state(3, 12, "0a0a803068e6096a803068e6096a000000000000"),
            Year::state(0, 0, "0c0c80000000000080000000000080d0c84ce241"),
            Year::state(0, 0, "0c0c80000000000080000000000080d0c84ce241"),
        ],
    },
    Case {
        name: "m12-v3-d6",
        start: "010001020001000000030306068000000000008000000000008100000000000c0c0c0100000600000100000000000000010000000000",
        years: &[
            Year::state(3, 12, "0707802868e6096a802868e6096a000000000000"),
            Year::state(3, 12, "0a0a7c0046008c117c0046008c11000000000000"),
            Year::state(3, 12, "0c0c7e8082669e207e8082669e20000000000000"),
            Year::state(0, 0, "0c0c800000000000800000000000810000000000"),
            Year::state(0, 0, "0c0c800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m12-v3-s3",
        start: "0100010200010000000303090c8000000000008000000000008100000000000c0c0c0100000600000100000000000000010000000000",
        years: &[
            Year::state(3, 12, "0b0c7fc0feffff7f800000000000000000000000"),
            Year::state(0, 0, "0c0c800000000000800000000000810000000000"),
            Year::state(0, 0, "0c0c800000000000800000000000810000000000"),
        ],
    },
];

const OBSERVER_CASES: &[Case] = &[
    Case {
        name: "m09-v1-d4",
        start: "01000102000100000005010808800000000000800000000000810000000000090c0c0100000600000100000000000000000000000000",
        years: &[
            Year::state(1, 9, "08088000000000008000000000005b0000000040"),
            Year::state(1, 9, "09097e00cccc13547e00cccc1354000000000000"),
            Year::state(1, 9, "0909800066e6096a800066e6096a000000000000"),
            Year::state(1, 9, "0a0a800099d90e1f800099d90e1f000000000000"),
            Year::state(1, 9, "0b0b7f00989927287f0098992728000000000000"),
            Year::state(1, 9, "0c0c7c00f0ff8b117c00f0ff8b11000000000000"),
            Year::state(1, 9, "0c0c8000000000008000000000007fc8b0ccb02f"),
            Year::state(0, 0, "0c0c800000000000800000000000000000000000"),
        ],
    },
    Case {
        name: "m09-v1-s3",
        start: "0100010200010000000501090c800000000000800000000000810000000000090c0c0100000600000100000000000000000000000000",
        years: &[
            Year::state(1, 9, "090c8000000000008000000000005b0000000040"),
            Year::state(1, 9, "0a0c80b000000000800000000000000000000000"),
            Year::state(1, 9, "0b0c807001000000800000000000000000000000"),
            Year::state(1, 9, "0c0c80000000000080000000000062000000140b"),
            Year::state(5, 1, "0c0c800000000000800000000000810000000000"),
            Year::state(0, 0, "0c0c800000000000800000000000000000000000"),
        ],
    },
    Case {
        name: "m09-v3-d4",
        start: "01000102000100000005030808800000000000800000000000810000000000090c0c0100000600000100000000000000000000000000",
        years: &[
            Year::state(3, 9, "0909803068e6096a803068e6096a000000000000"),
            Year::state(3, 9, "0c0c7c0047008c117c0047008c11000000000000"),
            Year::state(0, 0, "0c0c800000000000800000000000000000000000"),
        ],
    },
    Case {
        name: "m09-v3-s3",
        start: "0100010200010000000503090c800000000000800000000000810000000000090c0c0100000600000100000000000000000000000000",
        years: &[
            Year::state(3, 9, "0b0c7fc0feffff7f800000000000000000000000"),
            Year::state(0, 0, "0c0c800000000000800000000000000000000000"),
        ],
    },
    Case {
        name: "m09-v5-d4",
        start: "01000102000100000005050808800000000000800000000000810000000000090c0c0100000600000100000000000000000000000000",
        years: &[
            Year::state(5, 9, "0b0b7fa0a19927287fa0a1992728000000000000"),
            Year::state(0, 0, "0c0c800000000000800000000000000000000000"),
        ],
    },
    Case {
        name: "m09-v5-s3",
        start: "0100010200010000000505090c800000000000800000000000810000000000090c0c0100000600000100000000000000000000000000",
        years: &[
            Year::state(5, 1, "0c0c800000000000800000000000810000000000"),
            Year::state(0, 0, "0c0c800000000000800000000000000000000000"),
        ],
    },
    Case {
        name: "m09-v4-d1",
        start: "01000102000100000005040b0b800000000000800000000000810000000000090c0c0100000600000100000000000000000000000000",
        years: &[
            Year::state(5, 1, "0c0c8000000000008000000000007f6827766227"),
            Year::state(0, 0, "0c0c800000000000800000000000000000000000"),
        ],
    },
    Case {
        name: "m11-v1-d4",
        start: "010001020001000000060108088000000000008000000000008100000000000b0c0c0100000000000000000000000000000001000000",
        years: &[
            Year::state(1, 11, "08088000000000008000000000005b0000000040"),
            Year::state(1, 11, "09097e00cccc13547e00cccc1354000000000000"),
            Year::state(1, 11, "0909800066e6096a800066e6096a000000000000"),
            Year::state(1, 11, "0a0a800099d90e1f800099d90e1f000000000000"),
            Year::state(1, 11, "0b0b7f00989927287f0098992728000000000000"),
            Year::state(1, 11, "0c0c7c00f0ff8b117c00f0ff8b11000000000000"),
            Year::state(1, 11, "0c0c8000000000008000000000007fc8b0ccb02f"),
            Year::state(0, 11, "0c0c800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m11-v1-s3",
        start: "0100010200010000000601090c8000000000008000000000008100000000000b0c0c0100000000000000000000000000000001000000",
        years: &[
            Year::state(1, 11, "090c8000000000008000000000005b0000000040"),
            Year::state(1, 11, "0a0c80b000000000800000000000000000000000"),
            Year::state(1, 11, "0b0c807001000000800000000000000000000000"),
            Year::state(1, 11, "0c0c80000000000080000000000062000000140b"),
            Year::state(0, 11, "0c0c800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m11-v3-d4",
        start: "010001020001000000060308088000000000008000000000008100000000000b0c0c0100000000000000000000000000000001000000",
        years: &[
            Year::state(3, 11, "0909803068e6096a803068e6096a000000000000"),
            Year::state(3, 11, "0c0c7c0047008c117c0047008c11000000000000"),
            Year::state(0, 11, "0c0c800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m11-v3-s3",
        start: "0100010200010000000603090c8000000000008000000000008100000000000b0c0c0100000000000000000000000000000001000000",
        years: &[
            Year::state(3, 11, "0b0c7fc0feffff7f800000000000000000000000"),
            Year::state(0, 11, "0c0c800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m11-v5-d4",
        start: "010001020001000000060508088000000000008000000000008100000000000b0c0c0100000000000000000000000000000001000000",
        years: &[
            Year::state(5, 11, "0b0b7fa0a19927287fa0a1992728000000000000"),
            Year::state(0, 11, "0c0c800000000000800000000000810000000000"),
        ],
    },
    Case {
        name: "m11-v5-s3",
        start: "0100010200010000000605090c8000000000008000000000008100000000000b0c0c0100000000000000000000000000000001000000",
        years: &[Year::state(
            0,
            11,
            "0c0c800000000000800000000000810000000000",
        )],
    },
];

const ABSORBED_CASES: &[Case] = &[
    Case {
        name: "m13-v1-d4",
        start: "01000102000100000005010c098000000000008000000000008100000000000d100d0102000600000100000000000000000000000000",
        years: &[
            Year::state(1, 13, "0c098000000000008000000000005b0000000040"),
            Year::state(1, 13, "0d0a7e00cccc13547e00cccc1354000000000000"),
            Year::state(1, 13, "0d0a800066e6096a800066e6096a000000000000"),
            Year::state(1, 13, "0e0b800099d90e1f800099d90e1f000000000000"),
            Year::state(1, 13, "0f0c7f00989927287f0098992728000000000000"),
            Year::state(1, 13, "100d7c00f0ff8b117c00f0ff8b11000000000000"),
            Year::state(1, 13, "100d8000000000008000000000007fc8b0ccb02f"),
            Year::Absorbed,
        ],
    },
    Case {
        name: "m13-v1-s3",
        start: "01000102000100000005010d0d8000000000008000000000008100000000000d100d0102000600000100000000000000000000000000",
        years: &[
            Year::state(1, 13, "0d0d8000000000008000000000005b0000000040"),
            Year::state(1, 13, "0e0d80b000000000800000000000000000000000"),
            Year::state(1, 13, "0f0d807001000000800000000000000000000000"),
            Year::state(1, 13, "100d80000000000080000000000060000000500c"),
            Year::Absorbed,
        ],
    },
    Case {
        name: "m13-v3-d4",
        start: "01000102000100000005030c098000000000008000000000008100000000000d100d0102000600000100000000000000000000000000",
        years: &[
            Year::state(3, 13, "0d0a803068e6096a803068e6096a000000000000"),
            Year::state(3, 13, "100d7c0046008c117c0047008c11000000000000"),
            Year::Absorbed,
        ],
    },
    Case {
        name: "m13-v3-s3",
        start: "01000102000100000005030d0d8000000000008000000000008100000000000d100d0102000600000100000000000000000000000000",
        years: &[
            Year::state(3, 13, "0f0d7fc0feffff7f800000000000000000000000"),
            Year::Absorbed,
        ],
    },
    Case {
        name: "m13-v5-d4",
        start: "01000102000100000005050c098000000000008000000000008100000000000d100d0102000600000100000000000000000000000000",
        years: &[
            Year::state(5, 13, "0f0c7fa0a19927287fa0a1992728000000000000"),
            Year::Absorbed,
        ],
    },
    Case {
        name: "m13-v5-s3",
        start: "01000102000100000005050d0d8000000000008000000000008100000000000d100d0102000600000100000000000000000000000000",
        years: &[Year::Absorbed],
    },
    Case {
        name: "m15-v1-d4",
        start: "01000102000100000005010c098000000000008000000000008100000000000f100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(1, 15, "0c098000000000008000000000005b0000000040"),
            Year::state(1, 15, "0d0a7e00cccc13547e00cccc1354000000000000"),
            Year::state(1, 15, "0d0a800066e6096a800066e6096a000000000000"),
            Year::state(1, 15, "0e0b800099d90e1f800099d90e1f000000000000"),
            Year::state(1, 15, "0f0c7f00989927287f0098992728000000000000"),
            Year::state(1, 15, "100d7c00f0ff8b117c00f0ff8b11000000000000"),
            Year::state(1, 15, "100d8000000000008000000000007fc8b0ccb02f"),
            Year::Absorbed,
        ],
    },
    Case {
        name: "m15-v1-s3",
        start: "01000102000100000005010d0d8000000000008000000000008100000000000f100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(1, 15, "0d0d8000000000008000000000005b0000000040"),
            Year::state(1, 15, "0e0d80b000000000800000000000000000000000"),
            Year::state(1, 15, "0f0d807001000000800000000000000000000000"),
            Year::state(1, 15, "100d80000000000080000000000060000000500c"),
            Year::Absorbed,
        ],
    },
    Case {
        name: "m15-v3-d4",
        start: "01000102000100000005030c098000000000008000000000008100000000000f100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(3, 15, "0d0a803068e6096a803068e6096a000000000000"),
            Year::state(3, 15, "100d7c0046008c117c0047008c11000000000000"),
            Year::Absorbed,
        ],
    },
    Case {
        name: "m15-v3-s3",
        start: "01000102000100000005030d0d8000000000008000000000008100000000000f100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(3, 15, "0f0d7fc0feffff7f800000000000000000000000"),
            Year::Absorbed,
        ],
    },
    Case {
        name: "m15-v5-d4",
        start: "01000102000100000005050c098000000000008000000000008100000000000f100d0100000600000100000000000000000000000000",
        years: &[
            Year::state(5, 15, "0f0c7fa0a19927287fa0a1992728000000000000"),
            Year::Absorbed,
        ],
    },
    Case {
        name: "m15-v5-s3",
        start: "01000102000100000005050d0d8000000000008000000000008100000000000f100d0100000600000100000000000000000000000000",
        years: &[Year::Absorbed],
    },
];

const HOSTILE_CASES: &[Case] = &[
    Case {
        name: "m06-v1-s3",
        start: "01000102000100000005010c0d800000000000800000000000810000000000060f0d0100000600000100000000000000000000000000",
        years: &[
            Year::state(1, 6, "0c0d8000000000008000000000005b0000000040"),
            Year::state(1, 6, "0d0d80b000000000800000000000000000000000"),
            Year::state(1, 6, "0e0d807001000000800000000000000000000000"),
            Year::state(1, 6, "0f0d80000000000080000000000062000000140b"),
            Year::Strike,
        ],
    },
    Case {
        name: "m06-v1-d3",
        start: "01000102000100000005010c0a800000000000800000000000810000000000060f0d0100000600000100000000000000000000000000",
        years: &[
            Year::state(1, 6, "0c0a8000000000008000000000005b0000000040"),
            Year::state(1, 6, "0d0b7e00cccc13547e00cccc1354000000000000"),
            Year::state(1, 6, "0d0b800066e6096a800066e6096a000000000000"),
            Year::state(1, 6, "0e0c800099d90e1f800099d90e1f000000000000"),
            Year::state(1, 6, "0f0d7f00989927287f0098992728000000000000"),
            Year::state(1, 6, "0f0d800000000000800000000000804cc24ce241"),
            Year::Strike,
        ],
    },
    Case {
        name: "m06-v1-d6",
        start: "01000102000100000005010907800000000000800000000000810000000000060f0d0100000600000100000000000000000000000000",
        years: &[
            Year::state(1, 6, "09078000000000008000000000005b0000000040"),
            Year::state(1, 6, "0a087e00cccc13547e80d0cc1354000000000000"),
            Year::state(1, 6, "0a08800066e6096a802067e6096a000000000000"),
            Year::state(1, 6, "0b09800099d90e1f80209ad90e1f000000000000"),
            Year::state(1, 6, "0c0a7f00989927287f409a992728000000000000"),
            Year::state(1, 6, "0d0b7c00f0ff8b117c0002008c11000000000000"),
            Year::state(1, 6, "0d0b800032b31d3e802033b31d3e000000000000"),
            Year::state(1, 6, "0e0c7f00ca4c45667f40cc4c4566000000000000"),
            Year::state(1, 6, "0f0d7e0062669e207e8064669e20000000000000"),
            Year::state(1, 6, "0f0d800000000000800000000000802e8699c403"),
            Year::Strike,
        ],
    },
    Case {
        name: "m06-v3-s3",
        start: "01000102000100000005030c0d800000000000800000000000810000000000060f0d0100000600000100000000000000000000000000",
        years: &[
            Year::state(3, 6, "0e0d7fc0feffff7f800000000000000000000000"),
            Year::Strike,
        ],
    },
    Case {
        name: "m06-v3-d3",
        start: "01000102000100000005030c0a800000000000800000000000810000000000060f0d0100000600000100000000000000000000000000",
        years: &[
            Year::state(3, 6, "0d0b803068e6096a803068e6096a000000000000"),
            Year::state(3, 6, "0f0d80000000000080000000000080d0c84ce241"),
            Year::Strike,
        ],
    },
    Case {
        name: "m06-v3-d6",
        start: "01000102000100000005030907800000000000800000000000810000000000060f0d0100000600000100000000000000000000000000",
        years: &[
            Year::state(3, 6, "0a08803068e6096a802068e6096a000000000000"),
            Year::state(3, 6, "0d0b7c0047008c117c0046008c11000000000000"),
            Year::state(3, 6, "0f0d7ec082669e207e8082669e20000000000000"),
            Year::Strike,
        ],
    },
    Case {
        name: "m06-v5-s3",
        start: "01000102000100000005050c0d800000000000800000000000810000000000060f0d0100000600000100000000000000000000000000",
        years: &[Year::Strike],
    },
    Case {
        name: "m06-v5-d3",
        start: "01000102000100000005050c0a800000000000800000000000810000000000060f0d0100000600000100000000000000000000000000",
        years: &[
            Year::state(5, 6, "0f0d7f60a19927287f60a1992728000000000000"),
            Year::Strike,
        ],
    },
    Case {
        name: "m06-v5-d6",
        start: "01000102000100000005050907800000000000800000000000810000000000060f0d0100000600000100000000000000000000000000",
        years: &[
            Year::state(5, 6, "0c0a7f60a19927287f60a1992728000000000000"),
            Year::state(5, 6, "0f0d80000000000080000000000080739299c403"),
            Year::Strike,
        ],
    },
    Case {
        name: "m07-v1-s3",
        start: "01000102000100000005010c0d800000000000800000000000810000000000070f0d0100000600000100000002000200000000000000",
        years: &[
            Year::state(1, 7, "0c0d8000000000008000000000005b0000000040"),
            Year::state(1, 7, "0d0d80b000000000800000000000000000000000"),
            Year::state(1, 7, "0e0d807001000000800000000000000000000000"),
            Year::state(1, 7, "0f0d80000000000080000000000062000000140b"),
            Year::Strike,
        ],
    },
    Case {
        name: "m07-v1-d3",
        start: "01000102000100000005010c0a800000000000800000000000810000000000070f0d0100000600000100000002000200000000000000",
        years: &[
            Year::state(1, 7, "0c0a8000000000008000000000005b0000000040"),
            Year::state(1, 7, "0d0b7e00cccc13547e00cccc1354000000000000"),
            Year::state(1, 7, "0d0b800066e6096a800066e6096a000000000000"),
            Year::state(1, 7, "0e0c800099d90e1f800099d90e1f000000000000"),
            Year::state(1, 7, "0f0d7f00989927287f0098992728000000000000"),
            Year::state(1, 7, "0f0d800000000000800000000000804cc24ce241"),
            Year::Strike,
        ],
    },
    Case {
        name: "m07-v1-d6",
        start: "01000102000100000005010907800000000000800000000000810000000000070f0d0100000600000100000002000200000000000000",
        years: &[
            Year::state(1, 7, "09078000000000008000000000005b0000000040"),
            Year::state(1, 7, "0a087e00cccc13547e80d0cc1354000000000000"),
            Year::state(1, 7, "0a08800066e6096a802067e6096a000000000000"),
            Year::state(1, 7, "0b09800099d90e1f80209ad90e1f000000000000"),
            Year::state(1, 7, "0c0a7f00989927287f409a992728000000000000"),
            Year::state(1, 7, "0d0b7c00f0ff8b117c0002008c11000000000000"),
            Year::state(1, 7, "0d0b800032b31d3e802033b31d3e000000000000"),
            Year::state(1, 7, "0e0c7f00ca4c45667f40cc4c4566000000000000"),
            Year::state(1, 7, "0f0d7e0062669e207e8064669e20000000000000"),
            Year::state(1, 7, "0f0d800000000000800000000000802e8699c403"),
            Year::Strike,
        ],
    },
    Case {
        name: "m07-v3-s3",
        start: "01000102000100000005030c0d800000000000800000000000810000000000070f0d0100000600000100000002000200000000000000",
        years: &[
            Year::state(3, 7, "0e0d7fc0feffff7f800000000000000000000000"),
            Year::Strike,
        ],
    },
    Case {
        name: "m07-v3-d3",
        start: "01000102000100000005030c0a800000000000800000000000810000000000070f0d0100000600000100000002000200000000000000",
        years: &[
            Year::state(3, 7, "0d0b803068e6096a803068e6096a000000000000"),
            Year::state(3, 7, "0f0d80000000000080000000000080d0c84ce241"),
            Year::Strike,
        ],
    },
    Case {
        name: "m07-v3-d6",
        start: "01000102000100000005030907800000000000800000000000810000000000070f0d0100000600000100000002000200000000000000",
        years: &[
            Year::state(3, 7, "0a08803068e6096a802068e6096a000000000000"),
            Year::state(3, 7, "0d0b7c0047008c117c0046008c11000000000000"),
            Year::state(3, 7, "0f0d7ec082669e207e8082669e20000000000000"),
            Year::Strike,
        ],
    },
    Case {
        name: "m07-v5-s3",
        start: "01000102000100000005050c0d800000000000800000000000810000000000070f0d0100000600000100000002000200000000000000",
        years: &[Year::Strike],
    },
    Case {
        name: "m07-v5-d3",
        start: "01000102000100000005050c0a800000000000800000000000810000000000070f0d0100000600000100000002000200000000000000",
        years: &[
            Year::state(5, 7, "0f0d7f60a19927287f60a1992728000000000000"),
            Year::Strike,
        ],
    },
    Case {
        name: "m07-v5-d6",
        start: "01000102000100000005050907800000000000800000000000810000000000070f0d0100000600000100000002000200000000000000",
        years: &[
            Year::state(5, 7, "0c0a7f60a19927287f60a1992728000000000000"),
            Year::state(5, 7, "0f0d80000000000080000000000080739299c403"),
            Year::Strike,
        ],
    },
    Case {
        name: "m08-v1-s3",
        start: "01000102000100000005010c0d800000000000800000000000810000000000080f0d0100000000000000000002000200000000000000",
        years: &[
            Year::state(1, 8, "0c0d8000000000008000000000005b0000000040"),
            Year::state(1, 8, "0d0d80b000000000800000000000000000000000"),
            Year::state(1, 8, "0e0d807001000000800000000000000000000000"),
            Year::state(1, 8, "0f0d80000000000080000000000062000000140b"),
            Year::Strike,
        ],
    },
    Case {
        name: "m08-v1-d3",
        start: "01000102000100000005010c0a800000000000800000000000810000000000080f0d0100000000000000000002000200000000000000",
        years: &[
            Year::state(1, 8, "0c0a8000000000008000000000005b0000000040"),
            Year::state(1, 8, "0d0b7e00cccc13547e00cccc1354000000000000"),
            Year::state(1, 8, "0d0b800066e6096a800066e6096a000000000000"),
            Year::state(1, 8, "0e0c800099d90e1f800099d90e1f000000000000"),
            Year::state(1, 8, "0f0d7f00989927287f0098992728000000000000"),
            Year::state(1, 8, "0f0d800000000000800000000000804cc24ce241"),
            Year::Strike,
        ],
    },
    Case {
        name: "m08-v1-d6",
        start: "01000102000100000005010907800000000000800000000000810000000000080f0d0100000000000000000002000200000000000000",
        years: &[
            Year::state(1, 8, "09078000000000008000000000005b0000000040"),
            Year::state(1, 8, "0a087e00cccc13547e80d0cc1354000000000000"),
            Year::state(1, 8, "0a08800066e6096a802067e6096a000000000000"),
            Year::state(1, 8, "0b09800099d90e1f80209ad90e1f000000000000"),
            Year::state(1, 8, "0c0a7f00989927287f409a992728000000000000"),
            Year::state(1, 8, "0d0b7c00f0ff8b117c0002008c11000000000000"),
            Year::state(1, 8, "0d0b800032b31d3e802033b31d3e000000000000"),
            Year::state(1, 8, "0e0c7f00ca4c45667f40cc4c4566000000000000"),
            Year::state(1, 8, "0f0d7e0062669e207e8064669e20000000000000"),
            Year::state(1, 8, "0f0d800000000000800000000000802e8699c403"),
            Year::Strike,
        ],
    },
    Case {
        name: "m08-v3-s3",
        start: "01000102000100000005030c0d800000000000800000000000810000000000080f0d0100000000000000000002000200000000000000",
        years: &[
            Year::state(3, 8, "0e0d7fc0feffff7f800000000000000000000000"),
            Year::Strike,
        ],
    },
    Case {
        name: "m08-v3-d3",
        start: "01000102000100000005030c0a800000000000800000000000810000000000080f0d0100000000000000000002000200000000000000",
        years: &[
            Year::state(3, 8, "0d0b803068e6096a803068e6096a000000000000"),
            Year::state(3, 8, "0f0d80000000000080000000000080d0c84ce241"),
            Year::Strike,
        ],
    },
    Case {
        name: "m08-v3-d6",
        start: "01000102000100000005030907800000000000800000000000810000000000080f0d0100000000000000000002000200000000000000",
        years: &[
            Year::state(3, 8, "0a08803068e6096a802068e6096a000000000000"),
            Year::state(3, 8, "0d0b7c0047008c117c0046008c11000000000000"),
            Year::state(3, 8, "0f0d7ec082669e207e8082669e20000000000000"),
            Year::Strike,
        ],
    },
    Case {
        name: "m08-v5-s3",
        start: "01000102000100000005050c0d800000000000800000000000810000000000080f0d0100000000000000000002000200000000000000",
        years: &[Year::Strike],
    },
    Case {
        name: "m08-v5-d3",
        start: "01000102000100000005050c0a800000000000800000000000810000000000080f0d0100000000000000000002000200000000000000",
        years: &[
            Year::state(5, 8, "0f0d7f60a19927287f60a1992728000000000000"),
            Year::Strike,
        ],
    },
    Case {
        name: "m08-v5-d6",
        start: "01000102000100000005050907800000000000800000000000810000000000080f0d0100000000000000000002000200000000000000",
        years: &[
            Year::state(5, 8, "0c0a7f60a19927287f60a1992728000000000000"),
            Year::state(5, 8, "0f0d80000000000080000000000080739299c403"),
            Year::Strike,
        ],
    },
];
const SECTOR_ORDER_CASES: &[Case] = &[
    Case {
        name: "m01-v1-d3",
        start: "01000102000100000005010a0a800000000000800000000000810000000000010d0d0000000600000100000000000000000000000000",
        years: &[
            Year::state(1, 1, "0a0a8000000000008000000000005b0000000040"),
            Year::state(1, 1, "0b0b7e00cccc13547e00cccc1354000000000000"),
            Year::state(1, 1, "0b0b800066e6096a800066e6096a000000000000"),
            Year::state(1, 1, "0c0c800099d90e1f800099d90e1f000000000000"),
            Year::state(0, 0, "0d0d78000052ab0778000052ab07000000000000"),
            Year::state(0, 0, "0d0d78000052ab0778000052ab07000000000000"),
        ],
    },
    Case {
        name: "m01-v1-d6",
        start: "01000102000100000005010707800000000000800000000000810000000000010d0d0000000600000100000000000000000000000000",
        years: &[
            Year::state(1, 1, "07078000000000008000000000005b0000000040"),
            Year::state(1, 1, "08087e80d0cc13547e80d0cc1354000000000000"),
            Year::state(1, 1, "0808802067e6096a802067e6096a000000000000"),
            Year::state(1, 1, "090980209ad90e1f80209ad90e1f000000000000"),
            Year::state(1, 1, "0a0a7f409a9927287f409a992728000000000000"),
            Year::state(1, 1, "0b0b7c0002008c117c0002008c11000000000000"),
            Year::state(1, 1, "0b0b802033b31d3e802033b31d3e000000000000"),
            Year::state(1, 1, "0c0c7f40cc4c45667f40cc4c4566000000000000"),
            Year::state(0, 0, "0d0d7900f0755a6e7900f0755a6e000000000000"),
            Year::state(0, 0, "0d0d7900f0755a6e7900f0755a6e000000000000"),
        ],
    },
    Case {
        name: "m01-v1-s3",
        start: "01000102000100000005010a0d800000000000800000000000810000000000010d0d0000000600000100000000000000000000000000",
        years: &[
            Year::state(1, 1, "0a0d8000000000008000000000005b0000000040"),
            Year::state(1, 1, "0b0d80b000000000800000000000000000000000"),
            Year::state(1, 1, "0c0d807001000000800000000000000000000000"),
            Year::state(0, 0, "0d0d610000000068800000000000000000000000"),
            Year::state(0, 0, "0d0d610000000068800000000000000000000000"),
        ],
    },
    Case {
        name: "m01-v3-d3",
        start: "01000102000100000005030a0a800000000000800000000000810000000000010d0d0000000600000100000000000000000000000000",
        years: &[
            Year::state(3, 1, "0b0b803068e6096a803068e6096a000000000000"),
            Year::state(0, 0, "0d0d7b00b4ec5a007b00b4ec5a00000000000000"),
            Year::state(0, 0, "0d0d7b00b4ec5a007b00b4ec5a00000000000000"),
        ],
    },
    Case {
        name: "m01-v3-d6",
        start: "01000102000100000005030707800000000000800000000000810000000000010d0d0000000600000100000000000000000000000000",
        years: &[
            Year::state(3, 1, "0808802068e6096a802068e6096a000000000000"),
            Year::state(3, 1, "0b0b7c0046008c117c0046008c11000000000000"),
            Year::state(0, 0, "0d0d7c002bd1300d7c002bd1300d000000000000"),
            Year::state(0, 0, "0d0d7c002bd1300d7c002bd1300d000000000000"),
        ],
    },
    Case {
        name: "m01-v3-s3",
        start: "01000102000100000005030a0d800000000000800000000000810000000000010d0d0000000600000100000000000000000000000000",
        years: &[
            Year::state(3, 1, "0c0d7fc0feffff7f800000000000000000000000"),
            Year::state(0, 0, "0d0d7b0084d8891d800000000000000000000000"),
            Year::state(0, 0, "0d0d7b0084d8891d800000000000000000000000"),
        ],
    },
    Case {
        name: "m01-v5-d3",
        start: "01000102000100000005050a0a800000000000800000000000810000000000010d0d0000000600000100000000000000000000000000",
        years: &[
            Year::state(0, 0, "0d0d7c0054b945677c0054b94567000000000000"),
            Year::state(0, 0, "0d0d7c0054b945677c0054b94567000000000000"),
        ],
    },
    Case {
        name: "m01-v5-d6",
        start: "01000102000100000005050707800000000000800000000000810000000000010d0d0000000600000100000000000000000000000000",
        years: &[
            Year::state(5, 1, "0a0a7f60a19927287f60a1992728000000000000"),
            Year::state(0, 0, "0d0d7c003092e3447c003092e344000000000000"),
            Year::state(0, 0, "0d0d7c003092e3447c003092e344000000000000"),
        ],
    },
    Case {
        name: "m01-v5-s3",
        start: "01000102000100000005050a0d800000000000800000000000810000000000010d0d0000000600000100000000000000000000000000",
        years: &[
            Year::state(0, 0, "0d0d7c00f0c44e6c800000000000000000000000"),
            Year::state(0, 0, "0d0d7c00f0c44e6c800000000000000000000000"),
        ],
    },
    Case {
        name: "m03-v1-d3",
        start: "01000102000100000005010a0a800000000000800000000000810000000000030d0d0000000600000100000000000000000000000000",
        years: &[
            Year::state(1, 3, "0a0a8000000000008000000000005b0000000040"),
            Year::state(1, 3, "0b0b7e00cccc13547e00cccc1354000000000000"),
            Year::state(1, 3, "0b0b800066e6096a800066e6096a000000000000"),
            Year::state(1, 3, "0c0c800099d90e1f800099d90e1f000000000000"),
            Year::state(0, 3, "0d0d78000052ab0778000052ab07000000000000"),
        ],
    },
    Case {
        name: "m03-v1-d6",
        start: "01000102000100000005010707800000000000800000000000810000000000030d0d0000000600000100000000000000000000000000",
        years: &[
            Year::state(1, 3, "07078000000000008000000000005b0000000040"),
            Year::state(1, 3, "08087e80d0cc13547e80d0cc1354000000000000"),
            Year::state(1, 3, "0808802067e6096a802067e6096a000000000000"),
            Year::state(1, 3, "090980209ad90e1f80209ad90e1f000000000000"),
            Year::state(1, 3, "0a0a7f409a9927287f409a992728000000000000"),
            Year::state(1, 3, "0b0b7c0002008c117c0002008c11000000000000"),
            Year::state(1, 3, "0b0b802033b31d3e802033b31d3e000000000000"),
            Year::state(1, 3, "0c0c7f40cc4c45667f40cc4c4566000000000000"),
            Year::state(0, 3, "0d0d7900f0755a6e7900f0755a6e000000000000"),
        ],
    },
    Case {
        name: "m03-v1-s3",
        start: "01000102000100000005010a0d800000000000800000000000810000000000030d0d0000000600000100000000000000000000000000",
        years: &[
            Year::state(1, 3, "0a0d8000000000008000000000005b0000000040"),
            Year::state(1, 3, "0b0d80b000000000800000000000000000000000"),
            Year::state(1, 3, "0c0d807001000000800000000000000000000000"),
            Year::state(0, 3, "0d0d610000000068800000000000000000000000"),
        ],
    },
    Case {
        name: "m03-v3-d3",
        start: "01000102000100000005030a0a800000000000800000000000810000000000030d0d0000000600000100000000000000000000000000",
        years: &[
            Year::state(3, 3, "0b0b803068e6096a803068e6096a000000000000"),
            Year::state(0, 3, "0d0d7b00b4ec5a007b00b4ec5a00000000000000"),
        ],
    },
    Case {
        name: "m03-v3-d6",
        start: "01000102000100000005030707800000000000800000000000810000000000030d0d0000000600000100000000000000000000000000",
        years: &[
            Year::state(3, 3, "0808802068e6096a802068e6096a000000000000"),
            Year::state(3, 3, "0b0b7c0046008c117c0046008c11000000000000"),
            Year::state(0, 3, "0d0d7c002bd1300d7c002bd1300d000000000000"),
        ],
    },
    Case {
        name: "m03-v3-s3",
        start: "01000102000100000005030a0d800000000000800000000000810000000000030d0d0000000600000100000000000000000000000000",
        years: &[
            Year::state(3, 3, "0c0d7fc0feffff7f800000000000000000000000"),
            Year::state(0, 3, "0d0d7b0084d8891d800000000000000000000000"),
        ],
    },
    Case {
        name: "m03-v5-d3",
        start: "01000102000100000005050a0a800000000000800000000000810000000000030d0d0000000600000100000000000000000000000000",
        years: &[Year::state(
            0,
            3,
            "0d0d7c0054b945677c0054b94567000000000000",
        )],
    },
    Case {
        name: "m03-v5-d6",
        start: "01000102000100000005050707800000000000800000000000810000000000030d0d0000000600000100000000000000000000000000",
        years: &[
            Year::state(5, 3, "0a0a7f60a19927287f60a1992728000000000000"),
            Year::state(0, 3, "0d0d7c003092e3447c003092e344000000000000"),
        ],
    },
    Case {
        name: "m03-v5-s3",
        start: "01000102000100000005050a0d800000000000800000000000810000000000030d0d0000000600000100000000000000000000000000",
        years: &[Year::state(
            0,
            3,
            "0d0d7c00f0c44e6c800000000000000000000000",
        )],
    },
    Case {
        name: "m10-v1-d3",
        start: "01000102000100000006010a0a8000000000008000000000008100000000000a0d0d0000000000000000000000000000000001000000",
        years: &[
            Year::state(1, 10, "0a0a8000000000008000000000005b0000000040"),
            Year::state(1, 10, "0b0b7e00cccc13547e00cccc1354000000000000"),
            Year::state(1, 10, "0b0b800066e6096a800066e6096a000000000000"),
            Year::state(1, 10, "0c0c800099d90e1f800099d90e1f000000000000"),
            Year::state(0, 10, "0d0d78000052ab0778000052ab07000000000000"),
        ],
    },
    Case {
        name: "m10-v1-d6",
        start: "010001020001000000060107078000000000008000000000008100000000000a0d0d0000000000000000000000000000000001000000",
        years: &[
            Year::state(1, 10, "07078000000000008000000000005b0000000040"),
            Year::state(1, 10, "08087e80d0cc13547e80d0cc1354000000000000"),
            Year::state(1, 10, "0808802067e6096a802067e6096a000000000000"),
            Year::state(1, 10, "090980209ad90e1f80209ad90e1f000000000000"),
            Year::state(1, 10, "0a0a7f409a9927287f409a992728000000000000"),
            Year::state(1, 10, "0b0b7c0002008c117c0002008c11000000000000"),
            Year::state(1, 10, "0b0b802033b31d3e802033b31d3e000000000000"),
            Year::state(1, 10, "0c0c7f40cc4c45667f40cc4c4566000000000000"),
            Year::state(0, 10, "0d0d7900f0755a6e7900f0755a6e000000000000"),
        ],
    },
    Case {
        name: "m10-v1-s3",
        start: "01000102000100000006010a0d8000000000008000000000008100000000000a0d0d0000000000000000000000000000000001000000",
        years: &[
            Year::state(1, 10, "0a0d8000000000008000000000005b0000000040"),
            Year::state(1, 10, "0b0d80b000000000800000000000000000000000"),
            Year::state(1, 10, "0c0d807001000000800000000000000000000000"),
            Year::state(0, 10, "0d0d610000000068800000000000000000000000"),
        ],
    },
    Case {
        name: "m10-v3-d3",
        start: "01000102000100000006030a0a8000000000008000000000008100000000000a0d0d0000000000000000000000000000000001000000",
        years: &[
            Year::state(3, 10, "0b0b803068e6096a803068e6096a000000000000"),
            Year::state(0, 10, "0d0d7b00b4ec5a007b00b4ec5a00000000000000"),
        ],
    },
    Case {
        name: "m10-v3-d6",
        start: "010001020001000000060307078000000000008000000000008100000000000a0d0d0000000000000000000000000000000001000000",
        years: &[
            Year::state(3, 10, "0808802068e6096a802068e6096a000000000000"),
            Year::state(3, 10, "0b0b7c0046008c117c0046008c11000000000000"),
            Year::state(0, 10, "0d0d7c002bd1300d7c002bd1300d000000000000"),
        ],
    },
    Case {
        name: "m10-v3-s3",
        start: "01000102000100000006030a0d8000000000008000000000008100000000000a0d0d0000000000000000000000000000000001000000",
        years: &[
            Year::state(3, 10, "0c0d7fc0feffff7f800000000000000000000000"),
            Year::state(0, 10, "0d0d7b0084d8891d800000000000000000000000"),
        ],
    },
    Case {
        name: "m10-v5-d3",
        start: "01000102000100000006050a0a8000000000008000000000008100000000000a0d0d0000000000000000000000000000000001000000",
        years: &[Year::state(
            0,
            10,
            "0d0d7c0054b945677c0054b94567000000000000",
        )],
    },
    Case {
        name: "m10-v5-d6",
        start: "010001020001000000060507078000000000008000000000008100000000000a0d0d0000000000000000000000000000000001000000",
        years: &[
            Year::state(5, 10, "0a0a7f60a19927287f60a1992728000000000000"),
            Year::state(0, 10, "0d0d7c003092e3447c003092e344000000000000"),
        ],
    },
    Case {
        name: "m10-v5-s3",
        start: "01000102000100000006050a0d8000000000008000000000008100000000000a0d0d0000000000000000000000000000000001000000",
        years: &[Year::state(
            0,
            10,
            "0d0d7c00f0c44e6c800000000000000000000000",
        )],
    },
    Case {
        name: "m14-v1-d3",
        start: "01000102000100000005010a0a8000000000008000000000008100000000000e0d0d0000000600000100000000000000000000000000",
        years: &[
            Year::state(1, 14, "0a0a8000000000008000000000005b0000000040"),
            Year::state(1, 14, "0b0b7e00cccc13547e00cccc1354000000000000"),
            Year::state(1, 14, "0b0b800066e6096a800066e6096a000000000000"),
            Year::state(1, 14, "0c0c800099d90e1f800099d90e1f000000000000"),
            Year::state(0, 14, "0d0d78000052ab0778000052ab07000000000000"),
        ],
    },
    Case {
        name: "m14-v1-d6",
        start: "010001020001000000050107078000000000008000000000008100000000000e0d0d0000000600000100000000000000000000000000",
        years: &[
            Year::state(1, 14, "07078000000000008000000000005b0000000040"),
            Year::state(1, 14, "08087e80d0cc13547e80d0cc1354000000000000"),
            Year::state(1, 14, "0808802067e6096a802067e6096a000000000000"),
            Year::state(1, 14, "090980209ad90e1f80209ad90e1f000000000000"),
            Year::state(1, 14, "0a0a7f409a9927287f409a992728000000000000"),
            Year::state(1, 14, "0b0b7c0002008c117c0002008c11000000000000"),
            Year::state(1, 14, "0b0b802033b31d3e802033b31d3e000000000000"),
            Year::state(1, 14, "0c0c7f40cc4c45667f40cc4c4566000000000000"),
            Year::state(0, 14, "0d0d7900f0755a6e7900f0755a6e000000000000"),
        ],
    },
    Case {
        name: "m14-v1-s3",
        start: "01000102000100000005010a0d8000000000008000000000008100000000000e0d0d0000000600000100000000000000000000000000",
        years: &[
            Year::state(1, 14, "0a0d8000000000008000000000005b0000000040"),
            Year::state(1, 14, "0b0d80b000000000800000000000000000000000"),
            Year::state(1, 14, "0c0d807001000000800000000000000000000000"),
            Year::state(0, 14, "0d0d610000000068800000000000000000000000"),
        ],
    },
    Case {
        name: "m14-v3-d3",
        start: "01000102000100000005030a0a8000000000008000000000008100000000000e0d0d0000000600000100000000000000000000000000",
        years: &[
            Year::state(3, 14, "0b0b803068e6096a803068e6096a000000000000"),
            Year::state(0, 14, "0d0d7b00b4ec5a007b00b4ec5a00000000000000"),
        ],
    },
    Case {
        name: "m14-v3-d6",
        start: "010001020001000000050307078000000000008000000000008100000000000e0d0d0000000600000100000000000000000000000000",
        years: &[
            Year::state(3, 14, "0808802068e6096a802068e6096a000000000000"),
            Year::state(3, 14, "0b0b7c0046008c117c0046008c11000000000000"),
            Year::state(0, 14, "0d0d7c002bd1300d7c002bd1300d000000000000"),
        ],
    },
    Case {
        name: "m14-v3-s3",
        start: "01000102000100000005030a0d8000000000008000000000008100000000000e0d0d0000000600000100000000000000000000000000",
        years: &[
            Year::state(3, 14, "0c0d7fc0feffff7f800000000000000000000000"),
            Year::state(0, 14, "0d0d7b0084d8891d800000000000000000000000"),
        ],
    },
    Case {
        name: "m14-v5-d3",
        start: "01000102000100000005050a0a8000000000008000000000008100000000000e0d0d0000000600000100000000000000000000000000",
        years: &[Year::state(
            0,
            14,
            "0d0d7c0054b945677c0054b94567000000000000",
        )],
    },
    Case {
        name: "m14-v5-d6",
        start: "010001020001000000050507078000000000008000000000008100000000000e0d0d0000000600000100000000000000000000000000",
        years: &[
            Year::state(5, 14, "0a0a7f60a19927287f60a1992728000000000000"),
            Year::state(0, 14, "0d0d7c003092e3447c003092e344000000000000"),
        ],
    },
    Case {
        name: "m14-v5-s3",
        start: "01000102000100000005050a0d8000000000008000000000008100000000000e0d0d0000000600000100000000000000000000000000",
        years: &[Year::state(
            0,
            14,
            "0d0d7c00f0c44e6c800000000000000000000000",
        )],
    },
];

/// From the 2026-10-10 deep-base sweep: the base sits over the unowned
/// world at (12,12).
const DEEP_BASE_CASES: &[Case] = &[
    Case {
        name: "m04-v1-d3-deep",
        start: "01000102000100000005010909800000000000800000000000810000000000040c0c0101000600000100000000000000000000000000",
        years: &[
            Year::state(1, 4, "09098000000000008000000000005b0000000040"),
            Year::state(1, 4, "0a0a7e00cccc13547e00cccc1354000000000000"),
            Year::state(1, 4, "0a0a800066e6096a800066e6096a000000000000"),
            Year::state(1, 4, "0b0b800099d90e1f800099d90e1f000000000000"),
            Year::state(0, 4, "0c0c78000052ab0778000052ab07000000000000"),
            Year::state(0, 4, "0c0c78000052ab0778000052ab07000000000000"),
        ],
    },
    Case {
        name: "m04-v1-d6-deep",
        start: "01000102000100000005010606800000000000800000000000810000000000040c0c0101000600000100000000000000000000000000",
        years: &[
            Year::state(1, 4, "06068000000000008000000000005b0000000040"),
            Year::state(1, 4, "07077e60d2cc13547e60d2cc1354000000000000"),
            Year::state(1, 4, "0707803869e6096a803869e6096a000000000000"),
            Year::state(1, 4, "080880709cd90e1f80709cd90e1f000000000000"),
            Year::state(1, 4, "09097fe09e9927287fe09e992728000000000000"),
            Year::state(1, 4, "0a0a7c0027008c117c0027008c11000000000000"),
        ],
    },
    Case {
        name: "m04-v3-d3-deep",
        start: "01000102000100000005030909800000000000800000000000810000000000040c0c0101000600000100000000000000000000000000",
        years: &[
            Year::state(3, 4, "0a0a803068e6096a803068e6096a000000000000"),
            Year::state(0, 4, "0c0c7b00b4ec5a007b00b4ec5a00000000000000"),
            Year::state(0, 4, "0c0c7b00b4ec5a007b00b4ec5a00000000000000"),
            Year::state(0, 4, "0c0c7b00b4ec5a007b00b4ec5a00000000000000"),
            Year::state(0, 4, "0c0c7b00b4ec5a007b00b4ec5a00000000000000"),
            Year::state(0, 4, "0c0c7b00b4ec5a007b00b4ec5a00000000000000"),
        ],
    },
    Case {
        name: "m04-v3-d6-deep",
        start: "01000102000100000005030606800000000000800000000000810000000000040c0c0101000600000100000000000000000000000000",
        years: &[
            Year::state(3, 4, "0707802868e6096a802868e6096a000000000000"),
            Year::state(3, 4, "0a0a7c0046008c117c0046008c11000000000000"),
            Year::state(0, 4, "0c0c7c002bd1300d7c002bd1300d000000000000"),
            Year::state(0, 4, "0c0c7c002bd1300d7c002bd1300d000000000000"),
            Year::state(0, 4, "0c0c7c002bd1300d7c002bd1300d000000000000"),
            Year::state(0, 4, "0c0c7c002bd1300d7c002bd1300d000000000000"),
        ],
    },
];

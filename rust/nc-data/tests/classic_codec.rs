//! The classic codecs round-trip every preserved `.DAT` file outside the
//! bytes no classic program reads (`docs/dev/ec-file-formats.md`), and read
//! the values the byte maps document.

use std::fs;
use std::path::PathBuf;

use nc_compat::classic::PLANET_RECORD_SIZE;
use nc_compat::classic::{
    ClassicFile, decode_planet, encode_planet, ignored_bytes, parse_bases_dat, parse_conquest_dat,
    parse_fleets_dat, parse_ipbm_dat, parse_planets_dat, parse_player_dat, parse_setup_dat,
    serialize_bases_dat, serialize_conquest_dat, serialize_fleets_dat, serialize_ipbm_dat,
    serialize_planets_dat, serialize_player_dat, serialize_setup_dat, without_ignored_bytes,
};
use nc_data::Real48;
use nc_data::model::{
    BuildSlot, DiplomaticRelation, Order, Planet, PlayerMode, ProductionItemKind,
};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every game directory under fixtures/ and original/ that holds a
/// PLAYER.DAT.
fn game_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![repo_root().join("original/v1.5")];
    let mut stack = vec![repo_root().join("fixtures")];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).expect("read fixture dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().is_some_and(|name| name == "PLAYER.DAT") {
                dirs.push(dir.clone());
            }
        }
    }
    dirs.sort();
    dirs
}

fn round_trip(file: ClassicFile, data: &[u8]) -> Vec<u8> {
    match file {
        ClassicFile::Player => serialize_player_dat(&parse_player_dat(data).unwrap()),
        ClassicFile::Planets => serialize_planets_dat(&parse_planets_dat(data).unwrap()),
        ClassicFile::Fleets => serialize_fleets_dat(&parse_fleets_dat(data).unwrap()),
        ClassicFile::Bases => serialize_bases_dat(&parse_bases_dat(data).unwrap()),
        ClassicFile::Ipbm => serialize_ipbm_dat(&parse_ipbm_dat(data).unwrap()),
        ClassicFile::Setup => serialize_setup_dat(&parse_setup_dat(data).unwrap()),
        ClassicFile::Conquest => serialize_conquest_dat(&parse_conquest_dat(data).unwrap()),
    }
}

#[test]
fn every_preserved_dat_round_trips_outside_the_ignored_bytes() {
    let dirs = game_dirs();
    assert!(
        dirs.len() > 40,
        "expected the fixture tree, found {}",
        dirs.len()
    );
    let mut files = 0;
    for dir in dirs {
        for file in ClassicFile::ALL {
            let path = dir.join(file.file_name());
            if !path.is_file() {
                continue;
            }
            files += 1;
            let original = fs::read(&path).unwrap();
            let encoded = round_trip(file, &original);
            assert_eq!(encoded.len(), original.len(), "{}", path.display());
            assert_eq!(
                without_ignored_bytes(file, &encoded),
                without_ignored_bytes(file, &original),
                "{}",
                path.display()
            );
            // Outside the ignored ranges the codec changes nothing, so the
            // encoding already equals the masked original there.
            for range in ignored_bytes(file, &original) {
                assert!(range.end <= original.len(), "{} {range:?}", path.display());
            }
        }
    }
    assert!(files > 300, "expected every fixture file, found {files}");
}

#[test]
fn ignored_bytes_follow_the_byte_maps() {
    let fleets = fs::read(repo_root().join("original/v1.5/FLEETS.DAT")).unwrap();
    let ranges = ignored_bytes(ClassicFile::Fleets, &fleets);
    assert_eq!(ranges.len(), fleets.len() / 54);
    assert_eq!(ranges[0], 0x34..0x36);
    assert_eq!(ranges[1], 54 + 0x34..54 + 0x36);

    let setup = fs::read(repo_root().join("original/v1.5/SETUP.DAT")).unwrap();
    assert_eq!(ignored_bytes(ClassicFile::Setup, &setup), vec![13..512]);

    let conquest = fs::read(repo_root().join("original/v1.5/CONQUEST.DAT")).unwrap();
    assert_eq!(
        ignored_bytes(ClassicFile::Conquest, &conquest),
        vec![0x0A + 8..0x3C, 0x3C + 4..0x55, 0x55..0x825]
    );

    // The shipped sample is a four-empire game.
    let player = fs::read(repo_root().join("original/v1.5/PLAYER.DAT")).unwrap();
    let ranges = ignored_bytes(ClassicFile::Player, &player);
    let first_name_len = usize::from(player[0x1B]);
    assert_eq!(ranges[0], 0x1C + first_name_len..0x30);
    assert_eq!(ranges[1], 0x4D..0x4E);
    assert_eq!(ranges[2], 0x54 + 4..0x6D);

    let planets = fs::read(repo_root().join("original/v1.5/PLANETS.DAT")).unwrap();
    let ranges = ignored_bytes(ClassicFile::Planets, &planets);
    let first_name_len = usize::from(planets[0x0F]);
    assert_eq!(ranges[0], 0x10 + first_name_len..0x24);
    assert_eq!(ranges[1], 0x5E..0x60);

    assert!(ignored_bytes(ClassicFile::Bases, &[0; 35]).is_empty());
    assert!(ignored_bytes(ClassicFile::Ipbm, &[0; 32]).is_empty());
}

#[test]
fn ecutil_init_player_records_read_as_the_byte_map_says() {
    let data = fs::read(repo_root().join("fixtures/ecutil-init/v1.5/PLAYER.DAT")).unwrap();
    let players = parse_player_dat(&data).unwrap();
    let data = fs::read(repo_root().join("fixtures/ecutil-init/v1.5/PLANETS.DAT")).unwrap();
    let planets = parse_planets_dat(&data).unwrap();
    assert_eq!(players.len(), 4);
    for (idx, player) in players.iter().enumerate() {
        assert_eq!(player.mode, PlayerMode::CivilDisorder);
        assert_eq!(player.handle, "");
        assert_eq!(
            player.page_lines, 24,
            "ECUTIL F1 writes 24 lines per screen"
        );
        assert_eq!(player.empire_name, "In Civil Disorder");
        assert_eq!(
            planets[usize::from(player.homeworld) - 1].owner.0,
            (idx + 1) as u8,
            "ECUTIL F1 points each empire at its starting world"
        );
        assert_eq!(player.planet_count, 1);
        assert_eq!(player.production, 100);
        assert_eq!(player.fleets_first.0, (idx * 4 + 1) as u16);
        assert_eq!(player.fleets_last.0, (idx * 4 + 4) as u16);
        assert_eq!(player.ipbms_first.0, 0);
        assert_eq!(player.diplomacy[..4], [DiplomaticRelation::Neutral; 4]);
        assert!(!player.autopilot);
    }
}

#[test]
fn conquest_and_setup_read_as_the_byte_maps_say() {
    let data = fs::read(repo_root().join("fixtures/ecutil-init/v1.5/CONQUEST.DAT")).unwrap();
    let conquest = parse_conquest_dat(&data).unwrap();
    assert_eq!(conquest.game_year, 3000);
    assert_eq!(conquest.player_count, 4);
    assert_eq!(conquest.maintenance_days.0, [true; 7]);
    assert_eq!(conquest.previous_production[..4], [100; 4]);
    assert_eq!(conquest.previous_planet_count[..4], [1; 4]);

    let data = fs::read(repo_root().join("original/v1.5/CONQUEST.DAT")).unwrap();
    let conquest = parse_conquest_dat(&data).unwrap();
    assert_eq!(conquest.game_year, 3022);
    assert_eq!(conquest.player_count, 4);

    let data = fs::read(repo_root().join("original/v1.5/SETUP.DAT")).unwrap();
    let setup = parse_setup_dat(&data).unwrap();
    assert_eq!(&setup.version_tag, b"EC151");
    assert_eq!(setup.com_irq, [4, 3, 4, 3]);
    assert_eq!(setup.com_hw_flow_control, [true; 4]);
    assert!(setup.snoop);
    assert_eq!(setup.max_key_gap_minutes, 10);
    assert!(setup.remote_timeout);
    assert!(!setup.local_timeout);
    assert_eq!(setup.minimum_time_minutes, 0);
    assert_eq!(setup.purge_after_turns, 0);
    assert_eq!(setup.autopilot_after_turns, 0);
}

#[test]
fn shipped_fleets_planets_and_bases_read_as_the_byte_maps_say() {
    let data = fs::read(repo_root().join("original/v1.5/FLEETS.DAT")).unwrap();
    let fleets = parse_fleets_dat(&data).unwrap();
    assert_eq!(fleets.len(), 13);
    let guard = &fleets[0];
    assert_eq!(guard.fleet_number.0, 1);
    assert_eq!(guard.owner.0, 1);
    assert_eq!(guard.order, Order::GuardStarbase);
    assert_eq!(guard.mission_target, 1);
    assert_eq!(guard.x_fraction, Real48::HALF);
    assert_eq!(guard.depth, Real48::ONE);
    // ECUTIL's stray high byte in the Guard/Blockade fleets' mission word.
    assert_eq!(fleets[1].mission_target, 0x46b0);

    let data = fs::read(repo_root().join("fixtures/ecutil-init/v1.5/PLANETS.DAT")).unwrap();
    let planets = parse_planets_dat(&data).unwrap();
    let homeworld = planets
        .iter()
        .find(|planet| planet.owner.0 == 1)
        .expect("empire 1 homeworld");
    assert_eq!(homeworld.name, "Not Named Yet");
    assert_eq!(homeworld.potential_production, 100);
    assert_eq!(homeworld.production.to_f64(), 100.0);
    assert_eq!(homeworld.stored_goods.to_f64(), 35.0);
    assert_eq!(homeworld.armies, 10);
    assert_eq!(homeworld.ground_batteries, 4);
    assert_eq!(homeworld.years_under_owner, 2);
    assert!(!homeworld.scorch_earth);
    let unowned = planets
        .iter()
        .find(|planet| planet.owner.0 == 0)
        .expect("an unowned world");
    assert_eq!(unowned.name, "Unowned");
    assert_eq!(unowned.years_under_owner, 0);

    let data = fs::read(repo_root().join("original/v1.5/BASES.DAT")).unwrap();
    let bases = parse_bases_dat(&data).unwrap();
    assert_eq!(bases.len(), 1);
    assert_eq!(bases[0].escort.0, 1);
    assert_eq!(bases[0].base_id.0, 1);
    assert_eq!(bases[0].owner.0, 1);
    assert_eq!(bases[0].sector.to_array(), [16, 13]);
    assert!(!bases[0].moving);
    assert_eq!(bases[0].target_depth, 1);

    let data = fs::read(repo_root().join("fixtures/ecutil-init/v1.5/IPBM.DAT")).unwrap();
    assert!(parse_ipbm_dat(&data).unwrap().is_empty());
}

/// One PLANETS.DAT record with the given name and sector, and nothing queued.
fn queue_record(name: &str, sector: [u8; 2], queue: &[(usize, u16)]) -> [u8; PLANET_RECORD_SIZE] {
    let mut planet = Planet::default();
    planet.sector = nc_data::Coords::from_array(sector);
    planet.name = name.to_string();
    let mut bytes = encode_planet(&planet);
    // Classic's queue is ten u16 counts by kind at 0x24 + 2 * (kind - 1).
    for (kind, count) in queue {
        let at = 0x24 + 2 * (kind - 1);
        bytes[at..at + 2].copy_from_slice(&count.to_le_bytes());
    }
    bytes
}

#[test]
fn ecmaint_build_fixture_imports_counts_as_owed_points_and_round_trips() {
    let pre_bytes =
        fs::read(repo_root().join("fixtures/ecmaint-build-pre/v1.5/PLANETS.DAT")).unwrap();
    let post_bytes =
        fs::read(repo_root().join("fixtures/ecmaint-build-post/v1.5/PLANETS.DAT")).unwrap();
    let pre = parse_planets_dat(&pre_bytes).unwrap();
    let post = parse_planets_dat(&post_bytes).unwrap();

    // 3 destroyers and 1 ETAC are owed 15 and 20 points. Neither side has
    // charged for them, so the treasury stays at the file's 35.
    let planet = &pre[14];
    assert_eq!(
        planet.build_queue[..2],
        [
            BuildSlot {
                points_owed: 15,
                kind: ProductionItemKind::Destroyer
            },
            BuildSlot {
                points_owed: 20,
                kind: ProductionItemKind::Etac
            }
        ]
    );
    assert!(
        planet.build_queue[2..]
            .iter()
            .all(|slot| *slot == BuildSlot::EMPTY)
    );
    assert_eq!(planet.stored_goods.to_f64(), 35.0);

    // Whole units owed, so export writes the counts back, credits nothing,
    // and the whole file comes back byte for byte outside the ignored bytes.
    let exported = serialize_planets_dat(&pre);
    assert_eq!(
        without_ignored_bytes(ClassicFile::Planets, &exported),
        without_ignored_bytes(ClassicFile::Planets, &pre_bytes)
    );

    assert!(
        post[14]
            .build_queue
            .iter()
            .all(|slot| *slot == BuildSlot::EMPTY)
    );
    assert_eq!(post[14].stardock.destroyers, 3);
    assert_eq!(post[14].stardock.etacs, 1);
}

#[test]
fn classic_queue_kinds_map_to_rust_kinds_by_name() {
    // Classic kinds 1..7 are the ships and the starbase, 9 the army, 10 the
    // battery; Rust codes the battery 7, the army 8, and the starbase 9.
    let bytes = queue_record(
        "Names",
        [3, 4],
        &[
            (1, 1),
            (2, 2),
            (3, 3),
            (4, 4),
            (5, 5),
            (6, 6),
            (7, 7),
            (9, 9),
            (10, 10),
        ],
    );
    let planet = decode_planet(&bytes).unwrap();
    let queue = planet
        .build_queue
        .iter()
        .filter(|slot| slot.points_owed != 0)
        .map(|slot| (slot.kind, slot.points_owed))
        .collect::<Vec<_>>();
    assert_eq!(
        queue,
        vec![
            (ProductionItemKind::Destroyer, 5),
            (ProductionItemKind::Cruiser, 30),
            (ProductionItemKind::Battleship, 135),
            (ProductionItemKind::Scout, 60),
            (ProductionItemKind::Transport, 25),
            (ProductionItemKind::Etac, 120),
            (ProductionItemKind::Starbase, 350),
            (ProductionItemKind::Army, 18),
            (ProductionItemKind::GroundBattery, 200),
        ]
    );
    assert_eq!(encode_planet(&planet), bytes);
}

#[test]
fn a_queue_past_255_points_exports_all_its_units() {
    // 128 armies are 256 points, one past what a byte held; 13 batteries
    // are 260.
    let bytes = queue_record("Garrison", [1, 1], &[(9, 128), (10, 13), (7, 1_000)]);
    let planet = decode_planet(&bytes).unwrap();
    assert_eq!(planet.build_queue[0].points_owed, 50_000);
    assert_eq!(planet.build_queue[1].points_owed, 256);
    assert_eq!(planet.build_queue[2].points_owed, 260);
    assert_eq!(encode_planet(&planet), bytes);
}

#[test]
fn slot_8_orders_are_refused_naming_the_planet() {
    let bytes = queue_record("Dust Bowl", [16, 13], &[(8, 2)]);
    let error = decode_planet(&bytes).unwrap_err().to_string();
    assert!(error.contains("planet \"Dust Bowl\" at 16,13"), "{error}");
    assert!(error.contains("2 units"), "{error}");
    assert!(error.contains("reserved build slot 8"), "{error}");

    // A whole file refuses the same way.
    let mut file = encode_planet(&Planet::default()).to_vec();
    file.extend_from_slice(&bytes);
    assert_eq!(parse_planets_dat(&file).unwrap_err().to_string(), error);
}

#[test]
fn export_credits_the_paid_part_of_a_part_paid_unit_to_stored_goods() {
    let mut planet = Planet::default();
    planet.stored_goods = Real48::from_int(10);
    // 100 points of battleship (45 each) is three units with 35 paid; 3 and
    // 4 points of destroyer (5 each) are one unit each with 2 and 1 paid.
    planet.build_queue[0] = BuildSlot {
        points_owed: 100,
        kind: ProductionItemKind::Battleship,
    };
    planet.build_queue[1] = BuildSlot {
        points_owed: 3,
        kind: ProductionItemKind::Destroyer,
    };
    planet.build_queue[2] = BuildSlot {
        points_owed: 4,
        kind: ProductionItemKind::Destroyer,
    };
    let exported = decode_planet(&encode_planet(&planet)).unwrap();
    // Two destroyers and three battleships come back whole (classic keeps
    // destroyers first), with the three paid parts, 2, 1, and 35, added to
    // the treasury.
    assert_eq!(
        exported.build_queue[..2],
        [
            BuildSlot {
                points_owed: 10,
                kind: ProductionItemKind::Destroyer
            },
            BuildSlot {
                points_owed: 135,
                kind: ProductionItemKind::Battleship
            }
        ]
    );
    assert_eq!(exported.stored_goods.to_f64(), 10.0 + 35.0 + 2.0 + 1.0);
}

#[test]
fn export_leaves_the_treasury_less_the_queue_where_rust_would() {
    // Rust charges the points owed; ECMAINT charges whole units from the
    // exported treasury. Both end at G - sum(owed) because export credits
    // the paid part of each part-paid unit.
    let mut planet = Planet::default();
    planet.stored_goods = Real48::from_int(500);
    for (slot, (points, kind)) in [
        (100, ProductionItemKind::Battleship),
        (3, ProductionItemKind::Destroyer),
        (7, ProductionItemKind::Destroyer),
        (19, ProductionItemKind::Army),
        (21, ProductionItemKind::GroundBattery),
        (1, ProductionItemKind::Starbase),
        (28, ProductionItemKind::Scout),
    ]
    .into_iter()
    .enumerate()
    {
        planet.build_queue[slot] = BuildSlot {
            points_owed: points,
            kind,
        };
    }
    let owed = |planet: &Planet| {
        planet
            .build_queue
            .iter()
            .map(|s| s.points_owed)
            .sum::<u32>()
    };

    let exported = decode_planet(&encode_planet(&planet)).unwrap();
    assert_eq!(
        exported.stored_goods.to_f64() - f64::from(owed(&exported)),
        planet.stored_goods.to_f64() - f64::from(owed(&planet))
    );
}

#[test]
fn whole_unit_orders_leave_stored_goods_bits_alone() {
    let mut planet = Planet::default();
    planet.stored_goods = Real48::from_bytes([0x86, 0x12, 0x34, 0x56, 0x78, 0x0C]);
    planet.build_queue[0] = BuildSlot {
        points_owed: 90,
        kind: ProductionItemKind::Battleship,
    };
    assert_eq!(
        decode_planet(&encode_planet(&planet)).unwrap().stored_goods,
        planet.stored_goods
    );
}

#[test]
fn a_negative_classic_balance_is_carried_in_stored_goods() {
    let mut planet = Planet::default();
    planet.stored_goods = Real48::from_int(-12);
    let decoded = decode_planet(&encode_planet(&planet)).unwrap();
    assert_eq!(decoded.stored_goods.to_f64(), -12.0);
}

#[test]
fn rust_only_queue_slots_have_no_classic_form() {
    let mut planet = Planet::default();
    planet.build_queue[9] = BuildSlot {
        points_owed: 4,
        kind: ProductionItemKind::Unknown(0xAB),
    };
    assert_eq!(encode_planet(&planet)[0x24..0x38], [0; 20]);
    assert_eq!(Planet::default().build_queue, [BuildSlot::EMPTY; 10]);
}

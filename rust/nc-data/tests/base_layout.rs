//! BASES.DAT records follow the classic layout (t037): escort word at 0x00,
//! starbase id at 0x02, owner at 0x04, chain words at 0x05/0x07/0x09,
//! position and depth reals, moving flag at 0x1f, target at 0x20..0x22.
//! See docs/dev/ec-starbase-spec.md.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use nc_compat::{export_latest_snapshot_to_dir, import_directory_snapshot};
use nc_data::{BaseDat, BaseRecord, CampaignStore, CoreGameData, DEFAULT_CAMPAIGN_DB_NAME, Real48};
use nc_engine::run_maintenance_turn;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn temp_dir(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "{label}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time ok")
            .as_nanos()
    ))
}

fn copy_dir_files(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).expect("create temp dir");
    for entry in fs::read_dir(src).expect("read src dir") {
        let path = entry.expect("dir entry").path();
        if path.is_file() {
            fs::copy(&path, dst.join(path.file_name().unwrap())).expect("copy file");
        }
    }
}

fn fixture_dir(name: &str) -> PathBuf {
    repo_root().join("fixtures").join(name).join("v1.5")
}

fn load_fixture(name: &str) -> CoreGameData {
    CoreGameData::load(&fixture_dir(name)).unwrap_or_else(|e| panic!("load fixture {name}: {e}"))
}

/// Every game directory under fixtures/ and original/ that holds a BASES.DAT.
fn bases_dat_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![repo_root().join("original/v1.5")];
    let mut stack = vec![repo_root().join("fixtures")];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).expect("read fixture dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().is_some_and(|name| name == "BASES.DAT")
                && dir.join("PLAYER.DAT").is_file()
            {
                dirs.push(dir.clone());
            }
        }
    }
    dirs.sort();
    dirs
}

#[test]
fn shipped_base_reads_the_classic_layout() {
    let bytes = fs::read(repo_root().join("original/v1.5/BASES.DAT")).unwrap();
    let bases = BaseDat::parse(&bytes).unwrap();
    assert_eq!(bases.records.len(), 1);
    let base = &bases.records[0];
    assert_eq!(base.escort_fleet_slot_raw(), 1);
    assert_eq!(base.base_id_raw(), 1);
    assert_eq!(base.owner_empire_raw(), 1);
    assert_eq!(base.next_base_link_raw(), 0);
    assert_eq!(base.record_number_raw(), 1);
    assert_eq!(base.previous_base_link_raw(), 0);
    assert_eq!(base.coords_raw(), [16, 13]);
    assert_eq!(base.x_fraction(), Real48::HALF);
    assert_eq!(base.y_fraction(), Real48::HALF);
    assert_eq!(base.depth(), Real48::ONE);
    assert_eq!(base.moving_flag_raw(), 0);
    assert_eq!(base.target_coords_raw(), [16, 13]);
    assert_eq!(base.target_depth_raw(), 1);
}

#[test]
fn base_accessors_use_the_classic_offsets() {
    let mut base = BaseRecord::new_zeroed();
    base.set_escort_fleet_slot_raw(0x0102);
    base.set_base_id_raw(0x0304);
    base.set_owner_empire_raw(5);
    base.set_next_base_link_raw(0x0607);
    base.set_record_number_raw(0x0809);
    base.set_previous_base_link_raw(0x0a0b);
    base.set_coords_raw([0x0c, 0x0d]);
    base.set_x_fraction(Real48::from_bytes([0x11, 1, 2, 3, 4, 5]));
    base.set_y_fraction(Real48::from_bytes([0x12, 1, 2, 3, 4, 5]));
    base.set_depth(Real48::from_bytes([0x13, 1, 2, 3, 4, 5]));
    base.set_moving_flag_raw(1);
    base.set_target_coords_raw([0x0e, 0x0f]);
    base.set_target_depth_raw(1);

    assert_eq!(
        base.to_bytes(),
        [
            0x02, 0x01, 0x04, 0x03, 5, 0x07, 0x06, 0x09, 0x08, 0x0b, 0x0a, 0x0c, 0x0d, 0x11, 1, 2,
            3, 4, 5, 0x12, 1, 2, 3, 4, 5, 0x13, 1, 2, 3, 4, 5, 1, 0x0e, 0x0f, 1,
        ]
    );
    assert_eq!(base.escort_fleet_slot_raw(), 0x0102);
    assert_eq!(base.base_id_raw(), 0x0304);
    assert_eq!(base.owner_empire_raw(), 5);
    assert_eq!(base.next_base_link_raw(), 0x0607);
    assert_eq!(base.record_number_raw(), 0x0809);
    assert_eq!(base.previous_base_link_raw(), 0x0a0b);
    assert_eq!(base.target_depth_raw(), 1);
}

#[test]
fn every_preserved_bases_dat_round_trips_through_the_campaign_database() {
    let dirs = bases_dat_dirs();
    assert!(
        dirs.len() > 40,
        "expected the fixture tree, found {}",
        dirs.len()
    );
    let mut non_empty = 0;
    for dir in dirs {
        let original = fs::read(dir.join("BASES.DAT")).unwrap();
        if !original.is_empty() {
            non_empty += 1;
        }
        assert_eq!(
            BaseDat::parse(&original).unwrap().to_bytes(),
            original,
            "{}",
            dir.display()
        );

        let imported = temp_dir("nc-data-base-layout-import");
        let exported = temp_dir("nc-data-base-layout-export");
        copy_dir_files(&dir, &imported);
        let store =
            CampaignStore::open(imported.join(DEFAULT_CAMPAIGN_DB_NAME)).expect("open store");
        import_directory_snapshot(&store, &imported)
            .unwrap_or_else(|e| panic!("import {}: {e}", dir.display()));
        export_latest_snapshot_to_dir(&store, &exported).expect("export snapshot");
        assert_eq!(
            fs::read(exported.join("BASES.DAT")).unwrap(),
            original,
            "{}",
            dir.display()
        );
        let _ = fs::remove_dir_all(&imported);
        let _ = fs::remove_dir_all(&exported);
    }
    assert!(
        non_empty >= 7,
        "expected the starbase fixtures, found {non_empty}"
    );
}

#[test]
fn classic_moved_bases_import_and_pass_preflight() {
    // Lift: classic took the base out of orbit when its world changed hands.
    let lift = load_fixture("ecmaint-starbase-lift-post");
    let base = &lift.bases.records[0];
    assert_eq!(base.owner_empire_raw(), 1);
    assert_eq!(base.base_id_raw(), 1);
    assert_eq!(base.target_depth_raw(), 0);
    assert_eq!(base.moving_flag_raw(), 0);
    assert_eq!(
        base.depth().to_bytes(),
        [0x7b, 0x4a, 0x9e, 0xd8, 0x89, 0x1d]
    );
    // The escort rode along: same depth, retargeted to the base's layer.
    let escort = &lift.fleets.records[0];
    assert_eq!(escort.local_slot_word_raw(), base.escort_fleet_slot_raw());
    assert_eq!(escort.depth(), base.depth());
    assert_eq!(escort.world_target_flag_raw(), 0);

    // Move: the base is under way to (18,13) in deep space.
    let moving = load_fixture("ecmaint-starbase-move-post");
    let base = &moving.bases.records[0];
    assert_eq!(base.owner_empire_raw(), 1);
    assert_eq!(base.moving_flag_raw(), 1);
    assert_eq!(base.target_coords_raw(), [18, 13]);
    assert_eq!(base.target_depth_raw(), 0);
    assert_eq!(base.depth().round(), 0);

    for mut game_data in [lift, moving] {
        assert_eq!(
            game_data.ecmaint_structural_preflight_errors(),
            Vec::<String>::new()
        );
        run_maintenance_turn(&mut game_data).expect("maintenance accepts a moved base");
    }
}

#[test]
fn owner_check_reads_the_owner_byte() {
    let mut game_data = load_fixture("ecmaint-starbase-lift-post");
    assert!(
        game_data
            .current_known_base_owner_empire_errors()
            .is_empty()
    );
    game_data.bases.records[0].set_owner_empire_raw(0);
    assert_eq!(
        game_data.current_known_base_owner_empire_errors(),
        vec!["BASES[1].owner_empire expected 1..=4, got 0".to_string()]
    );
}

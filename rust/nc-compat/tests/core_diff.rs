use std::path::PathBuf;

use nc_compat::{
    core_file_exact_match_errors, current_known_baseline_diff_counts,
    current_known_baseline_diff_offsets, load_classic_core_files,
};
use nc_data::{CoreGameData, PLANET_RECORD_SIZE};

fn fixture(relative: &str) -> CoreGameData {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative);
    load_classic_core_files(&dir).expect("fixture should load")
}

fn post_maint_fixture() -> CoreGameData {
    fixture("fixtures/ecmaint-post/v1.5")
}

#[test]
fn current_known_baseline_diff_offsets_clear_player_file_on_clean_post_fixture() {
    let diffs = current_known_baseline_diff_offsets(&post_maint_fixture());
    let player_offsets = &diffs
        .iter()
        .find(|diff| diff.name == "PLAYER.DAT")
        .unwrap()
        .differing_offsets;
    assert!(player_offsets.is_empty());
}

#[test]
fn current_known_baseline_diff_offsets_clear_planet_file_on_clean_post_fixture() {
    let diffs = current_known_baseline_diff_offsets(&post_maint_fixture());
    let planet_offsets = &diffs
        .iter()
        .find(|diff| diff.name == "PLANETS.DAT")
        .unwrap()
        .differing_offsets;
    assert!(planet_offsets.is_empty());
}

#[test]
fn current_known_baseline_diff_offsets_clear_fleet_file_on_clean_post_fixture() {
    let diffs = current_known_baseline_diff_offsets(&post_maint_fixture());
    let fleet_offsets = &diffs
        .iter()
        .find(|diff| diff.name == "FLEETS.DAT")
        .unwrap()
        .differing_offsets;
    assert!(fleet_offsets.is_empty());
}

#[test]
fn exact_match_accepts_clean_post_fixture() {
    let data = post_maint_fixture();
    assert!(core_file_exact_match_errors(&data, &data, "self").is_empty());
}

#[test]
fn sync_current_known_initialized_post_maint_baseline_matches_canonical_from_init() {
    let mut data = fixture("fixtures/ecutil-init/v1.5");

    data.sync_current_known_initialized_post_maint_baseline();

    assert!(
        core_file_exact_match_errors(&data, &post_maint_fixture(), "canonical post-maint fixture")
            .is_empty()
    );
}

#[test]
fn current_known_baseline_diff_counts_detect_mutated_files() {
    let mut data = post_maint_fixture();

    let clean_diffs = current_known_baseline_diff_counts(&data);
    let clean_setup = clean_diffs
        .iter()
        .find(|diff| diff.name == "SETUP.DAT")
        .unwrap()
        .differing_bytes;
    let clean_planets = clean_diffs
        .iter()
        .find(|diff| diff.name == "PLANETS.DAT")
        .unwrap()
        .differing_bytes;
    assert_eq!(clean_setup, 0);

    data.setup.set_version_tag(b"BAD!!");
    data.planets.records[14].set_stored_goods(3.0);

    let diffs = current_known_baseline_diff_counts(&data);
    assert!(
        diffs
            .iter()
            .any(|diff| diff.name == "SETUP.DAT" && diff.differing_bytes > clean_setup)
    );
    assert!(
        diffs
            .iter()
            .any(|diff| diff.name == "PLANETS.DAT" && diff.differing_bytes > clean_planets)
    );
}

#[test]
fn current_known_baseline_diff_offsets_pinpoint_mutated_bytes() {
    let mut data = post_maint_fixture();

    let clean_diffs = current_known_baseline_diff_offsets(&data);
    let clean_setup = clean_diffs
        .iter()
        .find(|diff| diff.name == "SETUP.DAT")
        .unwrap()
        .differing_offsets
        .clone();
    let clean_planets = clean_diffs
        .iter()
        .find(|diff| diff.name == "PLANETS.DAT")
        .unwrap()
        .differing_offsets
        .clone();

    data.setup.set_version_tag(b"BAD!!");
    data.planets.records[14].set_stored_goods(3.0);

    let diffs = current_known_baseline_diff_offsets(&data);
    let setup_offsets = &diffs
        .iter()
        .find(|diff| diff.name == "SETUP.DAT")
        .unwrap()
        .differing_offsets;
    let planet_offsets = &diffs
        .iter()
        .find(|diff| diff.name == "PLANETS.DAT")
        .unwrap()
        .differing_offsets;

    for offset in 0..5 {
        assert!(setup_offsets.contains(&offset));
    }
    assert!(setup_offsets.len() >= clean_setup.len() + 5);

    let tax_offset = 14 * PLANET_RECORD_SIZE + 0x0E;
    assert!(planet_offsets.contains(&tax_offset));
    assert!(planet_offsets.len() > clean_planets.len());
}

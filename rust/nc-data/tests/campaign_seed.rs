use std::path::PathBuf;

use nc_compat::load_classic_core_files;
use nc_data::{CoreGameData, derive_campaign_seed_from_runtime};

fn init_fixture() -> CoreGameData {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/ecutil-init/v1.5");
    load_classic_core_files(&dir).expect("fixture should load")
}

// The seed hashes the typed game state in a fixed field order. A change to
// that order or to how a field feeds the hash moves every derived seed, so
// it shows up here first.
#[test]
fn derived_seed_is_pinned_for_the_init_fixture() {
    assert_eq!(
        derive_campaign_seed_from_runtime(&init_fixture(), &[], &[]),
        12_718_367_860_779_055_136
    );
}

#[test]
fn derived_seed_changes_with_a_single_field() {
    let game_data = init_fixture();
    let mut changed = game_data.clone();
    changed.fleets[0].rules_of_engagement += 1;

    assert_ne!(
        derive_campaign_seed_from_runtime(&game_data, &[], &[]),
        derive_campaign_seed_from_runtime(&changed, &[], &[])
    );
}

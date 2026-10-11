#![allow(dead_code)]

pub mod move_scenario;
pub mod order_trace;
pub mod production;

use nc_data::Fleet;
use std::fs;
use std::path::PathBuf;

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture_file(dir: &str, name: &str) -> PathBuf {
    repo_root().join(dir).join(name)
}

pub fn read_file(dir: &str, name: &str) -> Vec<u8> {
    fs::read(fixture_file(dir, name)).expect("fixture should exist")
}

pub fn read_fixture(name: &str) -> Vec<u8> {
    read_file("original/v1.5", name)
}

pub fn read_initialized_fixture(name: &str) -> Vec<u8> {
    read_file("fixtures/ecutil-init/v1.5", name)
}

pub fn read_post_maint_fixture(name: &str) -> Vec<u8> {
    read_file("fixtures/ecmaint-post/v1.5", name)
}

pub fn read_f3_owner_fixture(name: &str) -> Vec<u8> {
    read_file("fixtures/ecutil-f3-owner/v1.5", name)
}

pub fn read_ecmaint_starbase_pre_fixture(name: &str) -> Vec<u8> {
    read_file("fixtures/ecmaint-starbase-pre/v1.5", name)
}

pub fn read_ecmaint_build_pre_fixture(name: &str) -> Vec<u8> {
    read_file("fixtures/ecmaint-build-pre/v1.5", name)
}

pub fn read_ecmaint_build_post_fixture(name: &str) -> Vec<u8> {
    read_file("fixtures/ecmaint-build-post/v1.5", name)
}

pub fn read_ecmaint_fleet_pre_fixture(name: &str) -> Vec<u8> {
    read_file("fixtures/ecmaint-fleet-pre/v1.5", name)
}

pub fn read_ecmaint_fleet_post_fixture(name: &str) -> Vec<u8> {
    read_file("fixtures/ecmaint-fleet-post/v1.5", name)
}

pub fn read_ecmaint_bombard_pre_fixture(name: &str) -> Vec<u8> {
    read_file("fixtures/ecmaint-bombard-pre/v1.5", name)
}

pub fn read_ecmaint_bombard_arrive_fixture(name: &str) -> Vec<u8> {
    read_file("fixtures/ecmaint-bombard-arrive/v1.5", name)
}

pub fn read_ecmaint_bombard_post_fixture(name: &str) -> Vec<u8> {
    read_file("fixtures/ecmaint-bombard-post/v1.5", name)
}

pub fn read_ecmaint_bombard_army0_pre_fixture(name: &str) -> Vec<u8> {
    read_file("fixtures/ecmaint-bombard-army0-pre/v1.5", name)
}

pub fn read_ecmaint_bombard_army0_post_fixture(name: &str) -> Vec<u8> {
    read_file("fixtures/ecmaint-bombard-army0-post/v1.5", name)
}

pub fn read_ecmaint_bombard_army0_dev0_pre_fixture(name: &str) -> Vec<u8> {
    read_file("fixtures/ecmaint-bombard-army0-dev0-pre/v1.5", name)
}

pub fn read_ecmaint_bombard_army0_dev0_post_fixture(name: &str) -> Vec<u8> {
    read_file("fixtures/ecmaint-bombard-army0-dev0-post/v1.5", name)
}

pub fn read_ecmaint_bombard_army1_pre_fixture(name: &str) -> Vec<u8> {
    read_file("fixtures/ecmaint-bombard-army1-pre/v1.5", name)
}

pub fn read_ecmaint_bombard_army1_post_fixture(name: &str) -> Vec<u8> {
    read_file("fixtures/ecmaint-bombard-army1-post/v1.5", name)
}

pub fn read_ecmaint_bombard_army1_dev0_pre_fixture(name: &str) -> Vec<u8> {
    read_file("fixtures/ecmaint-bombard-army1-dev0-pre/v1.5", name)
}

pub fn read_ecmaint_bombard_army1_dev0_post_fixture(name: &str) -> Vec<u8> {
    read_file("fixtures/ecmaint-bombard-army1-dev0-post/v1.5", name)
}

pub fn read_ecmaint_bombard_army1_dev0_e0c_pre_fixture(name: &str) -> Vec<u8> {
    read_file("fixtures/ecmaint-bombard-army1-dev0-e0c-pre/v1.5", name)
}

pub fn read_ecmaint_bombard_army1_dev0_e0c_post_fixture(name: &str) -> Vec<u8> {
    read_file("fixtures/ecmaint-bombard-army1-dev0-e0c-post/v1.5", name)
}

pub fn read_ecmaint_bombard_army1_dev0_b08_pre_fixture(name: &str) -> Vec<u8> {
    read_file("fixtures/ecmaint-bombard-army1-dev0-b08-pre/v1.5", name)
}

pub fn read_ecmaint_bombard_army1_dev0_b08_post_fixture(name: &str) -> Vec<u8> {
    read_file("fixtures/ecmaint-bombard-army1-dev0-b08-post/v1.5", name)
}

pub fn read_ecmaint_bombard_army1_dev0_b09_pre_fixture(name: &str) -> Vec<u8> {
    read_file("fixtures/ecmaint-bombard-army1-dev0-b09-pre/v1.5", name)
}

pub fn read_ecmaint_bombard_army1_dev0_b09_post_fixture(name: &str) -> Vec<u8> {
    read_file("fixtures/ecmaint-bombard-army1-dev0-b09-post/v1.5", name)
}

pub fn read_ecmaint_bombard_heavy_pre_fixture(name: &str) -> Vec<u8> {
    read_file("fixtures/ecmaint-bombard-heavy-pre/v1.5", name)
}

pub fn read_ecmaint_bombard_heavy_post_fixture(name: &str) -> Vec<u8> {
    read_file("fixtures/ecmaint-bombard-heavy-post/v1.5", name)
}

/// A contact stalled where it stands: with no speed and no maximum speed the
/// dispatcher prelude has nothing to restore, so it ends the year in place
/// whatever its order and target.
pub fn stall_in_place(fleet: &mut Fleet) {
    fleet.speed = 0;
    fleet.max_speed = 0;
}

/// The first seeded year of `game_data` whose events satisfy `wanted`.
/// Encounters are random (sighting and intercept rolls each tick), so tests
/// of what happens in a battle search the seeds for a year that has one.
pub fn first_year(
    game_data: &nc_data::CoreGameData,
    diplomacy: &[nc_data::DiplomacyOverride],
    wanted: impl Fn(&nc_data::CoreGameData, &nc_data::MaintenanceEvents) -> bool,
) -> (nc_data::CoreGameData, nc_data::MaintenanceEvents) {
    for seed in 0..500 {
        let mut year = game_data.clone();
        let events =
            nc_engine::run_maintenance_turn_with_context_and_seed(&mut year, seed, &[], diplomacy)
                .expect("maintenance should succeed");
        if wanted(&year, &events) {
            return (year, events);
        }
    }
    panic!("no seed in 0..500 gave the wanted year");
}

/// A year with a fleet battle.
pub fn battle(_: &nc_data::CoreGameData, events: &nc_data::MaintenanceEvents) -> bool {
    !events.fleet_battle_events.is_empty()
}

/// Set every fleet of `empire` to ROE 0, so its fleets never start a fight
/// (they still defend themselves if attacked).
pub fn hold_fire(game_data: &mut nc_data::CoreGameData, empire: u8) {
    for fleet in &mut game_data.fleets {
        if fleet.owner.0 == empire {
            fleet.rules_of_engagement = 0;
        }
    }
}

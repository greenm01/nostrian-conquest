#![allow(dead_code)]

pub mod order_trace;
pub mod production;

use nc_data::FleetRecord;
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
pub fn stall_in_place(fleet: &mut FleetRecord) {
    fleet.set_current_speed(0);
    fleet.set_max_speed(0);
}

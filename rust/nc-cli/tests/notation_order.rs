//! Every compact composition the CLI and the shared planet and fleet
//! formatters write lists its classes in the notation's one order,
//! `SC BB CA DD TT* TT ETAC SB` (`docs/dev/ship-notation.md`).

mod common;

use common::{cleanup_dir, copy_fixture_dir, run_nc_cli, unique_temp_dir};
use nc_data::{
    AutoCommissionFleetEntry, CompactUnitSummaryStyle, Fleet, FleetDetachSelection, Planet,
    ProductionItemKind, fleet_notation, format_build_queue_summary, format_stardock_summary,
    planet_derived_summary, resolve_selection, set_armies_aboard, set_troop_transports,
    stardock_slot_for_kind,
};

/// The class codes of a line's composition tokens (`CA12!3` is `CA`), in
/// the order they appear.
fn codes(line: &str) -> Vec<String> {
    line.split_whitespace()
        .filter(|word| word.ends_with(|c: char| c.is_ascii_digit()))
        .filter_map(|word| {
            let code = word.trim_end_matches(|c: char| c.is_ascii_digit() || c == '!');
            (!code.is_empty() && code.chars().all(|c| c.is_ascii_uppercase() || c == '*'))
                .then(|| code.to_string())
        })
        .collect()
}

/// Asserts that `line` holds exactly `expected` classes, in that order.
fn assert_classes(line: &str, expected: &[&str]) {
    assert_eq!(codes(line), expected, "{line}");
}

fn all_classes_fleet() -> Fleet {
    let mut fleet = Fleet::default();
    fleet.ships.scouts = 1;
    fleet.ships.battleships = 2;
    fleet.ships.cruisers = 3;
    fleet.ships.destroyers = 4;
    set_troop_transports(&mut fleet, 9);
    set_armies_aboard(&mut fleet, 4);
    fleet.ships.etacs = 6;
    fleet
}

#[test]
fn fleet_commission_and_detach_notations_follow_the_order() {
    let fleet = all_classes_fleet();
    let all = ["SC", "BB", "CA", "DD", "TT*", "TT", "ETAC"];
    assert_eq!(fleet_notation(&fleet), "SC1 BB2 CA3 DD4 TT*4 TT5 ETAC6");
    assert_classes(&fleet_notation(&fleet), &all);

    let entry = AutoCommissionFleetEntry {
        fleet_number: 1,
        planet_name: String::new(),
        coords: [1, 1],
        destroyers: 4,
        cruisers: 3,
        battleships: 2,
        scouts: 1,
        transports: 5,
        etacs: 6,
    };
    assert_classes(
        &nc_data::auto_commission_notation(&entry),
        &["SC", "BB", "CA", "DD", "TT", "ETAC"],
    );

    let everything = FleetDetachSelection {
        scouts: 1,
        battleships: 2,
        cruisers: 3,
        destroyers: 4,
        full_transports: 4,
        empty_transports: 5,
        etacs: 6,
        ..FleetDetachSelection::default()
    };
    let resolved = resolve_selection(&fleet, &everything).expect("the fleet has every ship");
    assert_classes(&resolved.notation(), &all);
}

#[test]
fn planet_stardock_and_build_queue_summaries_follow_the_order() {
    let mut planet = Planet::default();
    // Queue and dock every kind, in the opposite order to the notation.
    for (slot, code) in (1u8..=9).rev().enumerate() {
        planet.build_queue[slot].kind = ProductionItemKind::from_code(code);
        planet.build_queue[slot].points_owed = 1000;
        if let Some(dock) = stardock_slot_for_kind(code) {
            planet.stardock[dock] = u32::from(code);
        }
    }

    let queue = format_build_queue_summary(&planet, CompactUnitSummaryStyle::Notation);
    assert_classes(
        &queue,
        &["SC", "BB", "CA", "DD", "TT", "ETAC", "SB", "AR", "GB"],
    );
    let dock = format_stardock_summary(&planet, CompactUnitSummaryStyle::Notation);
    assert_classes(&dock, &["SC", "BB", "CA", "DD", "TT", "ETAC", "SB"]);
    // The inspection summary reads the queue the same way.
    let inspected = planet_derived_summary(&planet);
    let build = inspected
        .split(" | ")
        .find_map(|part| part.strip_prefix("build="))
        .expect("a queued planet shows its build");
    assert_eq!(build, queue);
}

#[test]
fn cli_feedback_lines_follow_the_order() {
    let target = unique_temp_dir("nc-cli-notation-order");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);
    let dir = target.to_str().unwrap();

    let set = run_nc_cli(&["fleet-ships", dir, "1", "1", "2", "3", "4", "5", "2", "6"]);
    assert!(
        set.contains("Fleet 1 ships set: SC1 BB2 CA3 DD4 TT*2 TT3 ETAC6"),
        "{set}"
    );

    let detached = run_nc_cli(&[
        "fleet-detach",
        dir,
        "1",
        "1",
        "1",
        "1",
        "1",
        "1",
        "1",
        "1",
        "1",
        "3",
        "6",
    ]);
    assert_classes(&detached, &["SC", "BB", "CA", "DD", "TT*", "TT", "ETAC"]);

    let invade = run_nc_cli(&[
        "invade-onefleet",
        dir,
        "15",
        "13",
        "1",
        "2",
        "3",
        "4",
        "5",
        "9",
    ]);
    assert_classes(&invade, &["SC", "BB", "CA", "DD", "TT"]);

    let econ = run_nc_cli(&[
        "econ", dir, "15", "13", "1", "2", "3", "15", "13", "142", "15",
    ]);
    assert_classes(&econ, &["BB", "CA", "DD"]);

    cleanup_dir(&target);
}

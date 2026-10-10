mod common;

use std::collections::BTreeSet;
use std::fs;

use nc_data::{CampaignStore, IntelTier};

use common::{cleanup_dir, run_nc_cli, unique_temp_dir};

fn write_file(path: &std::path::Path, text: &str) {
    fs::write(path, text).unwrap();
}

#[test]
fn harness_run_scenario_creates_runtime_snapshot() {
    let scenario_root = unique_temp_dir("nc-cli-harness-scenario");
    let target_dir = unique_temp_dir("nc-cli-harness-scenario-out");
    let turn_path = scenario_root.join("player1-turn.kdl");
    let scenario_path = scenario_root.join("scenario.kdl");

    write_file(
        &turn_path,
        "turn player=1 year=3000\n\
         tax rate=40\n",
    );
    write_file(
        &scenario_path,
        r#"scenario player_count=4 year=3000 baseline="builder-compatible" seed=1515 label="CLI Scenario"

house record=1 handle="SYSOP" empire="Aurora" homeworld="Aurora Prime"
planet record=1 {
  name "Aurora Prime"
  stardock slot=1 kind="destroyer" count=2
  commission slot=1
}
turn-file path="player1-turn.kdl"
results-block player=1 "Ready to launch"
"#,
    );

    let stdout = run_nc_cli(&[
        "harness",
        "run-scenario",
        "--file",
        scenario_path.to_str().unwrap(),
        "--dir",
        target_dir.to_str().unwrap(),
    ]);

    assert!(stdout.contains("Saved runtime scenario"));
    let store = CampaignStore::open_default_in_dir(&target_dir).unwrap();
    let state = store.load_latest_runtime_state().unwrap().unwrap();
    assert_eq!(state.game_data.player.records[0].tax_rate(), 40);
    assert_eq!(state.report_block_rows.len(), 1);

    cleanup_dir(&scenario_root);
    cleanup_dir(&target_dir);
}

#[test]
fn harness_run_sweep_prints_case_summary() {
    let root = unique_temp_dir("nc-cli-harness-sweep");
    let combat_path = root.join("combat.kdl");
    let sweep_path = root.join("sweep.kdl");

    write_file(
        &combat_path,
        r#"combat-scenario player_count=4 year=3001 baseline="builder-compatible" seed=1515 turns=1 label="CLI Combat"

relation from=1 to=2 status="enemy"
relation from=2 to=1 status="enemy"

fleet record=1 {
  coords x=10 y=10
  ships bb=1 ca=0 dd=0 sc=0 tt=0 armies=0 etac=0
  roe value=10
  order kind="hold" speed=0 x=10 y=10
}

fleet record=5 {
  coords x=10 y=10
  ships bb=1 ca=0 dd=0 sc=0 tt=0 armies=0 etac=0
  roe value=10
  order kind="hold" speed=0 x=10 y=10
}
"#,
    );
    write_file(
        &sweep_path,
        r#"combat-sweep scenario="combat.kdl" turns=1 seed=99 max_cases=2
fleet-ship fleet=1 kind="bb" 1 2
"#,
    );

    let stdout = run_nc_cli(&[
        "harness",
        "run-sweep",
        "--file",
        sweep_path.to_str().unwrap(),
    ]);

    assert!(stdout.contains("Executed combat sweep."));
    assert!(stdout.contains("executed_cases=2"));

    cleanup_dir(&root);
}

#[test]
fn harness_list_report_families_prints_registry() {
    let stdout = run_nc_cli(&["harness", "list-report-families"]);
    assert!(stdout.contains("Available report preview families."));
    assert!(stdout.contains("bombard [implemented]"));
    assert!(stdout.contains("mission-retarget [stub]"));
}

#[test]
fn harness_preview_reports_prints_attacker_and_defender_sections() {
    let stdout = run_nc_cli(&[
        "harness",
        "preview-reports",
        "--family",
        "bombard",
        "--seed",
        "1515",
        "--samples",
        "1",
    ]);
    assert!(stdout.contains("Previewed report families."));
    assert!(stdout.contains("family=bombard"));
    assert!(stdout.contains("attacker (empire=1):"));
    assert!(stdout.contains("defender (empire=2):"));
    assert!(stdout.contains("ALERT: Orbital bombardment underway!"));
}

#[test]
fn harness_preview_reports_all_lists_stubbed_skips() {
    let stdout = run_nc_cli(&[
        "harness",
        "preview-reports",
        "--family",
        "all",
        "--seed",
        "1515",
        "--samples",
        "1",
    ]);
    assert!(stdout.contains("family=bombard"));
    assert!(stdout.contains("family=fleet-battle"));
    assert!(stdout.contains("stubbed families skipped="));
}

#[test]
fn harness_seed_player1_tui_stress_populates_player1_runtime_backlog_and_intel() {
    let target = unique_temp_dir("nc-cli-harness-player1-tui-stress");
    let stdout = run_nc_cli(&[
        "sysop",
        "new-game",
        target.to_str().unwrap(),
        "--players",
        "12",
        "--seed",
        "1515",
    ]);
    assert!(stdout.contains("Initialized new game"));

    for (record, handle, empire) in [
        ("1", "p1", "Aurora"),
        ("2", "p2", "Red Horizon Pact"),
        ("3", "p3", "Vela Syndicate"),
        ("4", "p4", "Helios Crown"),
    ] {
        run_nc_cli(&[
            "player-name",
            target.to_str().unwrap(),
            record,
            handle,
            empire,
        ]);
    }

    let stdout = run_nc_cli(&[
        "harness",
        "seed-player1-tui-stress",
        "--dir",
        target.to_str().unwrap(),
    ]);
    assert!(stdout.contains("Seeded player-1 TUI stress runtime state"));

    let store = CampaignStore::open_default_in_dir(&target).unwrap();
    let state = store.load_latest_runtime_state().unwrap().unwrap();
    let unique_coords = state
        .game_data
        .planets
        .records
        .iter()
        .map(|planet| planet.coords_raw())
        .collect::<BTreeSet<_>>();
    assert_eq!(state.game_data.planets.records.len(), 60);
    assert_eq!(unique_coords.len(), 60);
    assert!(
        unique_coords
            .iter()
            .all(|coords| { (1..=36).contains(&coords[0]) && (1..=36).contains(&coords[1]) })
    );
    assert!(state.report_block_rows.len() >= 8);
    assert!(
        state
            .queued_mail
            .iter()
            .all(|mail| mail.recipient_empire_id == 1),
        "expected player-1-only queued mail"
    );
    assert!(state.queued_mail.len() >= 10);
    assert!(
        state.game_data.fleets.records.iter().any(|fleet| {
            fleet.owner_empire_raw() == 1
                && fleet.current_location_coords_raw()
                    == state.game_data.planets.records[0].coords_raw()
                && fleet.troop_transport_count() >= 4
                && fleet.army_count() == 0
        }),
        "expected empty troop transports at player 1 homeworld"
    );
    assert!(
        state
            .game_data
            .fleets
            .records
            .iter()
            .any(|fleet| fleet.owner_empire_raw() == 1 && fleet.army_count() > 0),
        "expected at least one loaded player 1 transport fleet"
    );
    assert!(
        state
            .game_data
            .bases
            .records
            .iter()
            .any(|base| base.owner_empire_raw() == 1 && base.base_id_raw() != 0)
    );

    let viewer1 = store.latest_planet_intel_for_viewer(1).unwrap();
    assert!(
        viewer1
            .iter()
            .any(|snapshot| snapshot.intel_tier == IntelTier::Partial)
    );
    let full = viewer1
        .iter()
        .find(|snapshot| {
            snapshot.intel_tier == IntelTier::Full
                && snapshot.known_docked_summary.is_some()
                && snapshot.known_orbit_summary.is_some()
        })
        .unwrap();
    assert!(full.known_name.is_some());

    let seeded_foreign = viewer1
        .iter()
        .find(|snapshot| snapshot.known_name.as_deref() == Some("Vela 1"))
        .unwrap();
    assert_eq!(seeded_foreign.intel_tier, IntelTier::Full);
    assert!(seeded_foreign.known_docked_summary.is_some());

    let viewer2 = store.latest_planet_intel_for_viewer(2).unwrap();
    let viewer2_snapshot = viewer2
        .iter()
        .find(|snapshot| {
            snapshot.planet_record_index_1_based == seeded_foreign.planet_record_index_1_based
        })
        .unwrap();
    assert_eq!(viewer2_snapshot.intel_tier, IntelTier::Unknown);
    assert!(viewer2_snapshot.known_name.is_none());

    let out_txt = target.join("map.txt");
    run_nc_cli(&[
        "map-export",
        target.to_str().unwrap(),
        "1",
        out_txt.to_str().unwrap(),
    ]);
    let csv = fs::read_to_string(target.join("map.csv")).unwrap();
    assert_eq!(csv.matches('*').count(), 60);

    cleanup_dir(&target);
}

#[test]
fn harness_seed_nc_helm_lab_creates_one_seeded_campaign_per_map_size_tier() {
    let root = unique_temp_dir("nc-cli-harness-helm-lab");
    let stdout = run_nc_cli(&[
        "harness",
        "seed-nc-helm-lab",
        "--root",
        root.to_str().unwrap(),
        "--seed-base",
        "4000",
    ]);
    assert!(stdout.contains("Seeded nc-helm lab"));
    let mut saw_active_player1_starbase = false;

    for (dir_name, player_count, map_size) in [
        ("map18-p4", 4u8, 18u8),
        ("map27-p9", 9u8, 27u8),
        ("map36-p16", 16u8, 36u8),
        ("map45-p25", 25u8, 45u8),
    ] {
        let dir = root.join(dir_name);
        assert!(
            dir.join("ncgame.db").exists(),
            "expected ncgame.db in {dir_name}"
        );
        let store = CampaignStore::open_default_in_dir(&dir).unwrap();
        let state = store.load_latest_runtime_state().unwrap().unwrap();
        assert_eq!(state.game_data.conquest.player_count(), player_count);
        assert_eq!(
            state.game_data.planets.records.len(),
            player_count as usize * 5
        );
        assert!(
            state.game_data.planets.records.iter().all(|planet| {
                (1..=map_size).contains(&planet.coords_raw()[0])
                    && (1..=map_size).contains(&planet.coords_raw()[1])
            }),
            "expected all planet coordinates to fit inside {map_size}x{map_size} for {dir_name}"
        );
        assert!(state.report_block_rows.len() >= 8);
        assert!(state.queued_mail.len() >= 10);
        assert_eq!(
            state.game_data.player.records[0].controlled_empire_name_summary(),
            "Aurora League"
        );
        assert_eq!(
            state.game_data.player.records[0].assigned_player_handle_summary(),
            "p01"
        );
        let viewer1 = store.latest_planet_intel_for_viewer(1).unwrap();
        assert!(
            viewer1
                .iter()
                .any(|snapshot| snapshot.intel_tier == IntelTier::Partial)
        );
        saw_active_player1_starbase |= state
            .game_data
            .bases
            .records
            .iter()
            .any(|base| base.owner_empire_raw() == 1 && base.base_id_raw() != 0);
    }

    let manifest = fs::read_to_string(root.join("README.txt")).unwrap();
    assert!(manifest.contains("map45-p25"));
    assert!(manifest.contains("launch=cargo run -q -p nc-helm -- --dir"));
    assert!(
        saw_active_player1_starbase,
        "expected at least one lab campaign to include an active player-1 starbase"
    );

    cleanup_dir(&root);
}

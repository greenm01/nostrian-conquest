use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use nc_harness::{
    CombatScenarioSpec, CombatSweepSpec, ReportPreviewQuery, ScenarioSpec, build_scenario,
    list_report_preview_families, run_combat_scenario, run_combat_sweep, run_report_preview,
};

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let target = std::env::temp_dir().join(format!("{prefix}-{unique}"));
    fs::create_dir_all(&target).unwrap();
    target
}

fn write_file(path: &Path, text: &str) {
    fs::write(path, text).unwrap();
}

#[test]
fn scenario_build_applies_turn_files_reports_and_commissions() {
    let dir = unique_temp_dir("nc-harness-scenario");
    let turn_path = dir.join("player1-turn.kdl");
    write_file(
        &turn_path,
        "turn player=1 year=3000\n\
         tax rate=35\n",
    );

    let scenario_path = dir.join("scenario.kdl");
    write_file(
        &scenario_path,
        r#"scenario player_count=4 year=3000 baseline="builder-compatible" seed=1515 label="Playtest"

house record=1 handle="SYSOP" empire="Aurora" homeworld="Aurora Prime" tax=25
house record=2 handle="RIVAL" empire="Helios"

relation from=1 to=2 status="enemy"
relation from=2 to=1 status="enemy"

planet record=1 {
  name "Aurora Prime"
  production potential=140 present=120 stored=80 economy_marker=25
  defenses armies=14 batteries=6
  stardock slot=1 kind="destroyer" count=3
  commission slot=1
}

fleet record=1 {
  coords x=10 y=10
  ships bb=0 ca=1 dd=2 sc=0 tt=1 armies=1 etac=0
  roe value=8
  order kind="hold" speed=0 x=10 y=10
}

turn-file path="player1-turn.kdl"
queued-mail from=1 to=2 year=3000 subject="Border" body="Hold line."
results-block player=1 "Command summary\nTurn 3 ready."
messages-block player=1 "Incoming traffic\nStand by."
"#,
    );

    let spec = ScenarioSpec::load_kdl(&scenario_path).unwrap();
    let built = build_scenario(&spec).unwrap();

    assert_eq!(built.game_data.player.records[0].tax_rate(), 35);
    assert_eq!(
        built.game_data.player.records[0].assigned_player_handle_summary(),
        "SYSOP"
    );
    assert_eq!(
        built.game_data.player.records[0].controlled_empire_name_summary(),
        "Aurora"
    );
    assert_eq!(built.game_data.fleets.records.len(), 17);
    assert_eq!(built.queued_mail.len(), 2);
    assert_eq!(built.report_block_rows.len(), 1);
    assert!(
        built.game_data.player.records[0].has_classic_results_review_state()
            || built.game_data.player.records[0].classic_reports_pending_flag_raw() != 0
    );
}

#[test]
fn combat_scenario_runs_and_reports_battle_metrics() {
    let dir = unique_temp_dir("nc-harness-combat");
    let combat_path = dir.join("combat.kdl");
    write_file(
        &combat_path,
        r#"combat-scenario player_count=4 year=3001 baseline="builder-compatible" seed=1515 turns=1 label="Skirmish"

house record=1 empire="Aurora"
house record=2 empire="Helios"

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

    let spec = CombatScenarioSpec::load_kdl(&combat_path).unwrap();
    let run = run_combat_scenario(&spec).unwrap();

    assert!(run.report.fleet_battle_events >= 1);
    assert_eq!(run.report.maintenance_turns, 1);
    assert!(run.report.elapsed_millis <= u128::MAX);
}

#[test]
fn combat_sweep_caps_cases_and_reports_summary() {
    let dir = unique_temp_dir("nc-harness-sweep");
    let combat_path = dir.join("combat.kdl");
    let sweep_path = dir.join("sweep.kdl");
    write_file(
        &combat_path,
        r#"combat-scenario player_count=4 year=3001 baseline="builder-compatible" seed=1515 turns=1 label="Sweep Base"

house record=1 empire="Aurora"
house record=2 empire="Helios"

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
        r#"combat-sweep scenario="combat.kdl" turns=1 seed=99 max_cases=3
fleet-ship fleet=1 kind="bb" 1 2
fleet-roe fleet=5 6 10
"#,
    );

    let spec = CombatSweepSpec::load_kdl(&sweep_path).unwrap();
    let report = run_combat_sweep(&spec).unwrap();

    assert_eq!(report.total_possible_cases, 4);
    assert_eq!(report.executed_cases, 3);
    assert_eq!(report.cases.len(), 3);
}

#[test]
fn combat_scenario_submits_turn_files_before_their_turn() {
    let dir = unique_temp_dir("nc-harness-turns");
    write_file(
        &dir.join("turn-1.kdl"),
        "turn player=2 year=3000\ntax rate=40\n",
    );
    write_file(
        &dir.join("turn-2.kdl"),
        "turn player=2 year=3001\ntax rate=60\n",
    );
    let combat_path = dir.join("combat.kdl");
    write_file(
        &combat_path,
        r#"combat-scenario player_count=2 year=3000 baseline="builder-compatible" seed=7 turns=2

house record=1 autopilot=#true
house record=2

turn-file path="turn-2.kdl" turn=2
turn-file path="turn-1.kdl"
"#,
    );

    let spec = CombatScenarioSpec::load_kdl(&combat_path).unwrap();
    let built = build_scenario(&spec.scenario).unwrap();
    assert_eq!(built.game_data.player.records[1].tax_rate(), 40);
    assert!(built.game_data.player.records[1].is_active_human_player());
    assert_eq!(built.game_data.player.records[0].autopilot_flag(), 1);
    assert_eq!(built.game_data.player.records[1].autopilot_flag(), 0);

    let run = run_combat_scenario(&spec).unwrap();
    assert_eq!(run.report.final_year, 3002);
    assert_eq!(run.built.game_data.player.records[1].tax_rate(), 60);
}

#[test]
fn report_preview_family_registry_exposes_implemented_and_stub_families() {
    let families = list_report_preview_families();
    assert!(families.iter().any(|family| family.key == "bombard"));
    assert!(
        families
            .iter()
            .any(|family| family.key == "mission-retarget")
    );
}

#[test]
fn report_preview_is_deterministic_for_same_query() {
    let query = ReportPreviewQuery {
        family: "bombard".to_string(),
        seed: 1515,
        samples: 2,
    };
    let left = run_report_preview(&query).unwrap();
    let right = run_report_preview(&query).unwrap();
    assert_eq!(left, right);
}

#[test]
fn report_preview_changes_asset_mix_when_seed_changes() {
    let left = run_report_preview(&ReportPreviewQuery {
        family: "bombard".to_string(),
        seed: 1515,
        samples: 1,
    })
    .unwrap();
    let right = run_report_preview(&ReportPreviewQuery {
        family: "bombard".to_string(),
        seed: 1516,
        samples: 1,
    })
    .unwrap();
    assert_ne!(
        left.family_runs[0].cases[0].asset_summary,
        right.family_runs[0].cases[0].asset_summary
    );
}

#[test]
fn assault_preview_families_surface_attacker_and_defender_sections() {
    for family in ["bombard", "invade", "blitz"] {
        let run = run_report_preview(&ReportPreviewQuery {
            family: family.to_string(),
            seed: 1515,
            samples: 1,
        })
        .unwrap();
        let roles = run.family_runs[0].cases[0]
            .viewer_reports
            .iter()
            .map(|viewer| viewer.role)
            .collect::<Vec<_>>();
        assert!(
            roles.contains(&"attacker"),
            "{family} should expose attacker"
        );
        assert!(
            roles.contains(&"defender"),
            "{family} should expose defender"
        );
    }
}

#[test]
fn fleet_destroyed_preview_surfaces_both_destroyed_and_survivor_wording() {
    let run = run_report_preview(&ReportPreviewQuery {
        family: "fleet-destroyed".to_string(),
        seed: 1515,
        samples: 1,
    })
    .unwrap();
    let case = &run.family_runs[0].cases[0];
    let attacker = case
        .viewer_reports
        .iter()
        .find(|viewer| viewer.role == "attacker")
        .unwrap();
    let defender = case
        .viewer_reports
        .iter()
        .find(|viewer| viewer.role == "defender")
        .unwrap();
    assert!(
        attacker
            .reports
            .iter()
            .any(|report| report.contains("ALERT: Enemy fleet contact!"))
    );
    assert!(
        defender
            .reports
            .iter()
            .any(|report| report.contains("ALERT: Fleet contact lost!"))
    );
}

#[test]
fn encounter_preview_reports_retain_disposition_wording() {
    let run = run_report_preview(&ReportPreviewQuery {
        family: "encounter-retreated".to_string(),
        seed: 1515,
        samples: 1,
    })
    .unwrap();
    let case = &run.family_runs[0].cases[0];
    let reporting = case
        .viewer_reports
        .iter()
        .find(|viewer| viewer.role == "reporting-fleet")
        .unwrap();
    assert!(reporting.reports.iter().any(
        |report| report.contains("we withdrew toward") || report.contains("We withdrew toward")
    ));
}

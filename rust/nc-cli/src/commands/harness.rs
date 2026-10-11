use std::path::PathBuf;

use nc_harness::{
    BuiltScenario, CombatScenarioSpec, CombatSweepSpec, ScenarioBuildReport, ScenarioSpec,
    build_scenario, list_report_preview_families, run_combat_scenario, run_combat_sweep,
    run_report_preview, save_built_scenario,
};

use crate::commands::harness_campaign::{
    run_apply_turn_batch_args, run_claim_turn_args, run_init_campaign_args, run_open_turn_args,
    run_play_until_args, run_scan_turn_args,
};
use crate::commands::harness_helm_lab::run_seed_nc_helm_lab_args;
use crate::commands::harness_player1_tui_stress::run_seed_player1_tui_stress_args;
use crate::support::paths::resolve_repo_path;

pub(crate) fn run_harness_args(
    mut args: impl Iterator<Item = String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(cmd) = args.next() else {
        return Err("harness requires a subcommand".into());
    };

    match cmd.as_str() {
        "check-scenario" => {
            let file = parse_file_only(args.collect::<Vec<_>>())?;
            let spec = ScenarioSpec::load_kdl(&file)?;
            let built = build_scenario(&spec)?;
            print_scenario_report("Validated", &built);
        }
        "run-scenario" => {
            let (file, dir, export_classic) = parse_file_dir_args(args.collect::<Vec<_>>())?;
            let spec = ScenarioSpec::load_kdl(&file)?;
            let built = build_scenario(&spec)?;
            let saved = save_built_scenario(&built, &dir, export_classic)?;
            print_scenario_report("Built", &built);
            println!("Saved runtime scenario at {}.", dir.display());
            println!("  export_classic={}", saved.export_classic);
        }
        "check-combat" => {
            let file = parse_file_only(args.collect::<Vec<_>>())?;
            let spec = CombatScenarioSpec::load_kdl(&file)?;
            let built = build_scenario(&spec.scenario)?;
            print_scenario_report("Validated combat setup", &built);
            println!("  maintenance_turns={}", spec.maintenance_turns);
        }
        "run-combat" => {
            let (file, dir, export_classic) =
                parse_file_optional_dir_args(args.collect::<Vec<_>>())?;
            let spec = CombatScenarioSpec::load_kdl(&file)?;
            let run = run_combat_scenario(&spec)?;
            print_combat_report(&run.report);
            if let Some(dir) = dir {
                let saved = save_built_scenario(&run.built, &dir, export_classic)?;
                println!("Saved combat result at {}.", dir.display());
                println!("  export_classic={}", saved.export_classic);
            }
        }
        "run-sweep" => {
            let file = parse_file_only(args.collect::<Vec<_>>())?;
            let spec = CombatSweepSpec::load_kdl(&file)?;
            let report = run_combat_sweep(&spec)?;
            print_sweep_report(&report);
        }
        "list-report-families" => {
            print_report_preview_families();
        }
        "preview-reports" => {
            let query = parse_report_preview_args(args.collect::<Vec<_>>())?;
            let run = run_report_preview(&query)?;
            print_report_preview(&run);
        }
        "init-campaign" => {
            run_init_campaign_args(args.collect::<Vec<_>>())?;
        }
        "open-turn" => {
            run_open_turn_args(args.collect::<Vec<_>>())?;
        }
        "claim-turn" => {
            run_claim_turn_args(args.collect::<Vec<_>>())?;
        }
        "scan-turn" => {
            run_scan_turn_args(args.collect::<Vec<_>>())?;
        }
        "apply-turn-batch" => {
            run_apply_turn_batch_args(args.collect::<Vec<_>>())?;
        }
        "play-until" => {
            run_play_until_args(args.collect::<Vec<_>>())?;
        }
        "seed-player1-tui-stress" => {
            run_seed_player1_tui_stress_args(args.collect::<Vec<_>>())?;
        }
        "seed-nc-helm-lab" => {
            run_seed_nc_helm_lab_args(args.collect::<Vec<_>>())?;
        }
        other => return Err(format!("unknown harness subcommand: {other}").into()),
    }

    Ok(())
}

fn parse_file_only(args: Vec<String>) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut file = None;
    let mut remaining = args.into_iter();
    while let Some(arg) = remaining.next() {
        match arg.as_str() {
            "--file" => {
                let Some(value) = remaining.next() else {
                    return Err("missing path after --file".into());
                };
                file = Some(resolve_repo_path(&value));
            }
            other => return Err(format!("unknown harness argument: {other}").into()),
        }
    }
    file.ok_or_else(|| "harness command requires --file <path>".into())
}

fn parse_file_dir_args(
    args: Vec<String>,
) -> Result<(PathBuf, PathBuf, bool), Box<dyn std::error::Error>> {
    let (file, dir, export_classic) = parse_file_optional_dir_args(args)?;
    let Some(dir) = dir else {
        return Err("harness run-scenario requires --dir <target_dir>".into());
    };
    Ok((file, dir, export_classic))
}

fn parse_file_optional_dir_args(
    args: Vec<String>,
) -> Result<(PathBuf, Option<PathBuf>, bool), Box<dyn std::error::Error>> {
    let mut file = None;
    let mut dir = None;
    let mut export_classic = false;
    let mut remaining = args.into_iter();
    while let Some(arg) = remaining.next() {
        match arg.as_str() {
            "--file" => {
                let Some(value) = remaining.next() else {
                    return Err("missing path after --file".into());
                };
                file = Some(resolve_repo_path(&value));
            }
            "--dir" => {
                let Some(value) = remaining.next() else {
                    return Err("missing path after --dir".into());
                };
                dir = Some(resolve_repo_path(&value));
            }
            "--export-classic" => export_classic = true,
            other => return Err(format!("unknown harness argument: {other}").into()),
        }
    }
    let Some(file) = file else {
        return Err("harness command requires --file <path>".into());
    };
    Ok((file, dir, export_classic))
}

fn parse_report_preview_args(
    args: Vec<String>,
) -> Result<nc_harness::ReportPreviewQuery, Box<dyn std::error::Error>> {
    let mut family = None;
    let mut seed = 1515u64;
    let mut samples = 3usize;
    let mut remaining = args.into_iter();
    while let Some(arg) = remaining.next() {
        match arg.as_str() {
            "--family" => {
                let Some(value) = remaining.next() else {
                    return Err("missing family after --family".into());
                };
                family = Some(value);
            }
            "--seed" => {
                let Some(value) = remaining.next() else {
                    return Err("missing seed after --seed".into());
                };
                seed = value
                    .parse::<u64>()
                    .map_err(|_| format!("invalid --seed value: {value}"))?;
            }
            "--samples" => {
                let Some(value) = remaining.next() else {
                    return Err("missing sample count after --samples".into());
                };
                samples = value
                    .parse::<usize>()
                    .map_err(|_| format!("invalid --samples value: {value}"))?;
            }
            other => return Err(format!("unknown harness argument: {other}").into()),
        }
    }
    let Some(family) = family else {
        return Err("harness preview-reports requires --family <name|all>".into());
    };
    Ok(nc_harness::ReportPreviewQuery {
        family,
        seed,
        samples,
    })
}

fn print_scenario_report(verb: &str, built: &BuiltScenario) {
    print_scenario_build_report(verb, &built.report);
    println!("  queued_mail={}", built.queued_mail.len());
    println!("  report_blocks={}", built.report_block_rows.len());
}

fn print_scenario_build_report(verb: &str, report: &ScenarioBuildReport) {
    println!("{verb} scenario.");
    if let Some(label) = &report.label {
        println!("  label={label}");
    }
    println!("  players={}", report.player_count);
    println!("  year={}", report.year);
    println!("  planet_records={}", report.planet_records);
    println!("  fleet_records={}", report.fleet_records);
    println!("  queued_mail={}", report.queue_mail_count);
    println!("  results_blocks={}", report.results_blocks);
    println!("  message_blocks={}", report.message_blocks);
}

fn print_combat_report(report: &nc_harness::CombatRunReport) {
    print_scenario_build_report("Executed combat scenario", &report.scenario);
    println!("  maintenance_turns={}", report.maintenance_turns);
    println!("  final_year={}", report.final_year);
    println!("  fleet_battle_events={}", report.fleet_battle_events);
    println!("  bombard_events={}", report.bombard_events);
    println!("  assault_report_events={}", report.assault_report_events);
    println!("  ownership_changes={}", report.ownership_changes);
    println!("  elapsed_millis={}", report.elapsed_millis);
    for empire in &report.empires {
        println!(
            "  empire={} planets:{}->{} fleets:{}->{} ships:{}->{}",
            empire.empire,
            empire.planets_before,
            empire.planets_after,
            empire.fleets_before,
            empire.fleets_after,
            empire.ships_before,
            empire.ships_after
        );
    }
}

fn print_sweep_report(report: &nc_harness::CombatSweepReport) {
    println!("Executed combat sweep.");
    println!("  scenario={}", report.scenario_path.display());
    println!("  seed={}", report.seed);
    println!("  executed_cases={}", report.executed_cases);
    println!("  total_possible_cases={}", report.total_possible_cases);
    println!("  requested_max_cases={}", report.requested_max_cases);
    println!("  mean_millis={}", report.mean_millis);
    println!("  median_millis={}", report.median_millis);
    println!("  p95_millis={}", report.p95_millis);
    for case in &report.cases {
        println!(
            "  case={} label=\"{}\" elapsed={}ms battles={} bombard={} assaults={} ownership_changes={}",
            case.case_index,
            case.label,
            case.elapsed_millis,
            case.fleet_battle_events,
            case.bombard_events,
            case.assault_report_events,
            case.ownership_changes
        );
    }
}

fn print_report_preview_families() {
    println!("Available report preview families.");
    for family in list_report_preview_families() {
        let status = match family.status {
            nc_harness::PreviewFamilyStatus::Implemented => "implemented",
            nc_harness::PreviewFamilyStatus::Stub => "stub",
        };
        println!(
            "  {} [{}] {} - {}",
            family.key, status, family.category, family.description
        );
    }
}

fn print_report_preview(run: &nc_harness::ReportPreviewRun) {
    println!("Previewed report families.");
    println!("  requested_family={}", run.query.family);
    println!("  seed={}", run.query.seed);
    println!("  samples={}", run.query.samples);

    if let Some(family) = run.requested_stub_family {
        println!(
            "  family={} [{}] not implemented yet - {}",
            family.key, family.category, family.description
        );
        return;
    }

    for family_run in &run.family_runs {
        println!(
            "family={} [{}] {}",
            family_run.family.key, family_run.family.category, family_run.family.description
        );
        for case in &family_run.cases {
            println!(
                "  case={} variant={}",
                case.sample_index + 1,
                case.variant_label
            );
            for line in &case.asset_summary {
                println!("    asset: {line}");
            }
            for line in &case.event_summary {
                println!("    event: {line}");
            }
            for viewer in &case.viewer_reports {
                println!("    {} (empire={}):", viewer.role, viewer.empire);
                if viewer.reports.is_empty() {
                    println!("      <no visible report>");
                    continue;
                }
                for (idx, report) in viewer.reports.iter().enumerate() {
                    println!("      report {}:", idx + 1);
                    for line in report.lines() {
                        println!("        {line}");
                    }
                }
            }
        }
    }

    if !run.skipped_stub_families.is_empty() {
        let skipped = run
            .skipped_stub_families
            .iter()
            .map(|family| family.key)
            .collect::<Vec<_>>()
            .join(", ");
        println!("stubbed families skipped={skipped}");
    }
}

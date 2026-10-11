use std::path::PathBuf;

use nc_data::ProductionItemKind;

use crate::commands::compare::{
    compare_all_preserved_scenarios, compare_dirs, compare_preserved_scenario,
};
use crate::commands::compliance::{print_compliance_batch_report, print_compliance_report};
use crate::commands::core::{
    init_canonical_current_known_baseline, init_current_known_baseline,
    print_canonical_current_known_baseline_diff,
    print_canonical_current_known_baseline_diff_offsets, print_canonical_transition_clusters,
    print_canonical_transition_details, print_core_report, print_current_known_baseline_diff,
    print_current_known_baseline_diff_offsets, set_player_tax_rate,
    sync_canonical_current_known_baseline, sync_core_baseline, sync_core_counts,
    sync_current_known_baseline, sync_initialized_fleet_baseline, sync_initialized_planet_payloads,
    validate_core_state, validate_current_known_baseline_exact,
};
use crate::commands::fleet_order::{
    init_fleet_order_batch, init_fleet_order_scenario, print_fleet_order_report, set_fleet_order,
};
use crate::commands::fleet_setup::{
    detach_fleet_to_new_record, set_fleet_location, set_fleet_ships,
};
use crate::commands::guard_starbase::{
    init_guard_starbase_batch, init_guard_starbase_onebase, print_guard_starbase_report,
    set_guard_starbase_onebase,
};
use crate::commands::harness::run_harness_args;
use crate::commands::inspect::{
    dump_headers, inspect_classic_login, inspect_dir, inspect_fleet_movement, inspect_messages,
};
use crate::commands::ipbm::{
    init_ipbm_batch, init_ipbm_zero_records, print_ipbm_report, set_ipbm_record_prefix,
    set_ipbm_zero_records, validate_ipbm,
};
use crate::commands::planet_build::{
    init_planet_build_batch, init_planet_build_scenario, init_planet_original, order_planet_build,
    print_planet_build_report, set_planet_name, set_planet_owner, set_planet_potential,
    set_planet_present, set_planet_stardock_slot, set_planet_stats, set_planet_stored,
};
use crate::commands::player_setup::{join_player, prepare_classic_login, set_player_name};

use crate::commands::bombard::{init_bombard, init_bombard_batch, set_bombard_onefleet};
use crate::commands::econ::{init_econ, init_econ_batch, set_econ};
use crate::commands::economy::{
    init_starbase_tax_probe, init_tax_growth_probe, print_economy_report,
};
use crate::commands::fleet_battle::{init_fleet_battle, init_fleet_battle_batch, set_fleet_battle};
use crate::commands::invade::{init_invade, init_invade_batch, set_invade_onefleet};
use crate::commands::maint::{compare_maintenance, run_rust_maintenance_from_args};
use crate::commands::map_export::export_player_starmap;
use crate::commands::scenario::{
    KnownScenario, apply_known_scenario, apply_known_scenarios, init_all_known_scenarios,
    init_known_replayable_scenario, init_known_scenario, init_known_scenario_chain,
    print_known_scenario_details, print_known_scenarios, validate_all_known_scenarios,
    validate_all_preserved_scenarios, validate_known_scenario, validate_preserved_scenario,
};
use crate::commands::storage::{export_latest_db_snapshot, import_directory_to_db};
use crate::commands::submit_turn::run_submit_turn_args;
use crate::commands::sysop::run_sysop_args;
use crate::support::parse::{
    parse_optional_source_and_target, parse_optional_source_target_and_bombard_spec,
    parse_optional_source_target_and_coord_list, parse_optional_source_target_and_count,
    parse_optional_source_target_and_count_list, parse_optional_source_target_and_econ_spec,
    parse_optional_source_target_and_invade_spec, parse_optional_source_target_and_name,
    parse_optional_source_target_and_xy, parse_target_and_bombard_spec_list,
    parse_target_and_econ_spec_list, parse_target_and_fleet_battle_spec_list,
    parse_target_and_fleet_spec, parse_target_and_fleet_spec_list,
    parse_target_and_invade_spec_list, parse_target_and_planet_spec,
    parse_target_and_planet_spec_list, parse_u8_arg, parse_u16_arg, parse_usize_1_based,
};
use crate::support::paths::{default_fixture_dir, post_maint_fixture_dir, resolve_repo_path};
use crate::usage::print_usage;
use crate::workspace::{initialize_dir, match_fixture_set};

fn next_dir(args: &mut impl Iterator<Item = String>) -> PathBuf {
    args.next()
        .map(|arg| resolve_repo_path(&arg))
        .unwrap_or_else(default_fixture_dir)
}

pub fn run_args(mut args: impl Iterator<Item = String>) -> Result<(), Box<dyn std::error::Error>> {
    let Some(cmd) = args.next() else {
        print_usage();
        return Ok(());
    };

    match cmd.as_str() {
        "sysop" => {
            eprintln!("warning: `nc-cli sysop ...` is deprecated; use `nc-sysop ...`");
            run_sysop_args("nc-cli sysop", args)?
        }
        "inspect" => inspect_dir(&next_dir(&mut args))?,
        "inspect-fleet-movement" => {
            let Some(dir) = args.next().map(|arg| resolve_repo_path(&arg)) else {
                print_usage();
                return Ok(());
            };
            let Some(fleet_record_index_1_based) = args.next().and_then(|arg| arg.parse().ok())
            else {
                print_usage();
                return Ok(());
            };
            let prefer_live_directory = match args.next().as_deref() {
                None => false,
                Some("--live-dir") => true,
                Some(_) => {
                    print_usage();
                    return Ok(());
                }
            };
            if args.next().is_some() {
                print_usage();
                return Ok(());
            }
            inspect_fleet_movement(&dir, fleet_record_index_1_based, prefer_live_directory)?;
        }
        "inspect-messages" => inspect_messages(&next_dir(&mut args))?,
        "inspect-classic-login" => {
            let dir = next_dir(&mut args);
            let Some(caller_alias) = args.next() else {
                print_usage();
                return Ok(());
            };
            inspect_classic_login(&dir, &caller_alias)?;
        }
        "core-report" => print_core_report(&next_dir(&mut args))?,
        "core-diff-current-known-baseline" => {
            print_current_known_baseline_diff(&next_dir(&mut args))?
        }
        "core-diff-current-known-baseline-offsets" => {
            print_current_known_baseline_diff_offsets(&next_dir(&mut args))?
        }
        "core-diff-canonical-current-known-baseline" => {
            print_canonical_current_known_baseline_diff(&next_dir(&mut args))?
        }
        "core-diff-canonical-current-known-baseline-offsets" => {
            print_canonical_current_known_baseline_diff_offsets(&next_dir(&mut args))?
        }
        "core-report-canonical-transition-clusters" => {
            print_canonical_transition_clusters(&next_dir(&mut args))?
        }
        "core-report-canonical-transition-details" => {
            print_canonical_transition_details(&next_dir(&mut args))?
        }
        "core-validate" => validate_core_state(&next_dir(&mut args))?,
        "core-validate-current-known-baseline" => {
            validate_current_known_baseline_exact(&next_dir(&mut args))?
        }
        "core-sync-counts" => sync_core_counts(&next_dir(&mut args))?,
        "core-sync-baseline" => sync_core_baseline(&next_dir(&mut args))?,
        "core-sync-current-known-baseline" => sync_current_known_baseline(&next_dir(&mut args))?,
        "core-sync-canonical-current-known-baseline" => {
            sync_canonical_current_known_baseline(&next_dir(&mut args))?
        }
        "core-sync-initialized-fleets" => sync_initialized_fleet_baseline(&next_dir(&mut args))?,
        "core-sync-initialized-planets" => sync_initialized_planet_payloads(&next_dir(&mut args))?,
        "core-init-current-known-baseline" => {
            let remaining = args.collect::<Vec<_>>();
            let Some((source, target)) =
                parse_optional_source_and_target(remaining, post_maint_fixture_dir())
            else {
                print_usage();
                return Ok(());
            };
            init_current_known_baseline(&source, &target)?;
        }
        "core-init-canonical-current-known-baseline" => {
            let remaining = args.collect::<Vec<_>>();
            let Some((source, target)) =
                parse_optional_source_and_target(remaining, post_maint_fixture_dir())
            else {
                print_usage();
                return Ok(());
            };
            init_canonical_current_known_baseline(&source, &target)?;
        }
        "headers" => dump_headers(&next_dir(&mut args))?,
        "match" => match_fixture_set(&next_dir(&mut args))?,
        "compare" => {
            let Some(left) = args.next().map(|arg| resolve_repo_path(&arg)) else {
                print_usage();
                return Ok(());
            };
            let Some(right) = args.next().map(|arg| resolve_repo_path(&arg)) else {
                print_usage();
                return Ok(());
            };
            compare_dirs(&left, &right)?;
        }
        "init" => {
            let remaining = args.collect::<Vec<_>>();
            let Some((source, target)) =
                parse_optional_source_and_target(remaining, default_fixture_dir())
            else {
                print_usage();
                return Ok(());
            };
            initialize_dir(&source, &target)?;
        }
        "generate-gamestate" => {
            eprintln!(
                "warning: `nc-cli generate-gamestate ...` is a developer helper; `nc-sysop` uses `new-game` for normal campaign creation"
            );
            run_sysop_args(
                "nc-cli sysop",
                std::iter::once("generate-gamestate".to_string()).chain(args),
            )?
        }
        "maint-rust" => {
            eprintln!("warning: `nc-cli maint-rust ...` is deprecated; use `nc-sysop maint ...`");
            run_rust_maintenance_from_args("nc-cli maint-rust", args)?;
        }
        "maint-compare" => {
            let dir = next_dir(&mut args);
            let turns: Option<u16> = args.next().and_then(|s| s.parse().ok());
            compare_maintenance(&dir, turns)?;
        }
        "map-export" => {
            let dir = next_dir(&mut args);
            let Some(player) = args.next().and_then(|s| s.parse::<usize>().ok()) else {
                print_usage();
                return Ok(());
            };
            let Some(output) = args.next().map(PathBuf::from) else {
                print_usage();
                return Ok(());
            };
            export_player_starmap(&dir, player, &output)?;
        }
        "db-import" => {
            import_directory_to_db(&next_dir(&mut args))?;
        }
        "db-export" => {
            let source_dir = next_dir(&mut args);
            let Some(target_dir) = args.next().map(PathBuf::from) else {
                print_usage();
                return Ok(());
            };
            export_latest_db_snapshot(&source_dir, &target_dir)?;
        }
        "submit-turn" => {
            eprintln!(
                "warning: `nc-cli submit-turn ...` is deprecated; use `nc-game submit-turn ...`"
            );
            run_submit_turn_args(args.collect::<Vec<_>>())?;
        }
        "harness" => {
            run_harness_args(args)?;
        }
        "fleet-order" => {
            let dir = next_dir(&mut args);
            let Some(record_index) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(speed) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(order_code) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(target_x) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(target_y) = args.next() else {
                print_usage();
                return Ok(());
            };
            let aux0 = args.next();
            let aux1 = args.next();
            set_fleet_order(
                &dir,
                parse_usize_1_based(&record_index, "fleet record index")?,
                parse_u8_arg(&speed, "speed")?,
                parse_u8_arg(&order_code, "order code")?,
                parse_u8_arg(&target_x, "target_x")?,
                parse_u8_arg(&target_y, "target_y")?,
                aux0.as_deref()
                    .map(|value| parse_u8_arg(value, "aux0"))
                    .transpose()?,
                aux1.as_deref()
                    .map(|value| parse_u8_arg(value, "aux1"))
                    .transpose()?,
            )?;
        }
        "fleet-order-report" => {
            let dir = next_dir(&mut args);
            let record_index_arg = args.next();
            let record_index = record_index_arg.as_deref().unwrap_or("1");
            print_fleet_order_report(
                &dir,
                parse_usize_1_based(record_index, "fleet record index")?,
            )?;
        }
        "fleet-ships" => {
            let dir = next_dir(&mut args);
            let Some(record_index) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(scouts) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(battleships) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(cruisers) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(destroyers) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(transports) = args.next() else {
                print_usage();
                return Ok(());
            };
            let armies_loaded = args.next();
            let etacs = args.next();
            set_fleet_ships(
                &dir,
                parse_usize_1_based(&record_index, "fleet record index")?,
                parse_u16_arg(&scouts, "scouts")?,
                parse_u16_arg(&battleships, "battleships")?,
                parse_u16_arg(&cruisers, "cruisers")?,
                parse_u16_arg(&destroyers, "destroyers")?,
                parse_u16_arg(&transports, "transports")?,
                armies_loaded
                    .as_deref()
                    .map(|value| parse_u16_arg(value, "loaded armies"))
                    .transpose()?
                    .unwrap_or(0),
                etacs
                    .as_deref()
                    .map(|value| parse_u16_arg(value, "etacs"))
                    .transpose()?
                    .unwrap_or(0),
            )?;
        }
        "fleet-location" => {
            let dir = next_dir(&mut args);
            let Some(record_index) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(x) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(y) = args.next() else {
                print_usage();
                return Ok(());
            };
            set_fleet_location(
                &dir,
                parse_usize_1_based(&record_index, "fleet record index")?,
                parse_u8_arg(&x, "x")?,
                parse_u8_arg(&y, "y")?,
            )?;
        }
        "fleet-detach" => {
            let dir = next_dir(&mut args);
            let Some(player_index) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(donor_index) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(battleships) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(cruisers) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(destroyers) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(full_transports) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(empty_transports) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(scouts) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(etacs) = args.next() else {
                print_usage();
                return Ok(());
            };
            let donor_speed = args.next();
            let new_fleet_roe = args.next();
            detach_fleet_to_new_record(
                &dir,
                parse_usize_1_based(&player_index, "player record index")?,
                parse_usize_1_based(&donor_index, "donor fleet record index")?,
                parse_u16_arg(&battleships, "battleships")?,
                parse_u16_arg(&cruisers, "cruisers")?,
                parse_u16_arg(&destroyers, "destroyers")?,
                parse_u16_arg(&full_transports, "full transports")?,
                parse_u16_arg(&empty_transports, "empty transports")?,
                parse_u16_arg(&scouts, "scouts")?,
                parse_u16_arg(&etacs, "etacs")?,
                donor_speed
                    .as_deref()
                    .map(|value| parse_u8_arg(value, "donor speed"))
                    .transpose()?,
                new_fleet_roe
                    .as_deref()
                    .map(|value| parse_u8_arg(value, "new fleet roe"))
                    .transpose()?
                    .unwrap_or(nc_data::NEW_FLEET_DEFAULT_ROE),
            )?;
        }
        "fleet-order-init" => {
            let remaining = args.collect::<Vec<_>>();
            let Some((target, record_index, speed, order_code, target_x, target_y, aux0, aux1)) =
                parse_target_and_fleet_spec(remaining)
            else {
                print_usage();
                return Ok(());
            };
            init_fleet_order_scenario(
                &post_maint_fixture_dir(),
                &target,
                record_index,
                speed,
                order_code,
                target_x,
                target_y,
                aux0,
                aux1,
            )?;
        }
        "fleet-order-batch-init" => {
            let remaining = args.collect::<Vec<_>>();
            let Some((target_root, specs)) = parse_target_and_fleet_spec_list(remaining) else {
                print_usage();
                return Ok(());
            };
            init_fleet_order_batch(&post_maint_fixture_dir(), &target_root, &specs)?;
        }
        "planet-build" => {
            let dir = next_dir(&mut args);
            let Some(record_index) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(quantity) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(kind) = args.next() else {
                print_usage();
                return Ok(());
            };
            order_planet_build(
                &dir,
                parse_usize_1_based(&record_index, "planet record index")?,
                quantity.parse()?,
                ProductionItemKind::from_name(&kind)
                    .ok_or_else(|| format!("unknown build kind: {kind}"))?,
            )?;
        }
        "planet-owner" => {
            let dir = next_dir(&mut args);
            let Some(record_index) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(owner) = args.next() else {
                print_usage();
                return Ok(());
            };
            set_planet_owner(&dir, record_index.parse()?, owner.parse()?)?;
        }
        "planet-name" => {
            let dir = next_dir(&mut args);
            let Some(record_index) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(name) = args.next() else {
                print_usage();
                return Ok(());
            };
            set_planet_name(&dir, record_index.parse()?, &name)?;
        }
        "planet-stats" => {
            let dir = next_dir(&mut args);
            let Some(record_index) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(armies) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(batteries) = args.next() else {
                print_usage();
                return Ok(());
            };
            set_planet_stats(
                &dir,
                record_index.parse()?,
                armies.parse()?,
                batteries.parse()?,
            )?;
        }
        "planet-potential" => {
            let dir = next_dir(&mut args);
            let Some(record_index) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(p1) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(p2) = args.next() else {
                print_usage();
                return Ok(());
            };
            set_planet_potential(&dir, record_index.parse()?, p1.parse()?, p2.parse()?)?;
        }
        "planet-present" => {
            let dir = next_dir(&mut args);
            let Some(record_index) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(points) = args.next() else {
                print_usage();
                return Ok(());
            };
            set_planet_present(&dir, record_index.parse()?, points.parse()?)?;
        }
        "player-name" => {
            let dir = next_dir(&mut args);
            let Some(player_index) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(handle) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(empire_name) = args.next() else {
                print_usage();
                return Ok(());
            };
            set_player_name(
                &dir,
                parse_usize_1_based(&player_index, "player record index")?,
                &handle,
                &empire_name,
            )?;
        }
        "player-join" => {
            let dir = next_dir(&mut args);
            let Some(player_index) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(caller_alias) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(empire_name) = args.next() else {
                print_usage();
                return Ok(());
            };
            let homeworld_name = args.next();
            join_player(
                &dir,
                parse_usize_1_based(&player_index, "player record index")?,
                &caller_alias,
                &empire_name,
                homeworld_name.as_deref(),
            )?;
        }
        "classic-login-prepare" => {
            let dir = next_dir(&mut args);
            let Some(player_index) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(caller_alias) = args.next() else {
                print_usage();
                return Ok(());
            };
            let empire_name = args.next();
            prepare_classic_login(
                &dir,
                parse_usize_1_based(&player_index, "player record index")?,
                &caller_alias,
                empire_name.as_deref(),
            )?;
        }
        "planet-stored" => {
            let dir = next_dir(&mut args);
            let Some(record_index) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(points) = args.next() else {
                print_usage();
                return Ok(());
            };
            set_planet_stored(&dir, record_index.parse()?, points.parse()?)?;
        }
        "planet-stardock" => {
            let dir = next_dir(&mut args);
            let Some(record_index) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(slot) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(kind) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(count) = args.next() else {
                print_usage();
                return Ok(());
            };
            set_planet_stardock_slot(
                &dir,
                record_index.parse()?,
                slot.parse()?,
                kind.parse()?,
                count.parse()?,
            )?;
        }
        "planet-init-original" => init_planet_original(&next_dir(&mut args))?,
        "planet-build-report" => {
            let dir = next_dir(&mut args);
            let record_index_arg = args.next();
            let record_index = record_index_arg.as_deref().unwrap_or("15");
            print_planet_build_report(
                &dir,
                parse_usize_1_based(record_index, "planet record index")?,
            )?;
        }
        "planet-build-init" => {
            let remaining = args.collect::<Vec<_>>();
            let Some((target, record_index, quantity, kind)) =
                parse_target_and_planet_spec(remaining)
            else {
                print_usage();
                return Ok(());
            };
            init_planet_build_scenario(
                &post_maint_fixture_dir(),
                &target,
                record_index,
                quantity,
                kind,
            )?;
        }
        "planet-build-batch-init" => {
            let remaining = args.collect::<Vec<_>>();
            let Some((target_root, specs)) = parse_target_and_planet_spec_list(remaining) else {
                print_usage();
                return Ok(());
            };
            init_planet_build_batch(&post_maint_fixture_dir(), &target_root, &specs)?;
        }
        "guard-starbase-onebase" => {
            let dir = next_dir(&mut args);
            let Some(target_x) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(target_y) = args.next() else {
                print_usage();
                return Ok(());
            };
            set_guard_starbase_onebase(
                &dir,
                parse_u8_arg(&target_x, "target_x")?,
                parse_u8_arg(&target_y, "target_y")?,
            )?;
        }
        "guard-starbase-report" => print_guard_starbase_report(&next_dir(&mut args))?,
        "guard-starbase-init" => {
            let remaining = args.collect::<Vec<_>>();
            let Some((source, target, target_x, target_y)) =
                parse_optional_source_target_and_xy(remaining, post_maint_fixture_dir())
            else {
                print_usage();
                return Ok(());
            };
            init_guard_starbase_onebase(&source, &target, target_x, target_y)?;
        }
        "ipbm-report" => print_ipbm_report(&next_dir(&mut args))?,
        "player-tax" => {
            let dir = next_dir(&mut args);
            let Some(record_index) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(rate) = args.next() else {
                print_usage();
                return Ok(());
            };
            set_player_tax_rate(&dir, record_index.parse()?, rate.parse()?)?;
        }
        "ipbm-zero" => {
            let dir = next_dir(&mut args);
            let Some(count) = args.next() else {
                print_usage();
                return Ok(());
            };
            set_ipbm_zero_records(&dir, count.parse::<u16>()?)?;
        }
        "ipbm-record-set" => {
            let dir = next_dir(&mut args);
            let Some(record_index) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(primary) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(owner) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(gate) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(follow_on) = args.next() else {
                print_usage();
                return Ok(());
            };
            set_ipbm_record_prefix(
                &dir,
                parse_usize_1_based(&record_index, "ipbm record index")?,
                parse_u16_arg(&primary, "primary")?,
                parse_u8_arg(&owner, "owner")?,
                parse_u16_arg(&gate, "gate")?,
                parse_u16_arg(&follow_on, "follow_on")?,
            )?;
        }
        "ipbm-validate" => validate_ipbm(next_dir(&mut args).as_path())?,
        "ipbm-init" => {
            let remaining = args.collect::<Vec<_>>();
            let Some((source, target, count)) =
                parse_optional_source_target_and_count(remaining, post_maint_fixture_dir())
            else {
                print_usage();
                return Ok(());
            };
            init_ipbm_zero_records(&source, &target, count)?;
        }
        "compliance-report" => print_compliance_report(&next_dir(&mut args))?,
        "guard-starbase-batch-init" => {
            let remaining = args.collect::<Vec<_>>();
            let Some((source, target_root, coords)) =
                parse_optional_source_target_and_coord_list(remaining, post_maint_fixture_dir())
            else {
                print_usage();
                return Ok(());
            };
            init_guard_starbase_batch(&source, &target_root, &coords)?;
        }
        "ipbm-batch-init" => {
            let remaining = args.collect::<Vec<_>>();
            let Some((source, target_root, counts)) =
                parse_optional_source_target_and_count_list(remaining, post_maint_fixture_dir())
            else {
                print_usage();
                return Ok(());
            };
            init_ipbm_batch(&source, &target_root, &counts)?;
        }
        "bombard-onefleet" => {
            let dir = next_dir(&mut args);
            let Some(target_x) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(target_y) = args.next() else {
                print_usage();
                return Ok(());
            };
            let ca = args
                .next()
                .as_deref()
                .map(|v| parse_u16_arg(v, "ca"))
                .transpose()?
                .unwrap_or(3);
            let dd = args
                .next()
                .as_deref()
                .map(|v| parse_u16_arg(v, "dd"))
                .transpose()?
                .unwrap_or(5);
            set_bombard_onefleet(
                &dir,
                parse_u8_arg(&target_x, "target_x")?,
                parse_u8_arg(&target_y, "target_y")?,
                ca,
                dd,
            )?;
        }
        "bombard-init" => {
            let remaining = args.collect::<Vec<_>>();
            let Some((source, target, target_x, target_y, ca, dd)) =
                parse_optional_source_target_and_bombard_spec(remaining, post_maint_fixture_dir())
            else {
                print_usage();
                return Ok(());
            };
            init_bombard(&source, &target, target_x, target_y, ca, dd)?;
        }
        "bombard-batch-init" => {
            let remaining = args.collect::<Vec<_>>();
            let Some((source, target_root, specs)) =
                parse_target_and_bombard_spec_list(remaining, post_maint_fixture_dir())
            else {
                print_usage();
                return Ok(());
            };
            init_bombard_batch(&source, &target_root, &specs)?;
        }
        "invade-onefleet" => {
            let dir = next_dir(&mut args);
            let Some(target_x) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(target_y) = args.next() else {
                print_usage();
                return Ok(());
            };
            let sc = args
                .next()
                .as_deref()
                .map(|v| parse_u8_arg(v, "sc"))
                .transpose()?
                .unwrap_or(0);
            let bb = args
                .next()
                .as_deref()
                .map(|v| parse_u16_arg(v, "bb"))
                .transpose()?
                .unwrap_or(100);
            let ca = args
                .next()
                .as_deref()
                .map(|v| parse_u16_arg(v, "ca"))
                .transpose()?
                .unwrap_or(50);
            let dd = args
                .next()
                .as_deref()
                .map(|v| parse_u16_arg(v, "dd"))
                .transpose()?
                .unwrap_or(50);
            let tt = args
                .next()
                .as_deref()
                .map(|v| parse_u16_arg(v, "tt"))
                .transpose()?
                .unwrap_or(50);
            let armies = args
                .next()
                .as_deref()
                .map(|v| parse_u8_arg(v, "armies"))
                .transpose()?
                .unwrap_or(100);
            set_invade_onefleet(
                &dir,
                parse_u8_arg(&target_x, "target_x")?,
                parse_u8_arg(&target_y, "target_y")?,
                sc,
                bb,
                ca,
                dd,
                tt,
                armies,
            )?;
        }
        "invade-init" => {
            let remaining = args.collect::<Vec<_>>();
            let Some((source, target, x, y, sc, bb, ca, dd, tt, armies)) =
                parse_optional_source_target_and_invade_spec(remaining, post_maint_fixture_dir())
            else {
                print_usage();
                return Ok(());
            };
            init_invade(&source, &target, x, y, sc, bb, ca, dd, tt, armies)?;
        }
        "invade-batch-init" => {
            let remaining = args.collect::<Vec<_>>();
            let Some((source, target_root, specs)) =
                parse_target_and_invade_spec_list(remaining, post_maint_fixture_dir())
            else {
                print_usage();
                return Ok(());
            };
            init_invade_batch(&source, &target_root, &specs)?;
        }
        "fleet-battle" => {
            let dir = next_dir(&mut args);
            let Some(battle_x) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(battle_y) = args.next() else {
                print_usage();
                return Ok(());
            };
            let battle_x_val = parse_u8_arg(&battle_x, "battle_x")?;
            let battle_y_val = parse_u8_arg(&battle_y, "battle_y")?;
            let f0_roe = args
                .next()
                .as_deref()
                .map(|v| parse_u8_arg(v, "f0_roe"))
                .transpose()?
                .unwrap_or(100);
            let f0_bb = args
                .next()
                .as_deref()
                .map(|v| parse_u16_arg(v, "f0_bb"))
                .transpose()?
                .unwrap_or(50);
            let f0_ca = args
                .next()
                .as_deref()
                .map(|v| parse_u16_arg(v, "f0_ca"))
                .transpose()?
                .unwrap_or(50);
            let f0_dd = args
                .next()
                .as_deref()
                .map(|v| parse_u16_arg(v, "f0_dd"))
                .transpose()?
                .unwrap_or(50);
            let f2_ca = args
                .next()
                .as_deref()
                .map(|v| parse_u16_arg(v, "f2_ca"))
                .transpose()?
                .unwrap_or(50);
            let f2_dd = args
                .next()
                .as_deref()
                .map(|v| parse_u16_arg(v, "f2_dd"))
                .transpose()?
                .unwrap_or(50);
            let f4_sc = args
                .next()
                .as_deref()
                .map(|v| parse_u8_arg(v, "f4_sc"))
                .transpose()?
                .unwrap_or(0);
            let f4_bb = args
                .next()
                .as_deref()
                .map(|v| parse_u16_arg(v, "f4_bb"))
                .transpose()?
                .unwrap_or(100);
            let f4_ca = args
                .next()
                .as_deref()
                .map(|v| parse_u16_arg(v, "f4_ca"))
                .transpose()?
                .unwrap_or(0);
            let f8_loc_x = args
                .next()
                .as_deref()
                .map(|v| parse_u8_arg(v, "f8_loc_x"))
                .transpose()?
                .unwrap_or(9);
            let f8_loc_y = args
                .next()
                .as_deref()
                .map(|v| parse_u8_arg(v, "f8_loc_y"))
                .transpose()?
                .unwrap_or(battle_y_val);
            let f8_sc = args
                .next()
                .as_deref()
                .map(|v| parse_u8_arg(v, "f8_sc"))
                .transpose()?
                .unwrap_or(0);
            let f8_bb = args
                .next()
                .as_deref()
                .map(|v| parse_u16_arg(v, "f8_bb"))
                .transpose()?
                .unwrap_or(1);
            let f8_ca = args
                .next()
                .as_deref()
                .map(|v| parse_u16_arg(v, "f8_ca"))
                .transpose()?
                .unwrap_or(0);
            let p14_x = args
                .next()
                .as_deref()
                .map(|v| parse_u8_arg(v, "p14_x"))
                .transpose()?
                .unwrap_or(15);
            let p14_y = args
                .next()
                .as_deref()
                .map(|v| parse_u8_arg(v, "p14_y"))
                .transpose()?
                .unwrap_or(13);
            let p14_armies = args
                .next()
                .as_deref()
                .map(|v| parse_u8_arg(v, "p14_armies"))
                .transpose()?
                .unwrap_or(142);
            let p14_batteries = args
                .next()
                .as_deref()
                .map(|v| parse_u8_arg(v, "p14_batteries"))
                .transpose()?
                .unwrap_or(15);
            set_fleet_battle(
                &dir,
                battle_x_val,
                battle_y_val,
                f0_roe,
                f0_bb,
                f0_ca,
                f0_dd,
                f2_ca,
                f2_dd,
                f4_sc,
                f4_bb,
                f4_ca,
                f8_loc_x,
                f8_loc_y,
                f8_sc,
                f8_bb,
                f8_ca,
                p14_x,
                p14_y,
                p14_armies,
                p14_batteries,
            )?;
        }
        "fleet-battle-init" => {
            let remaining = args.collect::<Vec<_>>();
            // Parse: [source] <target> <battle_x> <battle_y> <f0_roe> <f0_bb> <f0_ca> <f0_dd> <f2_ca> <f2_dd> <f4_sc> <f4_bb> <f4_ca> <f8_loc_x> <f8_loc_y> <f8_sc> <f8_bb> <f8_ca> <p14_x> <p14_y> <p14_armies> <p14_batteries>
            // Minimum: target battle_x battle_y (5 args)
            if remaining.len() < 3 {
                print_usage();
                return Ok(());
            }
            let (source, target, args_slice) = if remaining.len() >= 22 {
                // Full form with source
                (
                    resolve_repo_path(&remaining[0]),
                    PathBuf::from(&remaining[1]),
                    &remaining[2..],
                )
            } else {
                // Short form with default source
                (
                    post_maint_fixture_dir(),
                    PathBuf::from(&remaining[0]),
                    &remaining[1..],
                )
            };
            let battle_x = parse_u8_arg(&args_slice[0], "battle_x")?;
            let battle_y = parse_u8_arg(&args_slice[1], "battle_y")?;
            let f0_roe = parse_u8_arg(&args_slice[2], "f0_roe").unwrap_or(100);
            let f0_bb = parse_u16_arg(&args_slice[3], "f0_bb").unwrap_or(50);
            let f0_ca = parse_u16_arg(&args_slice[4], "f0_ca").unwrap_or(50);
            let f0_dd = parse_u16_arg(&args_slice[5], "f0_dd").unwrap_or(50);
            let f2_ca = parse_u16_arg(&args_slice[6], "f2_ca").unwrap_or(50);
            let f2_dd = parse_u16_arg(&args_slice[7], "f2_dd").unwrap_or(50);
            let f4_sc = parse_u8_arg(&args_slice[8], "f4_sc").unwrap_or(0);
            let f4_bb = parse_u16_arg(&args_slice[9], "f4_bb").unwrap_or(100);
            let f4_ca = parse_u16_arg(&args_slice[10], "f4_ca").unwrap_or(0);
            let f8_loc_x = parse_u8_arg(&args_slice[11], "f8_loc_x").unwrap_or(9);
            let f8_loc_y = parse_u8_arg(&args_slice[12], "f8_loc_y").unwrap_or(battle_y);
            let f8_sc = parse_u8_arg(&args_slice[13], "f8_sc").unwrap_or(0);
            let f8_bb = parse_u16_arg(&args_slice[14], "f8_bb").unwrap_or(1);
            let f8_ca = parse_u16_arg(&args_slice[15], "f8_ca").unwrap_or(0);
            let p14_x = parse_u8_arg(&args_slice[16], "p14_x").unwrap_or(15);
            let p14_y = parse_u8_arg(&args_slice[17], "p14_y").unwrap_or(13);
            let p14_armies = parse_u8_arg(&args_slice[18], "p14_armies").unwrap_or(142);
            let p14_batteries = parse_u8_arg(&args_slice[19], "p14_batteries").unwrap_or(15);
            init_fleet_battle(
                &source,
                &target,
                battle_x,
                battle_y,
                f0_roe,
                f0_bb,
                f0_ca,
                f0_dd,
                f2_ca,
                f2_dd,
                f4_sc,
                f4_bb,
                f4_ca,
                f8_loc_x,
                f8_loc_y,
                f8_sc,
                f8_bb,
                f8_ca,
                p14_x,
                p14_y,
                p14_armies,
                p14_batteries,
            )?;
        }
        "fleet-battle-batch-init" => {
            let remaining = args.collect::<Vec<_>>();
            let Some((source, target_root, specs)) =
                parse_target_and_fleet_battle_spec_list(remaining, post_maint_fixture_dir())
            else {
                print_usage();
                return Ok(());
            };
            init_fleet_battle_batch(&source, &target_root, &specs)?;
        }
        "econ" => {
            let dir = next_dir(&mut args);
            let Some(target_x) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(target_y) = args.next() else {
                print_usage();
                return Ok(());
            };
            let bb = args
                .next()
                .as_deref()
                .map(|v| parse_u16_arg(v, "bb"))
                .transpose()?
                .unwrap_or(0); // BB not set in original econ scenario
            let ca = args
                .next()
                .as_deref()
                .map(|v| parse_u16_arg(v, "ca"))
                .transpose()?
                .unwrap_or(50);
            let dd = args
                .next()
                .as_deref()
                .map(|v| parse_u16_arg(v, "dd"))
                .transpose()?
                .unwrap_or(50);
            let p14_x = args
                .next()
                .as_deref()
                .map(|v| parse_u8_arg(v, "p14_x"))
                .transpose()?
                .unwrap_or(15);
            let p14_y = args
                .next()
                .as_deref()
                .map(|v| parse_u8_arg(v, "p14_y"))
                .transpose()?
                .unwrap_or(13);
            let p14_armies = args
                .next()
                .as_deref()
                .map(|v| parse_u8_arg(v, "p14_armies"))
                .transpose()?
                .unwrap_or(142);
            let p14_batteries = args
                .next()
                .as_deref()
                .map(|v| parse_u8_arg(v, "p14_batteries"))
                .transpose()?
                .unwrap_or(15);
            set_econ(
                &dir,
                parse_u8_arg(&target_x, "target_x")?,
                parse_u8_arg(&target_y, "target_y")?,
                bb,
                ca,
                dd,
                p14_x,
                p14_y,
                p14_armies,
                p14_batteries,
            )?;
        }
        "econ-init" => {
            let remaining = args.collect::<Vec<_>>();
            let Some((source, target, x, y, bb, ca, dd, p14_x, p14_y, p14_armies, p14_batteries)) =
                parse_optional_source_target_and_econ_spec(remaining, post_maint_fixture_dir())
            else {
                print_usage();
                return Ok(());
            };
            init_econ(
                &source,
                &target,
                x,
                y,
                bb,
                ca,
                dd,
                p14_x,
                p14_y,
                p14_armies,
                p14_batteries,
            )?;
        }
        "econ-batch-init" => {
            let remaining = args.collect::<Vec<_>>();
            let Some((source, target_root, specs)) =
                parse_target_and_econ_spec_list(remaining, post_maint_fixture_dir())
            else {
                print_usage();
                return Ok(());
            };
            init_econ_batch(&source, &target_root, &specs)?;
        }
        "economy-report" => {
            let dir = next_dir(&mut args);
            let player_record_index_1_based = args
                .next()
                .map(|value| value.parse::<usize>())
                .transpose()?
                .unwrap_or(1);
            print_economy_report(&dir, player_record_index_1_based)?;
        }
        "economy-tax-probe-init" => {
            let dir = next_dir(&mut args);
            let player_record_index_1_based = args
                .next()
                .map(|value| value.parse::<usize>())
                .transpose()?
                .unwrap_or(1);
            let tax_rate = args
                .next()
                .map(|value| value.parse::<u8>())
                .transpose()?
                .unwrap_or(50);
            init_tax_growth_probe(&dir, player_record_index_1_based, tax_rate)?;
        }
        "economy-starbase-probe-init" => {
            let dir = next_dir(&mut args);
            let player_record_index_1_based = args
                .next()
                .map(|value| value.parse::<usize>())
                .transpose()?
                .unwrap_or(1);
            let tax_rate = args
                .next()
                .map(|value| value.parse::<u8>())
                .transpose()?
                .unwrap_or(50);
            init_starbase_tax_probe(&dir, player_record_index_1_based, tax_rate)?;
        }
        "scenario" => {
            let dir = next_dir(&mut args);
            let selector = args.next();
            if selector.as_deref() == Some("list") {
                print_known_scenarios();
            } else if selector.as_deref() == Some("show") {
                match args.next().as_deref().and_then(KnownScenario::parse) {
                    Some(scenario) => print_known_scenario_details(scenario),
                    None => print_usage(),
                }
            } else if selector.as_deref() == Some("compose") {
                let Some(scenarios) = parse_known_scenarios(args.collect()) else {
                    print_usage();
                    return Ok(());
                };
                apply_known_scenarios(&dir, &scenarios)?;
            } else {
                match selector.as_deref().and_then(KnownScenario::parse) {
                    Some(scenario) => apply_known_scenario(&dir, scenario)?,
                    None => print_usage(),
                }
            }
        }
        "scenario-init-all" => {
            let remaining = args.collect::<Vec<_>>();
            let Some((source, target_root)) =
                parse_optional_source_and_target(remaining, post_maint_fixture_dir())
            else {
                print_usage();
                return Ok(());
            };
            init_all_known_scenarios(&source, &target_root)?;
        }
        "scenario-init" => {
            let remaining = args.collect::<Vec<_>>();
            let Some((source, target, scenario_name)) =
                parse_optional_source_target_and_name(remaining, post_maint_fixture_dir())
            else {
                print_usage();
                return Ok(());
            };
            match KnownScenario::parse(&scenario_name) {
                Some(scenario) => init_known_scenario(&source, &target, scenario)?,
                None => print_usage(),
            }
        }
        "scenario-init-replayable" => {
            let remaining = args.collect::<Vec<_>>();
            let Some((source, target, scenario_name)) =
                parse_optional_source_target_and_name(remaining, post_maint_fixture_dir())
            else {
                print_usage();
                return Ok(());
            };
            match KnownScenario::parse(&scenario_name) {
                Some(scenario) => init_known_replayable_scenario(&source, &target, scenario)?,
                None => print_usage(),
            }
        }
        "scenario-init-compose" => {
            let Some((source, target, scenarios)) = parse_scenario_chain_init_args(args.collect())
            else {
                print_usage();
                return Ok(());
            };
            init_known_scenario_chain(&source, &target, &scenarios)?;
        }
        "validate" => {
            let dir = next_dir(&mut args);
            match args.next().as_deref() {
                Some("all") => validate_all_known_scenarios(&dir)?,
                Some(name) => match KnownScenario::parse(name) {
                    Some(scenario) => validate_known_scenario(&dir, scenario)?,
                    None => print_usage(),
                },
                _ => print_usage(),
            }
        }
        "validate-preserved" => {
            let dir = next_dir(&mut args);
            match args.next().as_deref() {
                Some("all") => validate_all_preserved_scenarios(&dir)?,
                Some(name) => match KnownScenario::parse(name) {
                    Some(scenario) => validate_preserved_scenario(&dir, scenario)?,
                    None => print_usage(),
                },
                _ => print_usage(),
            }
        }
        "compare-preserved" => {
            let dir = next_dir(&mut args);
            match args.next().as_deref() {
                Some("all") => compare_all_preserved_scenarios(&dir)?,
                Some(name) => match KnownScenario::parse(name) {
                    Some(scenario) => compare_preserved_scenario(&dir, scenario)?,
                    None => print_usage(),
                },
                _ => print_usage(),
            }
        }
        "compliance-batch-report" => {
            let root = args
                .next()
                .map(|arg| resolve_repo_path(&arg))
                .unwrap_or_else(post_maint_fixture_dir);
            print_compliance_batch_report(&root)?;
        }
        _ => print_usage(),
    }

    Ok(())
}

fn parse_known_scenarios(args: Vec<String>) -> Option<Vec<KnownScenario>> {
    if args.is_empty() {
        return None;
    }

    args.into_iter()
        .map(|name| KnownScenario::parse(&name))
        .collect()
}

fn parse_scenario_chain_init_args(
    args: Vec<String>,
) -> Option<(PathBuf, PathBuf, Vec<KnownScenario>)> {
    match args.as_slice() {
        [target, scenario_names @ ..] if !scenario_names.is_empty() => Some((
            post_maint_fixture_dir(),
            resolve_repo_path(target),
            parse_known_scenarios(scenario_names.to_vec())?,
        )),
        [source, target, scenario_names @ ..] if !scenario_names.is_empty() => Some((
            resolve_repo_path(source),
            resolve_repo_path(target),
            parse_known_scenarios(scenario_names.to_vec())?,
        )),
        _ => None,
    }
}

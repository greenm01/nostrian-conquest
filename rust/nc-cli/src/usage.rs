pub fn print_usage() {
    println!("Usage:");
    println!("  nc-cli <developer command> ...");
    println!();
    println!("Public binaries:");
    println!("  nc-game --dir <game_dir> --player <1-based empire index>");
    println!(
        "  nc-game submit-turn [--check] --dir <game_dir> --player <record> --file <turn.kdl>"
    );
    println!("  nc-sysop <new-game|maint> ...");
    println!();
    println!("`nc-cli` remains the internal developer/oracle/compatibility tool.");
    println!();
    println!("  nc-cli inspect [dir]");
    println!("  nc-cli inspect-fleet-movement <dir> <fleet_record> [--live-dir]");
    println!("  nc-cli inspect-messages [dir]");
    println!("  nc-cli inspect-classic-login [dir] <caller_alias>");
    println!("  nc-cli map-export [dir] <player_record> <output_txt_path>");
    println!("  nc-cli db-import [dir]");
    println!("  nc-cli db-export [dir] <target_dir>");
    println!("  nc-cli submit-turn [--check] --dir <game_dir> --player <record> --file <turn.kdl>");
    println!("    deprecated alias; use `nc-game submit-turn ...`");
    println!("  nc-cli harness check-scenario --file <scenario.kdl>");
    println!(
        "  nc-cli harness run-scenario --file <scenario.kdl> --dir <target_dir> [--export-classic]"
    );
    println!("  nc-cli harness check-combat --file <combat-scenario.kdl>");
    println!(
        "  nc-cli harness run-combat --file <combat-scenario.kdl> [--dir <target_dir>] [--export-classic]"
    );
    println!("  nc-cli harness run-sweep --file <combat-sweep.kdl>");
    println!("  nc-cli harness list-report-families");
    println!("  nc-cli harness preview-reports --family <name|all> [--seed <u64>] [--samples <n>]");
    println!(
        "  nc-cli harness init-campaign --file <scenario.kdl> --dir <campaign_dir> --game-id <id> [--bundle-profile <human|llm>] [--export-classic]"
    );
    println!("  nc-cli harness open-turn --dir <campaign_dir>");
    println!("  nc-cli harness claim-turn --dir <campaign_dir> --player <record>");
    println!("  nc-cli harness scan-turn --dir <campaign_dir>");
    println!("  nc-cli harness apply-turn-batch --dir <campaign_dir>");
    println!("  nc-cli harness seed-player1-tui-stress --dir <campaign_dir>");
    println!("  nc-cli harness seed-nc-helm-lab [--root </tmp/nc-helm-lab>] [--seed-base <u64>]");
    println!(
        "  nc-cli harness play-until --file <scenario.kdl> --dir <campaign_dir> --game-id <id> --turn <n> [--bundle-profile <human|llm>] [--export-classic]"
    );
    println!("  nc-cli sysop <subcommand> ...");
    println!(
        "  nc-cli sysop new-game <target_dir> [--players <1-25>] [--year <u16>] [--seed <u64>] [--config <setup.kdl>]"
    );
    println!(
        "  nc-cli sysop generate-gamestate <target_dir> <player_count> <year> [<homeworld_x>:<homeworld_y>...]"
    );
    println!("  nc-cli core-report [dir]");
    println!("  nc-cli core-diff-current-known-baseline [dir]");
    println!("  nc-cli core-diff-current-known-baseline-offsets [dir]");
    println!("  nc-cli core-diff-canonical-current-known-baseline [dir]");
    println!("  nc-cli core-diff-canonical-current-known-baseline-offsets [dir]");
    println!("  nc-cli core-report-canonical-transition-clusters [dir]");
    println!("  nc-cli core-report-canonical-transition-details [dir]");
    println!("  nc-cli core-validate [dir]");
    println!("  nc-cli core-validate-current-known-baseline [dir]");
    println!("  nc-cli core-sync-counts [dir]");
    println!("  nc-cli core-sync-baseline [dir]");
    println!("  nc-cli core-sync-current-known-baseline [dir]");
    println!("  nc-cli core-sync-canonical-current-known-baseline [dir]");
    println!("  nc-cli core-sync-initialized-fleets [dir]");
    println!("  nc-cli core-sync-initialized-planets [dir]");
    println!("  nc-cli core-init-current-known-baseline [source_dir] <target_dir>");
    println!("  nc-cli core-init-canonical-current-known-baseline [source_dir] <target_dir>");
    println!("  nc-cli headers [dir]");
    println!(
        "  nc-cli fleet-order <dir> <fleet_record> <speed> <order_code> <target_x> <target_y> [aux0] [aux1]"
    );
    println!("  nc-cli fleet-order-report [dir] [fleet_record]");
    println!(
        "  nc-cli fleet-ships <dir> <fleet_record> <sc> <bb> <ca> <dd> <tt> [loaded_armies] [etac]"
    );
    println!("  nc-cli fleet-location <dir> <fleet_record> <x> <y>");
    println!(
        "  nc-cli fleet-detach <dir> <player_record> <donor_fleet_record> <bb> <ca> <dd> <full_tt> <empty_tt> <sc> <etac> [donor_speed] [new_roe]"
    );
    println!(
        "  nc-cli fleet-order-init <target_dir> <fleet_record> <speed> <order_code> <target_x> <target_y> [aux0] [aux1]"
    );
    println!(
        "  nc-cli fleet-order-batch-init <target_root> <fleet_record:speed:order:target_x:target_y[:aux0[:aux1]]>..."
    );
    println!("  nc-cli planet-build <dir> <planet_record> <quantity> <unit>");
    println!("  nc-cli player-name <dir> <player_record> <handle> <empire_name>");
    println!(
        "  nc-cli player-join <dir> <player_record> <caller_alias> <empire_name> [homeworld_name]"
    );
    println!("  nc-cli classic-login-prepare <dir> <player_record> <caller_alias> [empire_name]");
    println!("  nc-cli planet-owner <dir> <planet_record> <owner_slot>");
    println!("  nc-cli planet-name <dir> <planet_record> <name>");
    println!("  nc-cli planet-stats <dir> <planet_record> <armies> <batteries>");
    println!("  nc-cli planet-potential <dir> <planet_record> <p1_raw> <p2_raw>");
    println!("  nc-cli planet-present <dir> <planet_record> <points>");
    println!("  nc-cli planet-stored <dir> <planet_record> <points>");
    println!("  nc-cli planet-stardock <dir> <planet_record> <slot> <kind_raw> <count>");
    println!("  nc-cli planet-init-original <dir>");
    println!("  nc-cli planet-build-report [dir] [planet_record]");
    println!("  nc-cli planet-build-init <target_dir> <planet_record> <quantity> <unit>");
    println!("  nc-cli planet-build-batch-init <target_root> <planet_record:quantity:unit>...");
    println!("  nc-cli guard-starbase-onebase <dir> <target_x> <target_y>");
    println!("  nc-cli guard-starbase-report <dir>");
    println!("  nc-cli guard-starbase-init [source_dir] <target_dir> <target_x> <target_y>");
    println!("  nc-cli guard-starbase-batch-init [source_dir] <target_root> <x:y> <x:y>...");
    println!("  nc-cli bombard-onefleet <dir> <target_x> <target_y> [ca] [dd]");
    println!("  nc-cli bombard-init [source_dir] <target_dir> <target_x> <target_y> <ca> <dd>");
    println!("  nc-cli bombard-batch-init [source_dir] <target_root> <x:y:ca:dd> ...");
    println!(
        "  nc-cli invade-onefleet <dir> <target_x> <target_y> [sc] [bb] [ca] [dd] [tt] [armies]"
    );
    println!(
        "  nc-cli invade-init [source_dir] <target_dir> <target_x> <target_y> <sc> <bb> <ca> <dd> <tt> <armies>"
    );
    println!(
        "  nc-cli invade-batch-init [source_dir] <target_root> <x:y:sc:bb:ca:dd:tt:armies> ..."
    );
    println!(
        "  nc-cli fleet-battle <dir> <battle_x> <battle_y> [f0_roe] [f0_bb] [f0_ca] [f0_dd] [f2_ca] [f2_dd] [f4_sc] [f4_bb] [f4_ca] [f8_loc_x] [f8_loc_y] [f8_sc] [f8_bb] [f8_ca] [p14_x] [p14_y] [p14_armies] [p14_batteries]"
    );
    println!(
        "  nc-cli fleet-battle-init [source_dir] <target_dir> <battle_x> <battle_y> <f0_roe> <f0_bb> <f0_ca> <f0_dd> <f2_ca> <f2_dd> <f4_sc> <f4_bb> <f4_ca> <f8_loc_x> <f8_loc_y> <f8_sc> <f8_bb> <f8_ca> <p14_x> <p14_y> <p14_armies> <p14_batteries>"
    );
    println!(
        "  nc-cli fleet-battle-batch-init [source_dir] <target_root> <bx:by:f0r:f0bb:f0ca:f0dd:f2ca:f2dd:f4sc:f4bb:f4ca:f8lx:f8ly:f8sc:f8bb:f8ca:p14x:p14y:p14a:p14b> ..."
    );
    println!(
        "  nc-cli econ <dir> <target_x> <target_y> [bb] [ca] [dd] [p14_x] [p14_y] [p14_armies] [p14_batteries]"
    );
    println!(
        "  nc-cli econ-init [source_dir] <target_dir> <target_x> <target_y> <bb> <ca> <dd> <p14_x> <p14_y> <p14_armies> <p14_batteries>"
    );
    println!(
        "  nc-cli econ-batch-init [source_dir] <target_root> <x:y:bb:ca:dd:p14x:p14y:p14a:p14b> ..."
    );
    println!("  nc-cli economy-report [dir] [player_record]");
    println!("  nc-cli economy-tax-probe-init [dir] [player_record] [tax_rate]");
    println!("  nc-cli economy-starbase-probe-init [dir] [player_record] [tax_rate]");
    println!("  nc-cli ipbm-report <dir>");
    println!("  nc-cli player-tax <dir> <player_record> <rate>");
    println!("  nc-cli ipbm-zero <dir> <count>");
    println!("  nc-cli ipbm-record-set <dir> <record_index> <primary> <owner> <gate> <follow_on>");
    println!("  nc-cli ipbm-validate <dir>");
    println!("  nc-cli ipbm-init [source_dir] <target_dir> <count>");
    println!("  nc-cli ipbm-batch-init [source_dir] <target_root> <count> <count>...");
    println!("  nc-cli compliance-report <dir>");
    println!("  nc-cli compliance-batch-report <root>");
    println!(
        "  nc-cli scenario <dir> <fleet-order|planet-build|guard-starbase|ipbm|move|bombard|fleet-battle|invade|econ>"
    );
    println!(
        "  nc-cli scenario <dir> show <fleet-order|planet-build|guard-starbase|ipbm|move|bombard|fleet-battle|invade|econ>"
    );
    println!(
        "  nc-cli scenario <dir> compose <fleet-order|planet-build|guard-starbase|ipbm|move|bombard|fleet-battle|invade|econ>..."
    );
    println!("  nc-cli scenario <dir> list");
    println!("  nc-cli scenario-init-all [source_dir] <target_root>");
    println!(
        "  nc-cli scenario-init [source_dir] <target_dir> <fleet-order|planet-build|guard-starbase|ipbm|move|bombard|fleet-battle|invade|econ>"
    );
    println!(
        "  nc-cli scenario-init-replayable [source_dir] <target_dir> <fleet-order|planet-build|guard-starbase|ipbm|move|bombard|fleet-battle|invade|econ>"
    );
    println!(
        "  nc-cli scenario-init-compose [source_dir] <target_dir> <fleet-order|planet-build|guard-starbase|ipbm|move|bombard|fleet-battle|invade|econ>..."
    );
    println!(
        "  nc-cli validate <dir> <all|fleet-order|planet-build|guard-starbase|ipbm|move|bombard|fleet-battle|invade|econ>"
    );
    println!(
        "  nc-cli validate-preserved <dir> <all|fleet-order|planet-build|guard-starbase|ipbm|move|bombard|fleet-battle|invade|econ>"
    );
    println!(
        "  nc-cli compare-preserved <dir> <all|fleet-order|planet-build|guard-starbase|ipbm|move|bombard|fleet-battle|invade|econ>"
    );
    println!("  nc-cli match [dir]");
    println!("  nc-cli compare <left_dir> <right_dir>");
    println!("  nc-cli init [source_dir] <target_dir>");
    println!(
        "  nc-cli generate-gamestate <target_dir> <player_count> <year> [<homeworld_x>:<homeworld_y>...]"
    );
    println!("  nc-cli maint-rust <dir> [turns]");
    println!("  nc-cli maint-compare <dir> [turns]");
}

pub fn print_sysop_usage(program: &str) {
    println!("Usage:");
    if program.starts_with("nc-cli") {
        println!(
            "  {program} new-game <target_dir> [--players <1-25>] [--year <u16>] [--seed <u64>] [--config <setup.kdl>]"
        );
        println!(
            "  {program} generate-gamestate <target_dir> <player_count> <year> [<homeworld_x>:<homeworld_y>...]"
        );
    } else {
        println!(
            "  {program} new-game <target_dir> [--players <1-25>] [--year <u16>] [--seed <u64>]"
        );
    }
}

pub fn print_maintenance_usage(program: &str) {
    println!("Usage:");
    println!("  {program} <dir> [turns]");
}

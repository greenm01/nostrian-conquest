pub fn print_usage() {
    println!("Usage:");
    println!(
        "  nc-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] new-game <target_dir> [--name <title>] [--players <1-25>] [--seed <u64>]"
    );
    println!(
        "  nc-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] new-game --bbs <target_dir> [--seed <u64>]"
    );
    println!(
        "  nc-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] maint <dir> [turns]"
    );
    println!(
        "  nc-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] settings <show|set|reserve|unreserve|ai-seat|remove-ai-seat> ..."
    );
    println!(
        "  nc-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] map-export --dir <game_dir> --player <N> [--out-dir <path>]"
    );
}

pub fn print_new_game_usage() {
    println!("Usage:");
    println!(
        "  nc-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] new-game <target_dir> [--name <title>] [--players <1-25>] [--seed <u64>]"
    );
    println!(
        "  nc-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] new-game --bbs <target_dir> [--seed <u64>]"
    );
}

pub fn print_maint_usage() {
    println!("Usage:");
    println!(
        "  nc-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] maint <dir> [turns]"
    );
}

pub fn print_settings_usage() {
    println!("Usage:");
    println!(
        "  nc-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] settings show --dir <game_dir>"
    );
    println!(
        "  nc-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] settings set --dir <game_dir> [--game-name <title>] [--theme-key <key>] [--snoop on|off] [--session-max-idle <minutes>] [--session-minimum-time <minutes>] [--session-local-timeout on|off] [--session-remote-timeout on|off] [--inactivity-autopilot-after <turns>] [--maintenance-enabled on|off] [--maintenance-interval-minutes <minutes>] [--maintenance-next-due <unix-seconds>]"
    );
    println!(
        "  nc-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] settings reserve --dir <game_dir> --player <N> --alias <alias>"
    );
    println!(
        "  nc-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] settings unreserve --dir <game_dir> --player <N>"
    );
    println!(
        "  nc-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] settings ai-seat --dir <game_dir> --player <N> --difficulty <easy|normal|hard> [--personality <land-grab|rusher|turtle>]"
    );
    println!(
        "  nc-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] settings remove-ai-seat --dir <game_dir> --player <N>"
    );
    println!("  maint plays AI seats with the agent named by NC_AI_MODEL (an .onnx file).");
}

pub fn print_map_export_usage() {
    println!("Usage:");
    println!(
        "  nc-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] map-export --dir <game_dir> --player <N> [--out-dir <path>]"
    );
    println!("  Writes the player's star map as ECMAP-P<N>-Y<year> .TXT, .CSV and -DETAILS.CSV.");
    println!("  --out-dir defaults to <game_dir>/exports, where the in-game export writes.");
}

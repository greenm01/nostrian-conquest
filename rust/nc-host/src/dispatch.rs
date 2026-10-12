use std::path::PathBuf;

use crate::commands;

#[derive(Clone)]
struct ParsedArgs {
    log_file: Option<PathBuf>,
    log_level: nc_log::LogLevel,
    args: Vec<String>,
}

pub fn run_args(args: impl Iterator<Item = String>) -> Result<(), Box<dyn std::error::Error>> {
    let parsed = parse_args(args.skip(1))?;
    init_logging(&parsed)?;

    let mut args = parsed.args.iter().map(String::as_str);
    let Some(cmd) = args.next() else {
        print_usage();
        return Ok(());
    };
    let rest = args.collect::<Vec<_>>();

    match cmd {
        "--help" | "-h" | "help" => {
            print_usage();
            Ok(())
        }
        "new-game" => commands::new_game::run(rest.as_slice()),
        "admin" => crate::admin::cli(rest.as_slice()),
        "serve" => commands::serve::run(rest.as_slice()),
        "maint" => commands::maint::run(rest.as_slice()),
        "settings" => commands::settings::run(rest.as_slice()),
        "games" => commands::games::run(rest.as_slice()),
        "status" => commands::status::run(rest.as_slice()),
        "seats" => commands::seats::run(rest.as_slice()),
        "requests" => commands::requests::run(rest.as_slice()),
        "roster" => commands::roster::run(rest.as_slice()),
        "notices" => commands::notices::run(rest.as_slice()),
        "threads" => commands::threads::run(rest.as_slice()),
        "nostr" => commands::nostr::run(rest.as_slice()),
        _ => Err(format!("unknown subcommand: {cmd}").into()),
    }
}

fn init_logging(parsed: &ParsedArgs) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(log_file) = parsed.log_file.as_deref() {
        nc_log::init_file_logging(log_file, parsed.log_level)?;
    } else {
        nc_log::init_stderr_logging(parsed.log_level)?;
    }
    Ok(())
}

fn parse_args(
    args: impl Iterator<Item = String>,
) -> Result<ParsedArgs, Box<dyn std::error::Error>> {
    let mut args = args.peekable();
    let mut log_file = None;
    let mut log_level = nc_log::LogLevel::Info;
    let mut rest = Vec::new();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--log-file" => {
                let Some(value) = args.next() else {
                    return Err("missing value for --log-file".into());
                };
                log_file = Some(PathBuf::from(value));
            }
            "--log-level" => {
                let Some(value) = args.next() else {
                    return Err("missing value for --log-level".into());
                };
                log_level = nc_log::LogLevel::parse(&value)?;
            }
            _ => {
                rest.push(arg);
                rest.extend(args);
                break;
            }
        }
    }

    Ok(ParsedArgs {
        log_file,
        log_level,
        args: rest,
    })
}

fn print_usage() {
    println!("nc-host future hosted stack");
    println!();
    println!("Usage:");
    println!("  nc-host [--log-file <path>] [--log-level <level>] <command> [args]");
    println!();
    println!("Commands:");
    println!("  admin        Local JSON interface for browser campaign administration");
    println!("  new-game     Create a hosted game directory");
    println!("  serve        Run the hosted relay event loop");
    println!("  maint        Run hosted maintenance for one game");
    println!("  settings     Inspect or edit hosted game settings");
    println!("  games        List or inspect hosted games");
    println!("  status       Show host-wide hosted status");
    println!("  seats        Manage hosted seats and invite lifecycle");
    println!("  requests     Review or decide invite requests");
    println!("  roster       View player history across games");
    println!("  notices      Post public lobby notices");
    println!("  threads      Review or send private sysop thread messages");
    println!("  nostr        Initialize host relay identity/config");
}

use std::path::PathBuf;

pub fn run(args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() || args.iter().any(|arg| matches!(*arg, "--help" | "-h")) {
        print_usage();
        return Ok(());
    }

    let mut game_dir = None;
    let mut turns = 1;

    let mut i = 0;
    while i < args.len() {
        match args[i] {
            _ if args[i].starts_with("--") => {
                return Err(format!("unknown flag: {}", args[i]).into());
            }
            _ => {
                if game_dir.is_none() {
                    game_dir = Some(PathBuf::from(args[i]));
                } else if let Ok(n) = args[i].parse::<u32>() {
                    turns = n;
                } else {
                    return Err(format!("unexpected argument: {}", args[i]).into());
                }
                i += 1;
            }
        }
    }

    let game_dir = game_dir.ok_or("missing game directory argument")?;
    if !game_dir.join("hosted.db").exists() {
        return Err(format!("game not found at {}", game_dir.display()).into());
    }
    let game_id = game_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("game")
        .to_string();

    println!("Running maintenance for game {} ({} turns)", game_id, turns);
    let report = crate::game::maint::run_hosted_maintenance(&game_dir, &game_id, turns)?;
    println!(
        "  Year advanced: {} -> {}",
        report.start_year, report.end_year
    );
    println!(
        "  Submissions applied: {}, rejected: {}",
        report.applied_submissions, report.rejected_submissions
    );
    println!("  Seats sent their state: {}", report.published_seats);
    println!("Maintenance complete.");

    Ok(())
}

fn print_usage() {
    println!("Usage: nc-host maint <dir> [turns]");
    println!();
    println!("Arguments:");
    println!("  <dir>     Game directory path");
    println!("  [turns]   Number of turns to process (default: 1)");
}

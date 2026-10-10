#![allow(dead_code)]

use nc_compat::{
    import_directory_snapshot_with_seed, load_classic_core_files, save_classic_core_files,
};
use nc_data::classic::{ClassicFile, without_ignored_bytes};
use nc_data::{CampaignStore, DEFAULT_CAMPAIGN_DB_NAME, DiplomaticRelation};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// `bytes` with the bytes no classic program reads zeroed (planet and
/// empire name tails, the dead FLEETS word, ranking slots past the player
/// count, and so on), so a file the typed model wrote compares equal to
/// the preserved fixture it reproduces.
pub fn without_unread_bytes(file_name: &str, bytes: &[u8]) -> Vec<u8> {
    match ClassicFile::from_file_name(file_name) {
        Some(file) => without_ignored_bytes(file, bytes),
        None => bytes.to_vec(),
    }
}

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn rust_workspace() -> PathBuf {
    repo_root().join("rust")
}

pub fn run_nc_cli(args: &[&str]) -> String {
    run_nc_cli_in_dir(args, rust_workspace())
}

pub fn run_nc_cli_output_in_dir(args: &[&str], current_dir: PathBuf) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_nc-cli"))
        .current_dir(current_dir)
        .args(args)
        .output()
        .expect("nc-cli should run")
}

pub fn run_nc_cli_in_dir(args: &[&str], current_dir: PathBuf) -> String {
    let output = run_nc_cli_output_in_dir(args, current_dir);

    assert!(
        output.status.success(),
        "nc-cli failed: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("stdout should be utf-8")
}

pub fn run_nc_cli_failure_in_dir(args: &[&str], current_dir: PathBuf) -> String {
    let output = run_nc_cli_output_in_dir(args, current_dir);

    assert!(
        !output.status.success(),
        "nc-cli unexpectedly succeeded: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stderr).expect("stderr should be utf-8")
}

pub fn import_campaign_db(target: &Path) {
    run_nc_cli_in_dir(&["db-import", target.to_str().unwrap()], rust_workspace());
}

pub fn export_campaign_db(source_dir: &Path, target_dir: &Path) {
    run_nc_cli_in_dir(
        &[
            "db-export",
            source_dir.to_str().unwrap(),
            target_dir.to_str().unwrap(),
        ],
        rust_workspace(),
    );
}

pub fn run_maint_rust_with_export(target: &Path, turns: u16) -> String {
    import_campaign_db(target);
    let turns = turns.to_string();
    let stdout = run_nc_cli_in_dir(
        &["maint-rust", target.to_str().unwrap(), &turns],
        rust_workspace(),
    );
    export_campaign_db(target, target);
    stdout
}

/// `run_maint_rust_with_export` with the campaign seed pinned, for a
/// scenario whose outcome turns on a sighting roll.
pub fn run_maint_rust_with_export_and_seed(target: &Path, turns: u16, seed: u64) -> String {
    let store =
        CampaignStore::open(target.join(DEFAULT_CAMPAIGN_DB_NAME)).expect("open campaign store");
    import_directory_snapshot_with_seed(&store, target, Some(seed)).expect("import directory");
    drop(store);
    let turns = turns.to_string();
    let stdout = run_nc_cli_in_dir(
        &["maint-rust", target.to_str().unwrap(), &turns],
        rust_workspace(),
    );
    export_campaign_db(target, target);
    stdout
}

pub fn run_maint_rust_failure_after_import(target: &Path, turns: u16) -> String {
    import_campaign_db(target);
    let turns = turns.to_string();
    run_nc_cli_failure_in_dir(
        &["maint-rust", target.to_str().unwrap(), &turns],
        rust_workspace(),
    )
}

pub fn run_ecmaint_oracle(dir: &Path) -> String {
    let output = Command::new("python3")
        .current_dir(repo_root())
        .args(["tools/ecmaint_oracle.py", "run", dir.to_str().unwrap()])
        .env("SDL_VIDEODRIVER", "dummy")
        .env("SDL_AUDIODRIVER", "dummy")
        .output()
        .expect("ecmaint oracle should run");

    assert!(
        output.status.success(),
        "ecmaint oracle failed: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("oracle stdout should be utf-8")
}

pub fn ecmaint_oracle_available() -> bool {
    command_runs("python3", &["--version"]) && command_runs("dosbox-x", &["-version"])
}

fn command_runs(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .current_dir(repo_root())
        .args(args)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

pub fn unique_temp_dir(prefix: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let target = std::env::temp_dir().join(format!("{prefix}-{unique}"));
    fs::create_dir_all(&target).unwrap();
    target
}

pub fn cleanup_dir(path: &Path) {
    let _ = fs::remove_dir_all(path);
}

pub fn run_classic_ecgame_smoke(target: &Path, player_number: u8) -> String {
    run_classic_ecgame_smoke_with_alias(target, player_number, "Sysop")
}

pub fn run_classic_ecgame_smoke_with_alias(
    target: &Path,
    player_number: u8,
    caller_alias: &str,
) -> String {
    let output = Command::new("/usr/bin/bash")
        .current_dir(repo_root())
        .env("SDL_VIDEODRIVER_OVERRIDE", "dummy")
        .env("SDL_AUDIODRIVER_OVERRIDE", "dummy")
        .arg("-lc")
        .arg(format!(
            "/usr/bin/timeout 8s tools/run_ecgame.sh '{}' {} '{}'",
            target.display(),
            player_number,
            caller_alias
        ))
        .output()
        .expect("classic ECGAME smoke should run");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        output.status.code() == Some(124),
        "classic ECGAME smoke did not stay alive under timeout: status={:?} output={combined:?}",
        output.status.code()
    );

    let errors_path = target.join("ERRORS.TXT");
    if errors_path.exists() {
        let errors = fs::read_to_string(&errors_path).expect("ERRORS.TXT should be readable");
        assert!(
            !errors.contains("could not find a Door File")
                && !errors.contains("unexpected End Of File")
                && !errors.contains("found invalid data in file"),
            "classic ECGAME startup reported parser/file errors: {errors:?}"
        );
    }

    combined
}

pub fn decode_results_text(raw: &[u8]) -> String {
    const RESULTS_TEXT_SIZE: usize = 72;
    const RESULTS_TEXT_START: usize = 2;
    const RESULTS_TEXT_END: usize = RESULTS_TEXT_START + RESULTS_TEXT_SIZE;
    raw.chunks(84)
        .flat_map(|record| {
            if record.len() != 84 {
                return Vec::new();
            }
            let used = record[1] as usize;
            if used <= RESULTS_TEXT_SIZE
                && record[RESULTS_TEXT_START + used..RESULTS_TEXT_END]
                    .iter()
                    .all(|byte| *byte == 0)
            {
                return record[RESULTS_TEXT_START..RESULTS_TEXT_START + used].to_vec();
            }
            let text = &record[1..76];
            let end = text.iter().position(|b| *b == 0).unwrap_or(text.len());
            text[..end].to_vec()
        })
        .map(char::from)
        .collect()
}

pub fn copy_fixture_dir(relative: &str, target: &Path) {
    let fixture = repo_root().join(relative);
    copy_dir_files(&fixture, target);
}

pub fn copy_dir_files(source: &Path, target: &Path) {
    fs::create_dir_all(target).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        if !entry.file_type().unwrap().is_file() {
            continue;
        }
        fs::copy(entry.path(), target.join(entry.file_name())).unwrap();
    }
}

pub fn write_mutual_enemy_diplomacy(target: &Path, left: u8, right: u8) {
    let text = format!(
        "relation from={} to={} status=\"enemy\"\nrelation from={} to={} status=\"enemy\"\n",
        left, right, right, left
    );
    fs::write(target.join("diplomacy.kdl"), text).unwrap();
}

pub fn set_mutual_enemy_in_player_dat(target: &Path, left: u8, right: u8) {
    let mut game_data = load_classic_core_files(target).unwrap();
    game_data
        .set_stored_diplomatic_relation(left, right, DiplomaticRelation::Enemy)
        .unwrap();
    game_data
        .set_stored_diplomatic_relation(right, left, DiplomaticRelation::Enemy)
        .unwrap();
    save_classic_core_files(target, &game_data).unwrap();
}

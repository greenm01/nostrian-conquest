//! Player processes changing one campaign at once, killed mid-change and
//! started again, and maintenance run in the middle of their play (t205).
//! Each writer is the `stress_writer` example, which the test builds.

#[allow(dead_code)]
mod random_play;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::Duration;

use nc_data::{CampaignStore, CoreGameData};
use random_play::{Rng, seed_campaign};

const PLAYERS: u8 = 4;

struct TempDir(PathBuf);

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn temp_dir(name: &str) -> TempDir {
    let dir = std::env::temp_dir().join(format!("nc-process-stress-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    TempDir(dir)
}

/// The writer, built (when out of date) beside this test's own binary.
fn writer() -> PathBuf {
    static BUILT: std::sync::Once = std::sync::Once::new();
    BUILT.call_once(|| {
        let status = Command::new(env!("CARGO"))
            .args(["build", "-q", "-p", "nc-data", "--example", "stress_writer"])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .status()
            .expect("run cargo");
        assert!(status.success(), "building the stress writer failed");
    });
    let exe = std::env::current_exe().expect("test binary");
    let path = exe
        .parent()
        .and_then(Path::parent)
        .expect("target directory")
        .join("examples")
        .join(format!("stress_writer{}", std::env::consts::EXE_SUFFIX));
    assert!(path.exists(), "build the writer first: {}", path.display());
    path
}

fn spawn(dir: &Path, empire: u8, seed: u64, from: u64, seconds: Option<u64>) -> Child {
    let mut command = Command::new(writer());
    command
        .arg(dir)
        .arg(empire.to_string())
        .arg(seed.to_string());
    command.arg(dir.join(format!("log-{empire}")));
    command.args(["--from", &from.to_string()]);
    if let Some(seconds) = seconds {
        command.args(["--seconds", &seconds.to_string()]);
    }
    command.spawn().expect("start a writer")
}

/// The numbers each empire's writers logged as committed.
fn logged(dir: &Path) -> BTreeMap<u8, BTreeSet<u64>> {
    (1..=PLAYERS)
        .map(|empire| {
            let text =
                std::fs::read_to_string(dir.join(format!("log-{empire}"))).unwrap_or_default();
            (
                empire,
                text.lines().filter_map(|line| line.parse().ok()).collect(),
            )
        })
        .collect()
}

/// The numbers of each empire's changes the store holds, from their marker
/// mails.
fn stored(store: &CampaignStore) -> BTreeMap<u8, Vec<u64>> {
    let mut stored: BTreeMap<u8, Vec<u64>> = (1..=PLAYERS).map(|e| (e, Vec::new())).collect();
    let state = store
        .load_latest_runtime_state()
        .expect("load")
        .expect("a snapshot");
    for mail in &state.queued_mail {
        if let Some((empire, n)) = mail
            .subject
            .strip_prefix("stress ")
            .and_then(|rest| rest.split_once(':'))
        {
            stored
                .get_mut(&empire.parse().expect("empire"))
                .expect("a player")
                .push(n.parse().expect("number"));
        }
    }
    stored
}

fn integrity(store: &CampaignStore) {
    let conn = rusqlite::Connection::open(store.path()).expect("open");
    let check: String = conn
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .expect("integrity check");
    assert_eq!(check, "ok");
}

/// Fleet IDs are distinct and no fleet or base chain is broken.
fn chains_whole(game_data: &CoreGameData) {
    let ids: BTreeSet<_> = game_data
        .fleets
        .iter()
        .map(|fleet| fleet.record.0)
        .collect();
    assert_eq!(ids.len(), game_data.fleets.len(), "fleet IDs are distinct");
    for (idx, player) in game_data.players.iter().enumerate() {
        let empire = idx as u8 + 1;
        let mut reached = BTreeSet::new();
        let mut id = player.fleets_first.0;
        while id != 0 {
            let fleet = game_data
                .fleets
                .iter()
                .find(|fleet| fleet.owner.0 == empire && fleet.record.0 == id)
                .unwrap_or_else(|| panic!("empire {empire}'s chain names missing fleet {id}"));
            assert!(reached.insert(id), "empire {empire}'s chain loops at {id}");
            id = fleet.next.0;
        }
    }
    assert_eq!(
        game_data.ecmaint_structural_preflight_errors(),
        Vec::<String>::new()
    );
    assert_eq!(game_data.unchained_base_errors(), Vec::<String>::new());
}

/// Writers killed mid-change: every change they logged is stored, a
/// killed change either committed whole (then at most one beyond the log)
/// or left no trace, and the file is intact.
#[test]
fn killed_writers_lose_no_committed_change_and_leave_no_partial_one() {
    let dir = temp_dir("killed");
    let store = CampaignStore::open_default_in_dir(&dir.0).expect("open store");
    seed_campaign(&store, PLAYERS);
    let mut rng = Rng(0x5eed_5eed);
    let mut next: BTreeMap<u8, u64> = (1..=PLAYERS).map(|e| (e, 0)).collect();
    for round in 0..3u64 {
        let mut writers: Vec<Child> = (1..=PLAYERS)
            .map(|empire| {
                spawn(
                    &dir.0,
                    empire,
                    round * 31 + u64::from(empire),
                    next[&empire],
                    None,
                )
            })
            .collect();
        std::thread::sleep(Duration::from_millis(400 + rng.below(800) as u64));
        for writer in &mut writers {
            writer.kill().expect("kill -9");
            writer.wait().expect("reap");
        }
        integrity(&store);
        let (logged, stored) = (logged(&dir.0), stored(&store));
        for empire in 1..=PLAYERS {
            let held = &stored[&empire];
            let contiguous: Vec<u64> = (0..held.len() as u64).collect();
            let mut sorted = held.clone();
            sorted.sort();
            assert_eq!(
                sorted, contiguous,
                "round {round}: empire {empire}'s stored changes"
            );
            let last_logged = logged[&empire].iter().max().copied();
            assert!(
                logged[&empire].iter().all(|n| (*n as usize) < held.len()),
                "round {round}: empire {empire} lost a logged change"
            );
            assert!(
                held.len() as u64 <= last_logged.map_or(0, |n| n + 1) + 1,
                "round {round}: empire {empire} stored more than one unlogged change"
            );
            next.insert(empire, held.len() as u64);
        }
        let state = store
            .load_latest_runtime_state()
            .expect("load")
            .expect("a snapshot");
        chains_whole(&state.game_data);
    }
    assert!(
        next.values().all(|&n| n > 0),
        "every writer committed changes"
    );
}

/// Maintenance, as `nc-sysop maint` runs it, in the middle of four player
/// processes: each writer's changes are refused while it runs and go on in
/// the new year, every writer ends cleanly, and the campaign is intact.
#[test]
fn maintenance_in_the_middle_of_player_processes() {
    let dir = temp_dir("maintenance");
    let store = CampaignStore::open_default_in_dir(&dir.0).expect("open store");
    seed_campaign(&store, PLAYERS);
    let writers: Vec<Child> = (1..=PLAYERS)
        .map(|empire| spawn(&dir.0, empire, u64::from(empire) * 7, 0, Some(4)))
        .collect();
    std::thread::sleep(Duration::from_millis(1500));
    {
        let run = store.begin_maintenance().expect("begin");
        let mut year = nc_engine::CampaignMaintenance::load(&store).expect("load the year");
        year.run_turn().expect("run the year");
        year.save(run.store()).expect("maintenance saves");
    }
    for mut writer in writers {
        assert!(writer.wait().expect("writer").success(), "a writer failed");
    }
    integrity(&store);
    let state = store
        .load_latest_runtime_state()
        .expect("load")
        .expect("a snapshot");
    assert_eq!(state.game_year, 3001);
    chains_whole(&state.game_data);
    let stored = stored(&store);
    assert!(
        stored.values().all(|numbers| !numbers.is_empty()),
        "every writer went on changing the new year: {stored:?}"
    );
}

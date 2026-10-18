//! Two BBS door sessions on one campaign at once, with maintenance run while
//! both are open (t205): each caller's join is kept, a change made during
//! the run is refused as paused rather than lost, and the next sessions see
//! the new year.

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use nc_compat::import_directory_snapshot;
use nc_data::{CampaignSettings, CampaignStore};
use nc_engine::CampaignMaintenance;

fn temp_campaign() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "nc-game-door-sessions-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time ok")
            .as_nanos()
    ));
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/ecutil-init/v1.5");
    fs::create_dir_all(&root).expect("create temp dir");
    for entry in fs::read_dir(&fixture).expect("read fixture") {
        let entry = entry.expect("fixture entry");
        if entry.path().is_file() {
            fs::copy(entry.path(), root.join(entry.file_name())).expect("copy fixture file");
        }
    }
    let store = CampaignStore::open_default_in_dir(&root).expect("open campaign store");
    import_directory_snapshot(&store, &root).expect("seed sqlite snapshot");
    store
        .save_campaign_settings(&CampaignSettings::new("door-sessions", "Esterian Conquest"))
        .expect("seed campaign settings");
    root
}

/// A CHAIN.TXT dropfile for `alias`, in its own node folder as a BBS gives
/// each line.
fn write_dropfile(root: &Path, alias: &str) -> PathBuf {
    let node = root.join("nodes").join(alias);
    fs::create_dir_all(&node).expect("create node dir");
    let path = node.join("CHAIN.TXT");
    fs::write(
        &path,
        format!(
            "1\n{alias}\nReal Name\nNC0DE\n34\nM\n1000.0\n03/25/26\n80\n25\n100\n0\n0\n1\n1\n900\nC:\\BBS\\GFILES\\\nC:\\BBS\\DATA\\\nBBS.LOG\n38400\n1\n1\n1\n0\n0\n0\n0\n0\n0\n0\n0\n0\n"
        ),
    )
    .expect("write dropfile");
    path
}

/// One caller's `nc-door` process, driven through its piped terminal.
struct Session {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: Arc<Mutex<Vec<u8>>>,
    reader: Option<thread::JoinHandle<()>>,
    seen: usize,
    deadline: Instant,
}

impl Session {
    fn start(root: &Path, alias: &str) -> Self {
        let dropfile = write_dropfile(root, alias);
        let mut child = Command::new(env!("CARGO_BIN_EXE_nc-door"))
            .args([
                "--dir",
                root.to_str().expect("utf-8 path"),
                "--dropfile",
                dropfile.to_str().expect("utf-8 path"),
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("nc-door should start");
        let mut out = child.stdout.take().expect("piped stdout");
        let stdout = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&stdout);
        let reader = thread::spawn(move || {
            let mut buf = [0u8; 4096];
            while let Ok(read @ 1..) = out.read(&mut buf) {
                sink.lock()
                    .expect("stdout lock")
                    .extend_from_slice(&buf[..read]);
            }
        });
        Session {
            stdin: child.stdin.take(),
            child,
            stdout,
            reader: Some(reader),
            seen: 0,
            deadline: Instant::now() + Duration::from_secs(60),
        }
    }

    /// Wait until `needle` is drawn after everything waited for so far.
    fn wait_for(&mut self, needle: &str) {
        loop {
            {
                let bytes = self.stdout.lock().expect("stdout lock");
                if let Some(pos) = bytes[self.seen.min(bytes.len())..]
                    .windows(needle.len())
                    .position(|window| window == needle.as_bytes())
                {
                    self.seen += pos + needle.len();
                    return;
                }
                if Instant::now() >= self.deadline {
                    self.child.kill().ok();
                    panic!(
                        "timed out waiting for {needle:?} in nc-door stdout: {:?}",
                        String::from_utf8_lossy(&bytes[self.seen.min(bytes.len())..])
                    );
                }
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn send(&mut self, keys: &[u8]) {
        let stdin = self.stdin.as_mut().expect("session open");
        stdin.write_all(keys).expect("write keys");
        stdin.flush().expect("flush keys");
    }

    /// Hang up, as a caller dropping carrier would, and wait for the door to
    /// exit.
    fn hang_up(mut self) {
        drop(self.stdin.take());
        let deadline = Instant::now() + Duration::from_secs(30);
        let status = loop {
            if let Some(status) = self.child.try_wait().expect("poll door") {
                break status;
            }
            if Instant::now() >= deadline {
                self.child.kill().ok();
                panic!("nc-door did not exit after hang-up");
            }
            thread::sleep(Duration::from_millis(25));
        };
        self.reader
            .take()
            .expect("reader")
            .join()
            .expect("reader thread");
        let mut stderr = String::new();
        if let Some(mut err) = self.child.stderr.take() {
            err.read_to_string(&mut stderr).ok();
        }
        assert!(status.success(), "nc-door failed: {stderr}");
    }
}

/// Join from the first-time menu up to the confirmation, without confirming.
fn join_until_confirm(session: &mut Session, empire: &str) {
    session.wait_for("FIRST TIME COMMAND");
    session.send(b"j");
    session.wait_for("EMPIRE NAME");
    session.send(empire.as_bytes());
    session.send(b"\r");
    session.wait_for("Press N or Esc to go back");
}

fn empire_names(root: &Path) -> (u16, Vec<String>) {
    let store = CampaignStore::open_default_in_dir(root).expect("open store");
    let state = store
        .load_latest_runtime_state()
        .expect("load")
        .expect("a snapshot");
    let names = state
        .game_data
        .players
        .iter()
        .map(|player| player.empire_name.trim().to_string())
        .filter(|name| !name.is_empty())
        .collect();
    (state.game_data.conquest.game_year, names)
}

#[test]
fn two_door_sessions_keep_their_joins_through_maintenance() {
    let root = temp_campaign();
    let (start_year, _) = empire_names(&root);

    let mut alpha = Session::start(&root, "ALPHA");
    let mut bravo = Session::start(&root, "BRAVO");
    join_until_confirm(&mut alpha, "Alpha Realm");
    join_until_confirm(&mut bravo, "Bravo Realm");

    // Alpha joins while Bravo's session sits at its confirmation.
    alpha.send(b"y");
    alpha.wait_for("JOIN COMPLETE");

    // Maintenance runs with both sessions open. Bravo confirms during it.
    let store = CampaignStore::open_default_in_dir(&root).expect("open store");
    let run = store.begin_maintenance().expect("begin maintenance");
    bravo.send(b"y");
    bravo.wait_for("Campaign paused");
    let mut maintenance = CampaignMaintenance::load(&store).expect("load maintenance");
    maintenance.run_turn().expect("run turn");
    maintenance.save(run.store()).expect("save year");
    drop(run);

    let (year, names) = empire_names(&root);
    assert_eq!(year, start_year + 1, "maintenance advanced the year");
    assert!(names.contains(&"Alpha Realm".to_string()), "{names:?}");
    assert!(
        !names.contains(&"Bravo Realm".to_string()),
        "a join refused as paused left no trace: {names:?}"
    );
    alpha.hang_up();
    bravo.hang_up();

    // Bravo calls back and joins the new year; Alpha is a returning player.
    let mut bravo = Session::start(&root, "BRAVO");
    join_until_confirm(&mut bravo, "Bravo Realm");
    bravo.send(b"y");
    bravo.wait_for("JOIN COMPLETE");
    bravo.wait_for(&format!("The year is: {} A.D.", start_year + 1));
    let mut alpha = Session::start(&root, "ALPHA");
    alpha.wait_for("View the game introduction?");
    alpha.send(b"n");
    alpha.wait_for("Alpha Realm");
    bravo.hang_up();
    alpha.hang_up();

    let (_, names) = empire_names(&root);
    assert!(names.contains(&"Alpha Realm".to_string()), "{names:?}");
    assert!(names.contains(&"Bravo Realm".to_string()), "{names:?}");
    let conn = rusqlite::Connection::open(root.join("ncgame.db")).expect("open db");
    let check: String = conn
        .query_row("PRAGMA quick_check", [], |row| row.get(0))
        .expect("quick_check");
    assert_eq!(check, "ok");
    drop(conn);
    fs::remove_dir_all(&root).ok();
}

use std::io::Read;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs, path::Path};
use std::{io::Write, process::Child};

#[cfg(windows)]
use crossterm::event::KeyCode;
use nc_compat::{import_directory_snapshot, load_classic_core_files, save_classic_core_files};
use nc_data::{CampaignSettings, CampaignStore};
use nc_game::terminal::ColorMode;
#[cfg(windows)]
use nc_game::terminal::OutputEncoding;
#[cfg(windows)]
use nc_game::terminal::Terminal;
#[cfg(windows)]
use nc_game::terminal::door::{DoorTerminal, DoorTransport};
#[cfg(windows)]
use std::net::{TcpListener, TcpStream};
#[cfg(windows)]
use std::os::windows::io::IntoRawSocket;

static TEMP_DIR_SEQ: AtomicU64 = AtomicU64::new(0);
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn temp_fixture_copy() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "nc-game-basic-{}-{}-{}",
        std::process::id(),
        TEMP_DIR_SEQ.fetch_add(1, Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time ok")
            .as_nanos()
    ));
    copy_dir_all(&repo_root().join("fixtures/ecutil-init/v1.5"), &root);
    let store = CampaignStore::open_default_in_dir(&root).expect("open campaign store");
    import_directory_snapshot(&store, &root).expect("seed sqlite snapshot");
    store
        .save_campaign_settings(&CampaignSettings::new("fixture-game", "Esterian Conquest"))
        .expect("seed campaign settings");
    root
}

fn write_dropfile(root: &Path, alias: &str) -> PathBuf {
    let path = root.join("CHAIN.TXT");
    fs::write(
        &path,
        format!(
            "1\n{alias}\nReal Name\nNC0DE\n34\nM\n1000.0\n03/25/26\n80\n25\n100\n0\n0\n1\n1\n900\nC:\\BBS\\GFILES\\\nC:\\BBS\\DATA\\\nBBS.LOG\n38400\n1\n1\n1\n0\n0\n0\n0\n0\n0\n0\n0\n0\n"
        ),
    )
    .expect("write dropfile");
    path
}

fn write_reserved_config(root: &Path, alias: &str, player: usize) {
    let store = CampaignStore::open_default_in_dir(root).expect("open campaign store");
    let mut settings = store.load_campaign_settings().expect("load settings");
    settings.reservations = vec![nc_data::SeatReservation {
        player_record_index_1_based: player,
        alias: alias.to_string(),
    }];
    store
        .save_campaign_settings(&settings)
        .expect("write settings");
}

fn set_runtime_schema_version(root: &Path, version: i64) {
    let conn = rusqlite::Connection::open(root.join("ncgame.db")).expect("open runtime db");
    let rows = conn
        .execute(
            "UPDATE campaign_metadata
             SET int_value = ?1
             WHERE key = 'runtime_schema_version'",
            [version],
        )
        .expect("update runtime schema version");
    assert_eq!(rows, 1, "runtime schema version row should exist");
}

fn copy_dir_all(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).expect("create temp dir");
    for entry in fs::read_dir(src).expect("read src dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_all(&path, &target);
        } else {
            fs::copy(&path, &target).expect("copy file");
        }
    }
}

fn wait_for_exit(child: &mut Child, timeout: Duration) -> std::process::ExitStatus {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait().expect("poll child status") {
            return status;
        }
        if std::time::Instant::now() >= deadline {
            child.kill().expect("kill hung child");
            return child.wait().expect("wait after kill");
        }
        thread::sleep(Duration::from_millis(25));
    }
}

/// Collect a child's stdout on a background thread, so a test can wait for a
/// prompt before it sends the next key.
fn collect_stdout(child: &mut Child) -> (Arc<Mutex<Vec<u8>>>, thread::JoinHandle<()>) {
    let mut stdout = child.stdout.take().expect("stdout should be piped");
    let collected = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&collected);
    let reader = thread::spawn(move || {
        let mut buf = [0u8; 4096];
        while let Ok(read @ 1..) = stdout.read(&mut buf) {
            sink.lock()
                .expect("stdout lock")
                .extend_from_slice(&buf[..read]);
        }
    });
    (collected, reader)
}

/// Wait until `needle` appears in the collected stdout at or after byte
/// `from`, and return the offset just past it. Kill the child and fail if it
/// does not appear before `deadline`.
fn wait_for_stdout(
    child: &mut Child,
    stdout: &Mutex<Vec<u8>>,
    needle: &str,
    from: usize,
    deadline: Instant,
) -> usize {
    loop {
        {
            let bytes = stdout.lock().expect("stdout lock");
            if let Some(pos) = bytes[from.min(bytes.len())..]
                .windows(needle.len())
                .position(|window| window == needle.as_bytes())
            {
                return from + pos + needle.len();
            }
            if Instant::now() >= deadline {
                child.kill().ok();
                panic!(
                    "timed out waiting for {needle:?} in nc-game stdout: {:?}",
                    String::from_utf8_lossy(&bytes)
                );
            }
        }
        thread::sleep(Duration::from_millis(10));
    }
}

struct EnvGuard {
    key: &'static str,
    prior: Option<String>,
}

impl EnvGuard {
    fn set(key: &'static str, value: Option<&str>) -> Self {
        let prior = std::env::var(key).ok();
        unsafe {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
        Self { key, prior }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        unsafe {
            match &self.prior {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }
}

#[test]
fn client_renders_startup_splash_from_fixture() {
    let fixture_dir = temp_fixture_copy();
    let output = Command::new(env!("CARGO_BIN_EXE_nc-game"))
        .args([
            "--dir",
            fixture_dir.to_str().expect("fixture path should be utf-8"),
            "--player",
            "1",
        ])
        .stdin(Stdio::null())
        .output()
        .expect("nc-game should run");

    assert!(
        output.status.success(),
        "nc-game failed: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "successful nc-game launch should be silent on stderr: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("_   _  ___  ____"));
    assert!(stdout.contains(&format!("NC v{}", env!("CARGO_PKG_VERSION"))));
    assert!(stdout.contains("View the game introduction? Y/[N] ->"));
    assert!(!stdout.contains("config.kdl"));
    assert!(!stdout.contains("theme.kdl"));
}

#[test]
fn reserved_dropfile_alias_can_launch_without_player_flag() {
    let fixture_dir = temp_fixture_copy();
    write_reserved_config(&fixture_dir, "SYSOP", 1);
    let dropfile = write_dropfile(&fixture_dir, "SYSOP");

    let output = Command::new(env!("CARGO_BIN_EXE_nc-game"))
        .args([
            "--dir",
            fixture_dir.to_str().expect("fixture path should be utf-8"),
            "--dropfile",
            dropfile.to_str().expect("dropfile path should be utf-8"),
        ])
        .stdin(Stdio::null())
        .output()
        .expect("nc-game should run");

    assert!(
        output.status.success(),
        "nc-game failed: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "successful nc-game launch should be silent on stderr: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn reserved_dropfile_alias_rejects_mismatched_explicit_player() {
    let fixture_dir = temp_fixture_copy();
    write_reserved_config(&fixture_dir, "SYSOP", 1);
    let dropfile = write_dropfile(&fixture_dir, "SYSOP");

    let output = Command::new(env!("CARGO_BIN_EXE_nc-game"))
        .args([
            "--dir",
            fixture_dir.to_str().expect("fixture path should be utf-8"),
            "--player",
            "2",
            "--dropfile",
            dropfile.to_str().expect("dropfile path should be utf-8"),
        ])
        .stdin(Stdio::null())
        .output()
        .expect("nc-game should run");

    assert!(!output.status.success(), "nc-game should reject mismatch");
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("--player 2 does not match reserved seat 1"),
        "stderr={:?}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn unreserved_dropfile_alias_without_player_opens_first_time_menu() {
    let fixture_dir = temp_fixture_copy();
    write_reserved_config(&fixture_dir, "SYSOP", 1);
    let dropfile = write_dropfile(&fixture_dir, "RIVAL");

    let output = Command::new(env!("CARGO_BIN_EXE_nc-game"))
        .args([
            "--dir",
            fixture_dir.to_str().expect("fixture path should be utf-8"),
            "--dropfile",
            dropfile.to_str().expect("dropfile path should be utf-8"),
        ])
        .stdin(Stdio::null())
        .output()
        .expect("nc-game should run");

    assert!(
        output.status.success(),
        "nc-game failed: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "successful nc-game launch should be silent on stderr: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("FIRST TIME MENU:"));
    assert!(stdout.contains("FIRST TIME COMMAND"));
}

#[test]
fn piped_dropfile_launch_stays_interactive_without_tty_stdout() {
    let fixture_dir = temp_fixture_copy();
    write_reserved_config(&fixture_dir, "SYSOP", 1);
    let dropfile = write_dropfile(&fixture_dir, "RIVAL");

    let mut child = Command::new(env!("CARGO_BIN_EXE_nc-game"))
        .args([
            "--dir",
            fixture_dir.to_str().expect("fixture path should be utf-8"),
            "--dropfile",
            dropfile.to_str().expect("dropfile path should be utf-8"),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("nc-game should start");

    // Send each key only once the screen it answers has been drawn.
    let deadline = Instant::now() + Duration::from_secs(30);
    let (stdout, reader) = collect_stdout(&mut child);
    let mut stdin = child.stdin.take().expect("stdin should be piped");
    let mut seen = wait_for_stdout(&mut child, &stdout, "FIRST TIME COMMAND", 0, deadline);
    stdin.write_all(b"j").expect("write join key");
    stdin.flush().expect("flush join key");
    seen = wait_for_stdout(&mut child, &stdout, "EMPIRE NAME", seen, deadline);
    stdin.write_all(&[0x1b]).expect("write escape key");
    stdin.flush().expect("flush escape key");
    // Seat 1 is reserved for SYSOP; RIVAL holds no reservation, so Esc goes
    // back to the first-time menu rather than to a reserved setup.
    wait_for_stdout(&mut child, &stdout, "FIRST TIME COMMAND", seen, deadline);
    stdin.write_all(b"q").expect("write quit key");
    stdin.flush().expect("flush quit key");
    // Closing stdin ends the session.
    drop(stdin);

    let status = wait_for_exit(
        &mut child,
        deadline.saturating_duration_since(Instant::now()),
    );
    reader.join().expect("stdout reader should finish");
    let output = child.wait_with_output().expect("collect child output");
    let stdout = std::mem::take(&mut *stdout.lock().expect("stdout lock"));

    assert!(
        status.success(),
        "nc-game failed: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "successful nc-game launch should be silent on stderr: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("FIRST TIME MENU:"));
    assert!(stdout.contains("EMPIRE NAME"));
    assert!(
        !stdout.contains("RESERVED PLAYER") && !stdout.contains("reserved setup"),
        "an unreserved caller is never shown another caller's reservation"
    );
}

#[test]
fn explicit_player_out_of_range_is_refused() {
    let fixture_dir = temp_fixture_copy();

    let output = Command::new(env!("CARGO_BIN_EXE_nc-game"))
        .args([
            "--dir",
            fixture_dir.to_str().expect("fixture path should be utf-8"),
            "--player",
            "9",
        ])
        .stdin(Stdio::null())
        .output()
        .expect("nc-game should run");

    assert!(
        !output.status.success(),
        "nc-game should reject bad --player"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--player 9 exceeds player count 4"),
        "stderr={:?}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn detect_color_mode_treats_modern_ssh_terms_as_rich_color() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    let _colorterm = EnvGuard::set("COLORTERM", None);
    let _term = EnvGuard::set("TERM", Some("xterm-kitty"));

    assert_eq!(nc_game::cli::detect_color_mode(), ColorMode::TrueColor);
}

#[test]
fn detect_color_mode_uses_color256_for_non_dumb_term_without_legacy_hint() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    let _colorterm = EnvGuard::set("COLORTERM", None);
    let _term = EnvGuard::set("TERM", Some("xterm"));

    assert_eq!(nc_game::cli::detect_color_mode(), ColorMode::Color256);
}

#[test]
fn detect_color_mode_keeps_ansi16_for_dumb_term() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    let _colorterm = EnvGuard::set("COLORTERM", None);
    let _term = EnvGuard::set("TERM", Some("dumb"));

    assert_eq!(nc_game::cli::detect_color_mode(), ColorMode::Ansi16);
}

#[test]
fn reserved_dropfile_alias_rejects_conflicting_stored_player_handle() {
    let fixture_dir = temp_fixture_copy();
    let mut data = load_classic_core_files(&fixture_dir).expect("load fixture");
    data.join_player(1, "Codex Dominion")
        .expect("join player for mismatch test");
    nc_data::set_player_handle(&mut data.players[0], "OTHER");
    save_classic_core_files(&fixture_dir, &data).expect("save fixture");
    let store = CampaignStore::open_default_in_dir(&fixture_dir).expect("open campaign store");
    import_directory_snapshot(&store, &fixture_dir).expect("refresh sqlite snapshot");

    write_reserved_config(&fixture_dir, "SYSOP", 1);
    let dropfile = write_dropfile(&fixture_dir, "SYSOP");

    let output = Command::new(env!("CARGO_BIN_EXE_nc-game"))
        .args([
            "--dir",
            fixture_dir.to_str().expect("fixture path should be utf-8"),
            "--dropfile",
            dropfile.to_str().expect("dropfile path should be utf-8"),
        ])
        .stdin(Stdio::null())
        .output()
        .expect("nc-game should run");

    assert!(
        !output.status.success(),
        "nc-game should reject handle conflict"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("conflicts with stored player handle 'OTHER'"),
        "stderr={:?}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn nc_game_opt_in_log_file_captures_startup_without_stderr_noise() {
    let fixture_dir = temp_fixture_copy();
    let log_path = fixture_dir.join("nc-game.log");

    let output = Command::new(env!("CARGO_BIN_EXE_nc-game"))
        .args([
            "--dir",
            fixture_dir.to_str().expect("fixture path should be utf-8"),
            "--player",
            "1",
            "--log-file",
            log_path.to_str().expect("log path should be utf-8"),
            "--log-level",
            "debug",
        ])
        .stdin(Stdio::null())
        .output()
        .expect("nc-game should run");

    assert!(
        output.status.success(),
        "nc-game failed: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "successful nc-game launch should stay silent on stderr: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );

    let log = fs::read_to_string(&log_path).expect("log file should be created");
    assert!(log.contains("nc-game logging initialized"));
    assert!(log.contains("loaded nc-game app"));
}

#[test]
fn nc_game_rejects_invalid_log_level() {
    let fixture_dir = temp_fixture_copy();

    let output = Command::new(env!("CARGO_BIN_EXE_nc-game"))
        .args([
            "--dir",
            fixture_dir.to_str().expect("fixture path should be utf-8"),
            "--player",
            "1",
            "--log-level",
            "loud",
        ])
        .stdin(Stdio::null())
        .output()
        .expect("nc-game should run");

    assert!(!output.status.success(), "invalid log level should fail");
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("unknown log level 'loud'; expected error, warn, info, debug, or trace"),
        "stderr={:?}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn nc_door_rejects_schema_mismatch_with_dismissible_error_screen() {
    let fixture_dir = temp_fixture_copy();
    set_runtime_schema_version(&fixture_dir, 7);
    let dropfile = write_dropfile(&fixture_dir, "SYSOP");

    let mut child = Command::new(env!("CARGO_BIN_EXE_nc-door"))
        .args([
            "--dir",
            fixture_dir.to_str().expect("fixture path should be utf-8"),
            "--dropfile",
            dropfile.to_str().expect("dropfile path should be utf-8"),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("nc-door should start");

    let mut stdin = child.stdin.take().expect("stdin should be piped");
    let writer = thread::spawn(move || {
        thread::sleep(Duration::from_millis(150));
        stdin.write_all(b" ").expect("write dismiss key");
        stdin.flush().expect("flush dismiss key");
    });

    let status = wait_for_exit(&mut child, Duration::from_secs(5));
    writer.join().expect("writer thread should finish");
    let output = child.wait_with_output().expect("collect child output");

    assert!(!status.success(), "nc-door should reject stale schema");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("NOSTRIAN CONQUEST DOOR ERROR"),
        "stdout={stdout:?}"
    );
    assert!(
        stdout.contains("This game directory does not match this build."),
        "stdout={stdout:?}"
    );
    assert!(
        stdout.contains("Found runtime schema 7, but this build expects "),
        "stdout={stdout:?}"
    );
    assert!(
        stdout.contains("Recreate or refresh ncgame.db before launching again."),
        "stdout={stdout:?}"
    );
    assert!(stdout.contains("(slap a key)"), "stdout={stdout:?}");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("runtime sqlite schema version 7 is unsupported; expected "),
        "stderr={stderr:?}"
    );
}

#[cfg(windows)]
#[test]
fn windows_socket_door_terminal_reads_and_writes_over_inherited_descriptor() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("listener addr");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept socket");
        let mut buf = [0u8; 256];
        let mut rendered = Vec::new();
        loop {
            let bytes_read = stream.read(&mut buf).expect("read terminal output");
            if bytes_read == 0 {
                break;
            }
            rendered.extend_from_slice(&buf[..bytes_read]);
            if rendered
                .windows("SOCKET OK".len())
                .any(|window| window == b"SOCKET OK")
            {
                break;
            }
        }
        let rendered = String::from_utf8_lossy(&rendered);
        assert!(rendered.contains("SOCKET OK"), "rendered={rendered:?}");
        stream.write_all(b"q").expect("write keypress");
        stream.flush().expect("flush keypress");
    });

    let client = TcpStream::connect(addr).expect("connect client");
    let descriptor = client.into_raw_socket() as u64;
    let mut terminal = DoorTerminal::with_transport_and_color_mode(
        OutputEncoding::Utf8,
        ColorMode::Ansi16,
        nc_game::screen::ScreenGeometry::local_default(),
        DoorTransport::SocketDescriptor { descriptor },
    )
    .expect("socket transport should initialize");

    terminal
        .dump_text_capture("SOCKET OK")
        .expect("write over socket transport");
    let key = terminal.read_key().expect("read key from socket transport");
    assert_eq!(key.code, KeyCode::Char('q'));

    server.join().expect("server thread should finish");
}

#[cfg(windows)]
#[test]
fn windows_nonblocking_socket_door_terminal_handles_would_block() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("listener addr");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept socket");
        thread::sleep(Duration::from_millis(200));
        let mut buf = [0u8; 8192];
        let mut rendered = Vec::new();
        loop {
            let bytes_read = stream.read(&mut buf).expect("read terminal output");
            if bytes_read == 0 {
                break;
            }
            rendered.extend_from_slice(&buf[..bytes_read]);
            if rendered
                .windows("WOULD BLOCK OK".len())
                .any(|window| window == b"WOULD BLOCK OK")
            {
                break;
            }
        }
        let rendered = String::from_utf8_lossy(&rendered);
        assert!(rendered.contains("WOULD BLOCK OK"), "rendered={rendered:?}");
        stream.write_all(b"q").expect("write keypress");
        stream.flush().expect("flush keypress");
    });

    let client = TcpStream::connect(addr).expect("connect client");
    client
        .set_nonblocking(true)
        .expect("mark client socket nonblocking");
    let descriptor = client.into_raw_socket() as u64;
    let mut terminal = DoorTerminal::with_transport_and_color_mode(
        OutputEncoding::Utf8,
        ColorMode::Ansi16,
        nc_game::screen::ScreenGeometry::local_default(),
        DoorTransport::SocketDescriptor { descriptor },
    )
    .expect("socket transport should initialize");

    let payload = format!("{}WOULD BLOCK OK", "X".repeat(4 * 1024 * 1024));
    terminal
        .dump_text_capture(&payload)
        .expect("write over nonblocking socket transport");
    let key = terminal
        .read_key()
        .expect("read key from nonblocking socket transport");
    assert_eq!(key.code, KeyCode::Char('q'));

    server.join().expect("server thread should finish");
}

#[cfg(windows)]
#[test]
fn windows_tcp_connect_door_terminal_reads_and_writes_over_local_socket_server() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("listener addr");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept socket");
        let mut buf = [0u8; 256];
        let mut rendered = Vec::new();
        loop {
            let bytes_read = stream.read(&mut buf).expect("read terminal output");
            if bytes_read == 0 {
                break;
            }
            rendered.extend_from_slice(&buf[..bytes_read]);
            if rendered
                .windows("CONNECT BACK OK".len())
                .any(|window| window == b"CONNECT BACK OK")
            {
                break;
            }
        }
        let rendered = String::from_utf8_lossy(&rendered);
        assert!(
            rendered.contains("CONNECT BACK OK"),
            "rendered={rendered:?}"
        );
        stream.write_all(b"q").expect("write keypress");
        stream.flush().expect("flush keypress");
    });

    let mut terminal = DoorTerminal::with_transport_and_color_mode(
        OutputEncoding::Utf8,
        ColorMode::Ansi16,
        nc_game::screen::ScreenGeometry::local_default(),
        DoorTransport::TcpConnect {
            host: "127.0.0.1",
            port: addr.port(),
        },
    )
    .expect("tcp connect transport should initialize");

    terminal
        .dump_text_capture("CONNECT BACK OK")
        .expect("write over tcp connect transport");
    let key = terminal
        .read_key()
        .expect("read key from tcp connect transport");
    assert_eq!(key.code, KeyCode::Char('q'));

    server.join().expect("server thread should finish");
}

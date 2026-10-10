use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};

use nc_game::dropfile::{DoorConnectionType, DropfileError, DropfileInfo, parse};

static SEQ: AtomicUsize = AtomicUsize::new(0);

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn write_tmp(filename: &str, content: impl AsRef<[u8]>) -> std::path::PathBuf {
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "nc-game-dropfile-{}-{}-{}",
        std::process::id(),
        seq,
        filename
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(filename);
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(content.as_ref()).unwrap();
    path
}

// ---------------------------------------------------------------------------
// DOOR32.SYS
// ---------------------------------------------------------------------------

fn door32_content() -> &'static str {
    "2\r\n\
     4\r\n\
     38400\r\n\
     Mystic BBS\r\n\
     3\r\n\
     John Smith\r\n\
     Starlord\r\n\
     100\r\n\
     45\r\n\
     1\r\n\
     80\r\n\
     25\r\n"
}

#[test]
fn door32_extracts_alias_and_timeout() {
    let path = write_tmp("DOOR32.SYS", door32_content());
    let info = parse(&path).expect("should parse");
    assert_eq!(info.alias.as_deref(), Some("Starlord"));
    assert_eq!(info.timeout_minutes, Some(45));
    assert_eq!(info.connection_type, Some(DoorConnectionType::TelnetSocket));
    assert_eq!(info.socket_descriptor, Some(4));
}

#[test]
fn door32_accepts_lf_line_endings() {
    // Same content but with bare LF instead of CRLF.
    let content = door32_content().replace("\r\n", "\n");
    let path = write_tmp("DOOR32.SYS", &content);
    let info = parse(&path).expect("should parse");
    assert_eq!(info.alias.as_deref(), Some("Starlord"));
    assert_eq!(info.timeout_minutes, Some(45));
}

#[test]
fn door32_accepts_trailing_whitespace_on_lines() {
    let content = door32_content().replace("Starlord\r\n", "Starlord   \r\n");
    let path = write_tmp("DOOR32.SYS", &content);
    let info = parse(&path).expect("should parse");
    assert_eq!(info.alias.as_deref(), Some("Starlord"));
}

#[test]
fn door32_decodes_cp437_names() {
    // 0x82 is 'é' and 0x94 is 'ö' in CP437; the file is not valid UTF-8.
    let content = b"2\r\n4\r\n38400\r\nMystic BBS\r\n3\r\nJos\x82 Smith\r\nSt\x94rlord\r\n\
                    100\r\n45\r\n1\r\n80\r\n25\r\n";
    let path = write_tmp("DOOR32.SYS", content);
    let info = parse(&path).expect("CP437 dropfile should parse");
    assert_eq!(info.alias.as_deref(), Some("Störlord"));
    assert_eq!(info.timeout_minutes, Some(45));
}

#[test]
fn door32_keeps_utf8_names() {
    let content = door32_content().replace("Starlord", "Störlord");
    let path = write_tmp("DOOR32.SYS", &content);
    let info = parse(&path).expect("should parse");
    assert_eq!(info.alias.as_deref(), Some("Störlord"));
}

#[test]
fn door32_tolerates_short_file_missing_timeout() {
    // Only 8 lines — alias present on line 7, timeout on line 9 is missing.
    let content = "2\r\n4\r\n38400\r\nMystic BBS\r\n3\r\nJohn Smith\r\nStarlord\r\n100\r\n";
    let path = write_tmp("DOOR32.SYS", content);
    let info = parse(&path).expect("should parse");
    assert_eq!(info.alias.as_deref(), Some("Starlord"));
    assert_eq!(info.timeout_minutes, None);
}

#[test]
fn door32_tolerates_empty_file() {
    let path = write_tmp("DOOR32.SYS", "");
    let info = parse(&path).expect("should parse empty");
    assert_eq!(info, DropfileInfo::default());
}

#[test]
fn door32_case_insensitive_filename() {
    // Lowercase filename should still be recognised.
    let path = write_tmp("door32.sys", door32_content());
    let info = parse(&path).expect("should parse");
    assert_eq!(info.alias.as_deref(), Some("Starlord"));
}

#[test]
fn door32_preserves_unknown_connection_types() {
    let content = door32_content().replacen("2\r\n", "9\r\n", 1);
    let path = write_tmp("DOOR32.SYS", &content);
    let info = parse(&path).expect("should parse");

    assert_eq!(info.connection_type, Some(DoorConnectionType::Unknown(9)));
    assert_eq!(info.socket_descriptor, Some(4));
}

// ---------------------------------------------------------------------------
// DOOR.SYS
// ---------------------------------------------------------------------------

fn door_sys_content() -> String {
    // Build a ~21-line DOOR.SYS; lines 10=real name, 19=minutes, 20=ANSI, 21=rows.
    let lines: Vec<&str> = vec![
        "LOCAL:",     // 1  COM port
        "38400",      // 2  baud
        "8",          // 3  parity bits
        "1",          // 4  node number
        "Y",          // 5  BPS locked
        "1",          // 6  screen mode (color)
        "N",          // 7  printer
        "Y",          // 8  page bell
        "N",          // 9  caller alarm
        "Tachyon",    // 10 real name / handle ← alias
        "Cyberspace", // 11 location
        "555-1234",   // 12 phone home
        "555-5678",   // 13 phone data
        "",           // 14 password (blank)
        "50",         // 15 security level
        "42",         // 16 times on BBS
        "03/25/26",   // 17 last date on
        "2700",       // 18 seconds remaining
        "45",         // 19 minutes remaining ← timeout_minutes
        "Y",          // 20 ANSI flag
        "25",         // 21 screen rows
    ];
    lines.iter().map(|l| format!("{l}\r\n")).collect()
}

#[test]
fn door_sys_extracts_alias_and_timeout() {
    let path = write_tmp("DOOR.SYS", &door_sys_content());
    let info = parse(&path).expect("should parse");
    assert_eq!(info.alias.as_deref(), Some("Tachyon"));
    assert_eq!(info.timeout_minutes, Some(45));
}

#[test]
fn door_sys_accepts_lf_endings() {
    let content = door_sys_content().replace("\r\n", "\n");
    let path = write_tmp("DOOR.SYS", &content);
    let info = parse(&path).expect("should parse");
    assert_eq!(info.alias.as_deref(), Some("Tachyon"));
    assert_eq!(info.timeout_minutes, Some(45));
}

#[test]
fn door_sys_tolerates_short_file_missing_timeout() {
    // Only first 10 lines — alias present but timeout (line 19) is absent.
    let content: String = door_sys_content()
        .lines()
        .take(10)
        .map(|l| format!("{l}\r\n"))
        .collect();
    let path = write_tmp("DOOR.SYS", &content);
    let info = parse(&path).expect("should parse");
    assert_eq!(info.alias.as_deref(), Some("Tachyon"));
    assert_eq!(info.timeout_minutes, None);
}

#[test]
fn door_sys_case_insensitive_filename() {
    let path = write_tmp("door.sys", &door_sys_content());
    let info = parse(&path).expect("should parse");
    assert_eq!(info.alias.as_deref(), Some("Tachyon"));
}

// ---------------------------------------------------------------------------
// CHAIN.TXT
// ---------------------------------------------------------------------------

fn chain_txt_content() -> String {
    // Build a 32-line CHAIN.TXT.
    let fields: &[&str] = &[
        "7",                 // 1  user number
        "NebulaKing",        // 2  alias ← alias
        "Alice Vega",        // 3  real name
        "KD9XYZ",            // 4  callsign
        "34",                // 5  age
        "F",                 // 6  sex
        "1000.0",            // 7  gold
        "03/25/26",          // 8  last logon
        "80",                // 9  columns
        "25",                // 10 rows
        "100",               // 11 security level
        "0",                 // 12 co-sysop
        "0",                 // 13 sysop
        "1",                 // 14 ANSI
        "1",                 // 15 remote
        "2700",              // 16 seconds left (45 min) ← timeout_minutes
        "C:\\BBS\\GFILES\\", // 17
        "C:\\BBS\\DATA\\",   // 18
        "BBS.LOG",           // 19
        "38400",             // 20 user baud
        "1",                 // 21
        "1",                 // 22
        "1",                 // 23
        "0",                 // 24
        "0",                 // 25
        "0",                 // 26
        "0",                 // 27
        "0",                 // 28
        "0",                 // 29
        "0",                 // 30
        "0",                 // 31
        "0",                 // 32
    ];
    assert_eq!(fields.len(), 32, "CHAIN.TXT must have exactly 32 lines");
    fields.iter().map(|f| format!("{f}\r\n")).collect()
}

#[test]
fn chain_txt_extracts_alias_and_timeout() {
    let path = write_tmp("CHAIN.TXT", &chain_txt_content());
    let info = parse(&path).expect("should parse");
    assert_eq!(info.alias.as_deref(), Some("NebulaKing"));
    // 2700 seconds → 45 minutes
    assert_eq!(info.timeout_minutes, Some(45));
}

#[test]
fn chain_txt_seconds_rounded_down_to_minutes() {
    // 2750 seconds → 45 minutes (floor division)
    let content = chain_txt_content().replacen("2700\r\n", "2750\r\n", 1);
    let path = write_tmp("CHAIN.TXT", &content);
    let info = parse(&path).expect("should parse");
    assert_eq!(info.timeout_minutes, Some(45));
}

#[test]
fn chain_txt_accepts_lf_endings() {
    let content = chain_txt_content().replace("\r\n", "\n");
    let path = write_tmp("CHAIN.TXT", &content);
    let info = parse(&path).expect("should parse");
    assert_eq!(info.alias.as_deref(), Some("NebulaKing"));
    assert_eq!(info.timeout_minutes, Some(45));
}

#[test]
fn chain_txt_tolerates_short_file() {
    // Only first 2 lines — alias present, seconds (line 16) absent.
    let content = "7\r\nNebulaKing\r\n";
    let path = write_tmp("CHAIN.TXT", content);
    let info = parse(&path).expect("should parse");
    assert_eq!(info.alias.as_deref(), Some("NebulaKing"));
    assert_eq!(info.timeout_minutes, None);
}

#[test]
fn chain_txt_case_insensitive_filename() {
    let path = write_tmp("chain.txt", &chain_txt_content());
    let info = parse(&path).expect("should parse");
    assert_eq!(info.alias.as_deref(), Some("NebulaKing"));
}

// ---------------------------------------------------------------------------
// Unknown filename
// ---------------------------------------------------------------------------

#[test]
fn unknown_filename_returns_error() {
    let path = write_tmp("LOGIN.DAT", "some content\n");
    let err = parse(&path).expect_err("should fail for unknown filename");
    assert!(
        matches!(err, DropfileError::UnknownFormat(_)),
        "expected UnknownFormat, got: {err}"
    );
}

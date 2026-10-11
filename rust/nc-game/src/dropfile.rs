//! BBS dropfile parser.
//!
//! Supports three formats, auto-detected by filename (case-insensitive):
//!
//! - **DOOR32.SYS** — modern standard (Enigma, Mystic, Talisman, etc.)
//! - **DOOR.SYS**   — legacy, widest BBS software support
//! - **CHAIN.TXT**  — WWIV format
//!
//! All parsers are lenient:
//! - UTF-8 files are read as-is; anything else is decoded as CP437.
//! - Both CRLF and LF line endings are accepted.
//! - Leading and trailing whitespace on lines is stripped.
//! - Short files are tolerated; missing fields are `None`.
//! - Filenames are matched case-insensitively.
//!
//! Only the fields that `nc-game` actually uses are extracted:
//! - `alias` — player handle/alias for matching a game record
//! - `fallback_alias` — DOOR.SYS real name, tried when `alias` binds no seat
//! - `alias_is_real_name` — `alias` is the DOOR.SYS real name, not a handle
//! - `timeout_minutes` — session time limit sourced from the dropfile

use std::path::Path;

use crate::terminal::cp437::cp437_to_string;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// The fields extracted from a BBS dropfile that are relevant to `nc-game`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DropfileInfo {
    /// Player handle or alias (used to match a game player record).
    pub alias: Option<String>,
    /// DOOR.SYS real name (line 10) when the handle came from line 36. Seat
    /// binding tries it after `alias`, so seats bound under the real name
    /// before the handle was read keep working.
    pub fallback_alias: Option<String>,
    /// True when `alias` is the DOOR.SYS real name because line 36 carried no
    /// handle.
    pub alias_is_real_name: bool,
    /// Session time limit in minutes sourced from the dropfile.
    pub timeout_minutes: Option<u32>,
    /// Terminal width reported by the BBS, when the format provides it.
    pub screen_columns: Option<usize>,
    /// Terminal height reported by the BBS, when the format provides it.
    pub screen_rows: Option<usize>,
    /// Transport type reported by `DOOR32.SYS`, when present.
    pub connection_type: Option<DoorConnectionType>,
    /// Socket descriptor/handle reported by `DOOR32.SYS`, when present.
    pub socket_descriptor: Option<u64>,
}

/// Transport type encoded in `DOOR32.SYS`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoorConnectionType {
    Local,
    Serial,
    TelnetSocket,
    Unknown(u32),
}

impl DoorConnectionType {
    fn from_door32(value: u32) -> Self {
        match value {
            0 => Self::Local,
            1 => Self::Serial,
            2 => Self::TelnetSocket,
            other => Self::Unknown(other),
        }
    }
}

/// Errors that can occur while parsing a dropfile.
#[derive(Debug)]
pub enum DropfileError {
    /// The file could not be read.
    Io(std::io::Error),
    /// The filename is not one of the recognised dropfile names.
    UnknownFormat(String),
}

impl std::fmt::Display for DropfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DropfileError::Io(e) => write!(f, "dropfile I/O error: {e}"),
            DropfileError::UnknownFormat(name) => write!(
                f,
                "unrecognised dropfile name '{name}'; \
                 expected DOOR32.SYS, DOOR.SYS, or CHAIN.TXT"
            ),
        }
    }
}

impl std::error::Error for DropfileError {}

impl From<std::io::Error> for DropfileError {
    fn from(e: std::io::Error) -> Self {
        DropfileError::Io(e)
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Parse a BBS dropfile at `path`.
///
/// The format is auto-detected from the filename (case-insensitive).
/// Returns [`DropfileError::UnknownFormat`] for unrecognised filenames.
pub fn parse(path: &Path) -> Result<DropfileInfo, DropfileError> {
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_uppercase();

    // BBS software usually writes names in CP437; accept UTF-8 files as-is.
    let content = match String::from_utf8(std::fs::read(path)?) {
        Ok(text) => text,
        Err(err) => cp437_to_string(err.as_bytes()),
    };
    let lines = split_lines(&content);

    match filename.as_str() {
        "DOOR32.SYS" => Ok(parse_door32(&lines)),
        "DOOR.SYS" => Ok(parse_door_sys(&lines)),
        "CHAIN.TXT" => Ok(parse_chain_txt(&lines)),
        other => Err(DropfileError::UnknownFormat(other.to_string())),
    }
}

// ---------------------------------------------------------------------------
// Line splitting — strips CRLF and surrounding whitespace
// ---------------------------------------------------------------------------

fn split_lines(content: &str) -> Vec<&str> {
    content.split('\n').map(str::trim).collect()
}

/// Return the trimmed content of line `n` (1-based), or `None` if the file is
/// shorter than `n` lines or the line is empty after trimming.
fn line<'a>(lines: &'a [&'a str], n: usize) -> Option<&'a str> {
    lines
        .get(n.saturating_sub(1))
        .copied()
        .filter(|s| !s.is_empty())
}

/// Parse a positive integer from a line, returning `None` on any failure.
fn parse_u32(lines: &[&str], n: usize) -> Option<u32> {
    line(lines, n)?.parse().ok()
}

/// Parse an unsigned integer from a line, returning `None` on any failure.
fn parse_u64(lines: &[&str], n: usize) -> Option<u64> {
    line(lines, n)?.parse().ok()
}

// ---------------------------------------------------------------------------
// DOOR32.SYS
// ---------------------------------------------------------------------------
//
// Line layout (1-based):
//   1  Comm type (0=local, 1=serial, 2=telnet/socket)
//   2  Comm or socket handle
//   3  Baud rate
//   4  BBS software name
//   5  User record position (1-based)
//   6  User's real name
//   7  User's handle/alias          ← alias
//   8  Security level
//   9  Time remaining (minutes)     ← timeout_minutes
//  10  ANSI (1=yes, 0=no)
//  11  Screen columns
//  12  Screen rows

fn parse_door32(lines: &[&str]) -> DropfileInfo {
    DropfileInfo {
        alias: line(lines, 7).map(ToOwned::to_owned),
        fallback_alias: None,
        alias_is_real_name: false,
        timeout_minutes: parse_u32(lines, 9),
        screen_columns: parse_u32(lines, 11).map(|value| value as usize),
        screen_rows: parse_u32(lines, 12).map(|value| value as usize),
        connection_type: parse_u32(lines, 1).map(DoorConnectionType::from_door32),
        socket_descriptor: parse_u64(lines, 2),
    }
}

// ---------------------------------------------------------------------------
// DOOR.SYS
// ---------------------------------------------------------------------------
//
// Line layout (1-based, ~52 lines total):
//   1  COM port ("COM1:", "LOCAL:", etc.)
//   2  Baud rate
//   ...
//  10  User's real name
//  11  User's location/city
//  ...
//  19  Minutes remaining            ← timeout_minutes
//  20  ANSI flag (Y/N/G)
//  21  Screen rows
//  ...
//  36  User's handle/alias          ← alias
//  ...
//
// Short DOOR.SYS files stop before line 36, and some BBS software leaves it
// blank; the real name on line 10 is the alias then.

fn parse_door_sys(lines: &[&str]) -> DropfileInfo {
    let real_name = line(lines, 10).map(ToOwned::to_owned);
    let (alias, fallback_alias, alias_is_real_name) = match line(lines, 36) {
        Some(handle) => (Some(handle.to_owned()), real_name, false),
        None => (real_name, None, true),
    };
    DropfileInfo {
        alias,
        fallback_alias,
        alias_is_real_name,
        timeout_minutes: parse_u32(lines, 19),
        screen_columns: None,
        screen_rows: parse_u32(lines, 21).map(|value| value as usize),
        connection_type: None,
        socket_descriptor: None,
    }
}

// ---------------------------------------------------------------------------
// CHAIN.TXT (WWIV)
// ---------------------------------------------------------------------------
//
// Line layout (1-based, exactly 32 lines in strict WWIV):
//   1  User number (1-based)
//   2  Alias/handle                 ← alias
//   3  Real name
//   4  Callsign
//   5  Age
//   6  Sex (M/F)
//   7  Gold/points
//   8  Last logon date
//   9  Screen columns
//  10  Screen rows
//  11  Security level
//  12  Co-sysop flag
//  13  Sysop flag
//  14  ANSI flag
//  15  Remote flag (0=local, 1=remote)
//  16  Seconds left                 ← converted to minutes (rounded down);
//      WWIV writes it as a float, e.g. `   3600.00`
//  17–32  Various BBS system fields

fn parse_chain_txt(lines: &[&str]) -> DropfileInfo {
    let timeout_minutes = line(lines, 16)
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|secs| secs.is_finite() && *secs >= 0.0)
        .map(|secs| (secs / 60.0) as u32);
    DropfileInfo {
        alias: line(lines, 2).map(ToOwned::to_owned),
        fallback_alias: None,
        alias_is_real_name: false,
        timeout_minutes,
        screen_columns: parse_u32(lines, 9).map(|value| value as usize),
        screen_rows: parse_u32(lines, 10).map(|value| value as usize),
        connection_type: None,
        socket_descriptor: None,
    }
}

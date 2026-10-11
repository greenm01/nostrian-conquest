//! Parsers for legacy BBS dropfile formats (DOOR32.SYS, DOOR.SYS, CHAIN.TXT).

use crate::{DoorConnectionType, Dropfile, DropfileFormat, InputDevice};

/// Split content by newline and trim whitespace.
pub fn split_lines(content: &str) -> Vec<&str> {
    content.split('\n').map(str::trim).collect()
}

/// Return the trimmed content of line `n` (1-based), or `None` if the line is empty.
pub fn line<'a>(lines: &'a [&'a str], n: usize) -> Option<&'a str> {
    lines
        .get(n.saturating_sub(1))
        .copied()
        .filter(|s| !s.is_empty())
}

/// Parse an unsigned 32-bit integer from line `n`.
pub fn parse_u32(lines: &[&str], n: usize) -> Option<u32> {
    line(lines, n)?.parse().ok()
}

/// Parse an unsigned 64-bit integer from line `n`.
pub fn parse_u64(lines: &[&str], n: usize) -> Option<u64> {
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

pub fn parse_door32(lines: &[&str]) -> Dropfile {
    let connection_type = parse_u32(lines, 1).map(|v| match v {
        0 => DoorConnectionType::Local,
        1 => DoorConnectionType::Serial,
        2 => DoorConnectionType::TelnetSocket,
        other => DoorConnectionType::Unknown(other),
    });

    Dropfile {
        format: DropfileFormat::Door32,
        alias: line(lines, 7).map(ToOwned::to_owned),
        fallback_alias: line(lines, 6).map(ToOwned::to_owned),
        alias_is_real_name: false,
        timeout_minutes: parse_u32(lines, 9),
        screen_columns: parse_u32(lines, 11).map(|v| v as usize),
        screen_rows: parse_u32(lines, 12).map(|v| v as usize),
        connection_type,
        socket_descriptor: parse_u64(lines, 2),
        event_webhook: None,
        callback_secret: None,
        input_device: InputDevice::Keyboard,
        encoding: "cp437".to_string(),
        platform_id: None,
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

pub fn parse_door_sys(lines: &[&str]) -> Dropfile {
    let real_name = line(lines, 10).map(ToOwned::to_owned);
    let (alias, fallback_alias, alias_is_real_name) = match line(lines, 36) {
        Some(handle) => (Some(handle.to_owned()), real_name, false),
        None => (real_name, None, true),
    };

    Dropfile {
        format: DropfileFormat::DoorSys,
        alias,
        fallback_alias,
        alias_is_real_name,
        timeout_minutes: parse_u32(lines, 19),
        screen_columns: None,
        screen_rows: parse_u32(lines, 21).map(|v| v as usize),
        connection_type: None,
        socket_descriptor: None,
        event_webhook: None,
        callback_secret: None,
        input_device: InputDevice::Keyboard,
        encoding: "cp437".to_string(),
        platform_id: None,
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
//   ...
//   9  Screen columns
//  10  Screen rows
//  ...
//  16  Time remaining (seconds)

pub fn parse_chain_txt(lines: &[&str]) -> Dropfile {
    let timeout_seconds = parse_u32(lines, 16);
    let timeout_minutes = timeout_seconds.map(|s| s / 60);

    Dropfile {
        format: DropfileFormat::ChainTxt,
        alias: line(lines, 2).map(ToOwned::to_owned),
        fallback_alias: line(lines, 3).map(ToOwned::to_owned),
        alias_is_real_name: false,
        timeout_minutes,
        screen_columns: parse_u32(lines, 9).map(|v| v as usize),
        screen_rows: parse_u32(lines, 10).map(|v| v as usize),
        connection_type: None,
        socket_descriptor: None,
        event_webhook: None,
        callback_secret: None,
        input_device: InputDevice::Keyboard,
        encoding: "cp437".to_string(),
        platform_id: None,
    }
}

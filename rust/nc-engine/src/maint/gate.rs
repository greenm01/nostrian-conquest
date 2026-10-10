//! Schedule/token gate for the maintenance engine.
//!
//! Classic ECMAINT checks `CONQUEST.DAT[0x03..0x09]` (7 day-of-week enable
//! flags) against the current day before running maintenance.  It also
//! coordinates concurrent runs via a `Main.Tok` token file.
//!
//! This module provides the Rust equivalent helpers that can be wired into the
//! CLI layer.  The gate check is advisory — callers may bypass it with a
//! `--no-gate` flag for oracle testing or sysop overrides.

use std::path::Path;

use crate::{CONQUEST_DAT_SIZE, ConquestDat};

/// Result of a maintenance schedule gate check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateResult {
    /// Maintenance is allowed to run today.
    Allowed,
    /// Maintenance is not scheduled for this day.
    NotScheduled { day_name: &'static str },
    /// Another maintenance process holds the token file.
    TokenBusy,
}

/// Day-of-week index (0 = Sunday … 6 = Saturday), matching `time_t` `tm_wday`.
fn today_weekday() -> u8 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Days since epoch (Thu = 4).  (secs / 86400 + 4) % 7 gives 0 = Sun.
    ((secs / 86400 + 4) % 7) as u8
}

const DAY_NAMES: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

/// Check whether maintenance is scheduled for today according to `CONQUEST.DAT`.
///
/// `conquest.maintenance_schedule_enabled()` returns a 7-element array indexed
/// `[Sun, Mon, Tue, Wed, Thu, Fri, Sat]`.
pub fn check_maintenance_schedule(conquest: &ConquestDat) -> GateResult {
    let schedule = conquest.maintenance_schedule_enabled();
    let today = today_weekday() as usize;
    if schedule[today] {
        GateResult::Allowed
    } else {
        GateResult::NotScheduled {
            day_name: DAY_NAMES[today],
        }
    }
}

/// Check whether a maintenance token file is already held by another process.
pub fn check_token_files(dir: &Path) -> bool {
    dir.join("Main.Tok").exists()
}

/// Create the maintenance coordination token file.
pub fn create_maintenance_token(dir: &Path) -> std::io::Result<()> {
    std::fs::write(dir.join("Main.Tok"), b"")
}

/// Remove the maintenance coordination token file after a successful run.
pub fn remove_maintenance_token(dir: &Path) {
    let _ = std::fs::remove_file(dir.join("Main.Tok"));
}

/// Parse a raw CONQUEST.DAT byte slice into a `ConquestDat` for gate checks.
/// Returns an error if the data is the wrong size.
pub fn check_gate_conquest(raw: &[u8]) -> Result<ConquestDat, &'static str> {
    if raw.len() < CONQUEST_DAT_SIZE {
        return Err("CONQUEST.DAT too small for gate check");
    }
    let mut buf = [0u8; CONQUEST_DAT_SIZE];
    buf.copy_from_slice(&raw[..CONQUEST_DAT_SIZE]);
    Ok(ConquestDat::from_raw(buf))
}

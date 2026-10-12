use super::*;
use chrono::{TimeZone, Timelike};
use chrono_tz::Tz;
use nc_data::{CampaignStore, hosted};
use rusqlite::{Connection, OptionalExtension, params};

pub(super) fn option(conn: &Connection, key: &str) -> Result<Option<String>> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='web_options')",
        [],
        |r| r.get(0),
    )?;
    if !exists {
        return Ok(None);
    }
    Ok(conn
        .query_row("SELECT value FROM web_options WHERE name=?1", [key], |r| {
            r.get(0)
        })
        .optional()?)
}
pub(super) fn set_option(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS web_options(name TEXT PRIMARY KEY,value TEXT NOT NULL)",
    )?;
    conn.execute("INSERT INTO web_options VALUES (?1,?2) ON CONFLICT(name) DO UPDATE SET value=excluded.value",params![key,value])?;
    Ok(())
}
pub(super) fn description(conn: &Connection) -> Result<Value> {
    Ok(match option(conn, "schedule")?.as_deref() {
        Some("manual") => json!({"mode":"manual"}),
        Some(zone) => json!({"mode":"daily_midnight","time_zone":zone}),
        None => json!({"mode":"interval"}),
    })
}
pub(super) fn manual(conn: &Connection) -> Result<bool> {
    Ok(option(conn, "schedule")?.as_deref() == Some("manual"))
}
/// First local midnight strictly after the given Unix timestamp.
pub fn next_local_midnight(zone: &str, after: i64) -> Result<i64> {
    let tz: Tz = zone.parse().map_err(|_| "unknown IANA time zone")?;
    let local = tz
        .timestamp_opt(after, 0)
        .single()
        .ok_or("invalid timestamp")?;
    let date = local.date_naive().succ_opt().ok_or("date overflow")?;
    Ok(tz
        .from_local_datetime(&date.and_hms_opt(0, 0, 0).unwrap())
        .earliest()
        .ok_or("next local midnight does not exist in this zone")?
        .timestamp())
}
pub(super) fn next(conn: &Connection, after: i64, interval: u32) -> Result<Option<i64>> {
    match option(conn, "schedule")?.as_deref() {
        Some("manual") => Ok(None),
        Some(zone) => Ok(Some(next_local_midnight(zone, after)?)),
        None => Ok(Some(after + i64::from(interval) * 60)),
    }
}
pub(super) fn validate_due(conn: &Connection, due: Option<i64>) -> Result<()> {
    if manual(conn)? {
        if due.is_some() {
            return Err("manual-turn games have no deadline".into());
        }
        return Ok(());
    }
    let due = due
        .filter(|d| *d > now())
        .ok_or("first deadline must be in the future")?;
    if let Some(zone) = option(conn, "schedule")? {
        let tz: Tz = zone.parse().map_err(|_| "unknown time zone")?;
        let t = tz
            .timestamp_opt(due, 0)
            .single()
            .ok_or("invalid timestamp")?;
        if t.hour() != 0 || t.minute() != 0 || t.second() != 0 {
            return Err("deadline must be midnight in the configured time zone".into());
        }
    }
    Ok(())
}
pub(super) fn configure(root: &Path, game: &str, zone: Option<&str>) -> Result<Value> {
    let (dir, store) = store(root, game)?;
    let meta = hosted::get_game_metadata(store.connection(), game)?;
    if !matches!(
        meta.status.as_str(),
        "draft" | "recruiting" | "running" | "paused"
    ) {
        return Err("cannot configure this campaign state".into());
    }
    let next = zone.map(|z| next_local_midnight(z, now())).transpose()?;
    let runtime = CampaignStore::open_default_in_dir(&dir)?;
    let blocked = runtime.hosting_writes_blocked()?;
    runtime.set_hosting_write_blocked(true)?;
    let result = (|| -> Result<()> {
        let tx = store.connection().unchecked_transaction()?;
        set_option(&tx, "schedule", zone.unwrap_or("manual"))?;
        let due = if meta.status == "running" { next } else { None };
        tx.execute("UPDATE game_metadata SET maintenance_enabled=?1,maintenance_next_due_unix_seconds=?2 WHERE id=?3",params![due.is_some(),due,game])?;
        tx.execute(
            "INSERT INTO web_history(at,operation,detail) VALUES (?1,'schedule_configured',?2)",
            params![now(), json!({"time_zone":zone}).to_string()],
        )?;
        tx.commit()?;
        Ok(())
    })();
    runtime.set_hosting_write_blocked(blocked)?;
    result?;
    campaign::inspect(root, game)
}
pub(super) fn start(root: &Path, game: &str) -> Result<Value> {
    let (_, store) = store(root, game)?;
    if option(store.connection(), "schedule")?.is_none() {
        return Err("configure midnight or manual turns, or specify --first-due".into());
    }
    let due = next(store.connection(), now(), 1440)?;
    campaign::transition(root, game, "running", due, Some(1440))
}

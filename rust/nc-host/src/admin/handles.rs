//! A seat's player handle comes from its claimant's public name and is locked
//! for that game, so signing in later under another name never renames it.
//! Only the operator renames a locked handle.
use super::*;
use nc_data::hosted;
use rusqlite::{Connection, params};

pub(super) fn ensure(conn: &Connection) -> Result<()> {
    conn.execute_batch("CREATE TABLE IF NOT EXISTS web_handle_locks(seat INTEGER PRIMARY KEY)")?;
    Ok(())
}

pub(super) fn unlock(conn: &Connection, seat: u32) -> Result<()> {
    ensure(conn)?;
    conn.execute("DELETE FROM web_handle_locks WHERE seat=?1", [seat])?;
    Ok(())
}

/// Names are labels only; control characters are dropped and the game's own
/// handle limit applies when stored.
fn clean(name: &str) -> Result<String> {
    let name: String = name.chars().filter(|c| !c.is_control()).collect();
    let name = name.trim();
    if name.is_empty() || name.len() > 512 {
        return Err("invalid player name".into());
    }
    Ok(name.to_owned())
}

/// Stores `name` as the seat's handle when `force` is set or the seat has no
/// locked handle yet, then locks it. Returns the seat's handle.
pub(super) fn lock(root: &Path, game: &str, seat: u32, name: &str, force: bool) -> Result<String> {
    let name = clean(name)?;
    let (dir, store) = store(root, game)?;
    ensure(store.connection())?;
    let locked: bool = store.connection().query_row(
        "SELECT EXISTS(SELECT 1 FROM web_handle_locks WHERE seat=?1)",
        [seat],
        |r| r.get(0),
    )?;
    let runtime = nc_data::CampaignStore::open_default_in_dir(&dir)?;
    let mut snapshot = runtime
        .load_latest_runtime_state()?
        .ok_or("missing state")?;
    let player = snapshot
        .game_data
        .players
        .get_mut(seat as usize - 1)
        .ok_or("missing player")?;
    if locked && !force {
        return Ok(player.handle.clone());
    }
    let old = player.handle.clone();
    nc_data::set_player_handle(player, &name);
    let handle = player.handle.clone();
    if handle != old {
        runtime
            .expecting_snapshot(snapshot.snapshot_id)
            .save_runtime_state_structured(
                snapshot.game_year,
                &snapshot.game_data,
                &snapshot.report_block_rows,
                &snapshot.queued_mail,
            )?;
    }
    store.connection().execute(
        "INSERT OR IGNORE INTO web_handle_locks(seat) VALUES (?1)",
        [seat],
    )?;
    Ok(handle)
}

/// Called when a player opens the terminal: an unlocked seat (claimed before
/// handles were locked, or reassigned) takes this player's name once.
pub(super) fn on_play(root: &Path, game: &str, id: &AccountIdentity, name: &str) -> Result<Value> {
    let access = seats::authorize(root, game, id)?;
    if access["can_play"] != true {
        return Err("campaign is not accepting sessions".into());
    }
    let seat = access["seat"].as_u64().ok_or("missing seat")? as u32;
    let handle = lock(root, game, seat, name, false)?;
    Ok(json!({"seat":seat,"handle":handle}))
}

/// Operator rename; the new handle stays locked.
pub(super) fn rename(root: &Path, game: &str, seat: u32, name: &str) -> Result<Value> {
    let (_, store) = store(root, game)?;
    let meta = hosted::get_game_metadata(store.connection(), game)?;
    if !matches!(meta.status.as_str(), "recruiting" | "running" | "paused") {
        return Err("renames require a recruiting, running or paused game".into());
    }
    if seat == 0 || seat > meta.players {
        return Err("invalid seat".into());
    }
    let handle = lock(root, game, seat, name, true)?;
    store.connection().execute(
        "INSERT INTO web_history(at,operation,detail) VALUES (?1,'rename',?2)",
        params![now(), json!({"seat":seat,"handle":handle}).to_string()],
    )?;
    Ok(json!({"game":game,"seat":seat,"handle":handle}))
}

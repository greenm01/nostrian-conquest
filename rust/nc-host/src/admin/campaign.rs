use super::*;
use nc_data::{CampaignStore, hosted};
use rusqlite::params;

pub(super) fn list(root: &Path) -> Result<Value> {
    let mut rows = Vec::new();
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() || !entry.path().join("hosted.db").is_file() {
            continue;
        }
        let Some(game) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        match inspect(root, &game) {
            Ok(row) => rows.push(row),
            Err(e) => rows.push(json!({"game":game,"error":e.to_string()})),
        }
    }
    rows.sort_by_key(|row| row["game"].as_str().unwrap_or("").to_owned());
    Ok(json!(rows))
}

pub(super) fn inspect(root: &Path, game: &str) -> Result<Value> {
    let (dir, store) = store(root, game)?;
    let meta = hosted::get_game_metadata(store.connection(), game)?;
    let settings = hosted::get_settings(store.connection(), game)?;
    let runtime = CampaignStore::open_default_in_dir(&dir)?;
    let (_, year) = runtime
        .latest_snapshot_metadata()?
        .ok_or("missing campaign snapshot")?;
    let mut stmt = store
        .connection()
        .prepare("SELECT seat,account FROM web_seats ORDER BY seat,account")?;
    let mut grouped = std::collections::BTreeMap::<u32, Vec<AccountIdentity>>::new();
    for row in stmt.query_map([], |r| {
        Ok((r.get::<_, u32>(0)?, r.get::<_, AccountIdentity>(1)?))
    })? {
        let (seat, account) = row?;
        grouped.entry(seat).or_default().push(account);
    }
    let seats: Vec<_> = grouped
        .into_iter()
        .map(|(seat, accounts)| json!({"seat":seat,"accounts":accounts}))
        .collect();
    let mut runs = store.connection().prepare("SELECT request_id,expected_year,state FROM web_runs WHERE state!='done' ORDER BY started_at DESC")?;
    let unfinished=runs.query_map([],|r|Ok(json!({"request_id":r.get::<_,String>(0)?,"expected_year":r.get::<_,u16>(1)?,"state":r.get::<_,String>(2)?})))?.collect::<std::result::Result<Vec<_>,_>>()?;
    let last_backup: Option<String> =
        store
            .connection()
            .query_row("SELECT MAX(path) FROM web_backups", [], |r| r.get(0))?;
    Ok(
        json!({"game":game,"name":meta.name,"state":meta.status,"year":year,
        "players":meta.players,"claimed":seats.len(),"open":meta.players as usize-seats.len(),
        "seats":seats,"next_due":settings.maintenance_next_due_unix_seconds,
        "interval_minutes":settings.maintenance_interval_minutes,"last_backup":last_backup,
        "writes_blocked":runtime.hosting_writes_blocked()?,"unfinished_runs":unfinished,
        "invite_required":invites::required(store.connection())?,"schedule":scheduling::description(store.connection())?}),
    )
}

pub(super) fn create(
    root: &Path,
    game: &str,
    name: &str,
    players: u8,
    seed: Option<u64>,
) -> Result<Value> {
    if !nc_data::is_valid_player_count(players) || name.trim().is_empty() || name.len() > 120 {
        return Err("provide 1-25 players and a name of 1-120 bytes".into());
    }
    let dir = game_dir(root, game)?;
    std::fs::create_dir(&dir)?; // Never overwrite or adopt an existing campaign.
    let result = (|| -> Result<()> {
        let store = HostedStore::create(&dir.join("hosted.db"))?;
        store.connection().execute_batch(
            "CREATE TABLE web_seats(seat INTEGER NOT NULL, account TEXT PRIMARY KEY NOT NULL);
             CREATE TABLE web_runs(request_id TEXT PRIMARY KEY, expected_year INTEGER NOT NULL, state TEXT NOT NULL,
                 result TEXT, started_at INTEGER NOT NULL);
             CREATE TABLE web_history(id INTEGER PRIMARY KEY, at INTEGER NOT NULL, operation TEXT NOT NULL, detail TEXT NOT NULL);
             CREATE TABLE web_backups(path TEXT PRIMARY KEY, at INTEGER NOT NULL, kind TEXT NOT NULL);
             CREATE TABLE web_announcements(id TEXT PRIMARY KEY, payload TEXT NOT NULL);"
        )?;
        store.connection().execute(
            "INSERT INTO game_metadata(id,name,status,created_at,updated_at,players,maintenance_enabled,lobby_visibility)
             VALUES (?1,?2,'draft',?3,?3,?4,0,'private')", params![game,name,now(),players],
        )?;
        let codes = (0..players)
            .map(|i| format!("web-reserved-{i}"))
            .collect::<Vec<_>>();
        hosted::create_seats(store.connection(), game, &codes)?;
        crate::game::state::initialize_runtime_state(
            &dir,
            game,
            name,
            players,
            3000,
            seed.unwrap_or_else(nc_data::generate_campaign_seed),
        )
        .map_err(|e| e.to_string())?;
        CampaignStore::open_default_in_dir(&dir)?.set_hosting_write_blocked(true)?;
        record(&store, "create", &json!({"name":name,"players":players}))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&dir);
    }
    result?;
    inspect(root, game)
}

pub(super) fn transition(
    root: &Path,
    game: &str,
    to: &str,
    due: Option<i64>,
    interval: Option<u32>,
) -> Result<Value> {
    let (dir, store) = store(root, game)?;
    let from = hosted::get_game_metadata(store.connection(), game)?.status;
    let allowed = matches!(
        (from.as_str(), to),
        ("draft", "recruiting")
            | ("recruiting", "running")
            | ("running", "running")
            | ("running", "paused")
            | ("recruiting", "paused")
            | ("paused", "paused")
            | ("paused", "running")
            | ("paused", "archived")
            | ("draft", "archived")
            | ("recruiting", "archived")
    );
    if !allowed {
        return Err(format!("cannot change {from} to {to}").into());
    }
    if to == "running" {
        scheduling::validate_due(store.connection(), due)?;
    }
    if interval.is_some_and(|n| n == 0 || n > 525600) {
        return Err("interval must be 1-525600 minutes".into());
    }
    let runtime = CampaignStore::open_default_in_dir(&dir)?;
    runtime.set_hosting_write_blocked(true)?;
    let conn = store.connection();
    let tx = nc_data::sqlite::write_transaction(&conn)?;
    tx.execute("UPDATE game_metadata SET status=?1,maintenance_enabled=?2,
        maintenance_next_due_unix_seconds=?3,maintenance_interval_minutes=COALESCE(?4,maintenance_interval_minutes),
        recruiting=?5,lobby_visibility=?6,updated_at=?7 WHERE id=?8",
        params![to,to=="running" && due.is_some(),due,interval,if to=="recruiting" {"new_players"} else {"none"},
        if to=="archived" {"private"} else {"public"},now(),game])?;
    tx.execute(
        "INSERT INTO web_history(at,operation,detail) VALUES (?1,?2,?3)",
        params![now(), to, json!({"from":from,"next_due":due}).to_string()],
    )?;
    if to == "recruiting" {
        let meta = hosted::get_game_metadata(&tx, game)?;
        announcements::enqueue(
            &tx,
            "recruiting",
            &json!({"game":game,"name":meta.name,"kind":"recruiting","players":meta.players,"invite_required":invites::required(&tx)?}),
        )?;
    }
    tx.commit()?;
    runtime.set_hosting_write_blocked(!matches!(to, "running" | "recruiting"))?;
    inspect(root, game)
}

pub(super) fn record(store: &HostedStore, operation: &str, detail: &Value) -> Result<()> {
    store.connection().execute(
        "INSERT INTO web_history(at,operation,detail) VALUES (?1,?2,?3)",
        params![now(), operation, detail.to_string()],
    )?;
    Ok(())
}

/// Public fields only; never expose account bindings or local backup paths.
pub(super) fn catalog(root: &Path) -> Result<Value> {
    let rows = list(root)?;
    Ok(Value::Array(rows.as_array().ok_or("invalid catalog")?.iter()
        .filter(|row| matches!(row["state"].as_str(), Some("recruiting" | "running" | "paused")))
        .map(|row| json!({"game":row["game"],"name":row["name"],"state":row["state"],"year":row["year"],
            "open":row["open"],"players":row["players"],"next_due":row["next_due"],"interval_minutes":row["interval_minutes"],"invite_required":row["invite_required"],"schedule":row["schedule"]}))
        .collect()))
}

use super::*;
use nc_data::{CampaignStore, hosted};
use rusqlite::{Connection, DatabaseName, OptionalExtension, params};

pub(super) fn backup(root: &Path, game: &str) -> Result<Value> {
    let (dir, store) = store(root, game)?;
    let runtime = CampaignStore::open_default_in_dir(&dir)?;
    let blocked = runtime.hosting_writes_blocked()?;
    runtime.set_hosting_write_blocked(true)?;
    let result = backup_blocked(root, game, &dir, &store, "manual");
    runtime.set_hosting_write_blocked(blocked)?;
    result
}

fn backup_blocked(
    root: &Path,
    game: &str,
    dir: &Path,
    store: &HostedStore,
    kind: &str,
) -> Result<Value> {
    let destination =
        root.join(".backups")
            .join(format!("{game}-{}-{:016x}", now(), rand::random::<u64>()));
    std::fs::create_dir_all(&destination)?;
    let result = (|| -> Result<Value> {
        for name in ["ncgame.db", "hosted.db"] {
            let source = Connection::open(dir.join(name))?;
            source.backup(DatabaseName::Main, destination.join(name), None)?;
            let copy = Connection::open(destination.join(name))?;
            let check: String = copy.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
            if check != "ok" {
                return Err(format!("backup integrity check: {check}").into());
            }
        }
        let info = campaign::inspect(root, game)?;
        let mut manifest =
            json!({"version":1,"game":game,"year":info["year"],"created_at":now(),"kind":kind});
        manifest["files"] = recovery::file_hashes(&destination)?;
        std::fs::write(
            destination.join("manifest.json"),
            serde_json::to_vec_pretty(&manifest)?,
        )?;
        store.connection().execute(
            "INSERT INTO web_backups(path,at,kind) VALUES (?1,?2,?3)",
            params![destination.to_string_lossy(), now(), kind],
        )?;
        campaign::record(store, "backup", &json!({"path":destination}))?;
        Ok(json!({"path":destination,"manifest":manifest}))
    })();
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&destination);
    }
    result
}

pub(super) fn history(root: &Path, game: &str) -> Result<Value> {
    let (_, store) = store(root, game)?;
    let mut stmt = store
        .connection()
        .prepare("SELECT at,operation,detail FROM web_history ORDER BY id DESC LIMIT 200")?;
    let rows=stmt.query_map([],|r|Ok(json!({"at":r.get::<_,i64>(0)?,"operation":r.get::<_,String>(1)?,"detail":r.get::<_,String>(2)?})))?
        .collect::<std::result::Result<Vec<_>,_>>()?;
    Ok(json!(rows))
}

pub(super) fn run(root: &Path, game: &str, expected_year: u16, request_id: &str) -> Result<Value> {
    if request_id.is_empty()
        || request_id.len() > 128
        || !request_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_:".contains(&b))
    {
        return Err(
            "request ID must be 1-128 letters, digits, hyphens, colons or underscores".into(),
        );
    }
    let (dir, store) = store(root, game)?;
    let runtime = CampaignStore::open_default_in_dir(&dir)?;
    if let Some((original, end_year)) = runtime.maintenance_receipt(request_id)? {
        if original != expected_year {
            return Err("request ID belongs to another year".into());
        }
        finalize(&store, game, request_id, end_year)?;
        let state = hosted::get_game_metadata(store.connection(), game)?.status;
        runtime.set_hosting_write_blocked(!matches!(state.as_str(), "running" | "recruiting"))?;
        return Ok(json!({"game":game,"start_year":original,"end_year":end_year,"replayed":true}));
    }
    let meta = hosted::get_game_metadata(store.connection(), game)?;
    if !matches!(meta.status.as_str(), "running" | "maintenance") {
        return Err("campaign must be running to advance a turn".into());
    }
    let (_, year) = runtime
        .latest_snapshot_metadata()?
        .ok_or("missing snapshot")?;
    if year != expected_year {
        return Err(format!("expected year {expected_year}, found {year}").into());
    }
    let previous: Option<(u16, String)> = store
        .connection()
        .query_row(
            "SELECT expected_year,state FROM web_runs WHERE request_id=?1",
            [request_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    if previous
        .as_ref()
        .is_some_and(|(y, state)| *y != year || state == "failed")
    {
        return Err("request conflicts with a prior or failed operation".into());
    }
    let another: bool = store.connection().query_row(
        "SELECT EXISTS(SELECT 1 FROM web_runs WHERE state='running' AND request_id!=?1)",
        [request_id],
        |r| r.get(0),
    )?;
    if another || (meta.status == "maintenance" && previous.is_none()) {
        return Err("another maintenance request needs recovery first".into());
    }
    runtime.set_hosting_write_blocked(true)?;
    let result = (|| -> Result<Value> {
        store.connection().execute("INSERT INTO web_runs(request_id,expected_year,state,started_at) VALUES (?1,?2,'running',?3)
            ON CONFLICT(request_id) DO NOTHING",params![request_id,year,now()])?;
        store.connection().execute(
            "UPDATE game_metadata SET status='maintenance' WHERE id=?1",
            [game],
        )?;
        backup_blocked(root, game, &dir, &store, "turn")?;
        retain_backups(root, &store, "turn")?;
        let mut turn = nc_engine::CampaignMaintenance::load(&runtime).map_err(|e| e.to_string())?;
        turn.run_turn().map_err(|e| e.to_string())?;
        let end_year = turn.game_data.conquest.game_year;
        turn.save(&runtime.recording_maintenance(request_id, year)?)
            .map_err(|e| e.to_string())?;
        finalize(&store, game, request_id, end_year)?;
        runtime.set_hosting_write_blocked(false)?;
        Ok(json!({"game":game,"start_year":year,"end_year":end_year,"replayed":false}))
    })();
    if let Err(ref err) = result {
        store.connection().execute(
            "UPDATE game_metadata SET status='paused',maintenance_enabled=0 WHERE id=?1",
            [game],
        )?;
        store.connection().execute(
            "UPDATE web_runs SET state='failed',result=?1 WHERE request_id=?2",
            params![err.to_string(), request_id],
        )?;
        campaign::record(
            &store,
            "maintenance_failed",
            &json!({"request_id":request_id,"error":err.to_string()}),
        )?;
    }
    result
}

fn finalize(store: &HostedStore, game: &str, request_id: &str, year: u16) -> Result<()> {
    let settings = hosted::get_settings(store.connection(), game)?;
    let tx = store.connection().unchecked_transaction()?;
    let changed = tx.execute(
        "UPDATE web_runs SET state='done',result=?1 WHERE request_id=?2 AND state!='done'",
        params![year.to_string(), request_id],
    )?;
    if changed > 0 {
        let interval = i64::from(settings.maintenance_interval_minutes) * 60;
        let due = settings.maintenance_next_due_unix_seconds.unwrap_or(now());
        let next = if due + interval > now() {
            due + interval
        } else {
            now() + interval
        };
        tx.execute(
            "UPDATE game_metadata SET current_year=?1,current_turn=?2,
            status=CASE WHEN status='maintenance' THEN 'running' ELSE status END,
            maintenance_next_due_unix_seconds=?3 WHERE id=?4",
            params![year, year.saturating_sub(3000), next, game],
        )?;
        let meta = hosted::get_game_metadata(&tx, game)?;
        announcements::enqueue(
            &tx,
            request_id,
            &json!({"game":game,"name":meta.name,"kind":"turn","year":year,"next_due":next}),
        )?;
        tx.execute(
            "INSERT INTO web_history(at,operation,detail) VALUES (?1,'maintenance',?2)",
            params![
                now(),
                json!({"request_id":request_id,"year":year}).to_string()
            ],
        )?;
    }
    tx.commit()?;
    Ok(())
}

pub(super) fn due(root: &Path) -> Result<Value> {
    let listing = campaign::list(root)?;
    let mut results = Vec::new();
    for game in listing.as_array().ok_or("invalid listing")? {
        if game["state"] != "running" {
            continue;
        }
        let Some(deadline) = game["next_due"].as_i64() else {
            continue;
        };
        if deadline > now() {
            continue;
        }
        let id = game["game"].as_str().ok_or("missing game ID")?;
        let interval = game["interval_minutes"]
            .as_i64()
            .ok_or("missing interval")?
            * 60;
        if now() - deadline >= interval {
            campaign::transition(root, id, "paused", None, None)?;
            results.push(json!({"game":id,"paused":true,"reason":"multiple missed deadlines"}));
            continue;
        }
        let year = game["year"].as_u64().ok_or("missing year")? as u16;
        let request = format!("scheduled-{year}-{deadline}");
        results.push(match run(root, id, year, &request) {
            Ok(value) => value,
            Err(error) => json!({"game":id,"error":error.to_string()}),
        });
    }
    Ok(json!(results))
}

// Manual/pre-upgrade copies are never pruned automatically. Keep the newest
// 30 daily copies and 30 pre-turn copies independently for each campaign.
fn retain_backups(root: &Path, store: &HostedStore, kind: &str) -> Result<()> {
    let mut stmt = store.connection().prepare(
        "SELECT path FROM web_backups WHERE kind=?1 ORDER BY at DESC,path DESC LIMIT -1 OFFSET 30",
    )?;
    let paths = stmt
        .query_map([kind], |r| r.get::<_, String>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    for path in paths {
        let path = PathBuf::from(path);
        if path.parent() != Some(root.join(".backups").as_path()) {
            return Err("backup outside managed directory; refusing retention".into());
        }
        if path.exists() {
            if path.symlink_metadata()?.file_type().is_symlink() {
                return Err("backup is a symlink; refusing retention".into());
            }
            std::fs::remove_dir_all(&path)?;
        }
        store.connection().execute(
            "DELETE FROM web_backups WHERE path=?1",
            [path.to_string_lossy()],
        )?;
    }
    Ok(())
}
pub(super) fn daily_backups(root: &Path) -> Result<Value> {
    let games = campaign::list(root)?;
    let mut results = Vec::new();
    for row in games.as_array().ok_or("invalid catalog")? {
        if row.get("error").is_some() {
            results.push(row.clone());
            continue;
        }
        let game = row["game"].as_str().ok_or("missing game")?;
        if row["state"] == "maintenance" {
            results
                .push(json!({"game":game,"error":"recover interrupted maintenance before backup"}));
            continue;
        }
        let (dir, store) = store(root, game)?;
        let latest: Option<i64> = store.connection().query_row(
            "SELECT MAX(at) FROM web_backups WHERE kind='daily'",
            [],
            |r| r.get(0),
        )?;
        if latest.is_some_and(|at| at / 86400 == now() / 86400) {
            continue;
        }
        let runtime = CampaignStore::open_default_in_dir(&dir)?;
        let blocked = runtime.hosting_writes_blocked()?;
        runtime.set_hosting_write_blocked(true)?;
        let result = backup_blocked(root, game, &dir, &store, "daily");
        runtime.set_hosting_write_blocked(blocked)?;
        match result {
            Ok(value) => {
                retain_backups(root, &store, "daily")?;
                results.push(value);
            }
            Err(error) => results.push(json!({"game":game,"error":error.to_string()})),
        }
    }
    Ok(json!(results))
}

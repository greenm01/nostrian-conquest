use super::*;
use nc_data::CampaignStore;
use rusqlite::Connection;

pub(super) fn file_hashes(dir: &Path) -> Result<Value> {
    let mut files = serde_json::Map::new();
    for name in ["ncgame.db", "hosted.db"] {
        let path = dir.join(name);
        let metadata = path.symlink_metadata()?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err("backup database must be a regular file".into());
        }
        let mut reader = std::fs::File::open(path)?;
        let mut hash = blake3::Hasher::new();
        let mut buffer = [0u8; 65536];
        loop {
            let n = reader.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            hash.update(&buffer[..n]);
        }
        files.insert(name.into(), json!(hash.finalize().to_hex().to_string()));
    }
    Ok(Value::Object(files))
}

pub(super) fn verify(dir: &Path) -> Result<Value> {
    let manifest: Value = serde_json::from_slice(&std::fs::read(dir.join("manifest.json"))?)?;
    if manifest["version"] != 1 || manifest["files"] != file_hashes(dir)? {
        return Err("backup manifest or checksum mismatch".into());
    }
    for name in ["ncgame.db", "hosted.db"] {
        let conn = Connection::open_with_flags(
            dir.join(name),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;
        let integrity: String = conn.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
        if integrity != "ok" {
            return Err(format!("{name}: {integrity}").into());
        }
    }
    let runtime = Connection::open_with_flags(
        dir.join("ncgame.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    let year: u16 = runtime.query_row("SELECT MAX(game_year) FROM snapshots", [], |r| r.get(0))?;
    if manifest["year"] != year {
        return Err("backup year does not match manifest".into());
    }
    Ok(manifest)
}

pub(super) fn restore(root: &Path, source: &Path, game: &str) -> Result<Value> {
    let manifest = verify(source)?;
    if manifest["game"] != game {
        return Err("confirmation must match original game ID".into());
    }
    let destination = game_dir(root, game)?;
    std::fs::create_dir(&destination)?;
    let result = (|| -> Result<Value> {
        for name in ["ncgame.db", "hosted.db"] {
            std::fs::copy(source.join(name), destination.join(name))?;
        }
        let (_, store) = store(root, game)?;
        store.connection().execute("UPDATE game_metadata SET status='paused',maintenance_enabled=0,maintenance_next_due_unix_seconds=NULL WHERE id=?1",[game])?;
        // Backup locations belong to the source installation. Restore keeps
        // audit history, but starts its own backup inventory in this root.
        store.connection().execute("DELETE FROM web_backups", [])?;
        // A run interrupted when the backup was taken did not survive into
        // the restored game; leaving it running would block the next turn.
        store.connection().execute(
            "UPDATE web_runs SET state='failed',result='interrupted; restored from backup'
             WHERE state='running'",
            [],
        )?;
        CampaignStore::open_default_in_dir(&destination)?.set_hosting_write_blocked(true)?;
        campaign::record(
            &store,
            "restore",
            &json!({"source":source,"year":manifest["year"]}),
        )?;
        campaign::inspect(root, game)
    })();
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&destination);
    }
    result
}

use super::*;
use nc_data::hosted;
use rand::Rng;
use rusqlite::{Connection, OptionalExtension, params};

fn ensure(conn: &Connection) -> Result<()> {
    conn.execute_batch("CREATE TABLE IF NOT EXISTS web_invites(
        id TEXT PRIMARY KEY,seat INTEGER NOT NULL,code_hash TEXT NOT NULL UNIQUE,
        state TEXT NOT NULL,created_at INTEGER NOT NULL,claimed_by TEXT);
        CREATE UNIQUE INDEX IF NOT EXISTS web_invites_unused_seat ON web_invites(seat) WHERE state='unused';
        CREATE TABLE IF NOT EXISTS web_invite_attempts(account TEXT NOT NULL,at INTEGER NOT NULL);")?;
    Ok(())
}
pub(super) fn required(conn: &Connection) -> Result<bool> {
    Ok(scheduling::option(conn, "invite_only")?.as_deref() == Some("true"))
}
pub(super) fn restrict(root: &Path, game: &str) -> Result<Value> {
    let (_, store) = store(root, game)?;
    scheduling::set_option(store.connection(), "invite_only", "true")?;
    ensure(store.connection())?;
    campaign::record(&store, "invite_only", &json!({}))?;
    campaign::inspect(root, game)
}
pub(super) fn create(root: &Path, game: &str, count: u8) -> Result<Value> {
    let (_, store) = store(root, game)?;
    let conn = store.connection();
    if !required(conn)? {
        return Err("set this game to invite-only before issuing codes".into());
    }
    let meta = hosted::get_game_metadata(conn, game)?;
    if !matches!(meta.status.as_str(), "draft" | "recruiting")
        || count == 0
        || u32::from(count) > meta.players
    {
        return Err("codes require a draft or recruiting game and a valid count".into());
    }
    ensure(conn)?;
    let mut seats = Vec::new();
    for seat in 1..=meta.players {
        if conn.query_row("SELECT NOT EXISTS(SELECT 1 FROM discord_seats WHERE seat=?1) AND NOT EXISTS(SELECT 1 FROM web_invites WHERE seat=?1 AND state='unused')",[seat],|r|r.get::<_,bool>(0))? { seats.push(seat); }
    }
    if seats.len() < usize::from(count) {
        return Err("not enough unclaimed, unreserved seats".into());
    }
    let tx = conn.unchecked_transaction()?;
    let mut issued = Vec::new();
    const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    for seat in seats.into_iter().take(usize::from(count)) {
        let code = loop {
            let code: String = (0..8)
                .map(|_| ALPHABET[rand::thread_rng().gen_range(0..ALPHABET.len())] as char)
                .collect();
            let hash = blake3::hash(code.as_bytes()).to_hex().to_string();
            let exists: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM web_invites WHERE code_hash=?1)",
                [&hash],
                |r| r.get(0),
            )?;
            if !exists {
                break (code, hash);
            }
        };
        let id = format!("{:016x}", rand::random::<u64>());
        tx.execute(
            "INSERT INTO web_invites VALUES (?1,?2,?3,'unused',?4,NULL)",
            params![id, seat, code.1, now()],
        )?;
        tx.execute(
            "INSERT INTO web_history(at,operation,detail) VALUES (?1,'invite_created',?2)",
            params![now(), json!({"invite_id":id,"seat":seat}).to_string()],
        )?;
        issued.push(json!({"invite_id":id,"seat":seat,"code":code.0}));
    }
    tx.commit()?;
    Ok(json!({"game":game,"invites":issued}))
}
pub(super) fn list(root: &Path, game: &str) -> Result<Value> {
    let (_, store) = store(root, game)?;
    ensure(store.connection())?;
    let mut q = store.connection().prepare(
        "SELECT id,seat,state,created_at,claimed_by FROM web_invites ORDER BY created_at,id",
    )?;
    let rows=q.query_map([],|r|Ok(json!({"invite_id":r.get::<_,String>(0)?,"seat":r.get::<_,u32>(1)?,"state":r.get::<_,String>(2)?,"created_at":r.get::<_,i64>(3)?,"claimed_by":r.get::<_,Option<String>>(4)?})))?.collect::<std::result::Result<Vec<_>,_>>()?;
    Ok(json!({"game":game,"invites":rows}))
}
pub(super) fn revoke(root: &Path, game: &str, id: &str) -> Result<Value> {
    let (_, store) = store(root, game)?;
    ensure(store.connection())?;
    let n = store.connection().execute(
        "UPDATE web_invites SET state='revoked' WHERE id=?1 AND state='unused'",
        [id],
    )?;
    if n == 0 {
        return Err("no unused invite with that ID; revoking a code never evicts a player".into());
    }
    campaign::record(&store, "invite_revoked", &json!({"invite_id":id}))?;
    Ok(json!({"revoked":true,"invite_id":id}))
}
/// Only called after verifying recruitment and that this account has no seat.
/// Failed guesses are bounded per account and across the entire game.
pub(super) fn check(conn: &Connection, id: &str, code: &str) -> Result<(String, u32)> {
    ensure(conn)?;
    conn.execute(
        "DELETE FROM web_invite_attempts WHERE at<=?1",
        [now() - 600],
    )?;
    let (own, all): (u32, u32) = conn.query_row(
        "SELECT COALESCE(SUM(account=?1),0),COUNT(*) FROM web_invite_attempts",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    if own >= 5 || all >= 100 {
        return Err("too many failed invite claims; try again in ten minutes".into());
    }
    let normalized = code.trim().to_ascii_uppercase();
    let hash = blake3::hash(normalized.as_bytes()).to_hex().to_string();
    let invite = conn
        .query_row(
            "SELECT id,seat FROM web_invites WHERE code_hash=?1 AND state='unused'",
            [hash],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, u32>(1)?)),
        )
        .optional()?;
    if let Some(invite) = invite {
        return Ok(invite);
    }
    conn.execute(
        "INSERT INTO web_invite_attempts VALUES (?1,?2)",
        params![id, now()],
    )?;
    Err("invalid, used or revoked invite code".into())
}
pub(super) fn consume(conn: &Connection, invite_id: &str, id: &str) -> Result<()> {
    if conn.execute(
        "UPDATE web_invites SET state='claimed',claimed_by=?2 WHERE id=?1 AND state='unused'",
        params![invite_id, id],
    )? != 1
    {
        return Err("invite no longer available".into());
    }
    Ok(())
}
pub(super) fn revoke_seat(conn: &Connection, seat: u32) -> Result<()> {
    ensure(conn)?;
    conn.execute(
        "UPDATE web_invites SET state='revoked' WHERE seat=?1 AND state='unused'",
        [seat],
    )?;
    Ok(())
}

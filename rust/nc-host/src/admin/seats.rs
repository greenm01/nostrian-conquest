use super::*;
use nc_data::hosted;
use rusqlite::{OptionalExtension, params};

pub(super) fn authorize(root: &Path, game: &str, id: &AccountIdentity) -> Result<Value> {
    let access = membership(root, game, id)?;
    if access["seat"].is_null() {
        return Err("no seat belongs to this account in this game".into());
    }
    Ok(access)
}

pub(super) fn membership(root: &Path, game: &str, id: &AccountIdentity) -> Result<Value> {
    id.validate()?;
    let (_, store) = store(root, game)?;
    let seat: Option<u32> = store
        .connection()
        .query_row("SELECT seat FROM web_seats WHERE account=?1", [id], |r| {
            r.get(0)
        })
        .optional()?;
    let status = hosted::get_game_metadata(store.connection(), game)?.status;
    Ok(
        json!({"game":game,"seat":seat,"account":id,"state":status,"can_play":seat.is_some() && matches!(status.as_str(),"recruiting"|"running")}),
    )
}

pub(super) fn join(
    root: &Path,
    game: &str,
    id: &AccountIdentity,
    replacement: Option<u32>,
    code: Option<&str>,
    name: Option<&str>,
) -> Result<Value> {
    id.validate()?;
    let (_, store) = store(root, game)?;
    let meta = hosted::get_game_metadata(store.connection(), game)?;
    let existing: Option<u32> = store
        .connection()
        .query_row("SELECT seat FROM web_seats WHERE account=?1", [id], |r| {
            r.get(0)
        })
        .optional()?;
    if let Some(seat) = existing {
        if replacement.is_some_and(|wanted| wanted != seat) {
            return Err("account already holds another seat".into());
        }
        if replacement.is_none() {
            return authorize(root, game, id);
        }
    }
    let mut invite = None;
    let seat = if let Some(seat) = replacement {
        if !matches!(meta.status.as_str(), "running" | "paused" | "recruiting")
            || seat == 0
            || seat > meta.players
        {
            return Err("invalid replacement seat or campaign state".into());
        }
        seat
    } else {
        if meta.status != "recruiting" {
            return Err("this game is not recruiting".into());
        }
        if invites::required(store.connection())? {
            let code = code.ok_or("an invite code is required; enter the code from the host")?;
            let (invite_id, seat) = invites::check(store.connection(), id, code)?;
            invite = Some(invite_id);
            seat
        } else {
            (1..=meta.players)
                .find(|seat| {
                    store
                        .connection()
                        .query_row(
                            "SELECT NOT EXISTS(SELECT 1 FROM web_seats WHERE seat=?1)",
                            [seat],
                            |r| r.get::<_, bool>(0),
                        )
                        .unwrap_or(false)
                })
                .ok_or("this game is full")?
        }
    };
    let tx = store.connection().unchecked_transaction()?;
    if replacement.is_some() {
        invites::revoke_seat(&tx, seat)?;
        handles::unlock(&tx, seat)?;
        tx.execute("DELETE FROM web_seats WHERE seat=?1", [seat])?;
    } else {
        let occupied: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM web_seats WHERE seat=?1)",
            [seat],
            |r| r.get(0),
        )?;
        if occupied {
            return Err("reserved seat is no longer available".into());
        }
    }
    tx.execute(
        "INSERT INTO web_seats(seat,account) VALUES (?1,?2)",
        params![seat, id],
    )?;
    if let Some(invite_id) = invite {
        invites::consume(&tx, &invite_id, id)?;
    }
    tx.execute(
        "INSERT INTO web_history(at,operation,detail) VALUES (?1,?2,?3)",
        params![
            now(),
            if replacement.is_some() {
                "replace"
            } else {
                "join"
            },
            json!({"seat":seat,"account":id}).to_string()
        ],
    )?;
    tx.commit()?;
    // The claim stands even if the handle cannot be written now; the seat then
    // takes its player's name when they first press Play.
    if let (None, Some(name)) = (replacement, name) {
        let _ = handles::lock(root, game, seat, name, false);
    }
    authorize(root, game, id)
}

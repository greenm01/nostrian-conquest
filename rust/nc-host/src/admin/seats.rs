use super::*;
use nc_data::hosted;
use rusqlite::{OptionalExtension, params};

fn validate_id(id: &str) -> Result<()> {
    if id.is_empty() || id.len() > 20 || !id.bytes().all(|b| b.is_ascii_digit()) {
        return Err("invalid Discord account ID".into());
    }
    Ok(())
}

pub(super) fn authorize(root: &Path, game: &str, id: &str) -> Result<Value> {
    validate_id(id)?;
    let (_, store) = store(root, game)?;
    let seat: Option<u32> = store
        .connection()
        .query_row(
            "SELECT seat FROM discord_seats WHERE discord_id=?1",
            [id],
            |r| r.get(0),
        )
        .optional()?;
    let seat = seat.ok_or("no seat belongs to this account in this game")?;
    let status = hosted::get_game_metadata(store.connection(), game)?.status;
    Ok(
        json!({"game":game,"seat":seat,"discord_id":id,"state":status,"can_play":matches!(status.as_str(),"recruiting"|"running")}),
    )
}

pub(super) fn join(root: &Path, game: &str, id: &str, replacement: Option<u32>) -> Result<Value> {
    validate_id(id)?;
    let (_, store) = store(root, game)?;
    let meta = hosted::get_game_metadata(store.connection(), game)?;
    let existing: Option<u32> = store
        .connection()
        .query_row(
            "SELECT seat FROM discord_seats WHERE discord_id=?1",
            [id],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(seat) = existing {
        if replacement.is_some_and(|wanted| wanted != seat) {
            return Err("account already holds another seat".into());
        }
        return authorize(root, game, id);
    }
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
        (1..=meta.players)
            .find(|seat| {
                store
                    .connection()
                    .query_row(
                        "SELECT NOT EXISTS(SELECT 1 FROM discord_seats WHERE seat=?1)",
                        [seat],
                        |r| r.get::<_, bool>(0),
                    )
                    .unwrap_or(false)
            })
            .ok_or("this game is full")?
    };
    store.connection().execute(
        "INSERT INTO discord_seats(seat,discord_id) VALUES (?1,?2)
        ON CONFLICT(seat) DO UPDATE SET discord_id=excluded.discord_id",
        params![seat, id],
    )?;
    campaign::record(
        &store,
        if replacement.is_some() {
            "replace"
        } else {
            "join"
        },
        &json!({"seat":seat,"discord_id":id}),
    )?;
    authorize(root, game, id)
}

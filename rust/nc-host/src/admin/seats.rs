use super::*;
use nc_data::hosted;
use rusqlite::{OptionalExtension, params};

pub(super) fn sync_handle(root: &Path, game: &str, id: &str, display_name: &str) -> Result<Value> {
    let access = authorize(root, game, id)?;
    if access["can_play"] != true {
        return Err("campaign is not accepting sessions".into());
    }
    // Names are labels only. The authenticated ID determines the seat, even
    // when two accounts use the same name or one changes its name later.
    let name: String = display_name.chars().filter(|c| !c.is_control()).collect();
    if name.trim().is_empty() || name.len() > 512 {
        return Err("invalid display name".into());
    }
    let seat = access["seat"].as_u64().ok_or("missing seat")? as usize;
    let runtime = nc_data::CampaignStore::open_default_in_dir(&game_dir(root, game)?)?;
    let mut snapshot = runtime
        .load_latest_runtime_state()?
        .ok_or("missing state")?;
    let player = snapshot
        .game_data
        .players
        .get_mut(seat - 1)
        .ok_or("missing player")?;
    let old = player.handle.clone();
    nc_data::set_player_handle(player, name.trim());
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
    Ok(json!({"seat":seat,"handle":handle}))
}

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

pub(super) fn join(
    root: &Path,
    game: &str,
    id: &str,
    replacement: Option<u32>,
    code: Option<&str>,
) -> Result<Value> {
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
            let code =
                code.ok_or("an invite code is required; use /join with the code from the host")?;
            let (invite_id, seat) = invites::check(store.connection(), id, code)?;
            invite = Some(invite_id);
            seat
        } else {
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
        }
    };
    let tx = store.connection().unchecked_transaction()?;
    if replacement.is_some() {
        invites::revoke_seat(&tx, seat)?;
    } else {
        let occupied: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM discord_seats WHERE seat=?1)",
            [seat],
            |r| r.get(0),
        )?;
        if occupied {
            return Err("reserved seat is no longer available".into());
        }
    }
    tx.execute(
        "INSERT INTO discord_seats(seat,discord_id) VALUES (?1,?2)
        ON CONFLICT(seat) DO UPDATE SET discord_id=excluded.discord_id",
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
            json!({"seat":seat,"discord_id":id}).to_string()
        ],
    )?;
    tx.commit()?;
    authorize(root, game, id)
}

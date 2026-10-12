//! Operator-only account linking. Invite claims never call this module.
use super::*;
use nc_data::hosted;
use rusqlite::{OptionalExtension, params};

pub(super) fn change(
    root: &Path,
    game: &str,
    seat: u32,
    account: &AccountIdentity,
    link: bool,
) -> Result<Value> {
    account.validate()?;
    let (_, store) = store(root, game)?;
    let conn = store.connection();
    let status = hosted::get_game_metadata(conn, game)?.status;
    if !matches!(status.as_str(), "recruiting" | "running" | "paused") {
        return Err("account changes require a recruiting, running or paused game".into());
    }
    let bound: Option<u32> = conn
        .query_row(
            "SELECT seat FROM web_seats WHERE account=?1",
            [account],
            |r| r.get(0),
        )
        .optional()?;
    let accounts = conn
        .prepare("SELECT account FROM web_seats WHERE seat=?1")?
        .query_map([seat], |r| r.get::<_, AccountIdentity>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if accounts.is_empty() {
        return Err("linking requires an already claimed seat".into());
    }
    if link {
        if let Some(owned) = bound {
            if owned != seat {
                return Err("account already holds another seat".into());
            }
            return Ok(json!({"game":game,"seat":seat,"account":account,"linked":true}));
        }
        if accounts
            .iter()
            .any(|other| std::mem::discriminant(other) == std::mem::discriminant(account))
        {
            return Err(
                "seat already has an identity for this provider; unlink or replace it first".into(),
            );
        }
    } else {
        if bound != Some(seat) {
            return Err("account does not belong to this seat".into());
        }
        if accounts.len() == 1 {
            return Err(
                "cannot unlink the last identity; use replacement to transfer the seat".into(),
            );
        }
    }
    let tx = conn.unchecked_transaction()?;
    if link {
        tx.execute(
            "INSERT INTO web_seats(seat,account) VALUES (?1,?2)",
            params![seat, account],
        )?;
    } else {
        tx.execute(
            "DELETE FROM web_seats WHERE seat=?1 AND account=?2",
            params![seat, account],
        )?;
    }
    tx.execute(
        "INSERT INTO web_history(at,operation,detail) VALUES (?1,?2,?3)",
        params![
            now(),
            if link {
                "link_account"
            } else {
                "unlink_account"
            },
            json!({"seat":seat,"account":account}).to_string()
        ],
    )?;
    tx.commit()?;
    Ok(json!({"game":game,"seat":seat,"account":account,"linked":link}))
}

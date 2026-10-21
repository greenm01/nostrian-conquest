use super::*;
use rusqlite::{Connection, params};

pub(super) fn enqueue(conn: &Connection, id: &str, payload: &Value) -> Result<()> {
    conn.execute(
        "INSERT INTO web_announcements(id,payload) VALUES (?1,?2) ON CONFLICT(id) DO NOTHING",
        params![id, payload.to_string()],
    )?;
    Ok(())
}
/// Each channel records its own deliveries, so one channel's receipt never hides
/// an announcement from another. Created on use so existing games gain it too.
fn deliveries(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS web_announcement_deliveries(id TEXT NOT NULL, channel TEXT NOT NULL,
             delivered_at INTEGER NOT NULL, PRIMARY KEY(id,channel));",
    )?;
    Ok(())
}
fn channel_name(channel: Channel) -> &'static str {
    match channel {
        Channel::Discord => "discord",
        Channel::Matrix => "matrix",
    }
}
/// Whether an announcement still describes the game: recruitment is still open,
/// or the announced year is the current one. Channels acknowledge stale ones
/// without posting them, so a channel that was down does not post a backlog.
fn current(game: &Value, payload: &Value) -> bool {
    let state = game["state"].as_str();
    match payload["kind"].as_str() {
        Some("recruiting") => state == Some("recruiting"),
        Some("turn") => {
            matches!(state, Some("running" | "paused")) && payload["year"] == game["year"]
        }
        _ => false,
    }
}
pub(super) fn pending(root: &Path, channel: Channel) -> Result<Value> {
    let games = campaign::list(root)?;
    let mut result = Vec::new();
    for game in games.as_array().ok_or("invalid catalog")? {
        if game.get("error").is_some() {
            continue;
        }
        let id = game["game"].as_str().ok_or("missing game")?;
        let (_, store) = store(root, id)?;
        deliveries(store.connection())?;
        let mut statement = store.connection().prepare(
            "SELECT a.id,a.payload FROM web_announcements a WHERE NOT EXISTS
                 (SELECT 1 FROM web_announcement_deliveries d WHERE d.id=a.id AND d.channel=?1)
             ORDER BY a.rowid LIMIT 100",
        )?;
        let rows = statement.query_map([channel_name(channel)], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (event, payload) = row?;
            let payload = serde_json::from_str::<Value>(&payload)?;
            result.push(
                json!({"game":id,"id":event,"current":current(game, &payload),"payload":payload}),
            );
        }
    }
    Ok(json!(result))
}
pub(super) fn acknowledge(root: &Path, game: &str, id: &str, channel: Channel) -> Result<Value> {
    let (_, store) = store(root, game)?;
    let conn = store.connection();
    let known: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM web_announcements WHERE id=?1)",
        [id],
        |r| r.get(0),
    )?;
    if !known {
        return Err("unknown announcement".into());
    }
    deliveries(conn)?;
    conn.execute(
        "INSERT OR IGNORE INTO web_announcement_deliveries(id,channel,delivered_at) VALUES (?1,?2,?3)",
        params![id, channel_name(channel), now()],
    )?;
    Ok(json!({"acknowledged":true}))
}

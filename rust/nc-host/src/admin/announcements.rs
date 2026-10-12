use super::*;
use rusqlite::{Connection, params};

pub(super) fn enqueue(conn: &Connection, id: &str, payload: &Value) -> Result<()> {
    conn.execute(
        "INSERT INTO web_announcements(id,payload) VALUES (?1,?2) ON CONFLICT(id) DO NOTHING",
        params![id, payload.to_string()],
    )?;
    Ok(())
}
pub(super) fn pending(root: &Path) -> Result<Value> {
    let games = campaign::list(root)?;
    let mut result = Vec::new();
    for game in games.as_array().ok_or("invalid catalog")? {
        if game.get("error").is_some() {
            continue;
        }
        let id = game["game"].as_str().ok_or("missing game")?;
        let (_, store) = store(root, id)?;
        let mut statement=store.connection().prepare("SELECT id,payload FROM web_announcements WHERE delivered_at IS NULL ORDER BY rowid LIMIT 100")?;
        let rows =
            statement.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        for row in rows {
            let (event, payload) = row?;
            result.push(
                json!({"game":id,"id":event,"payload":serde_json::from_str::<Value>(&payload)?}),
            );
        }
    }
    Ok(json!(result))
}
pub(super) fn acknowledge(root: &Path, game: &str, id: &str) -> Result<Value> {
    let (_, store) = store(root, game)?;
    let count = store.connection().execute(
        "UPDATE web_announcements SET delivered_at=COALESCE(delivered_at,?1) WHERE id=?2",
        params![now(), id],
    )?;
    if count == 0 {
        return Err("unknown announcement".into());
    }
    Ok(json!({"acknowledged":true}))
}

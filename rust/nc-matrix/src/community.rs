//! Messages the bot posts in the community room, as (plain, HTML) pairs.
//! Pure formatting: callers fetch the host's catalog and announcements.
use serde_json::Value;

pub fn help(origin: &str) -> (String, String) {
    (
        format!(
            "Nostrian Conquest commands:\n\
            !games lists the open and running games.\n\
            !login sends you a private sign-in link.\n\
            Play at {origin}"
        ),
        format!(
            "Nostrian Conquest commands:<br>\
            <code>!games</code> lists the open and running games.<br>\
            <code>!login</code> sends you a private sign-in link.<br>\
            Play at <a href=\"{origin}\">{origin}</a>"
        ),
    )
}

/// The host's public catalog, one line a game.
pub fn games(catalog: &[Value], origin: &str) -> (String, String) {
    if catalog.is_empty() {
        let text = "No games are currently listed.";
        return (text.into(), text.into());
    }
    let lines: Vec<_> = catalog
        .iter()
        .map(|g| {
            let (game, name) = (text(&g["game"]), text(&g["name"]));
            let rest = format!(
                "{} | {}/{} seats open | year {}",
                text(&g["state"]),
                g["open"],
                g["players"],
                g["year"]
            );
            (
                format!("{game} — {name} | {rest}"),
                format!(
                    "<code>{}</code> — {} | {}",
                    escape(&game),
                    escape(&name),
                    escape(&rest)
                ),
            )
        })
        .collect();
    let (plain, html): (Vec<_>, Vec<_>) = lines.into_iter().unzip();
    (
        format!("{}\nSign in at {origin} to join.", plain.join("\n")),
        format!(
            "{}<br>Sign in at <a href=\"{origin}\">{origin}</a> to join.",
            html.join("<br>")
        ),
    )
}

/// One queued announcement from the host, or `None` for a kind it doesn't post.
pub fn announcement(event: &Value, origin: &str) -> Option<(String, String)> {
    let payload = &event["payload"];
    let game = text(&event["game"]);
    let link = format!("<a href=\"{origin}\">{origin}</a>");
    match payload["kind"].as_str()? {
        "recruiting" => {
            let join = if payload["invite_required"] == true {
                "Ask the host for a private code, then sign in to claim a seat."
            } else {
                "Sign in to claim a seat."
            };
            let head = format!("opened recruitment ({} seats). {join}", payload["players"]);
            Some((
                format!("{game} {head}\n{origin}"),
                format!("<code>{}</code> {head}<br>{link}", escape(&game)),
            ))
        }
        "turn" => {
            let next = payload["next_due"]
                .as_i64()
                .and_then(|t| chrono::DateTime::from_timestamp(t, 0))
                .map(|t| format!("Next turn: {}.", t.format("%Y-%m-%d %H:%M UTC")))
                .unwrap_or_else(|| "Turns run manually; no automatic deadline.".into());
            let head = format!(
                "— maintenance completed for year {}. Reports are ready in the game. {next}",
                payload["year"]
            );
            Some((
                format!("{game} {head}\n{origin}"),
                format!("<code>{}</code> {head}<br>{link}", escape(&game)),
            ))
        }
        _ => None,
    }
}

fn text(value: &Value) -> String {
    value.as_str().unwrap_or("?").to_owned()
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

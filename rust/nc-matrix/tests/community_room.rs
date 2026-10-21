//! The bot's posts in the community room: announcements, `!games` and `!help`.
#[path = "support/homeserver.rs"]
mod support;
use nc_host::admin::{Channel, Request, execute};
use serde_json::{Value, json};
use support::*;

fn create(root: &std::path::Path, game: &str, name: &str) {
    execute(
        root,
        Request::Create {
            game: game.into(),
            name: name.into(),
            players: 4,
            seed: Some(1515),
        },
    )
    .unwrap();
    execute(root, Request::Publish { game: game.into() }).unwrap();
}

fn said(body: &str) -> Value {
    json!({"rooms":{"join":{"!lobby:example.org":{"timeline":{"events":[
        {"type":"m.room.message","sender":"@alice:example.org","content":{"body":body}}
    ]}}}}})
}

#[tokio::test]
async fn current_announcements_and_commands_are_posted_in_the_community_room() {
    let root = tempfile::tempdir().unwrap();
    create(root.path(), "open", "Open <Game>");
    // Started games no longer recruit, so their recruitment notice is stale.
    create(root.path(), "begun", "Begun");
    execute(
        root.path(),
        Request::Start {
            game: "begun".into(),
            first_due: chrono::Utc::now().timestamp() + 3600,
            interval_minutes: 1440,
        },
    )
    .unwrap();
    let script = vec![json!({}), said("!games"), said(" !Help ")];
    let server = run_bot(root.path(), script, |s| s.sent.len() >= 3).await;
    let server = server.lock().unwrap();
    assert_eq!(server.calls[1..], ["send !lobby:example.org"; 3]);
    let body = |i: usize| server.sent[i]["body"].as_str().unwrap().to_owned();
    assert!(body(0).starts_with("open opened recruitment (4 seats)."));
    assert!(!body(0).contains("begun"));
    assert!(body(1).contains("begun — Begun | running | 4/4 seats open | year 3000"));
    assert!(body(1).contains("open — Open <Game> | recruiting"));
    let html = server.sent[1]["formatted_body"].as_str().unwrap();
    assert!(html.contains("Open &lt;Game&gt;"));
    assert!(body(2).contains("!games lists"));
    let pending = execute(
        root.path(),
        Request::Announcements {
            channel: Channel::Matrix,
        },
    )
    .unwrap();
    assert_eq!(pending, json!([]));
}

//! Sign-in links requested from the games page, delivered by the bot.
#[path = "support/homeserver.rs"]
mod support;
use nc_data::AccountIdentity;
use nc_data::matrix_login::{INVITE_LIMIT, Redeem, ensure, record_invite, redeem, request};
use serde_json::json;
use support::*;

fn queue(root: &std::path::Path, users: &[&str], full: bool) -> rusqlite::Connection {
    let conn = rusqlite::Connection::open(root.join(".web-sessions.db")).unwrap();
    ensure(&conn).unwrap();
    let now = chrono::Utc::now().timestamp();
    if full {
        for _ in 0..INVITE_LIMIT {
            record_invite(&conn, now).unwrap();
        }
    }
    for user in users {
        request(&conn, &AccountIdentity::Matrix((*user).into()), now).unwrap();
    }
    conn
}

#[tokio::test]
async fn queued_request_opens_a_private_room_and_sends_a_working_code() {
    let root = tempfile::tempdir().unwrap();
    let conn = queue(root.path(), &["@carol:example.org"], false);
    let server = run_bot(root.path(), vec![json!({})], |s| !s.sent.is_empty()).await;
    let server = server.lock().unwrap();
    assert_eq!(server.created.len(), 1);
    assert_eq!(server.created[0]["invite"], json!(["@carol:example.org"]));
    assert!(
        server.sent[0]["body"]
            .as_str()
            .unwrap()
            .starts_with("Someone asked to sign in")
    );
    let (code, _) = code_and_secret(&server.sent[0]);
    assert_eq!(
        redeem(&conn, &code, chrono::Utc::now().timestamp()).unwrap(),
        Redeem::SignedIn {
            account: AccountIdentity::Matrix("@carol:example.org".into()),
            display_name: "Alice".into()
        }
    );
}

#[tokio::test]
async fn hourly_room_cap_skips_strangers_but_not_existing_rooms() {
    let root = tempfile::tempdir().unwrap();
    queue(
        root.path(),
        &["@erin:example.org", "@dave:example.org"],
        true,
    );
    let member = json!({"type":"m.room.member","sender":"@erin:example.org",
        "state_key":"@erin:example.org","content":{"membership":"join"}});
    let first = json!({"rooms":{"join":{"!erin:example.org":{"state":{"events":[member]}}}}});
    let server = run_bot(root.path(), vec![first], |s| s.syncs >= 3).await;
    let server = server.lock().unwrap();
    assert!(server.created.is_empty());
    assert_eq!(
        server.calls,
        [
            "join #nostrian-conquest:example.org",
            "send !erin:example.org"
        ]
    );
}

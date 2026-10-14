//! Runs the bot against a minimal homeserver and redeems what it sends.
#[path = "support/homeserver.rs"]
mod support;
use nc_data::AccountIdentity;
use nc_data::matrix_login::{Redeem, redeem};
use serde_json::{Value, json};
use support::*;

const ALICE: &str = "@alice:example.org";

fn invite(room: &str) -> Value {
    json!({"rooms":{"invite":{room:{"invite_state":{"events":[
        {"type":"m.room.member","sender":ALICE,"state_key":BOT,"content":{"membership":"invite"}}
    ]}}}}})
}

#[tokio::test]
async fn invitation_gets_a_private_one_time_link_and_code() {
    let root = tempfile::tempdir().unwrap();
    let script = vec![
        // Old invitations are declined without a code.
        invite("!old:example.org"),
        invite("!dm:example.org"),
        // Asking again within the request interval does not send another code.
        json!({"rooms":{"join":{"!lobby:example.org":{
            "timeline":{"events":[{"type":"m.room.message","sender":ALICE,"content":{"body":"!login"}}]}
        }}}}),
    ];
    let server = run_bot(root.path(), script, |s| s.syncs >= 4).await;
    let server = server.lock().unwrap();
    assert!(server.syncs >= 4, "bot stopped syncing");
    assert_eq!(
        server.calls,
        [
            "join #nostrian-conquest:example.org",
            "leave !old:example.org",
            "leave !dm:example.org",
            "send !private1:example.org"
        ]
    );
    assert_eq!(server.created.len(), 1);
    assert_eq!(server.created[0]["invite"], json!([ALICE]));
    assert!(server.created[0].get("initial_state").is_none());
    let (code, secret) = code_and_secret(&server.sent[0]);
    let conn = rusqlite::Connection::open(root.path().join(".web-sessions.db")).unwrap();
    let now = chrono::Utc::now().timestamp();
    assert_eq!(
        redeem(&conn, &code, now).unwrap(),
        Redeem::SignedIn {
            account: AccountIdentity::Matrix(ALICE.into()),
            display_name: "Alice".into()
        }
    );
    assert_eq!(redeem(&conn, &secret, now).unwrap(), Redeem::Invalid);
}

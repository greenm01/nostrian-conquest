//! Runs the bot against a minimal homeserver and redeems what it sends.
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post, put},
};
use nc_data::AccountIdentity;
use nc_data::matrix_login::{Redeem, redeem};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const BOT: &str = "@bot:example.org";
const ALICE: &str = "@alice:example.org";

#[derive(Default)]
struct Server {
    calls: Vec<String>,
    sent: Vec<Value>,
    created: Vec<Value>,
    syncs: u32,
}
type Shared = Arc<Mutex<Server>>;

fn authorized(headers: &HeaderMap) -> Result<(), StatusCode> {
    (headers.get("authorization").and_then(|h| h.to_str().ok()) == Some("Bearer secret"))
        .then_some(())
        .ok_or(StatusCode::UNAUTHORIZED)
}

async fn sync(
    State(server): State<Shared>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> Result<Json<Value>, StatusCode> {
    authorized(&headers)?;
    let call = {
        let mut server = server.lock().unwrap();
        server.syncs += 1;
        server.syncs
    };
    let invite = |room: &str| {
        json!({room:{"invite_state":{"events":[
            {"type":"m.room.member","sender":ALICE,"state_key":BOT,"content":{"membership":"invite"}}
        ]}}})
    };
    Ok(Json(match (call, query.get("since").map(String::as_str)) {
        // Old invitations are declined without a code.
        (1, None) => json!({"next_batch":"s1","rooms":{"invite":invite("!old:example.org")}}),
        (2, Some("s1")) => json!({"next_batch":"s2","rooms":{"invite":invite("!dm:example.org")}}),
        // Asking again within the request interval does not send another code.
        (3, Some("s2")) => json!({"next_batch":"s3","rooms":{"join":{"!lobby:example.org":{
            "timeline":{"events":[{"type":"m.room.message","sender":ALICE,"content":{"body":"!login"}}]}
        }}}}),
        _ => {
            tokio::time::sleep(Duration::from_millis(200)).await;
            json!({"next_batch":"s4"})
        }
    }))
}

fn app(server: Shared) -> Router {
    let record = |server: &Shared, call: String| server.lock().unwrap().calls.push(call);
    Router::new()
        .route(
            "/_matrix/client/v3/account/whoami",
            get(|headers: HeaderMap| async move {
                authorized(&headers).map(|_| Json(json!({"user_id":BOT})))
            }),
        )
        .route(
            "/_matrix/client/v3/join/{room}",
            post(
                move |State(s): State<Shared>, Path(room): Path<String>| async move {
                    record(&s, format!("join {room}"));
                    Json(json!({"room_id":"!lobby:example.org"}))
                },
            ),
        )
        .route("/_matrix/client/v3/sync", get(sync))
        .route(
            "/_matrix/client/v3/rooms/{room}/leave",
            post(
                move |State(s): State<Shared>, Path(room): Path<String>| async move {
                    record(&s, format!("leave {room}"));
                    Json(json!({}))
                },
            ),
        )
        .route(
            "/_matrix/client/v3/profile/{user}/displayname",
            get(|| async { Json(json!({"displayname":"Alice\u{7}"})) }),
        )
        .route(
            "/_matrix/client/v3/createRoom",
            post(
                |State(s): State<Shared>, Json(body): Json<Value>| async move {
                    s.lock().unwrap().created.push(body);
                    Json(json!({"room_id":"!private:example.org"}))
                },
            ),
        )
        .route(
            "/_matrix/client/v3/rooms/{room}/send/m.room.message/{txn}",
            put(
                |State(s): State<Shared>,
                 Path((room, _)): Path<(String, String)>,
                 Json(body): Json<Value>| async move {
                    let mut server = s.lock().unwrap();
                    server.calls.push(format!("send {room}"));
                    server.sent.push(body);
                    Json(json!({"event_id":"$sent"}))
                },
            ),
        )
        .with_state(server)
}

#[tokio::test]
async fn invitation_gets_a_private_one_time_link_and_code() {
    let root = tempfile::tempdir().unwrap();
    let server: Shared = Default::default();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(axum::serve(listener, app(server.clone())).into_future());
    let bot = tokio::spawn(nc_matrix::run(nc_matrix::Config {
        homeserver: format!("http://{address}"),
        access_token: "secret".into(),
        room: "#nostrian-conquest:example.org".into(),
        games_root: root.path().into(),
        origin: "https://game.example".into(),
    }));
    for _ in 0..100 {
        if server.lock().unwrap().syncs >= 4 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    bot.abort();
    let server = server.lock().unwrap();
    assert!(server.syncs >= 4, "bot stopped syncing");
    assert_eq!(
        server.calls,
        [
            "join #nostrian-conquest:example.org",
            "leave !old:example.org",
            "leave !dm:example.org",
            "send !private:example.org"
        ]
    );
    assert_eq!(server.created.len(), 1);
    assert_eq!(server.created[0]["invite"], json!([ALICE]));
    assert!(server.created[0].get("initial_state").is_none());
    let body = server.sent[0]["body"].as_str().unwrap();
    let secret = body
        .split("https://game.example/#matrix-login=")
        .nth(1)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap();
    let code = body.split("enter code ").nth(1).unwrap()[..8].to_owned();
    let conn = rusqlite::Connection::open(root.path().join(".web-sessions.db")).unwrap();
    let now = chrono::Utc::now().timestamp();
    assert_eq!(
        redeem(&conn, &code, now).unwrap(),
        Redeem::SignedIn {
            account: AccountIdentity::Matrix(ALICE.into()),
            display_name: "Alice".into()
        }
    );
    assert_eq!(redeem(&conn, secret, now).unwrap(), Redeem::Invalid);
}

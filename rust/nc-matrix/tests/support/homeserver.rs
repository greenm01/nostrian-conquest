//! A minimal homeserver for running the bot: scripted sync replies, recorded calls.
#![allow(dead_code)]
use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post, put},
};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const BOT: &str = "@bot:example.org";

#[derive(Default)]
pub struct Server {
    /// Sync replies in order; later syncs return nothing new.
    pub script: Vec<Value>,
    pub calls: Vec<String>,
    pub sent: Vec<Value>,
    pub created: Vec<Value>,
    pub syncs: usize,
}
pub type Shared = Arc<Mutex<Server>>;

fn authorized(headers: &HeaderMap) -> Result<(), StatusCode> {
    (headers.get("authorization").and_then(|h| h.to_str().ok()) == Some("Bearer secret"))
        .then_some(())
        .ok_or(StatusCode::UNAUTHORIZED)
}

async fn sync(State(server): State<Shared>, headers: HeaderMap) -> Result<Json<Value>, StatusCode> {
    authorized(&headers)?;
    let reply = {
        let mut server = server.lock().unwrap();
        server.syncs += 1;
        server.script.get(server.syncs - 1).cloned()
    };
    let reply = match reply {
        Some(reply) => reply,
        None => {
            tokio::time::sleep(Duration::from_millis(100)).await;
            json!({})
        }
    };
    let syncs = server.lock().unwrap().syncs;
    let mut reply = reply;
    reply["next_batch"] = json!(format!("s{syncs}"));
    Ok(Json(reply))
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
                    let mut server = s.lock().unwrap();
                    server.created.push(body);
                    Json(
                        json!({"room_id": format!("!private{}:example.org", server.created.len())}),
                    )
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

/// Runs the bot until `done` holds or five seconds pass, then stops it.
pub async fn run_bot(
    root: &std::path::Path,
    script: Vec<Value>,
    done: impl Fn(&Server) -> bool,
) -> Shared {
    let server: Shared = Arc::new(Mutex::new(Server {
        script,
        ..Default::default()
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(axum::serve(listener, app(server.clone())).into_future());
    let bot = tokio::spawn(nc_matrix::run(nc_matrix::Config {
        homeserver: format!("http://{address}"),
        access_token: "secret".into(),
        room: "#nostrian-conquest:example.org".into(),
        games_root: root.into(),
        origin: "https://game.example".into(),
    }));
    for _ in 0..100 {
        if done(&server.lock().unwrap()) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    bot.abort();
    server
}

/// The typed code and link secret from a sent sign-in message.
pub fn code_and_secret(message: &Value) -> (String, String) {
    let body = message["body"].as_str().unwrap();
    let secret = body
        .split("https://game.example/#matrix-login=")
        .nth(1)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .to_owned();
    (
        body.split("enter code ").nth(1).unwrap()[..8].to_owned(),
        secret,
    )
}

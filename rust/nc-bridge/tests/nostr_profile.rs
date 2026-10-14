//! Nostr sign-in takes the key's public profile name from relays, and that name
//! becomes the locked player handle when the seat is claimed.
#[path = "support/nostr.rs"]
mod support;
use axum::{
    Router,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    http::StatusCode,
    routing::get,
};
use nc_bridge::community::router;
use nostr_sdk::{EventBuilder, Keys, Metadata};
use serde_json::{Value, json};
use support::*;

/// A relay that knows one profile event and answers every request with it.
async fn mock_relay(profile: Option<Value>) -> String {
    let app = Router::new().route(
        "/",
        get(move |ws: WebSocketUpgrade| {
            let profile = profile.clone();
            async move { ws.on_upgrade(move |socket| serve(socket, profile)) }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(axum::serve(listener, app).into_future());
    format!("ws://{address}")
}

async fn serve(mut socket: WebSocket, profile: Option<Value>) {
    while let Some(Ok(message)) = socket.recv().await {
        let Message::Text(text) = message else {
            continue;
        };
        let request: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
        if request[0] != "REQ" {
            continue;
        }
        let id = request[1].clone();
        if let Some(event) = &profile {
            let reply = json!(["EVENT", id, event]).to_string();
            let _ = socket.send(Message::Text(reply.into())).await;
        }
        let _ = socket
            .send(Message::Text(json!(["EOSE", id]).to_string().into()))
            .await;
    }
}

async fn app_with_relay(profile: Option<Value>) -> (tempfile::TempDir, Router) {
    let relay = mock_relay(profile).await;
    let root = tempfile::tempdir().unwrap();
    let mut config = config(root.path());
    config.nostr_relays = vec![relay];
    let app = router(config).unwrap();
    (root, app)
}

#[tokio::test]
async fn published_profile_name_becomes_the_locked_handle() {
    let keys = Keys::generate();
    let metadata = Metadata::new().name("bob").display_name("Captain\u{7} Bob");
    let event = EventBuilder::metadata(&metadata)
        .sign_with_keys(&keys)
        .unwrap();
    let (root, app) = app_with_relay(Some(serde_json::to_value(event).unwrap())).await;
    let session = login(&app, &keys).await;
    let (_, me, _) = send(&app, "/api/me", "GET", &session, "", "", "").await;
    assert_eq!(me["display_name"], "Captain Bob");
    let host = |v| nc_host::admin::execute(root.path(), serde_json::from_value(v).unwrap());
    host(json!({"action":"create","game":"one","name":"one","players":4,"seed":1515})).unwrap();
    host(json!({"action":"publish","game":"one"})).unwrap();
    let (status, _, _) = send(
        &app,
        "/api/games/one/join",
        "POST",
        &session,
        "",
        "https://game.example",
        "",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let account = json!({"provider":"nostr","id":keys.public_key().to_hex()});
    let played = host(json!({"action":"sync_player_handle","game":"one",
        "account":account,"display_name":"Someone Else"}))
    .unwrap();
    assert_eq!(played["handle"], "Captain Bob");
}

#[tokio::test]
async fn key_without_a_profile_gets_an_anonymous_name() {
    let (_root, app) = app_with_relay(None).await;
    let session = login(&app, &Keys::generate()).await;
    let (_, me, _) = send(&app, "/api/me", "GET", &session, "", "", "").await;
    let name = me["display_name"].as_str().unwrap();
    assert!(name.starts_with("anon") && name.len() == 8, "{name}");
}

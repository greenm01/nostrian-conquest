#![allow(dead_code)]
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use base64::Engine;
use nc_bridge::community::{Config, router};
use nostr_sdk::{EventBuilder, Keys, Kind, Tag, Timestamp};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tower::ServiceExt;

pub fn config(root: &std::path::Path) -> Config {
    Config {
        games_root: root.into(),
        nc_door: "/nonexistent/nc-door".into(),
        static_dir: "static".into(),
        origin: "https://game.example".into(),
        client_id: "123".into(),
        client_secret: "test-only".into(),
        max_sessions: 2,
        matrix: None,
        nostr_relays: Vec::new(),
    }
}
pub fn fixture() -> (tempfile::TempDir, Router) {
    let root = tempfile::tempdir().unwrap();
    let app = router(config(root.path())).unwrap();
    (root, app)
}
pub async fn send(
    app: &Router,
    path: &str,
    method: &str,
    cookie: &str,
    auth: &str,
    origin: &str,
    body: &str,
) -> (StatusCode, Value, String) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(path)
                .method(method)
                .header("cookie", cookie)
                .header("origin", origin)
                .header("authorization", auth)
                .header("content-type", "application/json")
                .body(Body::from(body.to_owned()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.headers()["cache-control"], "no-store");
    let cookie = response
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|v| v.to_str().unwrap().to_owned())
        .collect::<Vec<_>>()
        .join("; ");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 100_000).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        cookie,
    )
}
pub async fn challenge(app: &Router) -> (String, String) {
    let (status, value, cookie) = send(
        app,
        "/auth/nostr/challenge",
        "POST",
        "",
        "",
        "https://game.example",
        "",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        cookie.contains("HttpOnly") && cookie.contains("Secure") && cookie.contains("SameSite=Lax")
    );
    (
        value["challenge"].as_str().unwrap().into(),
        cookie.split(';').next().unwrap().into(),
    )
}
pub fn signed(keys: &Keys, body: &str, fault: &str) -> String {
    let mut tags = vec![
        vec!["u".into(), "https://game.example/auth/nostr".into()],
        vec!["method".into(), "POST".into()],
        vec![
            "payload".into(),
            format!("{:x}", Sha256::digest(body.as_bytes())),
        ],
    ];
    match fault {
        "url" => tags[0][1] = "https://evil.example/auth/nostr".into(),
        "method" => tags[1][1] = "GET".into(),
        "payload" => tags[2][1] = "0".repeat(64),
        "missing_payload" => {
            tags.pop();
        }
        "duplicate" => tags.push(tags[0].clone()),
        _ => (),
    }
    let now = Timestamp::now().as_secs();
    let at = match fault {
        "past" => now - 120,
        "future" => now + 120,
        _ => now,
    };
    let event = EventBuilder::new(
        Kind::Custom(if fault == "kind" { 1 } else { 27235 }),
        if fault == "content" { "unexpected" } else { "" },
    )
    .tags(tags.into_iter().map(|t| Tag::parse(t).unwrap()))
    .custom_created_at(Timestamp::from(at))
    .sign_with_keys(keys)
    .unwrap();
    let mut value = serde_json::to_value(event).unwrap();
    if fault == "signature" {
        value["sig"] = json!("0".repeat(128));
    }
    if fault == "id" {
        value["id"] = json!("0".repeat(64));
    }
    format!(
        "Nostr {}",
        base64::engine::general_purpose::STANDARD.encode(serde_json::to_vec(&value).unwrap())
    )
}
pub async fn login(app: &Router, keys: &Keys) -> String {
    let (nonce, cookie) = challenge(app).await;
    let body = json!({"challenge":nonce}).to_string();
    let (status, _, session) = send(
        app,
        "/auth/nostr",
        "POST",
        &cookie,
        &signed(keys, &body, ""),
        "https://game.example",
        &body,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    session.split(';').next().unwrap().into()
}

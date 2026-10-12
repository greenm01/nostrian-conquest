#[path = "support/nostr.rs"]
mod support;
use axum::http::StatusCode;
use nc_bridge::community::router;
use nostr_sdk::Keys;
use serde_json::json;
use support::*;

#[tokio::test]
async fn signed_login_names_logout_and_restart() {
    let (root, app) = fixture();
    let keys = Keys::generate();
    let session = login(&app, &keys, None).await;
    let (status, me, _) = send(&app, "/api/me", "GET", &session, "", "", "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        me["account"],
        json!({"provider":"nostr","id":keys.public_key().to_hex()})
    );
    assert_eq!(
        me["display_name"],
        format!("Nostr-{}", &keys.public_key().to_hex()[..12])
    );
    let session = login(&app, &keys, Some("New\r\nCommander\u{1b}")).await;
    assert_eq!(
        send(&app, "/api/me", "GET", &session, "", "", "").await.1["display_name"],
        "NewCommander"
    );
    let session = login(&app, &keys, None).await;
    assert_eq!(
        send(&app, "/api/me", "GET", &session, "", "", "").await.1["display_name"],
        "NewCommander"
    );
    assert_eq!(
        send(
            &app,
            "/auth/logout",
            "POST",
            &session,
            "",
            "https://game.example",
            ""
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        send(&app, "/api/me", "GET", &session, "", "", "").await.0,
        StatusCode::UNAUTHORIZED
    );
    let session = login(&app, &keys, None).await;
    drop(app);
    let app = router(config(root.path())).unwrap();
    assert_eq!(
        send(&app, "/api/me", "GET", &session, "", "", "").await.0,
        StatusCode::OK
    );
    rusqlite::Connection::open(root.path().join(".web-sessions.db"))
        .unwrap()
        .execute("UPDATE web_sessions SET expires=0", [])
        .unwrap();
    assert_eq!(
        send(&app, "/api/me", "GET", &session, "", "", "").await.0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn browser_claims_invite_with_verified_key_and_cannot_link_accounts() {
    let (root, app) = fixture();
    let host =
        |v| nc_host::admin::execute(root.path(), serde_json::from_value(v).unwrap()).unwrap();
    host(json!({"action":"create","game":"one","name":"one","players":4,"seed":1515}));
    host(json!({"action":"publish","game":"one"}));
    host(json!({"action":"invite_only","game":"one"}));
    let codes = host(json!({"action":"create_invites","game":"one","count":2}));
    let session = login(&app, &Keys::generate(), Some("Commander")).await;
    let code = codes["invites"][0]["code"].as_str().unwrap().to_lowercase();
    let body = json!({"code":code}).to_string();
    assert_eq!(
        send(
            &app,
            "/api/games/one/join",
            "POST",
            &session,
            "",
            "https://game.example",
            ""
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let (status, seat, _) = send(
        &app,
        "/api/games/one/join?discord_id=123&seat=4",
        "POST",
        &session,
        "",
        "https://game.example",
        &body,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(seat["seat"], 1);
    assert_eq!(seat["account"]["provider"], "nostr");
    assert_eq!(
        send(
            &app,
            "/api/games/one/join",
            "POST",
            &session,
            "",
            "https://game.example",
            ""
        )
        .await
        .0,
        StatusCode::OK
    );
    let impostor = login(&app, &Keys::generate(), Some("Commander")).await;
    assert_eq!(
        send(
            &app,
            "/api/games/one/join",
            "POST",
            &impostor,
            "",
            "https://game.example",
            &body
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        send(&app, "/api/games/one/access", "GET", &impostor, "", "", "")
            .await
            .0,
        StatusCode::CONFLICT
    );
    let injection =
        json!({"code":codes["invites"][1]["code"],"account":{"provider":"discord","id":"123"}})
            .to_string();
    assert_eq!(
        send(
            &app,
            "/api/games/one/join",
            "POST",
            &impostor,
            "",
            "https://game.example",
            &injection
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let game = host(json!({"action":"inspect","game":"one"}));
    assert_eq!(game["claimed"], 1);
    assert_eq!(game["state"], "recruiting");
    assert_eq!(game["year"], 3000);
    assert!(game["next_due"].is_null());
}

#[test]
fn old_session_database_requires_explicit_offline_conversion() {
    let root = tempfile::tempdir().unwrap();
    let db = rusqlite::Connection::open(root.path().join(".web-sessions.db")).unwrap();
    db.execute_batch("CREATE TABLE web_sessions(token TEXT PRIMARY KEY,discord_id TEXT,expires INTEGER); INSERT INTO web_sessions VALUES ('old','123',9999999999);").unwrap();
    let error = match router(config(root.path())) {
        Ok(_) => panic!("accepted legacy schema"),
        Err(error) => error,
    };
    assert!(error.contains("offline"));
    assert_eq!(
        db.query_row("SELECT discord_id FROM web_sessions", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "123"
    );
}

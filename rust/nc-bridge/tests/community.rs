use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use nc_bridge::community::{Config, router};
use nc_host::admin::{self, Request as HostRequest};
use serde_json::Value;
use tower::ServiceExt;

fn fixture() -> (tempfile::TempDir, Router) {
    let dir = tempfile::tempdir().unwrap();
    let app = router(Config {
        games_root: dir.path().into(),
        nc_door: "/nonexistent/nc-door".into(),
        static_dir: "static".into(),
        origin: "https://game.example".into(),
        client_id: "123".into(),
        client_secret: "test-only".into(),
        max_sessions: 2,
    })
    .unwrap();
    for game in ["public", "draft"] {
        admin::execute(
            dir.path(),
            HostRequest::Create {
                game: game.into(),
                name: game.into(),
                players: 4,
                seed: Some(1515),
            },
        )
        .unwrap();
    }
    admin::execute(
        dir.path(),
        HostRequest::Publish {
            game: "public".into(),
        },
    )
    .unwrap();
    (dir, app)
}
fn session(dir: &std::path::Path, token: &str, id: &str) {
    rusqlite::Connection::open(dir.join(".web-sessions.db"))
        .unwrap()
        .execute(
            "INSERT OR REPLACE INTO account_profiles VALUES (?1,'Test Commander')",
            [format!("discord:{id}")],
        )
        .unwrap();
    rusqlite::Connection::open(dir.join(".web-sessions.db"))
        .unwrap()
        .execute(
            "INSERT INTO web_sessions VALUES (?1,?2,?3)",
            rusqlite::params![
                blake3::hash(token.as_bytes()).to_hex().to_string(),
                format!("discord:{id}"),
                chrono::Utc::now().timestamp() + 300
            ],
        )
        .unwrap();
}
async fn call(
    app: &Router,
    path: &str,
    method: &str,
    token: Option<&str>,
    origin: Option<&str>,
) -> (StatusCode, String) {
    let mut request = Request::builder().uri(path).method(method);
    if let Some(token) = token {
        request = request.header("cookie", format!("nc_session={token}"));
    }
    if let Some(origin) = origin {
        request = request.header("origin", origin);
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.headers()["cache-control"], "no-store");
    let status = response.status();
    (
        status,
        String::from_utf8(
            to_bytes(response.into_body(), 1024 * 1024)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap(),
    )
}

#[tokio::test]
async fn catalog_and_join_do_not_trust_query_identity() {
    let (dir, app) = fixture();
    let (status, body) = call(&app, "/api/games", "GET", None, None).await;
    assert_eq!(status, StatusCode::OK);
    let games: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(games.as_array().unwrap().len(), 1);
    assert_eq!(games[0]["game"], "public");
    assert!(games[0].get("seats").is_none());
    assert_eq!(
        call(
            &app,
            "/api/games/public/join?discord_id=123&seat=1",
            "POST",
            None,
            Some("https://game.example")
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    let token = "a".repeat(64);
    session(dir.path(), &token, "123");
    assert_eq!(
        call(&app, "/api/games/public/join", "POST", Some(&token), None)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(
            &app,
            "/api/games/public/join",
            "POST",
            Some(&token),
            Some("https://evil.example")
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (status, body) = call(
        &app,
        "/api/games/public/join?discord_id=456&seat=4",
        "POST",
        Some(&token),
        Some("https://game.example"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let seat: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(seat["account"]["id"], "123");
    assert_eq!(seat["seat"], 1);
    assert_eq!(
        call(
            &app,
            "/api/games/draft/join",
            "POST",
            Some(&token),
            Some("https://game.example")
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
}

#[tokio::test]
async fn exports_are_seat_scoped_and_logout_revokes_session() {
    let (dir, app) = fixture();
    let a = "a".repeat(64);
    let b = "b".repeat(64);
    session(dir.path(), &a, "123");
    session(dir.path(), &b, "456");
    call(
        &app,
        "/api/games/public/join",
        "POST",
        Some(&a),
        Some("https://game.example"),
    )
    .await;
    let path = "/api/games/public/exports/map-txt?discord_id=123&seat=1";
    assert_eq!(
        call(&app, path, "GET", None, None).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(&app, path, "GET", Some(&b), None).await.0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        call(&app, path, "GET", Some(&a), None).await.0,
        StatusCode::OK
    );
    assert_eq!(
        call(
            &app,
            "/auth/logout",
            "POST",
            Some(&a),
            Some("https://game.example")
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        call(&app, path, "GET", Some(&a), None).await.0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn oauth_callback_requires_matching_unexpired_state() {
    let (_dir, app) = fixture();
    assert_eq!(
        call(
            &app,
            "/auth/callback?code=bogus&state=bogus",
            "GET",
            None,
            None
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let response = app
        .oneshot(
            Request::builder()
                .uri("/auth/login")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let cookie = response.headers()["set-cookie"].to_str().unwrap();
    assert!(cookie.contains("HttpOnly"));
    assert!(cookie.contains("Secure"));
    assert!(cookie.contains("SameSite=Lax"));
    let location = response.headers()["location"].to_str().unwrap();
    assert!(location.starts_with("https://discord.com/oauth2/authorize?"));
    assert!(location.contains("scope=identify"));
    assert!(!location.contains("test-only"));
}

#[tokio::test]
async fn zip_download_has_binary_body_and_requires_own_seat() {
    let (dir, app) = fixture();
    let token = "a".repeat(64);
    session(dir.path(), &token, "123");
    admin::execute(
        dir.path(),
        HostRequest::Join {
            game: "public".into(),
            account: nc_data::AccountIdentity::Discord("123".into()),
        },
    )
    .unwrap();
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/games/public/exports/map-zip")
                .header("cookie", format!("nc_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["content-type"], "application/zip");
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert!(
        response.headers()["content-disposition"]
            .to_str()
            .unwrap()
            .contains("ECMAP-public-P1-Y3000.zip")
    );
    let bytes = to_bytes(response.into_body(), 1_000_000).await.unwrap();
    assert!(bytes.starts_with(b"PK\x03\x04"));
    let (status, _) = call(&app, "/api/games/public/exports/map-zip", "GET", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = call(
        &app,
        "/api/games/draft/exports/map-zip",
        "GET",
        Some(&token),
        None,
    )
    .await;
    assert_ne!(status, StatusCode::OK);
}

#[tokio::test]
async fn old_sessions_need_a_verified_discord_profile_before_play() {
    let (dir, app) = fixture();
    let token = "b".repeat(64);
    session(dir.path(), &token, "123");
    admin::execute(
        dir.path(),
        HostRequest::Join {
            game: "public".into(),
            account: nc_data::AccountIdentity::Discord("123".into()),
        },
    )
    .unwrap();
    rusqlite::Connection::open(dir.path().join(".web-sessions.db"))
        .unwrap()
        .execute("DELETE FROM account_profiles", [])
        .unwrap();
    let (status, body) = call(
        &app,
        "/api/games/public/access?display_name=Impostor",
        "GET",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(body.contains("Sign in again"));
}

#[tokio::test]
async fn browser_join_cannot_bypass_invite_only_seats() {
    let (dir, app) = fixture();
    admin::execute(
        dir.path(),
        HostRequest::InviteOnly {
            game: "public".into(),
        },
    )
    .unwrap();
    let issued = admin::execute(
        dir.path(),
        HostRequest::CreateInvites {
            game: "public".into(),
            count: 1,
        },
    )
    .unwrap();
    let code = issued["invites"][0]["code"].as_str().unwrap();
    let token = "c".repeat(64);
    session(dir.path(), &token, "123");
    assert_eq!(
        call(
            &app,
            "/api/games/public/join",
            "POST",
            Some(&token),
            Some("https://game.example")
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let (_, body) = call(&app, "/api/games", "GET", None, None).await;
    let games: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(games[0]["invite_required"], true);
    assert!(!body.contains(code));
    // The bot has verified the account before sending this local operation.
    admin::execute(
        dir.path(),
        HostRequest::ClaimInvite {
            game: "public".into(),
            account: nc_data::AccountIdentity::Discord("123".into()),
            code: code.into(),
        },
    )
    .unwrap();
    assert_eq!(
        call(&app, "/api/games/public/access", "GET", Some(&token), None)
            .await
            .0,
        StatusCode::OK
    );
}

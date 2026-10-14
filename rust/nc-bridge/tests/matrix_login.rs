#[path = "support/nostr.rs"]
mod support;
use axum::http::StatusCode;
use nc_bridge::community::router;
use nc_data::AccountIdentity;
use nc_data::matrix_login::{FAILURE_LIMIT, Issue, Issued, issue};
use serde_json::json;
use support::*;

const ORIGIN: &str = "https://game.example";

fn issued(root: &std::path::Path, user: &str) -> Issued {
    let conn = rusqlite::Connection::open(root.join(".web-sessions.db")).unwrap();
    let account = AccountIdentity::Matrix(user.into());
    match issue(&conn, &account, "Alice", chrono::Utc::now().timestamp()).unwrap() {
        Issue::Issued(issued) => issued,
        Issue::TooSoon => panic!("unexpected throttle"),
    }
}
fn body(code: &str) -> String {
    json!({"code":code}).to_string()
}

#[tokio::test]
async fn code_signs_in_once_with_matrix_identity_and_name() {
    let (root, app) = fixture();
    let code = issued(root.path(), "@alice:matrix.org").code;
    let (status, _, cookie) =
        send(&app, "/auth/matrix", "POST", "", "", ORIGIN, &body(&code)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(cookie.contains("HttpOnly") && cookie.contains("Secure"));
    let session = cookie.split(';').next().unwrap();
    let (status, me, _) = send(&app, "/api/me", "GET", session, "", "", "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        me["account"],
        json!({"provider":"matrix","id":"@alice:matrix.org"})
    );
    assert_eq!(me["display_name"], "Alice");
    let (status, _, cookie) =
        send(&app, "/auth/matrix", "POST", "", "", ORIGIN, &body(&code)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(!cookie.contains("nc_session"));
}

#[tokio::test]
async fn link_secret_signs_in_and_cancels_the_typed_code() {
    let (root, app) = fixture();
    let issued = issued(root.path(), "@alice:matrix.org");
    let (status, _, cookie) = send(
        &app,
        "/auth/matrix",
        "POST",
        "",
        "",
        ORIGIN,
        &body(&issued.link_secret),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(cookie.contains("nc_session="));
    let (status, _, _) = send(
        &app,
        "/auth/matrix",
        "POST",
        "",
        "",
        ORIGIN,
        &body(&issued.code),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn rejects_foreign_origin_and_malformed_requests_without_consuming_code() {
    let (root, app) = fixture();
    let code = issued(root.path(), "@alice:matrix.org").code;
    for (origin, request) in [
        ("https://evil.example", body(&code)),
        ("", body(&code)),
        (
            ORIGIN,
            json!({"code":code,"account":"@mallory:evil.example"}).to_string(),
        ),
        (ORIGIN, json!({"code":"A".repeat(129)}).to_string()),
        (ORIGIN, "not json".into()),
    ] {
        let (status, _, cookie) =
            send(&app, "/auth/matrix", "POST", "", "", origin, &request).await;
        assert!(
            matches!(status, StatusCode::FORBIDDEN | StatusCode::BAD_REQUEST),
            "{status} {request}"
        );
        assert!(!cookie.contains("nc_session"));
    }
    let (status, _, _) = send(&app, "/auth/matrix", "POST", "", "", ORIGIN, &body(&code)).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn repeated_wrong_codes_throttle_sign_in() {
    let (root, app) = fixture();
    let code = issued(root.path(), "@alice:matrix.org").code;
    for _ in 0..FAILURE_LIMIT {
        let (status, _, _) = send(
            &app,
            "/auth/matrix",
            "POST",
            "",
            "",
            ORIGIN,
            &body("WRONG234"),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
    let (status, _, _) = send(&app, "/auth/matrix", "POST", "", "", ORIGIN, &body(&code)).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
}

fn request(id: &str) -> String {
    json!({"matrix_id":id}).to_string()
}

#[tokio::test]
async fn site_request_queues_one_message_per_matrix_id() {
    let (root, app) = fixture();
    let path = "/auth/matrix/request";
    let (status, _, _) = send(
        &app,
        path,
        "POST",
        "",
        "",
        ORIGIN,
        &request(" @alice:example.org "),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _, _) = send(
        &app,
        path,
        "POST",
        "",
        "",
        ORIGIN,
        &request("@alice:example.org"),
    )
    .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    let conn = rusqlite::Connection::open(root.path().join(".web-sessions.db")).unwrap();
    let now = chrono::Utc::now().timestamp();
    assert_eq!(
        nc_data::matrix_login::take_requests(&conn, now).unwrap(),
        [AccountIdentity::Matrix("@alice:example.org".into())]
    );
}

#[tokio::test]
async fn site_request_rejects_bad_ids_foreign_origins_and_oauth_accounts() {
    let (_root, app) = fixture();
    let path = "/auth/matrix/request";
    for bad in ["alice", "@alice", "@:example.org", ""] {
        let (status, _, _) = send(&app, path, "POST", "", "", ORIGIN, &request(bad)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad}");
    }
    let (status, _, _) = send(
        &app,
        path,
        "POST",
        "",
        "",
        "https://evil.example",
        &request("@a:b.org"),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let extra = json!({"matrix_id":"@a:b.org","code":"x"}).to_string();
    let (status, _, _) = send(&app, path, "POST", "", "", ORIGIN, &extra).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    // With one-click sign-in configured, its own server's accounts are sent there.
    let root = tempfile::tempdir().unwrap();
    let mut config = config(root.path());
    config.matrix = Some(nc_bridge::community::MatrixOAuth::matrix_org(
        "id".into(),
        "secret".into(),
    ));
    let app = router(config).unwrap();
    let (status, body, _) = send(
        &app,
        path,
        "POST",
        "",
        "",
        ORIGIN,
        &request("@alice:Matrix.org"),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        body["error"]
            .as_str()
            .unwrap()
            .contains("Sign in with Matrix")
    );
    let (status, _, _) = send(
        &app,
        path,
        "POST",
        "",
        "",
        ORIGIN,
        &request("@alice:example.org"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

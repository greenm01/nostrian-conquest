#[path = "support/nostr.rs"]
mod support;
use axum::http::StatusCode;
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

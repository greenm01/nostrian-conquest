//! One-click Matrix sign-in against a mock authorization server and homeserver.
#[path = "support/nostr.rs"]
mod support;
use axum::{
    Form, Json, Router,
    body::Body,
    extract::State,
    http::{HeaderMap, Request, StatusCode},
    routing::{get, post},
};
use base64::Engine;
use nc_bridge::community::{MatrixOAuth, router};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use support::{config, fixture, send};
use tower::ServiceExt;

#[derive(Default)]
struct Mock {
    /// Userinfo reply; tests remove `username` to simulate a bad response.
    userinfo: Value,
    profile: Option<String>,
    tokens: Vec<HashMap<String, String>>,
    authorization: Vec<String>,
    revoked: Vec<String>,
}
type Shared = Arc<Mutex<Mock>>;

async fn mock_server(mock: Shared) -> String {
    let app = Router::new()
        .route(
            "/oauth2/token",
            post(
                |State(m): State<Shared>,
                 headers: HeaderMap,
                 Form(form): Form<HashMap<String, String>>| async move {
                    let mut m = m.lock().unwrap();
                    m.authorization
                        .push(headers["authorization"].to_str().unwrap().into());
                    m.tokens.push(form);
                    Json(json!({"access_token":"matrix-token","token_type":"Bearer"}))
                },
            ),
        )
        .route(
            "/oauth2/userinfo",
            get(|State(m): State<Shared>, headers: HeaderMap| async move {
                assert_eq!(headers["authorization"], "Bearer matrix-token");
                Json(m.lock().unwrap().userinfo.clone())
            }),
        )
        .route(
            "/oauth2/revoke",
            post(
                |State(m): State<Shared>, Form(form): Form<HashMap<String, String>>| async move {
                    m.lock().unwrap().revoked.push(form["token"].clone());
                    Json(json!({}))
                },
            ),
        )
        .route(
            "/_matrix/client/v3/profile/{user}/displayname",
            get(|State(m): State<Shared>| async move {
                match m.lock().unwrap().profile.clone() {
                    Some(name) => (StatusCode::OK, Json(json!({"displayname":name}))),
                    None => (
                        StatusCode::NOT_FOUND,
                        Json(json!({"errcode":"M_NOT_FOUND"})),
                    ),
                }
            }),
        )
        .with_state(mock);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(axum::serve(listener, app).into_future());
    format!("http://{address}")
}

async fn fixture_with_mock(
    userinfo: Value,
    profile: Option<&str>,
) -> (tempfile::TempDir, Router, Shared) {
    let mock: Shared = Arc::new(Mutex::new(Mock {
        userinfo,
        profile: profile.map(Into::into),
        ..Default::default()
    }));
    let base = mock_server(mock.clone()).await;
    let root = tempfile::tempdir().unwrap();
    let mut config = config(root.path());
    config.matrix = Some(MatrixOAuth {
        client_id: "nc-client".into(),
        client_secret: "nc-secret".into(),
        issuer: base.clone(),
        server_name: "matrix.org".into(),
        homeserver: base,
    });
    (root, router(config).unwrap(), mock)
}

async fn get_raw(app: &Router, uri: &str, cookie: &str) -> (StatusCode, HeaderMap) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(uri)
                .header("cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    (response.status(), response.headers().clone())
}

/// Starts sign-in and returns the redirect's query and the browser's state cookie.
async fn start(app: &Router) -> (HashMap<String, String>, String) {
    let (status, headers) = get_raw(app, "/auth/matrix/login", "").await;
    assert!(status.is_redirection());
    let location = reqwest::Url::parse(headers["location"].to_str().unwrap()).unwrap();
    assert_eq!(location.path(), "/authorize");
    let cookie = headers["set-cookie"].to_str().unwrap();
    assert!(
        cookie.contains("HttpOnly") && cookie.contains("Secure") && cookie.contains("SameSite=Lax")
    );
    (
        location.query_pairs().into_owned().collect(),
        cookie.split(';').next().unwrap().into(),
    )
}

fn session(headers: &HeaderMap) -> Option<String> {
    headers
        .get_all("set-cookie")
        .iter()
        .map(|v| v.to_str().unwrap())
        .find(|v| v.starts_with("nc_session=") && !v.starts_with("nc_session=;"))
        .map(|v| v.split(';').next().unwrap().into())
}

#[tokio::test]
async fn signs_in_with_only_openid_and_revokes_the_token() {
    let (_root, app, mock) = fixture_with_mock(
        json!({"sub":"01ABC","username":"alice"}),
        Some("Alice\u{7}"),
    )
    .await;
    let (query, cookie) = start(&app).await;
    assert_eq!(query["client_id"], "nc-client");
    assert_eq!(query["scope"], "openid");
    assert_eq!(query["response_type"], "code");
    assert_eq!(
        query["redirect_uri"],
        "https://game.example/auth/matrix/callback"
    );
    assert_eq!(query["code_challenge_method"], "S256");
    assert_eq!(cookie, format!("nc_matrix_login={}", query["state"]));
    let callback = format!("/auth/matrix/callback?code=abc&state={}", query["state"]);
    let (status, headers) = get_raw(&app, &callback, &cookie).await;
    assert!(status.is_redirection(), "{status}");
    assert_eq!(headers["location"], "/");
    let session = session(&headers).expect("session cookie");
    let (status, me, _) = send(&app, "/api/me", "GET", &session, "", "", "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        me["account"],
        json!({"provider":"matrix","id":"@alice:matrix.org"})
    );
    assert_eq!(me["display_name"], "Alice");
    let mock = mock.lock().unwrap();
    let token = &mock.tokens[0];
    assert_eq!(token["code"], "abc");
    assert_eq!(token["grant_type"], "authorization_code");
    assert_eq!(
        token["redirect_uri"],
        "https://game.example/auth/matrix/callback"
    );
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(Sha256::digest(token["code_verifier"].as_bytes()));
    assert_eq!(challenge, query["code_challenge"]);
    let basic = base64::engine::general_purpose::STANDARD.encode("nc-client:nc-secret");
    assert_eq!(mock.authorization[0], format!("Basic {basic}"));
    assert_eq!(mock.revoked, ["matrix-token"]);
}

#[tokio::test]
async fn state_is_single_use_and_bound_to_the_browser() {
    let (_root, app, mock) = fixture_with_mock(json!({"username":"alice"}), None).await;
    let (query, cookie) = start(&app).await;
    let callback = format!("/auth/matrix/callback?code=abc&state={}", query["state"]);
    let (status, _) = get_raw(&app, &callback, "nc_matrix_login=other").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, headers) = get_raw(&app, &callback, &cookie).await;
    assert!(status.is_redirection());
    assert!(session(&headers).is_some());
    let (status, headers) = get_raw(&app, &callback, &cookie).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(session(&headers).is_none());
    assert_eq!(mock.lock().unwrap().tokens.len(), 1);
}

#[tokio::test]
async fn profile_failure_falls_back_to_the_matrix_id() {
    let (_root, app, _mock) = fixture_with_mock(json!({"username":"alice"}), None).await;
    let (query, cookie) = start(&app).await;
    let callback = format!("/auth/matrix/callback?code=abc&state={}", query["state"]);
    let (_, headers) = get_raw(&app, &callback, &cookie).await;
    let (_, me, _) = send(
        &app,
        "/api/me",
        "GET",
        &session(&headers).unwrap(),
        "",
        "",
        "",
    )
    .await;
    assert_eq!(me["display_name"], "@alice:matrix.org");
}

#[tokio::test]
async fn cancelled_or_unusable_sign_in_creates_no_session() {
    let (_root, app, mock) = fixture_with_mock(json!({"sub":"01ABC"}), None).await;
    let (query, cookie) = start(&app).await;
    let cancelled = format!(
        "/auth/matrix/callback?error=access_denied&state={}",
        query["state"]
    );
    let (status, headers) = get_raw(&app, &cancelled, &cookie).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(session(&headers).is_none());
    let (query, cookie) = start(&app).await;
    let callback = format!("/auth/matrix/callback?code=abc&state={}", query["state"]);
    let (status, headers) = get_raw(&app, &callback, &cookie).await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
    assert!(session(&headers).is_none());
    assert_eq!(mock.lock().unwrap().revoked, ["matrix-token"]);
}

#[tokio::test]
async fn unconfigured_bridge_offers_no_oauth_sign_in() {
    let (_root, app) = fixture();
    assert_eq!(
        get_raw(&app, "/auth/matrix/login", "").await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        get_raw(
            &app,
            "/auth/matrix/callback?code=a&state=b",
            "nc_matrix_login=b"
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
}

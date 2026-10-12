#[path = "support/nostr.rs"]
mod support;
use axum::http::StatusCode;
use nostr_sdk::Keys;
use serde_json::json;
use support::*;

#[tokio::test]
async fn rejects_forgery_wrong_scope_and_unbound_browser_without_consuming_challenge() {
    let (_root, app) = fixture();
    let keys = Keys::generate();
    let (nonce, cookie) = challenge(&app).await;
    let body = json!({"challenge":nonce}).to_string();
    for fault in [
        "url",
        "method",
        "payload",
        "missing_payload",
        "duplicate",
        "past",
        "future",
        "kind",
        "content",
        "signature",
        "id",
    ] {
        assert_eq!(
            send(
                &app,
                "/auth/nostr",
                "POST",
                &cookie,
                &signed(&keys, &body, fault),
                "https://game.example",
                &body
            )
            .await
            .0,
            StatusCode::UNAUTHORIZED,
            "{fault}"
        );
    }
    let auth = signed(&keys, &body, "");
    assert_eq!(
        send(
            &app,
            "/auth/nostr",
            "POST",
            "",
            &auth,
            "https://game.example",
            &body
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        send(
            &app,
            "/auth/nostr",
            "POST",
            &cookie,
            &auth,
            "https://evil.example",
            &body
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        send(
            &app,
            "/auth/nostr",
            "POST",
            &cookie,
            "Nostr invalid",
            "https://game.example",
            &body
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        send(
            &app,
            "/auth/nostr?x=1",
            "POST",
            &cookie,
            &auth,
            "https://game.example",
            &body
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let changed = json!({"challenge":nonce,"display_name":"Changed"}).to_string();
    assert_eq!(
        send(
            &app,
            "/auth/nostr",
            "POST",
            &cookie,
            &auth,
            "https://game.example",
            &changed
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        send(
            &app,
            "/auth/nostr",
            "POST",
            &cookie,
            &auth,
            "https://game.example",
            &body
        )
        .await
        .0,
        StatusCode::OK
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn challenge_is_single_use_even_with_concurrent_login_and_expires() {
    let (root, app) = fixture();
    let keys = Keys::generate();
    let (nonce, cookie) = challenge(&app).await;
    let body = json!({"challenge":nonce}).to_string();
    let auth = signed(&keys, &body, "");
    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(2));
    let tasks = (0..2)
        .map(|_| {
            let (app, cookie, auth, body, barrier) = (
                app.clone(),
                cookie.clone(),
                auth.clone(),
                body.clone(),
                barrier.clone(),
            );
            tokio::spawn(async move {
                barrier.wait().await;
                send(
                    &app,
                    "/auth/nostr",
                    "POST",
                    &cookie,
                    &auth,
                    "https://game.example",
                    &body,
                )
                .await
                .0
                .as_u16()
            })
        })
        .collect::<Vec<_>>();
    let mut statuses = Vec::new();
    for task in tasks {
        statuses.push(task.await.unwrap());
    }
    statuses.sort();
    assert_eq!(statuses, [200, 401]);
    let (nonce, cookie) = challenge(&app).await;
    rusqlite::Connection::open(root.path().join(".web-sessions.db"))
        .unwrap()
        .execute("UPDATE nostr_challenges SET expires=0", [])
        .unwrap();
    let body = json!({"challenge":nonce}).to_string();
    assert_eq!(
        send(
            &app,
            "/auth/nostr",
            "POST",
            &cookie,
            &signed(&keys, &body, ""),
            "https://game.example",
            &body
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        send(
            &app,
            "/auth/nostr/challenge",
            "POST",
            "",
            "",
            "https://evil.example",
            ""
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
}

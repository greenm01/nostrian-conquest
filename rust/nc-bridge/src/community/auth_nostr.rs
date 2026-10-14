//! NIP-98 login signed by a NIP-07 extension; nothing is sent to a relay.
use super::*;
use base64::Engine;
use nc_data::AccountIdentity;
use nostr_sdk::Event;
use rusqlite::params;
use sha2::{Digest, Sha256};

pub(super) async fn challenge(State(state): State<Arc<Community>>, headers: HeaderMap) -> Response {
    if let Err(response) = check_origin(&state, &headers) {
        return response;
    }
    let token = auth::token();
    let expires = chrono::Utc::now().timestamp() + 300;
    let result = auth::connection(&state).and_then(|mut conn| {
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        tx.execute(
            "DELETE FROM nostr_challenges WHERE expires<=?1",
            [expires - 300],
        )
        .map_err(|e| e.to_string())?;
        if let Some(old) = auth::cookie(&headers, "nc_nostr_login") {
            tx.execute(
                "DELETE FROM nostr_challenges WHERE token=?1",
                [auth::digest(&old)],
            )
            .map_err(|e| e.to_string())?;
        }
        let count: u32 = tx
            .query_row("SELECT COUNT(*) FROM nostr_challenges", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if count >= 4096 {
            return Err("Login busy".into());
        }
        tx.execute(
            "INSERT INTO nostr_challenges VALUES (?1,?2)",
            params![auth::digest(&token), expires],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    });
    if result.is_err() {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Login unavailable; try again shortly",
        );
    }
    let mut response = Json(json!({"challenge":token,"expires":expires})).into_response();
    response.headers_mut().insert(
        "set-cookie",
        auth::set_cookie(&state, "nc_nostr_login", &token, 300)
            .parse()
            .unwrap(),
    );
    response
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LoginBody {
    challenge: String,
}

/// The key's public profile name from the configured relays, if any answers in
/// time. Relays sign nothing for us: the profile event must be signed by the key.
async fn profile_name(relays: &[String], key: nostr_sdk::PublicKey) -> Option<String> {
    if relays.is_empty() {
        return None;
    }
    let client = nostr_sdk::Client::default();
    for relay in relays {
        let _ = client.add_relay(relay.as_str()).await;
    }
    client.try_connect(std::time::Duration::from_secs(2)).await;
    let metadata = client
        .fetch_metadata(key, std::time::Duration::from_secs(3))
        .await;
    client.shutdown().await;
    let metadata = metadata.ok()??;
    [metadata.display_name, metadata.name]
        .into_iter()
        .flatten()
        .map(|name| {
            name.chars()
                .filter(|c| !c.is_control())
                .take(64)
                .collect::<String>()
                .trim()
                .to_owned()
        })
        .find(|name| !name.is_empty())
}

fn exact_tag(event: &Event, name: &str, expected: &str) -> bool {
    let mut tags = event
        .tags
        .iter()
        .map(|tag| tag.as_slice())
        .filter(|tag| tag.first().is_some_and(|value| value == name));
    matches!(tags.next(), Some(tag) if tag.len() == 2 && tag[1] == expected)
        && tags.next().is_none()
}

pub(super) async fn login(
    State(state): State<Arc<Community>>,
    headers: HeaderMap,
    uri: axum::http::Uri,
    body: axum::body::Bytes,
) -> Response {
    if let Err(response) = check_origin(&state, &headers) {
        return response;
    }
    // This endpoint has no query parameters. Reject rather than accidentally
    // validating a signature for a different absolute URL.
    if uri.query().is_some() {
        return error(StatusCode::BAD_REQUEST, "Unexpected login query");
    }
    let request: LoginBody = match serde_json::from_slice(&body) {
        Ok(request) => request,
        Err(_) => return error(StatusCode::BAD_REQUEST, "Invalid login request"),
    };
    if auth::cookie(&headers, "nc_nostr_login").as_deref() != Some(request.challenge.as_str()) {
        return error(
            StatusCode::UNAUTHORIZED,
            "Login challenge mismatch; try again",
        );
    }
    let event = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .filter(|h| h.len() <= 8192)
        .and_then(|h| h.strip_prefix("Nostr "))
        .and_then(|h| base64::engine::general_purpose::STANDARD.decode(h).ok())
        .and_then(|bytes| serde_json::from_slice::<Event>(&bytes).ok());
    let Some(event) = event else {
        return error(
            StatusCode::UNAUTHORIZED,
            "Missing or invalid Nostr signature",
        );
    };
    let now = chrono::Utc::now().timestamp();
    if event.verify().is_err()
        || event.kind.as_u16() != 27235
        || event.created_at.as_secs().abs_diff(now as u64) > 60
        || !event.content.is_empty()
        || !exact_tag(&event, "u", &format!("{}/auth/nostr", state.config.origin))
        || !exact_tag(&event, "method", "POST")
        || !exact_tag(&event, "payload", &format!("{:x}", Sha256::digest(&body)))
    {
        return error(
            StatusCode::UNAUTHORIZED,
            "Nostr signature invalid or expired; try again",
        );
    }
    let account = AccountIdentity::Nostr(event.pubkey.to_hex());
    let name = profile_name(&state.config.nostr_relays, event.pubkey).await;
    let result = auth::connection(&state).and_then(|mut conn| {
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        let consumed = tx
            .execute(
                "DELETE FROM nostr_challenges WHERE token=?1 AND expires>?2",
                params![auth::digest(&request.challenge), now],
            )
            .map_err(|e| e.to_string())?;
        if consumed != 1 {
            return Ok(None);
        }
        let token = auth::save_session(&tx, &account, name.as_deref())?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(Some(token))
    });
    match result {
        Ok(Some(token)) => {
            let mut response = Json(json!({"ok":true})).into_response();
            auth::session_cookie(&state, &mut response, &token);
            response.headers_mut().append(
                "set-cookie",
                auth::set_cookie(&state, "nc_nostr_login", "", 0)
                    .parse()
                    .unwrap(),
            );
            response
        }
        Ok(None) => error(
            StatusCode::UNAUTHORIZED,
            "Login challenge expired or used; try again",
        ),
        Err(_) => error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Session storage unavailable",
        ),
    }
}

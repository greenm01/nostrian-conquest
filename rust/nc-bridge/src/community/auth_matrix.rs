//! Matrix sign-in. Accounts on the configured authentication service (matrix.org)
//! use OAuth with only the `openid` scope, so the site never gains Matrix API
//! access. Other accounts use codes that `nc-matrix` sends to the verified sender
//! of a request; `login` redeems those.
use super::*;
use axum::response::Redirect;
use base64::Engine;
use nc_data::AccountIdentity;
use nc_data::matrix_login::{self, Redeem};
use rusqlite::{OptionalExtension, params};
use sha2::{Digest, Sha256};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LoginBody {
    code: String,
}

pub(super) async fn login(
    State(state): State<Arc<Community>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    if let Err(response) = check_origin(&state, &headers) {
        return response;
    }
    let request = match serde_json::from_slice::<LoginBody>(&body) {
        Ok(request) if request.code.len() <= 128 => request,
        _ => return error(StatusCode::BAD_REQUEST, "Invalid login request"),
    };
    let result = auth::connection(&state).and_then(|mut conn| {
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        let outcome = matrix_login::redeem(&tx, &request.code, chrono::Utc::now().timestamp())
            .map_err(|e| e.to_string())?;
        let token = match &outcome {
            Redeem::SignedIn {
                account,
                display_name,
            } => Some(auth::save_session(&tx, account, Some(display_name))?),
            Redeem::Invalid | Redeem::Throttled => None,
        };
        // Commit failures too, so they count toward the guessing limit.
        tx.commit().map_err(|e| e.to_string())?;
        Ok((outcome, token))
    });
    match result {
        Ok((_, Some(token))) => {
            let mut response = Json(json!({"ok":true})).into_response();
            auth::session_cookie(&state, &mut response, &token);
            response
        }
        Ok((Redeem::Throttled, _)) => error(
            StatusCode::TOO_MANY_REQUESTS,
            "Too many incorrect sign-in codes; try again in a few minutes",
        ),
        Ok(_) => error(
            StatusCode::UNAUTHORIZED,
            "Sign-in code is incorrect, used or expired; ask the bot for a new one",
        ),
        Err(_) => error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Session storage unavailable",
        ),
    }
}

pub(super) fn initialize(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS matrix_oauth_states(token TEXT PRIMARY KEY,
            verifier TEXT NOT NULL,expires INTEGER NOT NULL);",
    )
}

fn redirect_uri(state: &Community) -> String {
    format!("{}/auth/matrix/callback", state.config.origin)
}

pub(super) async fn oauth_login(State(state): State<Arc<Community>>) -> Response {
    let Some(oauth) = &state.config.matrix else {
        return error(StatusCode::NOT_FOUND, "Matrix sign-in is not configured");
    };
    let token = auth::token();
    let verifier = auth::token();
    let now = chrono::Utc::now().timestamp();
    let stored = auth::connection(&state).and_then(|conn| {
        conn.execute("DELETE FROM matrix_oauth_states WHERE expires<=?1", [now])
            .and_then(|_| {
                conn.execute(
                    "INSERT INTO matrix_oauth_states VALUES (?1,?2,?3)",
                    params![auth::digest(&token), verifier, now + 300],
                )
            })
            .map_err(|e| e.to_string())
    });
    if stored.is_err() {
        return error(StatusCode::SERVICE_UNAVAILABLE, "Login unavailable");
    }
    let challenge =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(Sha256::digest(&verifier));
    let mut url = match reqwest::Url::parse(&format!("{}/authorize", oauth.issuer)) {
        Ok(url) => url,
        Err(_) => return error(StatusCode::INTERNAL_SERVER_ERROR, "Invalid Matrix issuer"),
    };
    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", &oauth.client_id)
        .append_pair("redirect_uri", &redirect_uri(&state))
        .append_pair("scope", "openid")
        .append_pair("state", &token)
        .append_pair("code_challenge", &challenge)
        .append_pair("code_challenge_method", "S256");
    let mut response = Redirect::to(url.as_str()).into_response();
    response.headers_mut().insert(
        "set-cookie",
        auth::set_cookie(&state, "nc_matrix_login", &token, 300)
            .parse()
            .unwrap(),
    );
    response
}

#[derive(Deserialize)]
pub(super) struct Callback {
    code: Option<String>,
    state: String,
}

pub(super) async fn oauth_callback(
    State(state): State<Arc<Community>>,
    headers: HeaderMap,
    Query(query): Query<Callback>,
) -> Response {
    let Some(oauth) = state.config.matrix.clone() else {
        return error(StatusCode::NOT_FOUND, "Matrix sign-in is not configured");
    };
    if auth::cookie(&headers, "nc_matrix_login").as_deref() != Some(query.state.as_str()) {
        return error(StatusCode::BAD_REQUEST, "Login state mismatch");
    }
    let verifier = auth::connection(&state).and_then(|conn| {
        conn.query_row(
            "DELETE FROM matrix_oauth_states WHERE token=?1 AND expires>?2 RETURNING verifier",
            params![auth::digest(&query.state), chrono::Utc::now().timestamp()],
            |r| r.get::<_, String>(0),
        )
        .optional()
        .map_err(|e| e.to_string())
    });
    let Ok(Some(verifier)) = verifier else {
        return error(StatusCode::BAD_REQUEST, "Login expired; try again");
    };
    let Some(code) = query.code else {
        return error(StatusCode::BAD_REQUEST, "Matrix sign-in was cancelled");
    };
    let Ok((account, name)) = verify(&state, &oauth, &code, &verifier).await else {
        return error(StatusCode::BAD_GATEWAY, "Matrix sign-in failed; try again");
    };
    let result = auth::connection(&state).and_then(|mut conn| {
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        let token = auth::save_session(&tx, &account, Some(&name))?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(token)
    });
    let Ok(token) = result else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Session storage unavailable",
        );
    };
    let mut response = Redirect::to("/").into_response();
    auth::session_cookie(&state, &mut response, &token);
    response.headers_mut().append(
        "set-cookie",
        auth::set_cookie(&state, "nc_matrix_login", "", 0)
            .parse()
            .unwrap(),
    );
    response
}

/// Redeems the code, reads the account's username, and revokes the token: the
/// session needs nothing more from the authentication service.
async fn verify(
    state: &Community,
    oauth: &MatrixOAuth,
    code: &str,
    verifier: &str,
) -> Result<(AccountIdentity, String), Box<dyn std::error::Error + Send + Sync>> {
    let reply = state
        .http
        .post(format!("{}/oauth2/token", oauth.issuer))
        .basic_auth(&oauth.client_id, Some(&oauth.client_secret))
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri(state).as_str()),
            ("code_verifier", verifier),
        ])
        .send()
        .await?
        .error_for_status()?
        .json::<Value>()
        .await?;
    let access = reply["access_token"]
        .as_str()
        .ok_or("missing access token")?;
    let user = state
        .http
        .get(format!("{}/oauth2/userinfo", oauth.issuer))
        .bearer_auth(access)
        .send()
        .await?
        .error_for_status()?
        .json::<Value>()
        .await;
    let _ = state
        .http
        .post(format!("{}/oauth2/revoke", oauth.issuer))
        .basic_auth(&oauth.client_id, Some(&oauth.client_secret))
        .form(&[("token", access)])
        .send()
        .await;
    let username = user?["username"]
        .as_str()
        .ok_or("missing username")?
        .to_owned();
    let id = format!("@{username}:{}", oauth.server_name);
    let account = AccountIdentity::Matrix(id.clone());
    account.validate()?;
    let name = display_name(state, oauth, &id).await.unwrap_or(id);
    Ok((account, name))
}

async fn display_name(state: &Community, oauth: &MatrixOAuth, id: &str) -> Option<String> {
    let mut url = reqwest::Url::parse(&oauth.homeserver).ok()?;
    url.path_segments_mut()
        .ok()?
        .extend(["_matrix", "client", "v3", "profile", id, "displayname"]);
    let reply = state
        .http
        .get(url)
        .send()
        .await
        .ok()?
        .json::<Value>()
        .await
        .ok()?;
    let name: String = reply["displayname"]
        .as_str()?
        .chars()
        .filter(|c| !c.is_control())
        .take(64)
        .collect();
    Some(name.trim().to_owned()).filter(|name| !name.is_empty())
}

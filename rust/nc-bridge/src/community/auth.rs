use super::*;
use axum::response::Redirect;
use nc_data::AccountIdentity;
use rusqlite::{Connection, OptionalExtension, params};

pub(super) fn cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get("cookie")?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|part| {
            let (key, value) = part.trim().split_once('=')?;
            (key == name && value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit()))
                .then(|| value.to_owned())
        })
}
pub(super) fn digest(value: &str) -> String {
    blake3::hash(value.as_bytes()).to_hex().to_string()
}
pub(super) fn token() -> String {
    (0..32)
        .map(|_| format!("{:02x}", rand::random::<u8>()))
        .collect()
}
pub(super) fn connection(state: &Community) -> Result<Connection, String> {
    Connection::open(state.config.games_root.join(".web-sessions.db")).map_err(|e| e.to_string())
}
pub(super) fn initialize(config: &Config) -> Result<(), String> {
    std::fs::create_dir_all(&config.games_root).map_err(|e| e.to_string())?;
    let conn =
        Connection::open(config.games_root.join(".web-sessions.db")).map_err(|e| e.to_string())?;
    let legacy: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM pragma_table_info('web_sessions') WHERE name='discord_id')", [], |r| r.get(0)).map_err(|e| e.to_string())?;
    if legacy {
        return Err(
            "Run the offline community identity conversion before starting this bridge".into(),
        );
    }
    conn.execute_batch("CREATE TABLE IF NOT EXISTS web_sessions(token TEXT PRIMARY KEY,account TEXT NOT NULL,expires INTEGER NOT NULL);
        CREATE TABLE IF NOT EXISTS oauth_states(token TEXT PRIMARY KEY,expires INTEGER NOT NULL);
        CREATE TABLE IF NOT EXISTS account_profiles(account TEXT PRIMARY KEY,display_name TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS nostr_challenges(token TEXT PRIMARY KEY,expires INTEGER NOT NULL);") .map_err(|e|e.to_string())?;
    nc_data::matrix_login::ensure(&conn).map_err(|e| e.to_string())?;
    super::auth_matrix::initialize(&conn).map_err(|e| e.to_string())?;
    Ok(())
}
pub(super) fn identity(
    state: &Community,
    headers: &HeaderMap,
) -> Result<AccountIdentity, Response> {
    let token =
        cookie(headers, "nc_session").ok_or_else(|| error(StatusCode::UNAUTHORIZED, "Sign in"))?;
    let id = connection(state)
        .and_then(|conn| {
            conn.query_row(
                "SELECT account FROM web_sessions WHERE token=?1 AND expires>?2",
                params![digest(&token), chrono::Utc::now().timestamp()],
                |r| r.get::<_, AccountIdentity>(0),
            )
            .optional()
            .map_err(|e| e.to_string())
        })
        .map_err(|_| {
            error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Session storage unavailable",
            )
        })?;
    id.ok_or_else(|| error(StatusCode::UNAUTHORIZED, "Sign in"))
}
pub(super) fn set_cookie(state: &Community, name: &str, value: &str, age: u32) -> String {
    format!(
        "{name}={value}; Path=/; HttpOnly; SameSite=Lax; Max-Age={age}{}",
        if state.config.origin.starts_with("https://") {
            "; Secure"
        } else {
            ""
        }
    )
}
pub(super) async fn login(State(state): State<Arc<Community>>) -> Response {
    let token = token();
    let result = connection(&state).and_then(|conn| {
        conn.execute(
            "DELETE FROM oauth_states WHERE expires<=?1",
            [chrono::Utc::now().timestamp()],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO oauth_states(token,expires) VALUES (?1,?2)",
            params![digest(&token), chrono::Utc::now().timestamp() + 300],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    });
    if result.is_err() {
        return error(StatusCode::SERVICE_UNAVAILABLE, "Login unavailable");
    }
    let mut url = reqwest::Url::parse("https://discord.com/oauth2/authorize").unwrap();
    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", &state.config.client_id)
        .append_pair("scope", "identify")
        .append_pair(
            "redirect_uri",
            &format!("{}/auth/callback", state.config.origin),
        )
        .append_pair("state", &token);
    let mut response = Redirect::to(url.as_str()).into_response();
    response.headers_mut().insert(
        "set-cookie",
        set_cookie(&state, "nc_login", &token, 300).parse().unwrap(),
    );
    response
}
#[derive(Deserialize)]
pub(super) struct Callback {
    code: String,
    state: String,
}
pub(super) async fn callback(
    State(state): State<Arc<Community>>,
    headers: HeaderMap,
    Query(query): Query<Callback>,
) -> Response {
    if cookie(&headers, "nc_login").as_deref() != Some(query.state.as_str()) {
        return error(StatusCode::BAD_REQUEST, "Login state mismatch");
    }
    let consumed = connection(&state).and_then(|conn| {
        conn.execute(
            "DELETE FROM oauth_states WHERE token=?1 AND expires>?2",
            params![digest(&query.state), chrono::Utc::now().timestamp()],
        )
        .map_err(|e| e.to_string())
    });
    if !matches!(consumed, Ok(1)) {
        return error(StatusCode::BAD_REQUEST, "Login expired; try again");
    }
    let result = async {
        let reply = state
            .http
            .post("https://discord.com/api/oauth2/token")
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", query.code.as_str()),
                ("client_id", state.config.client_id.as_str()),
                ("client_secret", state.config.client_secret.as_str()),
                (
                    "redirect_uri",
                    &format!("{}/auth/callback", state.config.origin),
                ),
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
            .get("https://discord.com/api/users/@me")
            .bearer_auth(access)
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;
        let id = user["id"].as_str().ok_or("missing account ID")?.to_owned();
        if id.is_empty() || id.len() > 20 || !id.bytes().all(|b| b.is_ascii_digit()) {
            return Err("invalid identity".into());
        }
        let name = user["global_name"]
            .as_str()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| user["username"].as_str())
            .ok_or("missing display name")?
            .to_owned();
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>((id, name))
    }
    .await;
    let Ok((id, name)) = result else {
        return error(StatusCode::BAD_GATEWAY, "Discord login failed; try again");
    };
    let account = AccountIdentity::Discord(id);
    let result = connection(&state).and_then(|mut conn| {
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        let token = save_session(&tx, &account, Some(&name))?;
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
    session_cookie(&state, &mut response, &token);
    response.headers_mut().append(
        "set-cookie",
        set_cookie(&state, "nc_login", "", 0).parse().unwrap(),
    );
    response
}

/// Caller owns the transaction, including challenge consumption for Nostr.
pub(super) fn save_session(
    conn: &Connection,
    account: &AccountIdentity,
    name: Option<&str>,
) -> Result<String, String> {
    if let Some(name) = name {
        conn.execute(
            "INSERT INTO account_profiles(account,display_name) VALUES (?1,?2)
            ON CONFLICT(account) DO UPDATE SET display_name=excluded.display_name",
            params![account, name],
        )
        .map_err(|e| e.to_string())?;
    } else if let AccountIdentity::Nostr(key) = account {
        conn.execute(
            "INSERT OR IGNORE INTO account_profiles(account,display_name) VALUES (?1,?2)",
            params![account, format!("Nostr-{}", &key[..12])],
        )
        .map_err(|e| e.to_string())?;
    }
    let token = token();
    conn.execute(
        "INSERT INTO web_sessions(token,account,expires) VALUES (?1,?2,?3)",
        params![
            digest(&token),
            account,
            chrono::Utc::now().timestamp() + 86400 * 14
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(token)
}

pub(super) fn session_cookie(state: &Community, response: &mut Response, token: &str) {
    response.headers_mut().append(
        "set-cookie",
        set_cookie(state, "nc_session", token, 86400 * 14)
            .parse()
            .unwrap(),
    );
}

pub(super) fn display_name(state: &Community, id: &AccountIdentity) -> Result<String, Response> {
    let name = connection(state)
        .and_then(|conn| {
            conn.query_row(
                "SELECT display_name FROM account_profiles WHERE account=?1",
                [id],
                |r| r.get::<_, String>(0),
            )
            .optional()
            .map_err(|e| e.to_string())
        })
        .map_err(|_| {
            error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Profile storage unavailable",
            )
        })?;
    name.ok_or_else(|| {
        error(
            StatusCode::UNAUTHORIZED,
            "Sign in again to load your display name",
        )
    })
}
pub(super) async fn logout(State(state): State<Arc<Community>>, headers: HeaderMap) -> Response {
    if let Err(response) = check_origin(&state, &headers) {
        return response;
    }
    if let Some(token) = cookie(&headers, "nc_session") {
        if connection(&state)
            .and_then(|conn| {
                conn.execute("DELETE FROM web_sessions WHERE token=?1", [digest(&token)])
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            })
            .is_err()
        {
            return error(StatusCode::SERVICE_UNAVAILABLE, "Logout failed");
        }
    }
    let mut response = Json(json!({"ok":true})).into_response();
    response.headers_mut().insert(
        "set-cookie",
        set_cookie(&state, "nc_session", "", 0).parse().unwrap(),
    );
    response
}

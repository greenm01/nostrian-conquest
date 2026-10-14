//! Matrix sign-in bot. Players message it, or send `!login` in the community
//! room; it replies in a private room it creates with a one-time sign-in link
//! and code. It reads only senders, so encrypted direct messages also work.
pub mod sync;

use nc_data::AccountIdentity;
use nc_data::matrix_login::{self, Issue, Issued};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::time::Duration;
use sync::{Action, Rooms};

pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub struct Config {
    /// Client-server API base URL, such as `https://matrix-client.matrix.org`.
    pub homeserver: String,
    pub access_token: String,
    /// Community room alias or ID, such as `#nostrian-play:matrix.org`.
    pub room: String,
    pub games_root: PathBuf,
    /// Public games page origin, used in sign-in links.
    pub origin: String,
}

pub struct Client {
    http: reqwest::Client,
    base: String,
    token: String,
    txn: u64,
}

impl Client {
    pub fn new(homeserver: &str, token: &str) -> Result<Self, Error> {
        Ok(Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(90))
                .build()?,
            base: format!("{}/_matrix/client/v3", checked_url(homeserver)?),
            token: token.into(),
            txn: 0,
        })
    }

    /// Retries when the homeserver asks the bot to slow down.
    async fn call(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value, Error> {
        for _ in 0..5 {
            let mut request = self
                .http
                .request(method.clone(), format!("{}{path}", self.base))
                .bearer_auth(&self.token);
            if let Some(body) = &body {
                request = request.json(body);
            }
            let response = request.send().await?;
            let status = response.status();
            let value: Value = response.json().await.unwrap_or(Value::Null);
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                let wait = value["retry_after_ms"].as_u64().unwrap_or(5000).min(60_000);
                tokio::time::sleep(Duration::from_millis(wait)).await;
                continue;
            }
            if !status.is_success() {
                return Err(format!(
                    "{status} {}",
                    value["errcode"].as_str().unwrap_or("unknown error")
                )
                .into());
            }
            return Ok(value);
        }
        Err("homeserver rate limit persisted".into())
    }

    pub async fn whoami(&self) -> Result<String, Error> {
        let value = self
            .call(reqwest::Method::GET, "/account/whoami", None)
            .await?;
        Ok(value["user_id"].as_str().ok_or("missing user_id")?.into())
    }

    pub async fn join(&self, room: &str) -> Result<String, Error> {
        let value = self
            .call(
                reqwest::Method::POST,
                &format!("/join/{}", encode(room)),
                Some(json!({})),
            )
            .await?;
        Ok(value["room_id"].as_str().ok_or("missing room_id")?.into())
    }

    pub async fn sync(&self, since: Option<&str>) -> Result<Value, Error> {
        let filter = json!({
            "presence": {"not_types": ["*"]},
            "account_data": {"not_types": ["*"]},
            "room": {"timeline": {"limit": 20}, "ephemeral": {"not_types": ["*"]}}
        });
        let mut path = format!("/sync?timeout=5000&filter={}", encode(&filter.to_string()));
        if let Some(since) = since {
            path.push_str(&format!("&since={}", encode(since)));
        }
        self.call(reqwest::Method::GET, &path, None).await
    }

    async fn leave(&self, room: &str) -> Result<(), Error> {
        self.call(
            reqwest::Method::POST,
            &format!("/rooms/{}/leave", encode(room)),
            Some(json!({})),
        )
        .await
        .map(drop)
    }

    async fn display_name(&self, user: &str) -> Option<String> {
        let value = self
            .call(
                reqwest::Method::GET,
                &format!("/profile/{}/displayname", encode(user)),
                None,
            )
            .await
            .ok()?;
        let name: String = value["displayname"]
            .as_str()?
            .chars()
            .filter(|c| !c.is_control())
            .take(64)
            .collect();
        Some(name.trim().to_owned()).filter(|name| !name.is_empty())
    }

    /// An unencrypted private room, so the bot can read replies without keys.
    async fn create_direct(&self, user: &str) -> Result<String, Error> {
        let value = self
            .call(
                reqwest::Method::POST,
                "/createRoom",
                Some(json!({
                    "preset": "private_chat",
                    "is_direct": true,
                    "invite": [user],
                    "name": "Nostrian Conquest sign-in"
                })),
            )
            .await?;
        Ok(value["room_id"].as_str().ok_or("missing room_id")?.into())
    }

    async fn send(&mut self, room: &str, body: String, html: String) -> Result<(), Error> {
        self.txn += 1;
        let txn = format!("nc{}-{}", chrono::Utc::now().timestamp_millis(), self.txn);
        let path = format!("/rooms/{}/send/m.room.message/{txn}", encode(room));
        let content = json!({
            "msgtype": "m.text",
            "body": body,
            "format": "org.matrix.custom.html",
            "formatted_body": html
        });
        self.call(reqwest::Method::PUT, &path, Some(content))
            .await
            .map(drop)
    }
}

pub async fn run(config: Config) -> Result<(), Error> {
    let origin = checked_url(&config.origin)?;
    let database = config.games_root.join(".web-sessions.db");
    matrix_login::ensure(&rusqlite::Connection::open(&database)?)?;
    let mut client = Client::new(&config.homeserver, &config.access_token)?;
    let bot = client.whoami().await?;
    let community = client.join(&config.room).await?;
    tracing::info!(%bot, %community, "Matrix sign-in bot started");
    let mut rooms = Rooms {
        bot,
        community,
        direct: Default::default(),
    };
    let mut since: Option<String> = None;
    let mut delay = 1;
    loop {
        let batch = match client.sync(since.as_deref()).await {
            Ok(batch) => {
                delay = 1;
                batch
            }
            Err(error) => {
                tracing::warn!(%error, "Matrix sync failed");
                tokio::time::sleep(Duration::from_secs(delay)).await;
                delay = (delay * 2).min(60);
                continue;
            }
        };
        for action in sync::plan(&mut rooms, &batch, since.is_none()) {
            if let Err(error) = act(&mut client, &mut rooms, &database, &origin, action).await {
                tracing::warn!(%error, "Matrix action failed");
            }
        }
        since = batch["next_batch"].as_str().map(Into::into);
        let now = chrono::Utc::now().timestamp();
        let requests = match with_db(&database, move |conn| {
            Ok(matrix_login::take_requests(conn, now)?)
        })
        .await
        {
            Ok(requests) => requests,
            Err(error) => {
                tracing::warn!(%error, "Matrix site requests unavailable");
                Vec::new()
            }
        };
        for account in requests {
            if let AccountIdentity::Matrix(user) = account {
                let sent = send_code(&mut client, &mut rooms, &database, &origin, user, true).await;
                if let Err(error) = sent {
                    tracing::warn!(%error, "Matrix site request failed");
                }
            }
        }
    }
}

async fn act(
    client: &mut Client,
    rooms: &mut Rooms,
    database: &Path,
    origin: &str,
    action: Action,
) -> Result<(), Error> {
    match action {
        Action::Reject(room) | Action::Leave(room) => client.leave(&room).await,
        Action::Request(user) => send_code(client, rooms, database, origin, user, false).await,
    }
}

/// Sends a sign-in link and code. A request typed on the games page may name
/// anyone, so opening a new private room for one counts toward the hourly cap.
async fn send_code(
    client: &mut Client,
    rooms: &mut Rooms,
    database: &Path,
    origin: &str,
    user: String,
    from_site: bool,
) -> Result<(), Error> {
    let account = AccountIdentity::Matrix(user.clone());
    account.validate()?;
    let now = chrono::Utc::now().timestamp();
    if from_site
        && !rooms.direct.contains_key(&user)
        && !with_db(database, move |conn| {
            Ok(matrix_login::may_invite(conn, now)?)
        })
        .await?
    {
        tracing::warn!(%user, "Matrix site request skipped: hourly new-room limit reached");
        return Ok(());
    }
    let name = client
        .display_name(&user)
        .await
        .unwrap_or_else(|| user.clone());
    let issued = with_db(database, move |conn| {
        Ok(matrix_login::issue(conn, &account, &name, now)?)
    })
    .await?;
    let Issue::Issued(issued) = issued else {
        return Ok(());
    };
    let room = match rooms.direct.get(&user) {
        Some(room) => room.clone(),
        None => {
            let room = client.create_direct(&user).await?;
            rooms.direct.insert(user, room.clone());
            if from_site {
                with_db(database, move |conn| {
                    Ok(matrix_login::record_invite(conn, now)?)
                })
                .await?;
            }
            room
        }
    };
    let (body, html) = message(origin, &issued);
    client.send(&room, body, html).await
}

async fn with_db<T: Send + 'static>(
    database: &Path,
    f: impl FnOnce(&rusqlite::Connection) -> Result<T, Error> + Send + 'static,
) -> Result<T, Error> {
    let database = database.to_owned();
    tokio::task::spawn_blocking(move || f(&rusqlite::Connection::open(database)?)).await?
}

pub fn message(origin: &str, issued: &Issued) -> (String, String) {
    let link = format!("{origin}/#matrix-login={}", issued.link_secret);
    let minutes = matrix_login::LIFETIME_SECS / 60;
    (
        format!(
            "Someone asked to sign in to Nostrian Conquest as this Matrix account.\n\
            If it was you, open {link}\n\
            Or enter code {} on the games page.\n\
            The link and code work once and expire in {minutes} minutes. \
            If it wasn't you, ignore this message.",
            issued.code
        ),
        format!(
            "Someone asked to sign in to Nostrian Conquest as this Matrix account.<br>\
            If it was you, <a href=\"{link}\">sign in here</a>, \
            or enter code <code>{}</code> on the games page.<br>\
            The link and code work once and expire in {minutes} minutes. \
            If it wasn't you, ignore this message.",
            issued.code
        ),
    )
}

/// Prints a long-lived access token for the bot from a password login.
pub async fn login(homeserver: &str, user: &str, password: &str) -> Result<String, Error> {
    let url = format!("{}/_matrix/client/v3/login", checked_url(homeserver)?);
    let response = reqwest::Client::new()
        .post(url)
        .json(&json!({
            "type": "m.login.password",
            "identifier": {"type": "m.id.user", "user": user},
            "password": password,
            "initial_device_display_name": "nc-matrix"
        }))
        .send()
        .await?;
    let status = response.status();
    let value: Value = response.json().await.unwrap_or(Value::Null);
    if !status.is_success() {
        return Err(format!(
            "login failed: {status} {}",
            value["errcode"].as_str().unwrap_or("unknown error")
        )
        .into());
    }
    Ok(value["access_token"]
        .as_str()
        .ok_or("missing access_token")?
        .into())
}

/// HTTPS origin without path, or plain HTTP on loopback for tests.
fn checked_url(value: &str) -> Result<String, Error> {
    let url = reqwest::Url::parse(value)?;
    let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    if !(url.scheme() == "https" || url.scheme() == "http" && local)
        || url.host_str().is_none()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
        || !url.username().is_empty()
    {
        return Err(format!("{value} must be an HTTPS origin").into());
    }
    Ok(value.trim_end_matches('/').into())
}

fn encode(value: &str) -> String {
    utf8_percent_encode(value, NON_ALPHANUMERIC).to_string()
}

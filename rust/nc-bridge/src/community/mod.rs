//! Authenticated browser hosting. Legacy single-game bridge routes are separate.
mod auth;
mod auth_nostr;
mod terminal;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use nc_host::admin::Request;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

#[derive(Clone)]
pub struct Config {
    pub games_root: PathBuf,
    pub nc_door: PathBuf,
    pub static_dir: PathBuf,
    pub origin: String,
    pub client_id: String,
    pub client_secret: String,
    pub max_sessions: usize,
}
pub struct Community {
    _instance_lock: std::fs::File,
    config: Config,
    http: reqwest::Client,
    active: Mutex<HashMap<(String, u32), tokio::sync::watch::Sender<bool>>>,
}
pub fn router(config: Config) -> Result<Router, String> {
    let url = reqwest::Url::parse(&config.origin).map_err(|e| e.to_string())?;
    if url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("origin must be scheme and host only".into());
    }
    let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    if url.scheme() != "https" && !(url.scheme() == "http" && local) {
        return Err("HTTPS required except localhost".into());
    }
    if config.client_id.is_empty() || config.client_secret.is_empty() || config.max_sessions == 0 {
        return Err("Discord credentials and a positive session limit are required".into());
    }
    std::fs::create_dir_all(&config.games_root).map_err(|e| e.to_string())?;
    let instance_lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(config.games_root.join(".bridge.lock"))
        .map_err(|e| e.to_string())?;
    instance_lock
        .try_lock()
        .map_err(|_| "another bridge owns this games root".to_string())?;
    auth::initialize(&config)?;
    let assets = config.static_dir.clone();
    let state = Arc::new(Community {
        _instance_lock: instance_lock,
        config,
        http: reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| e.to_string())?,
        active: Mutex::new(HashMap::new()),
    });
    Ok(Router::new()
        .route(
            "/",
            get(|| async { Html(include_str!("../../static/community.html")) }),
        )
        .route("/auth/login", get(auth::login))
        .route("/auth/callback", get(auth::callback))
        .route("/auth/logout", post(auth::logout))
        .route("/auth/nostr/challenge", post(auth_nostr::challenge))
        .route("/auth/nostr", post(auth_nostr::login))
        .route("/api/games", get(games))
        .route("/api/me", get(me))
        .route("/api/games/{game}/join", post(join))
        .route("/api/games/{game}/access", get(access))
        .route("/api/games/{game}/reconnect", post(terminal::reconnect))
        .route("/api/games/{game}/exports/{format}", get(export))
        .route("/games/{game}/ws", get(terminal::connect))
        .route(
            "/health",
            get(|| async { Json(json!({"service":"nc-community","ok":true})) }),
        )
        .nest_service("/terminal", tower_http::services::ServeDir::new(assets))
        .layer(axum::extract::DefaultBodyLimit::max(16 * 1024))
        .layer(axum::middleware::from_fn(private_response))
        .with_state(state))
}
fn error(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({"error":message}))).into_response()
}
fn check_origin(state: &Community, headers: &HeaderMap) -> Result<(), Response> {
    if headers.get("origin").and_then(|h| h.to_str().ok()) != Some(state.config.origin.as_str()) {
        Err(error(StatusCode::FORBIDDEN, "Origin rejected"))
    } else {
        Ok(())
    }
}
async fn host(state: &Community, request: Request) -> Result<Value, Response> {
    let root = state.config.games_root.clone();
    tokio::task::spawn_blocking(move || nc_host::admin::execute(&root, request))
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "Host operation failed"))?
        .map_err(|e| error(StatusCode::CONFLICT, &e.to_string()))
}
async fn games(State(state): State<Arc<Community>>) -> Response {
    match host(&state, Request::Catalog).await {
        Ok(value) => Json(value).into_response(),
        Err(response) => response,
    }
}
async fn me(State(state): State<Arc<Community>>, headers: HeaderMap) -> Response {
    match auth::identity(&state, &headers) {
        Ok(id) => match auth::display_name(&state, &id) {
            Ok(name) => Json(json!({"account":id,"display_name":name})).into_response(),
            Err(response) => response,
        },
        Err(response) => response,
    }
}
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct JoinBody {
    code: Option<String>,
}

async fn join(
    State(state): State<Arc<Community>>,
    headers: HeaderMap,
    Path(game): Path<String>,
    body: axum::body::Bytes,
) -> Response {
    if let Err(response) = check_origin(&state, &headers) {
        return response;
    }
    let id = match auth::identity(&state, &headers) {
        Ok(id) => id,
        Err(response) => return response,
    };
    let body: JoinBody = if body.is_empty() {
        JoinBody::default()
    } else {
        match serde_json::from_slice(&body) {
            Ok(body) => body,
            Err(_) => return error(StatusCode::BAD_REQUEST, "Invalid join request"),
        }
    };
    let request = match body.code {
        Some(code) => Request::ClaimInvite {
            game,
            account: id,
            code,
        },
        None => Request::Join { game, account: id },
    };
    match host(&state, request).await {
        Ok(data) => Json(data).into_response(),
        Err(response) => response,
    }
}
async fn export(
    State(state): State<Arc<Community>>,
    headers: HeaderMap,
    Path((game, format)): Path<(String, String)>,
) -> Response {
    let id = match auth::identity(&state, &headers) {
        Ok(id) => id,
        Err(response) => return response,
    };
    let request = Request::Export {
        game,
        account: id,
        format,
    };
    match host(&state, request).await {
        Ok(data) => {
            let bytes = match nc_host::map_archive::download_bytes(&data) {
                Ok(bytes) => bytes,
                Err(_) => return error(StatusCode::INTERNAL_SERVER_ERROR, "Invalid download"),
            };
            let mut response = bytes.into_response();
            response.headers_mut().insert(
                "content-type",
                data["content_type"]
                    .as_str()
                    .unwrap_or("text/plain")
                    .parse()
                    .unwrap(),
            );
            response
                .headers_mut()
                .insert("cache-control", "no-store".parse().unwrap());
            if let Some(name) = data["filename"].as_str() {
                response.headers_mut().insert(
                    "content-disposition",
                    format!("attachment; filename=\"{name}\"").parse().unwrap(),
                );
            }
            response
        }
        Err(response) => response,
    }
}

async fn private_response(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert("cache-control", "no-store".parse().unwrap());
    response
        .headers_mut()
        .insert("referrer-policy", "no-referrer".parse().unwrap());
    response
        .headers_mut()
        .insert("x-content-type-options", "nosniff".parse().unwrap());
    response
}

async fn access(
    State(state): State<Arc<Community>>,
    headers: HeaderMap,
    Path(game): Path<String>,
) -> Response {
    let id = match auth::identity(&state, &headers) {
        Ok(id) => id,
        Err(response) => return response,
    };
    match host(
        &state,
        Request::Membership {
            game: game.clone(),
            account: id.clone(),
        },
    )
    .await
    {
        Ok(data) => {
            if data["can_play"] == true {
                if let Err(response) = auth::display_name(&state, &id) {
                    return response;
                }
            }
            let connected = data["seat"].as_u64().is_some_and(|seat| {
                state
                    .active
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .contains_key(&(game, seat as u32))
            });
            Json(json!({"seat":data["seat"],"can_play":data["can_play"],"state":data["state"],"connected":connected}))
                .into_response()
        }
        Err(response) => response,
    }
}

use super::*;
use axum::extract::{
    WebSocketUpgrade,
    ws::{Message, WebSocket},
};
use futures::{SinkExt, StreamExt};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    process::Command,
};

struct Lease {
    state: Arc<Community>,
    key: (String, u32),
    files: PathBuf,
    headers: HeaderMap,
}
impl Drop for Lease {
    fn drop(&mut self) {
        self.state
            .active
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&self.key);
        let _ = std::fs::remove_dir_all(&self.files);
    }
}
pub(super) async fn connect(
    State(state): State<Arc<Community>>,
    Path(game): Path<String>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    if let Err(response) = check_origin(&state, &headers) {
        return response;
    }
    let id = match auth::identity(&state, &headers) {
        Ok(id) => id,
        Err(response) => return response,
    };
    let access = match host(
        &state,
        Request::Authorize {
            game: game.clone(),
            discord_id: id.clone(),
        },
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return response,
    };
    if access["can_play"] != true {
        return error(StatusCode::CONFLICT, "Campaign is not accepting sessions");
    }
    let seat = access["seat"].as_u64().unwrap() as u32;
    let key = (game.clone(), seat);
    let (sender, cancel) = tokio::sync::watch::channel(false);
    {
        let mut active = state.active.lock().unwrap_or_else(|e| e.into_inner());
        if active.contains_key(&key) {
            return error(
                StatusCode::CONFLICT,
                "Seat already connected; use Reconnect",
            );
        }
        if active.len() >= state.config.max_sessions {
            return error(
                StatusCode::SERVICE_UNAVAILABLE,
                "All terminals are busy; try again shortly",
            );
        }
        active.insert(key.clone(), sender);
    }
    let files = state.config.games_root.join(".nodes").join(auth::token());
    let lease = Lease {
        state: state.clone(),
        key,
        files,
        headers,
    };
    ws.max_message_size(4096)
        .on_upgrade(move |socket| run(socket, lease, id, cancel))
}
pub(super) async fn reconnect(
    State(state): State<Arc<Community>>,
    Path(game): Path<String>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = check_origin(&state, &headers) {
        return response;
    }
    let id = match auth::identity(&state, &headers) {
        Ok(id) => id,
        Err(response) => return response,
    };
    let access = match host(
        &state,
        Request::Authorize {
            game: game.clone(),
            discord_id: id,
        },
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return response,
    };
    let key = (game, access["seat"].as_u64().unwrap() as u32);
    if let Some(sender) = state
        .active
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&key)
    {
        let _ = sender.send(true);
    }
    Json(json!({"ok":true})).into_response()
}
async fn run(
    mut ws: WebSocket,
    lease: Lease,
    id: String,
    mut cancel: tokio::sync::watch::Receiver<bool>,
) {
    if let Err(error) = pump(&mut ws, &lease, &id, &mut cancel).await {
        tracing::warn!(game=%lease.key.0,seat=lease.key.1,error=%error,"Managed terminal ended");
        let _ = ws
            .send(Message::Text(
                "\r\nSession ended. Return to the game list to reconnect.\r\n".into(),
            ))
            .await;
    }
    let _ = ws.close().await;
}
async fn pump(
    ws: &mut WebSocket,
    lease: &Lease,
    id: &str,
    cancel: &mut tokio::sync::watch::Receiver<bool>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    std::fs::create_dir_all(&lease.files)?;
    let dropfile = lease.files.join("DOOR32.SYS");
    // The legacy dropfile conveys terminal settings only. The authenticated
    // seat is passed separately by this trusted launcher, never by the browser.
    std::fs::write(
        &dropfile,
        format!(
            "2\r\n0\r\n38400\r\nNC Community\r\n1\r\nPlayer\r\nweb{}\r\n30\r\n30\r\n1\r\n80\r\n25\r\n",
            lease.key.1
        ),
    )?;
    let dir = nc_host::admin::game_dir(&lease.state.config.games_root, &lease.key.0)?;
    let mut child = Command::new(&lease.state.config.nc_door)
        .arg("--dir")
        .arg(dir)
        .arg("--player")
        .arg(lease.key.1.to_string())
        .arg("--dropfile")
        .arg(dropfile)
        .arg("--socket-port")
        .arg(port.to_string())
        .args(["--encoding", "utf8", "--color-mode", "ansi16"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()?;
    let result = async {
        let (mut tcp, _) =
            tokio::time::timeout(std::time::Duration::from_secs(5), listener.accept()).await??;
        let mut buffer = [0u8; 4096];
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(1800);
        let mut last_input = tokio::time::Instant::now();
        let mut heartbeat = tokio::time::interval(std::time::Duration::from_secs(5));
        loop {
            tokio::select! {
                _ = cancel.changed() => break,
                _ = tokio::time::sleep_until(deadline) => break,
                _ = heartbeat.tick() => {
                    if last_input.elapsed() > std::time::Duration::from_secs(300)
                        || !still_authorized(lease, id).await {
                        break;
                    }
                },
                read = tcp.read(&mut buffer) => {
                    let n = read?;
                    if n == 0 { break; }
                    ws.send(Message::Binary(buffer[..n].to_vec().into())).await?;
                },
                message = ws.next() => {
                    let bytes = match message {
                        Some(Ok(Message::Text(text))) => text.as_bytes().to_vec(),
                        Some(Ok(Message::Binary(bytes))) => bytes.to_vec(),
                        Some(Ok(Message::Ping(_) | Message::Pong(_))) => continue,
                        _ => break,
                    };
                    if !still_authorized(lease, id).await { break; }
                    tcp.write_all(&bytes).await?;
                    last_input = tokio::time::Instant::now();
                },
                _ = child.wait() => break,
            }
        }
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }
    .await;
    let _ = child.kill().await;
    let _ = child.wait().await;
    result
}

async fn still_authorized(lease: &Lease, id: &str) -> bool {
    if auth::identity(&lease.state, &lease.headers).ok().as_deref() != Some(id) {
        return false;
    }
    let access = host(
        &lease.state,
        Request::Authorize {
            game: lease.key.0.clone(),
            discord_id: id.into(),
        },
    )
    .await;
    matches!(access, Ok(ref value) if value["can_play"] == true && value["seat"] == lease.key.1)
}

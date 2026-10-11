use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::process::Child;
use tracing::{debug, error, info};

/// Pump bytes bidirectionally between the browser's WebSocket and `nc-door`'s TCP stream.
pub async fn bridge_session(
    ws: WebSocket,
    mut tcp: TcpStream,
    mut child: Child,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (mut tcp_read, mut tcp_write) = tcp.split();
    let (mut ws_sink, mut ws_stream) = ws.split();

    let mut tcp_buf = [0u8; 4096];

    loop {
        tokio::select! {
            // Read from nc-door TCP stream -> send to WebSocket as binary frame
            read_res = tcp_read.read(&mut tcp_buf) => {
                match read_res {
                    Ok(0) => {
                        debug!("nc-door TCP stream closed");
                        break;
                    }
                    Ok(n) => {
                        let data = tcp_buf[..n].to_vec();
                        if let Err(e) = ws_sink.send(Message::Binary(data.into())).await {
                            debug!("WebSocket send failed: {e}");
                            break;
                        }
                    }
                    Err(e) => {
                        error!("TCP read error: {e}");
                        break;
                    }
                }
            }

            // Read from WebSocket -> send bytes to nc-door TCP stream
            ws_msg = ws_stream.next() => {
                match ws_msg {
                    Some(Ok(Message::Binary(bin))) => {
                        if let Err(e) = tcp_write.write_all(&bin).await {
                            error!("TCP write error: {e}");
                            break;
                        }
                    }
                    Some(Ok(Message::Text(txt))) => {
                        if let Err(e) = tcp_write.write_all(txt.as_bytes()).await {
                            error!("TCP write error: {e}");
                            break;
                        }
                    }
                    Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) => {}
                    Some(Ok(Message::Close(_))) | None => {
                        debug!("WebSocket closed by client");
                        break;
                    }
                    Some(Err(e)) => {
                        debug!("WebSocket error: {e}");
                        break;
                    }
                }
            }

            // Monitor if child process finishes
            exit_status = child.wait() => {
                match exit_status {
                    Ok(status) => {
                        info!("nc-door process exited: {status}");
                    }
                    Err(e) => {
                        error!("Failed to wait on child process: {e}");
                    }
                }
                break;
            }
        }
    }

    // Terminate child if it's still alive after WebSocket disconnect
    let _ = child.kill().await;
    Ok(())
}

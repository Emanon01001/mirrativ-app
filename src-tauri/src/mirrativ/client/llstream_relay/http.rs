use tauri::AppHandle;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, watch};
use tokio::task::JoinHandle;
use tokio::time::Duration;

use super::relay_log;

pub(super) fn spawn_http_relay_task(
    app: AppHandle,
    listener: TcpListener,
    bootstrap: Vec<u8>,
    packet_tx: broadcast::Sender<Vec<u8>>,
    mut shutdown_rx: watch::Receiver<bool>,
    label: &str,
) -> JoinHandle<()> {
    let label = format!("llstream {} relay http", label);
    tokio::spawn(async move {
        relay_log(&app, &format!("{} listening", label));
        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        relay_log(&app, &format!("{} stopping", label));
                        return;
                    }
                }
                accepted = listener.accept() => {
                    match accepted {
                        Ok((socket, peer)) => {
                            relay_log(&app, &format!("{} client connected: {}", label, peer));
                            let mut rx = packet_tx.subscribe();
                            let bootstrap = bootstrap.clone();
                            let mut client_shutdown_rx = shutdown_rx.clone();
                            tokio::spawn(async move {
                                let _ = handle_http_client(socket, bootstrap, &mut rx, &mut client_shutdown_rx).await;
                            });
                        }
                        Err(e) => {
                            relay_log(&app, &format!("{} accept error: {}", label, e));
                            return;
                        }
                    }
                }
            }
        }
    })
}

async fn handle_http_client(
    mut socket: TcpStream,
    bootstrap: Vec<u8>,
    rx: &mut broadcast::Receiver<Vec<u8>>,
    shutdown_rx: &mut watch::Receiver<bool>,
) -> Result<(), String> {
    let mut req_buf = vec![0u8; 4096];
    let n = tokio::time::timeout(Duration::from_secs(3), socket.read(&mut req_buf))
        .await
        .map_err(|_| "http request timeout".to_string())?
        .map_err(|e| format!("http read failed: {}", e))?;
    if n == 0 {
        return Ok(());
    }

    let req = String::from_utf8_lossy(&req_buf[..n]);
    let first_line = req.lines().next().unwrap_or_default();
    let is_stream_request = first_line.starts_with("GET /live.ts") || first_line.starts_with("GET / ");

    if !is_stream_request {
        socket
            .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .await
            .map_err(|e| format!("http write 404 failed: {}", e))?;
        return Ok(());
    }

    socket
        .write_all(
            b"HTTP/1.1 200 OK\r\n\
              Content-Type: video/mp2t\r\n\
              Cache-Control: no-store\r\n\
              Access-Control-Allow-Origin: *\r\n\
              Connection: keep-alive\r\n\
              \r\n",
        )
        .await
        .map_err(|e| format!("http write header failed: {}", e))?;
    socket
        .write_all(&bootstrap)
        .await
        .map_err(|e| format!("http write bootstrap failed: {}", e))?;

    loop {
        tokio::select! {
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    return Ok(());
                }
            }
            recv = rx.recv() => {
                match recv {
                    Ok(chunk) => {
                        socket.write_all(&chunk).await.map_err(|e| format!("http stream write failed: {}", e))?;
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(broadcast::error::RecvError::Closed) => return Ok(()),
                }
            }
        }
    }
}

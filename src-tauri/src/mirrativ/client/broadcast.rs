use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, watch, RwLock};
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration, MissedTickBehavior};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// WebSocketストリームの型エイリアス
type WsStream =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;
type WsSink = futures_util::stream::SplitSink<WsStream, Message>;
type WsRead = futures_util::stream::SplitStream<WsStream>;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub struct BroadcastManager {
    shutdown_tx: Arc<RwLock<Option<watch::Sender<bool>>>>,
    task_handle: Arc<RwLock<Option<JoinHandle<()>>>>,
    outgoing_tx: Arc<RwLock<Option<mpsc::Sender<String>>>>,
}

impl BroadcastManager {
    pub fn new() -> Self {
        Self {
            shutdown_tx: Arc::new(RwLock::new(None)),
            task_handle: Arc::new(RwLock::new(None)),
            outgoing_tx: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn disconnect(&self) {
        *self.outgoing_tx.write().await = None;
        if let Some(tx) = self.shutdown_tx.write().await.take() {
            let _ = tx.send(true);
        }
        if let Some(handle) = self.task_handle.write().await.take() {
            handle.abort();
            let _ = handle.await;
        }
    }
}

/// Broadcast WSへ接続し、SUBまで完了したら broadcast://status に "subscribed" をemitする。
#[tauri::command]
pub async fn connect_broadcast(
    app: AppHandle,
    state: tauri::State<'_, BroadcastManager>,
    bcsvr_key: String,
    broadcast_host: String,
    // 任意: Cookie/UA を外から渡せるようにしておく（未指定ならブラウザっぽい固定値）
    cookie: Option<String>,
    user_agent: Option<String>,
) -> Result<(), String> {
    state.disconnect().await;

    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let (outgoing_tx, outgoing_rx) = mpsc::channel::<String>(64);

    *state.shutdown_tx.write().await = Some(shutdown_tx);
    *state.outgoing_tx.write().await = Some(outgoing_tx);

    let handle = tokio::spawn(ws_loop(
        app,
        bcsvr_key,
        broadcast_host,
        cookie,
        user_agent,
        shutdown_rx,
        outgoing_rx,
    ));
    *state.task_handle.write().await = Some(handle);

    Ok(())
}

#[tauri::command]
pub async fn disconnect_broadcast(
    app: AppHandle,
    state: tauri::State<'_, BroadcastManager>,
) -> Result<(), String> {
    state.disconnect().await;
    let _ = app.emit("broadcast://status", "disconnected");
    Ok(())
}

#[tauri::command]
pub async fn send_broadcast(
    state: tauri::State<'_, BroadcastManager>,
    message: String,
) -> Result<(), String> {
    let tx = state.outgoing_tx.read().await;
    match tx.as_ref() {
        Some(s) => s
            .send(message)
            .await
            .map_err(|_| "broadcast not connected".to_string()),
        None => Err("broadcast not connected".to_string()),
    }
}

// ---------------------------------------------------------------------------
// Internal
// ---------------------------------------------------------------------------

const BROWSER_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
     (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

fn blog(app: &AppHandle, msg: &str) {
    eprintln!("{}", msg);
    let _ = app.emit("broadcast://log", msg);
}

fn retry_delay(attempt: u32) -> Duration {
    use rand::RngExt;
    let base = 2000u64 * 2u64.saturating_pow(attempt);
    let jitter = rand::rng().random::<u64>() % 1000;
    Duration::from_millis(base.min(30_000) + jitter)
}

async fn ws_loop(
    app: AppHandle,
    bcsvr_key: String,
    broadcast_host: String,
    cookie: Option<String>,
    user_agent: Option<String>,
    mut shutdown_rx: watch::Receiver<bool>,
    mut outgoing_rx: mpsc::Receiver<String>,
) {
    let max_retries: u32 = 8;
    let mut retry_count: u32 = 0;

    loop {
        if *shutdown_rx.borrow() {
            return;
        }

        let url = format!("wss://{}/", broadcast_host);

        // --- request ---
        let request = match build_request(&url, cookie.as_deref(), user_agent.as_deref()) {
            Ok(r) => r,
            Err(e) => {
                blog(&app, &format!("broadcast: request error: {}", e));
                let _ = app.emit("broadcast://status", "error");
                if !wait_retry(&app, &mut shutdown_rx, &mut retry_count, max_retries).await {
                    return;
                }
                continue;
            }
        };

        // --- connect ---
        let ws = match connect_async(request).await {
            Ok((ws, _resp)) => ws,
            Err(e) => {
                blog(&app, &format!("broadcast: connect failed: {}", e));
                let _ = app.emit("broadcast://status", "error");
                if !wait_retry(&app, &mut shutdown_rx, &mut retry_count, max_retries).await {
                    return;
                }
                continue;
            }
        };

        retry_count = 0;
        blog(&app, &format!("broadcast: connected {}", url));
        let _ = app.emit("broadcast://status", "connected");

        let (mut sink, mut read) = ws.split();

        // --- phase 1: optional ping ---
        // ブラウザ互換: "PING\t" を送信（改行ではなくタブ終端）
        let _ = send_tab(&mut sink, "PING").await;
        let _ = wait_for_any(
            &app,
            &mut sink,
            &mut read,
            &mut shutdown_rx,
            &mut outgoing_rx,
            Duration::from_millis(1200),
            WaitPredicate::AckOrErrOrMsg,
        )
        .await;

        // --- phase 2: subscribe ---
        match subscribe(
            &app,
            &mut sink,
            &mut read,
            &mut shutdown_rx,
            &mut outgoing_rx,
            &bcsvr_key,
        )
        .await
        {
            SubOutcome::Subscribed => {
                blog(&app, "broadcast: subscribed");
                let _ = app.emit("broadcast://status", "subscribed");
            }
            SubOutcome::Shutdown => {
                let _ = sink.close().await;
                let _ = app.emit("broadcast://status", "disconnected");
                return;
            }
            SubOutcome::Disconnected => {
                let _ = app.emit("broadcast://status", "disconnected");
                if !wait_retry(&app, &mut shutdown_rx, &mut retry_count, max_retries).await {
                    return;
                }
                continue;
            }
        }

        // --- phase 3: main loop ---
        let mut ping_iv = tokio::time::interval(Duration::from_secs(20));
        ping_iv.set_missed_tick_behavior(MissedTickBehavior::Delay);
        let _ = ping_iv.tick().await; // 初回skip

        let disconnected = loop {
            tokio::select! {
                _ = ping_iv.tick() => {
                    if send_tab(&mut sink, "PING").await.is_err() {
                        blog(&app, "broadcast: keepalive failed");
                        break true;
                    }
                }
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        let _ = sink.close().await;
                        let _ = app.emit("broadcast://status", "disconnected");
                        return;
                    }
                }
                Some(msg) = outgoing_rx.recv() => {
                    // 行プロトコルの可能性が高いので、末尾に改行が無ければ付ける
                    if send_raw_user_text(&mut sink, &msg).await.is_err() {
                        blog(&app, "broadcast: send failed");
                        break true;
                    }
                }
                frame = read.next() => {
                    match frame {
                        Some(Ok(Message::Text(text))) => {
                            let _flags = handle_payload(&app, &mut sink, &text).await;
                        }
                        Some(Ok(Message::Binary(data))) => {
                            let text = String::from_utf8_lossy(&data);
                            let _flags = handle_payload(&app, &mut sink, &text).await;
                        }
                        Some(Ok(Message::Ping(data))) => {
                            let _ = sink.send(Message::Pong(data)).await;
                        }
                        Some(Ok(Message::Close(reason))) => {
                            blog(&app, &format!("broadcast: close {:?}", reason));
                            break true;
                        }
                        None => {
                            blog(&app, "broadcast: stream ended");
                            break true;
                        }
                        Some(Err(e)) => {
                            blog(&app, &format!("broadcast: ws error: {}", e));
                            break true;
                        }
                        _ => {}
                    }
                }
            }
        };

        if disconnected {
            let _ = app.emit("broadcast://status", "disconnected");
        }

        if !wait_retry(&app, &mut shutdown_rx, &mut retry_count, max_retries).await {
            return;
        }
    }
}

enum SubOutcome {
    Subscribed,
    Disconnected,
    Shutdown,
}

#[derive(Default, Debug, Clone)]
struct PayloadFlags {
    has_ack: bool,
    has_err: bool,
    has_msg: bool,
}

enum WaitPredicate {
    AckOrErrOrMsg,
}

async fn subscribe(
    app: &AppHandle,
    sink: &mut WsSink,
    read: &mut WsRead,
    shutdown_rx: &mut watch::Receiver<bool>,
    outgoing_rx: &mut mpsc::Receiver<String>,
    bcsvr_key: &str,
) -> SubOutcome {
    // ブラウザ互換: "SUB\t{key}" (タブ区切り、改行なし)
    let cmd = format!("SUB\t{}", bcsvr_key);

    blog(
        app,
        &format!("broadcast: tx {}", escape_for_log(cmd.trim_end())),
    );

    if sink.send(Message::Text(cmd.into())).await.is_err() {
        return SubOutcome::Disconnected;
    }

    // ACK/MSG/ERRを待つ。静かな配信ではメッセージが来ないことがある。
    let flags = wait_for_any(
        app,
        sink,
        read,
        shutdown_rx,
        outgoing_rx,
        Duration::from_millis(5000),
        WaitPredicate::AckOrErrOrMsg,
    )
    .await;

    if *shutdown_rx.borrow() {
        return SubOutcome::Shutdown;
    }

    if flags.has_err {
        blog(app, "broadcast: SUB rejected (ERR)");
        return SubOutcome::Disconnected;
    }

    if flags.has_msg || flags.has_ack {
        blog(app, "broadcast: SUB confirmed (ACK/MSG)");
    } else {
        // タイムアウト = ERRが返ってないので受理されたと見なす
        // (視聴者の少ない配信ではMSGが来ない)
        blog(app, "broadcast: SUB assumed ok (no ERR within timeout)");
    }

    SubOutcome::Subscribed
}

async fn wait_for_any(
    app: &AppHandle,
    sink: &mut WsSink,
    read: &mut WsRead,
    shutdown_rx: &mut watch::Receiver<bool>,
    outgoing_rx: &mut mpsc::Receiver<String>,
    timeout: Duration,
    _pred: WaitPredicate,
) -> PayloadFlags {
    let mut out = PayloadFlags::default();
    let t = sleep(timeout);
    tokio::pin!(t);

    loop {
        tokio::select! {
            _ = &mut t => {
                return out;
            }
            _ = shutdown_rx.changed() => {
                return out;
            }
            // subscribe中は outgoing を捨てる（必要ならキューに積む実装にしてもOK）
            Some(_msg) = outgoing_rx.recv() => {
                // drop
            }
            frame = read.next() => {
                match frame {
                    Some(Ok(Message::Text(text))) => {
                        let flags = handle_payload(app, sink, &text).await;
                        out.has_ack |= flags.has_ack;
                        out.has_err |= flags.has_err;
                        out.has_msg |= flags.has_msg;
                        if out.has_msg || out.has_ack || out.has_err { return out; }
                    }
                    Some(Ok(Message::Binary(data))) => {
                        let text = String::from_utf8_lossy(&data);
                        let flags = handle_payload(app, sink, &text).await;
                        out.has_ack |= flags.has_ack;
                        out.has_err |= flags.has_err;
                        out.has_msg |= flags.has_msg;
                        if out.has_msg || out.has_ack || out.has_err { return out; }
                    }
                    Some(Ok(Message::Ping(data))) => {
                        let _ = sink.send(Message::Pong(data)).await;
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        return out;
                    }
                    Some(Err(e)) => {
                        blog(app, &format!("broadcast: ws error: {}", e));
                        return out;
                    }
                    _ => {}
                }
            }
        }
    }
}

/// ブラウザと同等のヘッダーでリクエスト構築
fn build_request(
    url: &str,
    cookie: Option<&str>,
    user_agent: Option<&str>,
) -> Result<tokio_tungstenite::tungstenite::http::Request<()>, tokio_tungstenite::tungstenite::Error>
{
    let mut req = url.into_client_request()?;
    let h = req.headers_mut();

    h.insert(
        "Origin",
        HeaderValue::from_static("https://www.mirrativ.com"),
    );
    h.insert(
        "User-Agent",
        HeaderValue::from_str(user_agent.unwrap_or(BROWSER_UA))
            .unwrap_or_else(|_| HeaderValue::from_static(BROWSER_UA)),
    );
    h.insert("Accept-Language", HeaderValue::from_static("ja"));
    h.insert("Cache-Control", HeaderValue::from_static("no-cache"));
    h.insert("Pragma", HeaderValue::from_static("no-cache"));

    if let Some(c) = cookie {
        if let Ok(v) = HeaderValue::from_str(c) {
            h.insert("Cookie", v);
        }
    }

    Ok(req)
}

/// ブラウザ互換: タブ終端で送信 (e.g. "PING\t")
async fn send_tab(sink: &mut WsSink, cmd: &str) -> Result<(), ()> {
    let mut s = cmd.to_string();
    if !s.ends_with('\t') {
        s.push('\t');
    }
    sink.send(Message::Text(s.into())).await.map_err(|_| ())
}

async fn send_line(sink: &mut WsSink, cmd: &str) -> Result<(), ()> {
    let mut s = cmd.to_string();
    if !s.ends_with('\n') {
        s.push('\n');
    }
    sink.send(Message::Text(s.into())).await.map_err(|_| ())
}

/// フロントから来た文字列は「既に改行付き」の可能性があるので、無ければ付けるだけ。
async fn send_raw_user_text(sink: &mut WsSink, msg: &str) -> Result<(), ()> {
    let trimmed = msg.trim_end_matches(['\r', '\n']);
    let mut s = trimmed.to_string();
    s.push('\n');
    sink.send(Message::Text(s.into())).await.map_err(|_| ())
}

async fn wait_retry(
    app: &AppHandle,
    shutdown_rx: &mut watch::Receiver<bool>,
    retry_count: &mut u32,
    max_retries: u32,
) -> bool {
    *retry_count += 1;
    if *retry_count > max_retries {
        blog(
            app,
            &format!("broadcast: max retries ({}) reached", max_retries),
        );
        let _ = app.emit("broadcast://status", "failed");
        return false;
    }
    blog(
        app,
        &format!("broadcast: reconnecting ({}/{})", retry_count, max_retries),
    );
    let delay = retry_delay(*retry_count - 1);
    tokio::select! {
        _ = sleep(delay) => {}
        _ = shutdown_rx.changed() => {
            if *shutdown_rx.borrow() { return false; }
        }
    }
    true
}

/// サーバーペイロードを行ごとに処理（ACK/ERR/MSG をフラグ化して返す）
async fn handle_payload(app: &AppHandle, sink: &mut WsSink, payload: &str) -> PayloadFlags {
    let mut flags = PayloadFlags::default();

    for raw in payload.lines() {
        let line = raw.trim_matches(|c: char| c.is_whitespace() || c == '\0');
        if line.is_empty() {
            continue;
        }

        let mut parts = line.splitn(3, |c: char| c == '\t' || c == ' ');
        let cmd = parts.next().unwrap_or("");

        match cmd {
            "PING" => {
                // サーバからのアプリPING（WSフレームPINGとは別）
                let _ = send_tab(sink, "PONG").await;
            }
            "ACK" => {
                flags.has_ack = true;
            }
            "ERR" => {
                flags.has_err = true;
                blog(app, &format!("broadcast: rx {}", escape_for_log(line)));
            }
            "MSG" => {
                flags.has_msg = true;
                let _key = parts.next();
                if let Some(json_str) = parts.next() {
                    match serde_json::from_str::<Value>(json_str) {
                        Ok(val) => {
                            blog(app, &format!("broadcast: {}", summarize_msg(&val)));
                            let _ = app.emit("broadcast://message", &val);
                        }
                        Err(e) => {
                            blog(app, &format!("broadcast: JSON error: {}", e));
                        }
                    }
                } else {
                    blog(app, &format!("broadcast: rx {}", escape_for_log(line)));
                }
            }
            _ => {
                blog(app, &format!("broadcast: rx {}", escape_for_log(line)));
            }
        }
    }

    flags
}

/// 制御文字を可視化するログ用エスケープ
fn escape_for_log(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\t' => "\\t".to_string(),
            '\n' => "\\n".to_string(),
            '\r' => "\\r".to_string(),
            '\0' => "\\0".to_string(),
            c if c.is_control() => format!("\\x{:02x}", c as u32),
            c => c.to_string(),
        })
        .collect()
}

fn summarize_msg(v: &Value) -> String {
    let t = v.get("t").and_then(|x| x.as_i64()).unwrap_or(-1);
    match t {
        1 => {
            let lci = v.get("lci").and_then(|x| x.as_i64()).unwrap_or_default();
            let u = v.get("u").and_then(|x| x.as_str()).unwrap_or_default();
            let ac = v.get("ac").and_then(|x| x.as_str()).unwrap_or_default();
            let cm = v.get("cm").and_then(|x| x.as_str()).unwrap_or_default();
            format!(
                "MSG t=1 lci={} u={} ac={} cm={}",
                lci,
                u,
                ac,
                truncate(cm, 80)
            )
        }
        3 => {
            let u = v.get("u").and_then(|x| x.as_str()).unwrap_or_default();
            let ac = v.get("ac").and_then(|x| x.as_str()).unwrap_or_default();
            let n = v
                .get("online_viewer_num")
                .and_then(|x| x.as_i64())
                .unwrap_or_default();
            let text = v
                .get("cm")
                .and_then(|x| x.as_str())
                .or_else(|| v.get("message").and_then(|x| x.as_str()))
                .or_else(|| v.get("speech").and_then(|x| x.as_str()))
                .unwrap_or_default();
            if !ac.is_empty() {
                format!("MSG t=3 (join) u={} ac={} viewers={}", u, ac, n)
            } else if text.is_empty() {
                format!("MSG t=3 viewers={}", n)
            } else {
                format!("MSG t=3 viewers={} {}", n, truncate(text, 80))
            }
        }
        38 => "MSG t=38 (keepalive)".to_string(),
        123 => "MSG t=123 (broadcast ended)".to_string(),
        _ => format!("MSG t={}", t),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max).collect();
        out.push_str("...");
        out
    }
}

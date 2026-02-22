use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
#[cfg(windows)]
use tokio::net::windows::named_pipe::ServerOptions;
use tokio::sync::{broadcast, mpsc, watch, RwLock};
use tokio::task::JoinHandle;
use tokio::time::Duration;
use uuid::Uuid;

mod http;
mod mux;
mod parser;
mod ws;

use http::spawn_http_relay_task;
use mux::{build_bootstrap_tables, build_bootstrap_tables_av, AvMpegTsMuxer};
use parser::{
    ensure_annexb, extract_parameter_sets, has_nal_type, parse_video_packet,
    FRAME_KIND_IDR, FRAME_KIND_PPS, FRAME_KIND_SPS, NAL_START_CODE,
};
use ws::{
    run_audio_ws_to_av_samples_loop, run_video_ws_loop, run_video_ws_to_annexb_loop,
    run_video_ws_to_av_samples_loop,
};

// ---------------------------------------------------------------------------
// Manager
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct LlstreamRelayManager {
    shutdown_tx: Arc<RwLock<Option<watch::Sender<bool>>>>,
    task_handles: Arc<RwLock<Vec<JoinHandle<()>>>>,
    relay_url: Arc<RwLock<Option<String>>>,
}

impl LlstreamRelayManager {
    pub fn new() -> Self {
        Self::default()
    }

    async fn set_running(
        &self,
        shutdown_tx: watch::Sender<bool>,
        task_handles: Vec<JoinHandle<()>>,
        relay_url: String,
    ) {
        *self.shutdown_tx.write().await = Some(shutdown_tx);
        *self.task_handles.write().await = task_handles;
        *self.relay_url.write().await = Some(relay_url);
    }

    pub async fn current_url(&self) -> Option<String> {
        self.relay_url.read().await.clone()
    }

    pub async fn stop(&self) {
        *self.relay_url.write().await = None;

        if let Some(tx) = self.shutdown_tx.write().await.take() {
            let _ = tx.send(true);
        }

        let handles = {
            let mut guard = self.task_handles.write().await;
            std::mem::take(&mut *guard)
        };

        for handle in handles {
            handle.abort();
            let _ = handle.await;
        }
    }
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct LlstreamRelayInfo {
    pub playlist_url: String,
    pub mode: String,
    pub source: String,
}

#[tauri::command]
pub async fn start_llstream_video_ts_relay(
    app: AppHandle,
    state: tauri::State<'_, LlstreamRelayManager>,
    video_ws_url: String,
) -> Result<LlstreamRelayInfo, String> {
    let video_ws_url = video_ws_url.trim().to_string();
    if video_ws_url.is_empty() {
        return Err("video_ws_url is empty".to_string());
    }

    state.stop().await;

    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let (packet_tx, _packet_rx) = broadcast::channel::<Vec<u8>>(512);

    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .map_err(|e| format!("failed to bind relay server: {}", e))?;
    let addr = listener
        .local_addr()
        .map_err(|e| format!("failed to get local addr: {}", e))?;
    let relay_url = format!("http://127.0.0.1:{}/live.ts", addr.port());

    let bootstrap = build_bootstrap_tables();
    let http_task = spawn_http_relay_task(
        app.clone(), listener, bootstrap, packet_tx.clone(), shutdown_rx.clone(), "video",
    );

    let app_for_ws = app.clone();
    let mut ws_shutdown_rx = shutdown_rx.clone();
    let ws_task = tokio::spawn(async move {
        let result =
            run_video_ws_loop(&app_for_ws, &video_ws_url, packet_tx, &mut ws_shutdown_rx).await;
        if let Err(e) = result {
            relay_log(&app_for_ws, &format!("llstream relay ws error: {}", e));
        }
    });

    state
        .set_running(shutdown_tx, vec![http_task, ws_task], relay_url.clone())
        .await;
    let _ = app.emit("llstream://status", "started");

    Ok(LlstreamRelayInfo {
        playlist_url: relay_url,
        mode: "mpegts-video".to_string(),
        source: "llstream-video".to_string(),
    })
}

#[tauri::command]
pub async fn start_llstream_av_ts_relay(
    app: AppHandle,
    state: tauri::State<'_, LlstreamRelayManager>,
    video_ws_url: String,
    audio_ws_url: String,
) -> Result<LlstreamRelayInfo, String> {
    let video_ws_url = video_ws_url.trim().to_string();
    if video_ws_url.is_empty() {
        return Err("video_ws_url is empty".to_string());
    }
    let audio_ws_url = audio_ws_url.trim().to_string();
    if audio_ws_url.is_empty() {
        return Err("audio_ws_url is empty".to_string());
    }

    state.stop().await;

    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let (packet_tx, _packet_rx) = broadcast::channel::<Vec<u8>>(2048);
    let (sample_tx, mut sample_rx) = mpsc::channel::<AvSample>(4096);

    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .map_err(|e| format!("failed to bind relay server: {}", e))?;
    let addr = listener
        .local_addr()
        .map_err(|e| format!("failed to get local addr: {}", e))?;
    let relay_url = format!("http://127.0.0.1:{}/live.ts", addr.port());

    let bootstrap = build_bootstrap_tables_av();
    let http_task = spawn_http_relay_task(
        app.clone(), listener, bootstrap, packet_tx.clone(), shutdown_rx.clone(), "av",
    );

    let sample_tx_for_video = sample_tx.clone();
    let sample_tx_for_audio = sample_tx.clone();
    drop(sample_tx);

    let app_for_video_ws = app.clone();
    let mut video_shutdown_rx = shutdown_rx.clone();
    let video_ws_task = tokio::spawn(async move {
        let result = run_video_ws_to_av_samples_loop(
            &app_for_video_ws,
            &video_ws_url,
            sample_tx_for_video,
            &mut video_shutdown_rx,
        )
        .await;
        if let Err(e) = result {
            relay_log(&app_for_video_ws, &format!("llstream av video ws error: {}", e));
        }
    });

    let app_for_audio_ws = app.clone();
    let mut audio_shutdown_rx = shutdown_rx.clone();
    let audio_ws_task = tokio::spawn(async move {
        let result = run_audio_ws_to_av_samples_loop(
            &app_for_audio_ws,
            &audio_ws_url,
            sample_tx_for_audio,
            &mut audio_shutdown_rx,
        )
        .await;
        if let Err(e) = result {
            relay_log(&app_for_audio_ws, &format!("llstream av audio ws error: {}", e));
        }
    });

    let app_for_mux = app.clone();
    let mut mux_shutdown_rx = shutdown_rx.clone();
    let packet_tx_for_mux = packet_tx.clone();
    let mux_task = tokio::spawn(async move {
        let mut muxer = AvMpegTsMuxer::new();
        loop {
            tokio::select! {
                _ = mux_shutdown_rx.changed() => {
                    if *mux_shutdown_rx.borrow() {
                        relay_log(&app_for_mux, "llstream av relay mux stopping");
                        return;
                    }
                }
                sample = sample_rx.recv() => {
                    let Some(sample) = sample else {
                        return;
                    };

                    let chunk = match sample {
                        AvSample::Video { timestamp_ns, annexb } => {
                            muxer.push_video_access_unit(&annexb, timestamp_ns)
                        }
                        AvSample::Audio { timestamp_ns, adts_frame } => {
                            muxer.push_audio_adts_frame(&adts_frame, timestamp_ns)
                        }
                    };

                    if !chunk.is_empty() {
                        let _ = packet_tx_for_mux.send(chunk);
                    }
                }
            }
        }
    });

    state
        .set_running(
            shutdown_tx,
            vec![http_task, video_ws_task, audio_ws_task, mux_task],
            relay_url.clone(),
        )
        .await;
    let _ = app.emit("llstream://status", "started");

    Ok(LlstreamRelayInfo {
        playlist_url: relay_url,
        mode: "mpegts-av".to_string(),
        source: "llstream-av".to_string(),
    })
}

#[tauri::command]
pub async fn start_llstream_video_pipe_relay(
    app: AppHandle,
    state: tauri::State<'_, LlstreamRelayManager>,
    video_ws_url: String,
) -> Result<LlstreamRelayInfo, String> {
    let video_ws_url = video_ws_url.trim().to_string();
    if video_ws_url.is_empty() {
        return Err("video_ws_url is empty".to_string());
    }

    state.stop().await;

    #[cfg(not(windows))]
    {
        let _ = app;
        let _ = video_ws_url;
        return Err("named pipe relay is supported only on Windows".to_string());
    }

    #[cfg(windows)]
    {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let (au_tx, _au_rx) = broadcast::channel::<Vec<u8>>(1024);
        let pipe_path = format!(r"\\.\pipe\mirrativ_llstream_video_{}", Uuid::new_v4().simple());

        let app_for_pipe = app.clone();
        let mut pipe_shutdown_rx = shutdown_rx.clone();
        let pipe_path_for_task = pipe_path.clone();
        let au_tx_for_pipe = au_tx.clone();
        let pipe_task = tokio::spawn(async move {
            pipe_writer_loop(
                &app_for_pipe,
                &pipe_path_for_task,
                au_tx_for_pipe,
                &mut pipe_shutdown_rx,
            )
            .await;
        });

        let app_for_ws = app.clone();
        let mut ws_shutdown_rx = shutdown_rx.clone();
        let ws_url = video_ws_url.clone();
        let ws_task = tokio::spawn(async move {
            let result =
                run_video_ws_to_annexb_loop(&app_for_ws, &ws_url, au_tx, &mut ws_shutdown_rx).await;
            if let Err(e) = result {
                relay_log(&app_for_ws, &format!("llstream pipe ws error: {}", e));
            }
        });

        state
            .set_running(shutdown_tx, vec![pipe_task, ws_task], pipe_path.clone())
            .await;
        let _ = app.emit("llstream://status", "started");

        Ok(LlstreamRelayInfo {
            playlist_url: pipe_path,
            mode: "annexb-pipe".to_string(),
            source: "llstream-video".to_string(),
        })
    }
}

#[tauri::command]
pub async fn stop_llstream_relay(
    app: AppHandle,
    state: tauri::State<'_, LlstreamRelayManager>,
) -> Result<(), String> {
    state.stop().await;
    let _ = app.emit("llstream://status", "stopped");
    Ok(())
}

#[tauri::command]
pub async fn get_llstream_relay_url(
    state: tauri::State<'_, LlstreamRelayManager>,
) -> Result<Option<String>, String> {
    Ok(state.current_url().await)
}

// ---------------------------------------------------------------------------
// Shared helpers (used by submodules via super::)
// ---------------------------------------------------------------------------

fn relay_log(app: &AppHandle, msg: &str) {
    eprintln!("{}", msg);
    let _ = app.emit("llstream://log", msg);
}

fn reconnect_backoff(attempt: u64) -> Duration {
    // 250ms, 500ms, 1s, 2s ... capped at 4s
    let shift = attempt.min(4);
    Duration::from_millis(250u64.saturating_mul(1u64 << shift))
}

async fn wait_reconnect_or_shutdown(
    shutdown_rx: &mut watch::Receiver<bool>,
    delay: Duration,
) -> bool {
    tokio::select! {
        _ = shutdown_rx.changed() => *shutdown_rx.borrow(),
        _ = tokio::time::sleep(delay) => false,
    }
}

enum AvSample {
    Video { timestamp_ns: u64, annexb: Vec<u8> },
    Audio { timestamp_ns: u64, adts_frame: Vec<u8> },
}

// ---------------------------------------------------------------------------
// VideoFrameAssembler — shared SPS/PPS tracking + access unit construction
// ---------------------------------------------------------------------------

struct AssembledFrame {
    access_unit: Vec<u8>,
    timestamp_ns: u64,
    kind: u8,
}

struct VideoFrameAssembler {
    last_sps: Option<Vec<u8>>,
    last_pps: Option<Vec<u8>>,
    started: bool,
    waiting_log_counter: u64,
    prepend_aud: bool,
}

impl VideoFrameAssembler {
    fn new(prepend_aud: bool) -> Self {
        Self {
            last_sps: None,
            last_pps: None,
            started: false,
            waiting_log_counter: 0,
            prepend_aud,
        }
    }

    fn process(&mut self, data: &[u8], app: &AppHandle, label: &str) -> Option<AssembledFrame> {
        let frame = parse_video_packet(data)?;
        if frame.payload.is_empty() {
            return None;
        }

        let annexb = ensure_annexb(frame.payload);
        if annexb.is_empty() {
            return None;
        }

        if frame.kind == FRAME_KIND_SPS {
            let (sps, _) = extract_parameter_sets(&annexb);
            self.last_sps = sps.or_else(|| Some(annexb.clone()));
            return None;
        }
        if frame.kind == FRAME_KIND_PPS {
            let (_, pps) = extract_parameter_sets(&annexb);
            self.last_pps = pps.or_else(|| Some(annexb.clone()));
            return None;
        }

        let (sps_from_data, pps_from_data) = extract_parameter_sets(&annexb);
        if sps_from_data.is_some() {
            self.last_sps = sps_from_data;
        }
        if pps_from_data.is_some() {
            self.last_pps = pps_from_data;
        }

        let is_idr = frame.kind == FRAME_KIND_IDR || has_nal_type(&annexb, 5);

        if !self.started {
            if !(is_idr && self.last_sps.is_some() && self.last_pps.is_some()) {
                self.waiting_log_counter += 1;
                if self.waiting_log_counter % 120 == 0 {
                    relay_log(
                        app,
                        &format!(
                            "{} waiting keyframe/sps/pps: idr={} sps={} pps={}",
                            label, is_idr, self.last_sps.is_some(), self.last_pps.is_some()
                        ),
                    );
                }
                return None;
            }
            self.started = true;
            relay_log(app, &format!("{} start at first decodable keyframe", label));
        }

        let mut access_unit = Vec::with_capacity(annexb.len() + 512);

        if self.prepend_aud {
            access_unit.extend_from_slice(&NAL_START_CODE);
            access_unit.extend_from_slice(&[0x09, 0xF0]);
        }

        if is_idr {
            if !has_nal_type(&annexb, 7) {
                if let Some(sps) = self.last_sps.as_deref() {
                    access_unit.extend_from_slice(sps);
                }
            }
            if !has_nal_type(&annexb, 8) {
                if let Some(pps) = self.last_pps.as_deref() {
                    access_unit.extend_from_slice(pps);
                }
            }
        }

        access_unit.extend_from_slice(&annexb);

        Some(AssembledFrame {
            access_unit,
            timestamp_ns: frame.timestamp_ns,
            kind: frame.kind,
        })
    }
}

// ---------------------------------------------------------------------------
// Named pipe relay (Windows only)
// ---------------------------------------------------------------------------

#[cfg(windows)]
async fn pipe_writer_loop(
    app: &AppHandle,
    pipe_path: &str,
    au_tx: broadcast::Sender<Vec<u8>>,
    shutdown_rx: &mut watch::Receiver<bool>,
) {
    relay_log(app, &format!("llstream pipe listening: {}", pipe_path));

    loop {
        if *shutdown_rx.borrow() {
            relay_log(app, "llstream pipe stopping");
            return;
        }

        let mut pipe = match ServerOptions::new().create(pipe_path) {
            Ok(p) => p,
            Err(e) => {
                relay_log(app, &format!("llstream pipe create failed: {}", e));
                tokio::time::sleep(Duration::from_millis(250)).await;
                continue;
            }
        };

        let connect_result = tokio::select! {
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    relay_log(app, "llstream pipe connect cancelled");
                    return;
                }
                continue;
            }
            result = pipe.connect() => result
        };

        if let Err(e) = connect_result {
            relay_log(app, &format!("llstream pipe connect failed: {}", e));
            tokio::time::sleep(Duration::from_millis(150)).await;
            continue;
        }

        relay_log(app, "llstream pipe client connected");
        let mut rx = au_tx.subscribe();

        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        relay_log(app, "llstream pipe writer stopping");
                        return;
                    }
                }
                recv = rx.recv() => {
                    match recv {
                        Ok(au) => {
                            if let Err(e) = pipe.write_all(&au).await {
                                relay_log(app, &format!("llstream pipe client disconnected: {}", e));
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(skipped)) => {
                            relay_log(app, &format!("llstream pipe lagged: skipped {}", skipped));
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            relay_log(app, "llstream pipe channel closed");
                            return;
                        }
                    }
                }
            }
        }
    }
}

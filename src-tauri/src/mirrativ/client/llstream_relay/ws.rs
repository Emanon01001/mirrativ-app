use futures_util::StreamExt;
use std::ops::ControlFlow;
use tauri::AppHandle;
use tokio::sync::{broadcast, mpsc, watch};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use super::{relay_log, reconnect_backoff, wait_reconnect_or_shutdown, AvSample, VideoFrameAssembler};
use crate::mirrativ::client::llstream_relay::mux::{ns_to_90k, MpegTsMuxer};
use crate::mirrativ::client::llstream_relay::parser::{
    hex_prefix, nal_types_preview, parse_audio_packet, AacConfig, AUDIO_KIND_AAC, AUDIO_KIND_ASC,
};

// ---------------------------------------------------------------------------
// WsEvent + ws_reconnect_loop
// ---------------------------------------------------------------------------

pub(super) enum WsEvent {
    Connected,
    Binary(Vec<u8>),
}

/// Connects to a WebSocket URL with automatic reconnection and shutdown support.
/// The callback receives `WsEvent::Connected` once per connection and `WsEvent::Binary`
/// for each binary message. Return `ControlFlow::Break(())` to stop the loop entirely.
pub(super) async fn ws_reconnect_loop<F>(
    app: &AppHandle,
    ws_url: &str,
    label: &str,
    shutdown_rx: &mut watch::Receiver<bool>,
    mut on_event: F,
) -> Result<(), String>
where
    F: FnMut(WsEvent) -> ControlFlow<()>,
{
    let mut reconnect_attempt = 0u64;

    loop {
        if *shutdown_rx.borrow() {
            relay_log(app, &format!("{} stopping", label));
            return Ok(());
        }

        relay_log(app, &format!("{} connect: {}", label, ws_url));
        let ws = match connect_async(ws_url).await {
            Ok((ws, _resp)) => {
                reconnect_attempt = 0;
                relay_log(app, &format!("{} connected", label));
                ws
            }
            Err(e) => {
                reconnect_attempt += 1;
                let delay = reconnect_backoff(reconnect_attempt);
                relay_log(
                    app,
                    &format!(
                        "{} connect failed (attempt {}): {}. retry in {}ms",
                        label, reconnect_attempt, e, delay.as_millis()
                    ),
                );
                if wait_reconnect_or_shutdown(shutdown_rx, delay).await {
                    relay_log(app, &format!("{} stopping", label));
                    return Ok(());
                }
                continue;
            }
        };

        let _ = on_event(WsEvent::Connected);
        let (_write, mut read) = ws.split();
        let mut should_stop = false;

        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        relay_log(app, &format!("{} stopping", label));
                        return Ok(());
                    }
                }
                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Binary(data))) => {
                            if let ControlFlow::Break(()) = on_event(WsEvent::Binary(data.to_vec())) {
                                should_stop = true;
                                break;
                            }
                        }
                        Some(Ok(Message::Close(_))) => {
                            relay_log(app, &format!("{} closed, reconnecting", label));
                            break;
                        }
                        Some(Ok(_)) => {}
                        Some(Err(e)) => {
                            relay_log(app, &format!("{} read failed: {}", label, e));
                            break;
                        }
                        None => {
                            relay_log(app, &format!("{} ended, reconnecting", label));
                            break;
                        }
                    }
                }
            }
        }

        if should_stop {
            return Ok(());
        }

        reconnect_attempt += 1;
        let delay = reconnect_backoff(reconnect_attempt);
        if wait_reconnect_or_shutdown(shutdown_rx, delay).await {
            relay_log(app, &format!("{} stopping", label));
            return Ok(());
        }
    }
}

// ---------------------------------------------------------------------------
// Video WS → Annex B (for pipe relay)
// ---------------------------------------------------------------------------

pub(super) async fn run_video_ws_to_annexb_loop(
    app: &AppHandle,
    video_ws_url: &str,
    au_tx: broadcast::Sender<Vec<u8>>,
    shutdown_rx: &mut watch::Receiver<bool>,
) -> Result<(), String> {
    let mut assembler = VideoFrameAssembler::new(true);
    let mut au_sent: u64 = 0;
    let app_clone = app.clone();

    ws_reconnect_loop(
        app,
        video_ws_url,
        "llstream pipe ws",
        shutdown_rx,
        |event| {
            match event {
                WsEvent::Connected => {
                    assembler = VideoFrameAssembler::new(true);
                    au_sent = 0;
                }
                WsEvent::Binary(data) => {
                    if let Some(frame) = assembler.process(&data, &app_clone, "llstream pipe") {
                        au_sent += 1;
                        if au_sent <= 8 {
                            relay_log(
                                &app_clone,
                                &format!(
                                    "llstream au#{} kind=0x{:02x} bytes={} nals=[{}] head={}",
                                    au_sent, frame.kind, frame.access_unit.len(),
                                    nal_types_preview(&frame.access_unit, 8),
                                    hex_prefix(&frame.access_unit, 24)
                                ),
                            );
                        } else if au_sent % 300 == 0 {
                            relay_log(&app_clone, &format!("llstream au sent: {}", au_sent));
                        }
                        let _ = au_tx.send(frame.access_unit);
                    }
                }
            }
            ControlFlow::Continue(())
        },
    )
    .await
}

// ---------------------------------------------------------------------------
// Video WS → AvSample (for av mux)
// ---------------------------------------------------------------------------

pub(super) async fn run_video_ws_to_av_samples_loop(
    app: &AppHandle,
    video_ws_url: &str,
    sample_tx: mpsc::Sender<AvSample>,
    shutdown_rx: &mut watch::Receiver<bool>,
) -> Result<(), String> {
    let mut assembler = VideoFrameAssembler::new(true);
    let app_clone = app.clone();

    ws_reconnect_loop(
        app,
        video_ws_url,
        "llstream av video ws",
        shutdown_rx,
        |event| {
            match event {
                WsEvent::Connected => {
                    assembler = VideoFrameAssembler::new(true);
                }
                WsEvent::Binary(data) => {
                    if let Some(frame) = assembler.process(&data, &app_clone, "llstream av") {
                        match sample_tx.try_send(AvSample::Video {
                            timestamp_ns: frame.timestamp_ns,
                            annexb: frame.access_unit,
                        }) {
                            Ok(()) => {}
                            Err(mpsc::error::TrySendError::Closed(_)) => {
                                return ControlFlow::Break(());
                            }
                            Err(mpsc::error::TrySendError::Full(_)) => {
                                // Muxer is behind; drop this frame to avoid blocking.
                            }
                        }
                    }
                }
            }
            ControlFlow::Continue(())
        },
    )
    .await
}

// ---------------------------------------------------------------------------
// Audio WS → AvSample (for av mux)
// ---------------------------------------------------------------------------

pub(super) async fn run_audio_ws_to_av_samples_loop(
    app: &AppHandle,
    audio_ws_url: &str,
    sample_tx: mpsc::Sender<AvSample>,
    shutdown_rx: &mut watch::Receiver<bool>,
) -> Result<(), String> {
    let mut aac_config = AacConfig::default();
    let mut sent_audio = 0u64;
    let app_clone = app.clone();

    ws_reconnect_loop(
        app,
        audio_ws_url,
        "llstream av audio ws",
        shutdown_rx,
        |event| {
            match event {
                WsEvent::Connected => {
                    aac_config = AacConfig::default();
                    sent_audio = 0;
                }
                WsEvent::Binary(data) => {
                    let Some(frame) = parse_audio_packet(&data) else {
                        return ControlFlow::Continue(());
                    };
                    if frame.payload.is_empty() {
                        return ControlFlow::Continue(());
                    }

                    if frame.kind == AUDIO_KIND_ASC {
                        if let Some(new_cfg) = AacConfig::from_asc(frame.payload) {
                            aac_config = new_cfg;
                            relay_log(
                                &app_clone,
                                &format!(
                                    "llstream av audio config: aot={} sr_idx={} ch={}",
                                    aac_config.audio_object_type,
                                    aac_config.sample_rate_index,
                                    aac_config.channel_config
                                ),
                            );
                        }
                        return ControlFlow::Continue(());
                    }
                    if frame.kind != AUDIO_KIND_AAC {
                        return ControlFlow::Continue(());
                    }

                    let mut adts_frame = Vec::with_capacity(frame.payload.len() + 7);
                    adts_frame.extend_from_slice(&aac_config.adts_header(frame.payload.len()));
                    adts_frame.extend_from_slice(frame.payload);

                    sent_audio += 1;
                    if sent_audio <= 4 {
                        relay_log(
                            &app_clone,
                            &format!(
                                "llstream av audio#{} bytes={} head={}",
                                sent_audio, adts_frame.len(), hex_prefix(&adts_frame, 20)
                            ),
                        );
                    }

                    match sample_tx.try_send(AvSample::Audio {
                        timestamp_ns: frame.timestamp_ns,
                        adts_frame,
                    }) {
                        Ok(()) => {}
                        Err(mpsc::error::TrySendError::Closed(_)) => {
                            return ControlFlow::Break(());
                        }
                        Err(mpsc::error::TrySendError::Full(_)) => {
                            // Muxer is behind; drop this frame to avoid blocking.
                        }
                    }
                }
            }
            ControlFlow::Continue(())
        },
    )
    .await
}

// ---------------------------------------------------------------------------
// Video WS → MPEG-TS (for video-only ts relay)
// ---------------------------------------------------------------------------

pub(super) async fn run_video_ws_loop(
    app: &AppHandle,
    video_ws_url: &str,
    packet_tx: broadcast::Sender<Vec<u8>>,
    shutdown_rx: &mut watch::Receiver<bool>,
) -> Result<(), String> {
    let mut assembler = VideoFrameAssembler::new(false);
    let mut muxer = MpegTsMuxer::new();
    let app_clone = app.clone();

    ws_reconnect_loop(
        app,
        video_ws_url,
        "llstream ws",
        shutdown_rx,
        |event| {
            match event {
                WsEvent::Connected => {
                    assembler = VideoFrameAssembler::new(false);
                    muxer = MpegTsMuxer::new();
                }
                WsEvent::Binary(data) => {
                    if let Some(frame) = assembler.process(&data, &app_clone, "llstream relay") {
                        let pts_90k = ns_to_90k(frame.timestamp_ns);
                        let chunk = muxer.push_video_access_unit(&frame.access_unit, pts_90k);
                        let _ = packet_tx.send(chunk);
                    }
                }
            }
            ControlFlow::Continue(())
        },
    )
    .await
}

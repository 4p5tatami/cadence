use std::sync::{mpsc, Arc};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio::sync::{broadcast, oneshot};
use tokio_tungstenite::tungstenite::Message;

use crate::player::{LoadReply, PlayerMessage, StatusResponse};
use cadence_core::{Library, OutputState, PlayerMode, TrackRecord};

/// State broadcast sent to all clients every 100 ms.
#[derive(Serialize)]
struct StateMsg<'a> {
    #[serde(rename = "type")]
    msg_type: &'static str,
    track_path: &'a str,
    title: Option<&'a str>,
    artist: Option<&'a str>,
    duration_ms: u64,
    position_ms: u64,
    playing: bool,
    output_state: OutputState,
    playback_error: Option<&'a str>,
    snapshot_at_ms: u64,
    mode: &'a PlayerMode,
    volume: f32,
}

/// Broadcast instead of [`StateMsg`] when no track is loaded.
#[derive(Serialize)]
struct StoppedMsg {
    #[serde(rename = "type")]
    msg_type: &'static str,
    volume: f32,
}

/// Search results sent only to the requesting client.
#[derive(Serialize)]
struct SearchResultsMsg {
    #[serde(rename = "type")]
    msg_type: &'static str,
    query: String,
    tracks: Vec<TrackRecord>,
}

/// A failed command, sent only to the client that issued it.
#[derive(Serialize)]
struct ErrorMsg<'a> {
    #[serde(rename = "type")]
    msg_type: &'static str,
    message: &'a str,
}

fn error_json(message: &str) -> String {
    serde_json::to_string(&ErrorMsg {
        msg_type: "error",
        message,
    })
    .unwrap()
}

/// Sends a track-loading command and waits for the player's verdict.
/// Returns the error JSON to forward to the client if loading failed.
async fn load_command(
    ptx: &mpsc::Sender<PlayerMessage>,
    make: impl FnOnce(LoadReply) -> PlayerMessage,
) -> Option<String> {
    let (tx, rx) = oneshot::channel();
    ptx.send(make(tx)).ok()?;
    match rx.await {
        Ok(Err(message)) => Some(error_json(&message)),
        _ => None,
    }
}

/// Commands sent from clients to the server.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMsg {
    Play { path: String },
    Pause,
    Resume,
    Stop,
    Next,
    Previous,
    Seek { to_ms: u64 },
    SetVolume { volume: f32 },
    Search { query: String },
    SetMode { mode: PlayerMode },
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn state_json(status: &StatusResponse, volume: f32) -> String {
    let msg = StateMsg {
        msg_type: "state",
        track_path: &status.path,
        title: status.title.as_deref(),
        artist: status.artist.as_deref(),
        duration_ms: status.duration_ms,
        position_ms: status.position_ms,
        playing: status.is_playing(),
        output_state: status.output_state,
        playback_error: status.playback_error.as_deref(),
        snapshot_at_ms: now_ms(),
        mode: &status.mode,
        volume,
    };
    serde_json::to_string(&msg).unwrap()
}

fn stopped_json(volume: f32) -> String {
    serde_json::to_string(&StoppedMsg {
        msg_type: "stopped",
        volume,
    })
    .unwrap()
}

pub async fn serve(
    listener: TcpListener,
    player_tx: mpsc::Sender<PlayerMessage>,
    library: Arc<Library>,
    mut on_playback: impl FnMut(bool, Option<&StatusResponse>) + Send + 'static,
) {
    let (broadcast_tx, _) = broadcast::channel::<String>(32);
    let broadcast_tx = Arc::new(broadcast_tx);

    {
        let ptx = player_tx.clone();
        let btx = broadcast_tx.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(100)).await;
                let ptx2 = ptx.clone();
                let result = tokio::task::spawn_blocking(move || {
                    let (tx, rx) = mpsc::sync_channel(1);
                    ptx2.send(PlayerMessage::Status(tx)).ok();
                    rx.recv().ok()
                })
                .await;

                if let Ok(Some(snapshot)) = result {
                    on_playback(btx.receiver_count() > 0, snapshot.track.as_ref());
                    let wire = match &snapshot.track {
                        Some(status) => state_json(status, snapshot.volume),
                        None => stopped_json(snapshot.volume),
                    };
                    btx.send(wire).ok();
                }
            }
        });
    }

    while let Ok((stream, _addr)) = listener.accept().await {
        let ptx = player_tx.clone();
        let lib = Arc::clone(&library);
        let btx = broadcast_tx.clone();

        tokio::spawn(async move {
            let Ok(ws) = tokio_tungstenite::accept_async(stream).await else {
                return;
            };
            let mut brx = btx.subscribe();
            let (mut write, mut read) = ws.split();

            loop {
                tokio::select! {
                    msg = read.next() => {
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                let Ok(cmd) = serde_json::from_str::<ClientMsg>(&text) else { continue };
                                match cmd {
                                    ClientMsg::Play { path } => {
                                        let failure = load_command(&ptx, |tx| PlayerMessage::Play(path.into(), tx)).await;
                                        if let Some(reply) = failure {
                                            if write.send(Message::Text(reply)).await.is_err() { break; }
                                        }
                                    }
                                    ClientMsg::Next => {
                                        if let Some(reply) = load_command(&ptx, PlayerMessage::Next).await {
                                            if write.send(Message::Text(reply)).await.is_err() { break; }
                                        }
                                    }
                                    ClientMsg::Previous => {
                                        if let Some(reply) = load_command(&ptx, PlayerMessage::Previous).await {
                                            if write.send(Message::Text(reply)).await.is_err() { break; }
                                        }
                                    }
                                    ClientMsg::Pause    => { ptx.send(PlayerMessage::Pause).ok(); }
                                    ClientMsg::Resume   => { ptx.send(PlayerMessage::Resume).ok(); }
                                    ClientMsg::Stop     => { ptx.send(PlayerMessage::Stop).ok(); }
                                    ClientMsg::Seek { to_ms } => {
                                        ptx.send(PlayerMessage::Seek(to_ms)).ok();
                                    }
                                    ClientMsg::SetVolume { volume } => {
                                        ptx.send(PlayerMessage::SetVolume(volume)).ok();
                                    }
                                    ClientMsg::Search { query } => {
                                        let lib2 = Arc::clone(&lib);
                                        let q = query.clone();
                                        let tracks = tokio::task::spawn_blocking(move || {
                                            lib2.search(&q).unwrap_or_default()
                                        }).await.unwrap_or_default();

                                        let reply = serde_json::to_string(&SearchResultsMsg {
                                            msg_type: "search_results",
                                            query,
                                            tracks,
                                        }).unwrap();
                                        if write.send(Message::Text(reply)).await.is_err() { break; }
                                    }
                                    ClientMsg::SetMode { mode } => {
                                        ptx.send(PlayerMessage::SetMode(mode)).ok();
                                    }
                                }
                            }
                            Some(Ok(Message::Close(_)) | Err(_)) | None => break,
                            _ => {}
                        }
                    }
                    state = brx.recv() => {
                        match state {
                            Ok(s) => { if write.send(Message::Text(s)).await.is_err() { break; } }
                            Err(broadcast::error::RecvError::Lagged(_)) => {}
                            Err(_) => break,
                        }
                    }
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_clock_stops_during_recovery_and_errors() {
        let mut status = StatusResponse {
            path: "track.wav".into(),
            duration_ms: 60000,
            position_ms: 30000,
            paused: false,
            output_state: OutputState::Recovering,
            playback_error: None,
            title: None,
            artist: None,
            mode: PlayerMode::Default,
        };
        for (state, paused, playing) in [
            (OutputState::Recovering, false, false),
            (OutputState::Ready, true, false),
            (OutputState::Ready, false, true),
        ] {
            status.output_state = state;
            status.paused = paused;
            let wire: serde_json::Value = serde_json::from_str(&state_json(&status, 0.5)).unwrap();
            assert_eq!(wire["playing"], playing);
            assert_eq!(wire["volume"], 0.5);
            assert_eq!(wire["position_ms"], 30000);
            assert_eq!(wire["output_state"], serde_json::to_value(state).unwrap());
            assert_eq!(wire["type"], "state");
        }
        status.playback_error = Some("Cannot restore playback position".into());
        let wire: serde_json::Value = serde_json::from_str(&state_json(&status, 0.5)).unwrap();
        assert_eq!(wire["playback_error"], "Cannot restore playback position");
        assert_eq!(wire["output_state"], "ready");
        assert_eq!(wire["playing"], false);
    }

    #[test]
    fn error_message_wire_format() {
        let wire: serde_json::Value =
            serde_json::from_str(&error_json("Couldn't find \"x.flac\"")).unwrap();
        assert_eq!(wire["type"], "error");
        assert_eq!(wire["message"], "Couldn't find \"x.flac\"");
    }

    #[test]
    fn volume_is_reported_while_playing_and_stopped() {
        let wire: serde_json::Value = serde_json::from_str(&stopped_json(0.25)).unwrap();
        assert_eq!(wire["type"], "stopped");
        assert_eq!(wire["volume"], 0.25);
    }

    #[test]
    fn set_volume_command_parses() {
        let cmd: ClientMsg = serde_json::from_str(r#"{"type":"set_volume","volume":0.3}"#).unwrap();
        assert!(matches!(cmd, ClientMsg::SetVolume { volume } if volume == 0.3));
    }
}

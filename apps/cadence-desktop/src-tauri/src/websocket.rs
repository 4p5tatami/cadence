use std::sync::{mpsc, Arc};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::Message;

use cadence_core::{Library, PlayerMode, TrackRecord};
use crate::{PlayerMessage, StatusResponse};

/// State broadcast sent to all clients every 500 ms.
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
    snapshot_at_ms: u64,
    mode: &'a PlayerMode,
}

/// Search results sent only to the requesting client.
#[derive(Serialize)]
struct SearchResultsMsg {
    #[serde(rename = "type")]
    msg_type: &'static str,
    query: String,
    tracks: Vec<TrackRecord>,
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
    Search { query: String },
    SetMode { mode: PlayerMode },
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn state_json(status: &StatusResponse) -> String {
    let msg = StateMsg {
        msg_type: "state",
        track_path: &status.path,
        title: status.title.as_deref(),
        artist: status.artist.as_deref(),
        duration_ms: status.duration_ms,
        position_ms: status.position_ms,
        playing: !status.paused,
        snapshot_at_ms: now_ms(),
        mode: &status.mode,
    };
    serde_json::to_string(&msg).unwrap()
}

pub async fn serve(
    player_tx: mpsc::Sender<PlayerMessage>,
    library: Arc<Library>,
) {
    let listener = TcpListener::bind("0.0.0.0:7878").await
        .expect("Failed to bind WS server on port 7878");

    let (broadcast_tx, _) = broadcast::channel::<String>(32);
    let broadcast_tx = Arc::new(broadcast_tx);

    // Poll player every 500 ms and broadcast state to all clients.
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
                    rx.recv().ok().flatten()
                }).await;

                match result {
                    Ok(Some(status)) => { btx.send(state_json(&status)).ok(); }
                    Ok(None) => { btx.send(r#"{"type":"stopped"}"#.to_string()).ok(); }
                    _ => {}
                }
            }
        });
    }

    while let Ok((stream, _addr)) = listener.accept().await {
        let ptx = player_tx.clone();
        let lib = Arc::clone(&library);
        let mut brx = broadcast_tx.subscribe();

        tokio::spawn(async move {
            let Ok(ws) = tokio_tungstenite::accept_async(stream).await else { return };
            let (mut write, mut read) = ws.split();

            loop {
                tokio::select! {
                    msg = read.next() => {
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                let Ok(cmd) = serde_json::from_str::<ClientMsg>(&text) else { continue };
                                match cmd {
                                    ClientMsg::Pause    => { ptx.send(PlayerMessage::Pause).ok(); }
                                    ClientMsg::Resume   => { ptx.send(PlayerMessage::Resume).ok(); }
                                    ClientMsg::Stop     => { ptx.send(PlayerMessage::Stop).ok(); }
                                    ClientMsg::Next     => { ptx.send(PlayerMessage::Next).ok(); }
                                    ClientMsg::Previous => { ptx.send(PlayerMessage::Previous).ok(); }
                                    ClientMsg::Seek { to_ms } => {
                                        let (tx, _) = mpsc::sync_channel(1);
                                        ptx.send(PlayerMessage::Seek(to_ms, tx)).ok();
                                    }
                                    ClientMsg::Play { path } => {
                                        let (tx, _) = mpsc::sync_channel(1);
                                        ptx.send(PlayerMessage::Play(path.into(), tx)).ok();
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
                            Some(Ok(Message::Close(_))) | None => break,
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

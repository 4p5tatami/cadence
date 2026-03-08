use std::sync::mpsc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::Message;

use crate::{PlayerMessage, StatusResponse};

/// Message sent from server to clients on every state change.
#[derive(Serialize)]
struct ServerMsg<'a> {
    #[serde(rename = "type")]
    msg_type: &'static str,
    track_path: &'a str,
    duration_ms: u64,
    position_ms: u64,
    playing: bool,
    /// Wall-clock ms when this snapshot was taken.
    /// Receivers use: position_ms + (now - snapshot_at_ms) to extrapolate current position.
    snapshot_at_ms: u64,
}

/// Commands sent from clients to server.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMsg {
    Play { path: String },
    Pause,
    Resume,
    Stop,
    Seek { to_ms: u64 },
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn broadcast_json(status: &StatusResponse) -> String {
    let msg = ServerMsg {
        msg_type: "state",
        track_path: &status.path,
        duration_ms: status.duration_ms,
        position_ms: status.position_ms,
        playing: !status.paused,
        snapshot_at_ms: now_ms(),
    };
    serde_json::to_string(&msg).unwrap()
}

fn handle_client_msg(text: &str, player_tx: &mpsc::Sender<PlayerMessage>) {
    let Ok(msg) = serde_json::from_str::<ClientMsg>(text) else { return };
    match msg {
        ClientMsg::Pause  => { player_tx.send(PlayerMessage::Pause).ok(); }
        ClientMsg::Resume => { player_tx.send(PlayerMessage::Resume).ok(); }
        ClientMsg::Stop   => { player_tx.send(PlayerMessage::Stop).ok(); }
        ClientMsg::Seek { to_ms } => {
            let (tx, _) = mpsc::sync_channel(1);
            player_tx.send(PlayerMessage::Seek(to_ms, tx)).ok();
        }
        ClientMsg::Play { path } => {
            let (tx, _) = mpsc::sync_channel(1);
            player_tx.send(PlayerMessage::Play(path.into(), tx)).ok();
        }
    }
}

pub async fn serve(player_tx: mpsc::Sender<PlayerMessage>) {
    let listener = TcpListener::bind("0.0.0.0:7878").await
        .expect("Failed to bind WS server on port 7878");

    // Broadcast channel — all connected client tasks subscribe to this.
    let (broadcast_tx, _) = broadcast::channel::<String>(32);
    let broadcast_tx = std::sync::Arc::new(broadcast_tx);

    // Poll player every 500ms and broadcast current state to all clients.
    {
        let ptx = player_tx.clone();
        let btx = broadcast_tx.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(500)).await;
                let ptx2 = ptx.clone();
                let result = tokio::task::spawn_blocking(move || {
                    let (tx, rx) = mpsc::sync_channel(1);
                    ptx2.send(PlayerMessage::Status(tx)).ok();
                    rx.recv().ok().flatten()
                }).await;

                if let Ok(Some(status)) = result {
                    btx.send(broadcast_json(&status)).ok();
                }
            }
        });
    }

    // Accept connections.
    while let Ok((stream, _addr)) = listener.accept().await {
        let ptx = player_tx.clone();
        let mut brx = broadcast_tx.subscribe();

        tokio::spawn(async move {
            let Ok(ws) = tokio_tungstenite::accept_async(stream).await else { return };
            let (mut write, mut read) = ws.split();

            loop {
                tokio::select! {
                    // Incoming message from this client.
                    msg = read.next() => {
                        match msg {
                            Some(Ok(Message::Text(text))) => handle_client_msg(&text, &ptx),
                            Some(Ok(Message::Close(_))) | None => break,
                            _ => {}
                        }
                    }
                    // Outgoing broadcast to this client.
                    state = brx.recv() => {
                        match state {
                            Ok(s) => { if write.send(Message::Text(s)).await.is_err() { break; } }
                            Err(broadcast::error::RecvError::Lagged(_)) => {} // skip, not fatal
                            Err(_) => break,
                        }
                    }
                }
            }
        });
    }
}

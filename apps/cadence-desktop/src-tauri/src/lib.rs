use cadence_core::{Player, TrackInfo};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::mpsc;
use tauri::State;

enum PlayerMessage {
    Play(PathBuf, mpsc::SyncSender<Result<TrackInfo, String>>),
    Pause,
    Resume,
    Stop,
    Advance(i64, mpsc::SyncSender<Result<(), String>>),
    Status(mpsc::SyncSender<StatusResponse>),
}

struct PlayerHandle {
    tx: mpsc::Sender<PlayerMessage>,
}

// Safety: Sender<T> is Send+Sync when T: Send, which holds here
unsafe impl Sync for PlayerHandle {}

#[derive(Serialize)]
struct StatusResponse {
    path: Option<String>,
    duration_ms: Option<u64>,
    position_ms: u64,
    paused: bool,
}

fn spawn_player_thread() -> mpsc::Sender<PlayerMessage> {
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let mut player = Player::new().expect("Failed to create player");

        while let Ok(cmd) = rx.recv() {
            match cmd {
                PlayerMessage::Play(path, reply) => {
                    let result = player.load_and_play(path).map_err(|e| e.to_string());
                    reply.send(result).ok();
                }
                PlayerMessage::Pause => {
                    player.pause();
                }
                PlayerMessage::Resume => {
                    player.resume();
                }
                PlayerMessage::Stop => {
                    player.stop();
                }
                PlayerMessage::Advance(delta_ms, reply) => {
                    let result = player.advance_or_rewind(delta_ms).map_err(|e| e.to_string());
                    reply.send(result).ok();
                }
                PlayerMessage::Status(reply) => {
                    let position_ms = player.current_position_ms();
                    let paused = player
                        .current_track()
                        .map(|t| t.last_playback_time.is_none())
                        .unwrap_or(false);
                    reply.send(StatusResponse {
                        path: player.current_track().map(|t| t.info.path.to_string_lossy().into_owned()),
                        duration_ms: player.current_track().and_then(|t| t.info.duration_ms),
                        position_ms,
                        paused,
                    }).ok();
                }
            }
        }
    });

    tx
}

#[tauri::command]
fn play(path: String, handle: State<PlayerHandle>) -> Result<TrackInfo, String> {
    let (tx, rx) = mpsc::sync_channel(1);
    handle.tx.send(PlayerMessage::Play(PathBuf::from(path), tx)).ok();
    rx.recv().map_err(|_| "Player thread died".to_string())?
}

#[tauri::command]
fn pause(handle: State<PlayerHandle>) {
    handle.tx.send(PlayerMessage::Pause).ok();
}

#[tauri::command]
fn resume(handle: State<PlayerHandle>) {
    handle.tx.send(PlayerMessage::Resume).ok();
}

#[tauri::command]
fn stop(handle: State<PlayerHandle>) {
    handle.tx.send(PlayerMessage::Stop).ok();
}

#[tauri::command]
fn advance(delta_ms: i64, handle: State<PlayerHandle>) -> Result<(), String> {
    let (tx, rx) = mpsc::sync_channel(1);
    handle.tx.send(PlayerMessage::Advance(delta_ms, tx)).ok();
    rx.recv().map_err(|_| "Player thread died".to_string())?
}

#[tauri::command]
fn status(handle: State<PlayerHandle>) -> StatusResponse {
    let (tx, rx) = mpsc::sync_channel(1);
    handle.tx.send(PlayerMessage::Status(tx)).ok();
    rx.recv().unwrap_or(StatusResponse {
        path: None,
        duration_ms: None,
        position_ms: 0,
        paused: false,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(PlayerHandle { tx: spawn_player_thread() })
        .invoke_handler(tauri::generate_handler![
            play, pause, resume, stop, advance, status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

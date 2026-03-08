mod websocket;

use cadence_core::{Library, LibraryRecord, Player, TrackInfo, TrackRecord};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::mpsc;
use tauri::{Manager, State};

pub(crate) enum PlayerMessage {
    Play(PathBuf, mpsc::SyncSender<Result<TrackInfo, String>>),
    Pause,
    Resume,
    Stop,
    Advance(i64, mpsc::SyncSender<Result<(), String>>),
    Seek(u64, mpsc::SyncSender<Result<(), String>>),
    Status(mpsc::SyncSender<Option<StatusResponse>>),
}

struct PlayerHandle {
    tx: mpsc::Sender<PlayerMessage>,
}

// Safety: Sender<T> is Send+Sync when T: Send, which holds here
unsafe impl Sync for PlayerHandle {}

#[derive(Serialize)]
pub(crate) struct StatusResponse {
    pub path: String,
    pub duration_ms: u64,
    pub position_ms: u64,
    pub paused: bool,
    pub title: Option<String>,
    pub artist: Option<String>,
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
                PlayerMessage::Seek(to_ms, reply) => {
                    let result = player.seek(to_ms).map_err(|e| e.to_string());
                    reply.send(result).ok();
                }
                PlayerMessage::Status(reply) => {
                    let status = player.current_track().map(|track| StatusResponse {
                        path: track.info.path.to_string_lossy().into_owned(),
                        duration_ms: track.info.duration_ms,
                        position_ms: player.current_position_ms(),
                        paused: track.last_playback_timestamp.is_none(),
                        title: track.info.title.clone(),
                        artist: track.info.artist.clone(),
                    });
                    reply.send(status).ok();
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
fn seek(to_ms: u64, handle: State<PlayerHandle>) -> Result<(), String> {
    let (tx, rx) = mpsc::sync_channel(1);
    handle.tx.send(PlayerMessage::Seek(to_ms, tx)).ok();
    rx.recv().map_err(|_| "Player thread died".to_string())?
}

#[tauri::command]
fn status(handle: State<PlayerHandle>) -> Option<StatusResponse> {
    let (tx, rx) = mpsc::sync_channel(1);
    handle.tx.send(PlayerMessage::Status(tx)).ok();
    rx.recv().ok().flatten()
}

#[tauri::command]
fn ws_address() -> String {
    // Determine the local LAN IP by routing toward an external address.
    // No packet is actually sent — UDP connect just sets the local address.
    let ip = (|| -> Option<String> {
        let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
        socket.connect("8.8.8.8:80").ok()?;
        Some(socket.local_addr().ok()?.ip().to_string())
    })()
    .unwrap_or_else(|| "localhost".to_string());

    format!("ws://{}:7878", ip)
}

#[tauri::command]
fn index_library(path: String, library: State<Library>) -> Result<usize, String> {
    library.index_directory(std::path::Path::new(&path))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn search_tracks(query: String, library: State<Library>) -> Result<Vec<TrackRecord>, String> {
    library.search(&query).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_libraries(library: State<Library>) -> Result<Vec<LibraryRecord>, String> {
    library.list_libraries().map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_library(id: i64, library: State<Library>) -> Result<(), String> {
    library.delete_library(id).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let player_tx = spawn_player_thread();

    // Spawn the WebSocket server on Tauri's async runtime.
    tauri::async_runtime::spawn(websocket::serve(player_tx.clone()));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let db_path = app.path().app_data_dir()
                .expect("Failed to get app data dir")
                .join("cadence.db");
            let library = Library::open(&db_path)
                .expect("Failed to open library database");
            app.manage(library);
            Ok(())
        })
        .manage(PlayerHandle { tx: player_tx })
        .invoke_handler(tauri::generate_handler![
            play, pause, resume, stop, advance, seek, status, ws_address,
            index_library, search_tracks, list_libraries, delete_library
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

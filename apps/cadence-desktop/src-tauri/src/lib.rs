mod websocket;

use cadence_core::{Library, LibraryRecord, Player, PlayerMode, TrackInfo, TrackRecord};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::{mpsc, Arc};
use tauri::{Manager, State};

pub(crate) enum PlayerMessage {
    Play(PathBuf, mpsc::SyncSender<Result<TrackInfo, String>>),
    Pause,
    Resume,
    Stop,
    Previous,
    Next,
    Seek(u64, mpsc::SyncSender<Result<(), String>>),
    SetMode(PlayerMode),
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
    pub mode: PlayerMode,
}

fn spawn_player_thread(lib_rx: mpsc::Receiver<Arc<Library>>) -> mpsc::Sender<PlayerMessage> {
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let library = lib_rx.recv().expect("Library init failed");
        let mut player = Player::new().expect("Failed to create player");
        // history[history_pos] is always the currently playing track (when non-empty).
        let mut history: Vec<PathBuf> = Vec::new();
        let mut history_pos: usize = 0;

        // Advance to the next track: replay forward history or pick a random one.
        let advance = |player: &mut Player, history: &mut Vec<PathBuf>, history_pos: &mut usize, library: &Library| {
            if *history_pos + 1 < history.len() {
                *history_pos += 1;
                player.load_and_play(history[*history_pos].clone()).ok();
            } else {
                let current = player.current_track().map(|t| t.info.path.clone());
                if let Ok(paths) = library.all_track_paths() {
                    use rand::seq::SliceRandom;
                    let candidates: Vec<&PathBuf> = paths.iter()
                        .filter(|p| Some(*p) != current.as_ref())
                        .collect();
                    if let Some(next_path) = candidates.choose(&mut rand::thread_rng()) {
                        let next_path = (*next_path).clone();
                        history.push(next_path.clone());
                        *history_pos = history.len() - 1;
                        player.load_and_play(next_path).ok();
                    }
                }
            }
        };

        while let Ok(cmd) = rx.recv() {
            match cmd {
                PlayerMessage::Play(path, reply) => {
                    // Truncate any forward history, then append the new track.
                    if !history.is_empty() {
                        history.truncate(history_pos + 1);
                    }
                    history.push(path.clone());
                    history_pos = history.len() - 1;
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
                PlayerMessage::Previous => {
                    if history_pos > 0 {
                        history_pos -= 1;
                        player.load_and_play(history[history_pos].clone()).ok();
                    }
                }
                PlayerMessage::Next => {
                    advance(&mut player, &mut history, &mut history_pos, &library);
                }
                PlayerMessage::Seek(to_ms, reply) => {
                    let result = player.seek(to_ms).map_err(|e| e.to_string());
                    reply.send(result).ok();
                }
                PlayerMessage::SetMode(mode) => {
                    player.set_mode(mode);
                }
                PlayerMessage::Status(reply) => {
                    // Auto-stop when rodio's sink runs dry (track reached EOF).
                    if player.current_track().is_some() && player.is_finished() {
                        match player.get_mode() {
                            PlayerMode::Default => { player.stop() }
                            PlayerMode::Replay => {
                                let path = player.current_track().as_ref().unwrap().info.path.clone();
                                player.load_and_play(path).ok();
                            }
                            PlayerMode::Shuffle => {
                                advance(&mut player, &mut history, &mut history_pos, &library);
                            }
                        }
                    }
                    let status = player.current_track().map(|track| StatusResponse {
                        path: track.info.path.to_string_lossy().into_owned(),
                        duration_ms: track.info.duration_ms,
                        position_ms: player.current_position_ms(),
                        paused: track.last_playback_timestamp.is_none(),
                        title: track.info.title.clone(),
                        artist: track.info.artist.clone(),
                        mode: player.get_mode(),
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
fn next(handle: State<PlayerHandle>) {
    handle.tx.send(PlayerMessage::Next).ok();
}

#[tauri::command]
fn previous(handle: State<PlayerHandle>) {
    handle.tx.send(PlayerMessage::Previous).ok();
}

#[tauri::command]
fn seek(to_ms: u64, handle: State<PlayerHandle>) -> Result<(), String> {
    let (tx, rx) = mpsc::sync_channel(1);
    handle.tx.send(PlayerMessage::Seek(to_ms, tx)).ok();
    rx.recv().map_err(|_| "Player thread died".to_string())?
}

#[tauri::command]
fn set_mode(mode: PlayerMode, handle: State<PlayerHandle>) {
    handle.tx.send(PlayerMessage::SetMode(mode)).ok();
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
fn index_library(path: String, library: State<Arc<Library>>) -> Result<usize, String> {
    library.index_directory(std::path::Path::new(&path))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn search_tracks(query: String, library: State<Arc<Library>>) -> Result<Vec<TrackRecord>, String> {
    library.search(&query).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_libraries(library: State<Arc<Library>>) -> Result<Vec<LibraryRecord>, String> {
    library.list_libraries().map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_library(id: i64, library: State<Arc<Library>>) -> Result<(), String> {
    library.delete_library(id).map_err(|e| e.to_string())
}

fn local_ipv4() -> Option<std::net::Ipv4Addr> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    match socket.local_addr().ok()?.ip() {
        std::net::IpAddr::V4(ip) => Some(ip),
        _ => None,
    }
}

fn advertise_mdns() {
    use mdns_sd::{ServiceDaemon, ServiceInfo};

    let hostname = gethostname::gethostname()
        .to_string_lossy()
        .to_string();

    let Some(ip) = local_ipv4() else {
        eprintln!("mDNS: could not determine local IP, skipping advertisement");
        return;
    };

    std::thread::spawn(move || {
        let Ok(mdns) = ServiceDaemon::new() else { return };
        let host_name = format!("{}.local.", hostname);
        let Ok(info) = ServiceInfo::new(
            "_cadence._tcp.local.",
            &hostname,
            &host_name,
            std::net::IpAddr::V4(ip),
            7878,
            None,
        ) else { return };

        mdns.register(info).ok();

        // Park the thread to keep the daemon (and thus the advertisement) alive.
        loop { std::thread::sleep(std::time::Duration::from_secs(60)); }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Library is created in setup (needs app data dir).
    // Both the player thread and WS server need it — send via separate sync channels.
    let (player_lib_tx, player_lib_rx) = mpsc::sync_channel::<Arc<Library>>(1);
    let player_tx = spawn_player_thread(player_lib_rx);

    let (ws_lib_tx, ws_lib_rx) = tokio::sync::oneshot::channel::<Arc<Library>>();
    let player_tx_for_ws = player_tx.clone();
    tauri::async_runtime::spawn(async move {
        let Ok(library) = ws_lib_rx.await else { return };
        websocket::serve(player_tx_for_ws, library).await;
    });

    advertise_mdns();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            let db_path = app.path().app_data_dir()
                .expect("Failed to get app data dir")
                .join("cadence.db");
            let library = Arc::new(Library::open(&db_path)
                .expect("Failed to open library database"));
            player_lib_tx.send(Arc::clone(&library)).ok();
            ws_lib_tx.send(Arc::clone(&library)).ok();
            app.manage(library);
            Ok(())
        })
        .manage(PlayerHandle { tx: player_tx })
        .invoke_handler(tauri::generate_handler![
            play, pause, resume, stop, next, previous, seek, set_mode, status, ws_address,
            index_library, search_tracks, list_libraries, delete_library
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

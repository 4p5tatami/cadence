#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use cadence_core::{Player, TrackInfo};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use tauri::Manager;

type Reply<T> = Sender<Result<T, String>>;

enum Command {
    Play {
        path: String,
        respond_to: Reply<TrackInfo>,
    },
    Pause,
    Resume,
    Stop,
}

#[derive(Clone)]
struct PlayerService {
    sender: Sender<Command>,
}

impl PlayerService {
    fn new() -> Result<Self, String> {
        let (sender, receiver) = mpsc::channel::<Command>();
        let (ready_tx, ready_rx) = mpsc::channel::<Result<(), String>>();

        thread::Builder::new()
            .name("cadence-player".into())
            .spawn(move || player_loop(receiver, ready_tx))
            .map_err(|e| e.to_string())?;

        // Wait for the playback backend to be initialised before returning.
        match ready_rx.recv().map_err(|e| e.to_string())? {
            Ok(()) => Ok(Self { sender }),
            Err(err) => Err(err),
        }
    }

    fn play(&self, path: String) -> Result<TrackInfo, String> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.sender
            .send(Command::Play {
                path,
                respond_to: reply_tx,
            })
            .map_err(|e| e.to_string())?;
        reply_rx.recv().map_err(|e| e.to_string())?
    }

    fn send_simple(&self, command: Command) -> Result<(), String> {
        self.sender.send(command).map_err(|e| e.to_string())
    }
}

fn player_loop(receiver: Receiver<Command>, ready: Sender<Result<(), String>>) {
    let player = match Player::new() {
        Ok(player) => {
            let _ = ready.send(Ok(()));
            player
        }
        Err(err) => {
            let _ = ready.send(Err(err.to_string()));
            return;
        }
    };

    while let Ok(command) = receiver.recv() {
        match command {
            Command::Play { path, respond_to } => {
                let result = player.load_and_play(&path).map_err(|e| e.to_string());
                let _ = respond_to.send(result);
            }
            Command::Pause => player.pause(),
            Command::Resume => player.resume(),
            Command::Stop => player.stop(),
        }
    }
}

#[tauri::command]
fn play(state: tauri::State<PlayerService>, path: String) -> Result<TrackInfo, String> {
    state.play(path)
}

#[tauri::command]
fn pause(state: tauri::State<PlayerService>) -> Result<(), String> {
    state.send_simple(Command::Pause)
}

#[tauri::command]
fn resume(state: tauri::State<PlayerService>) -> Result<(), String> {
    state.send_simple(Command::Resume)
}

#[tauri::command]
fn stop(state: tauri::State<PlayerService>) -> Result<(), String> {
    state.send_simple(Command::Stop)
}

#[tauri::command]
fn pick_file() -> Result<Option<String>, String> {
    use tauri::api::dialog::FileDialogBuilder;

    let (tx, rx) = mpsc::channel();

    FileDialogBuilder::new()
        .set_title("Select audio file")
        .add_filter(
            "Audio",
            &["flac", "mp3", "wav", "ogg", "m4a", "aac", "opus", "aiff"],
        )
        .pick_file(move |response| {
            let path = response.map(|p| p.to_string_lossy().into_owned());
            let _ = tx.send(path);
        });

    rx.recv().map_err(|e| e.to_string())
}

fn main() {
    let service = PlayerService::new()
        .expect("Failed to initialise audio playback (is an output device available?)");

    tauri::Builder::default()
        .manage(service)
        .invoke_handler(tauri::generate_handler![
            play, pause, resume, stop, pick_file
        ])
        .setup(|app| {
            if let Some(window) = app.get_window("main") {
                window.set_focus().ok();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

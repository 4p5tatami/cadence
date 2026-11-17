use cadence_core::{Library, OutputState, Player, PlayerMode};
use std::path::PathBuf;
use std::sync::{mpsc, Arc};

pub(crate) enum PlayerMessage {
    Play(PathBuf),
    Pause,
    Resume,
    Stop,
    Previous,
    Next,
    Seek(u64),
    SetMode(PlayerMode),
    Status(mpsc::SyncSender<Option<StatusResponse>>),
}

pub(crate) struct StatusResponse {
    pub path: String,
    pub duration_ms: u64,
    pub position_ms: u64,
    pub paused: bool,
    pub output_state: OutputState,
    pub playback_error: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub mode: PlayerMode,
}

impl StatusResponse {
    pub fn is_playing(&self) -> bool {
        !self.paused && self.output_state == OutputState::Ready && self.playback_error.is_none()
    }
}

pub(crate) fn spawn(library: Arc<Library>) -> mpsc::Sender<PlayerMessage> {
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let mut player = Player::new().expect("Failed to create player");
        // history[history_pos] is always the currently playing track (when non-empty).
        let mut history: Vec<PathBuf> = Vec::new();
        let mut history_pos: usize = 0;

        // Advance to the next track: replay forward history or pick a random one.
        let advance = |player: &mut Player,
                       history: &mut Vec<PathBuf>,
                       history_pos: &mut usize,
                       library: &Library| {
            if *history_pos + 1 < history.len() {
                *history_pos += 1;
                player.load_and_play(history[*history_pos].clone()).ok();
            } else {
                let current = player.current_track().map(|t| t.path.clone());
                if let Ok(paths) = library.all_track_paths() {
                    use rand::seq::SliceRandom;
                    let candidates: Vec<&PathBuf> = paths
                        .iter()
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

        loop {
            player.maintain_audio_output();
            let cmd = match rx.recv_timeout(std::time::Duration::from_millis(25)) {
                Ok(cmd) => cmd,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            };
            player.maintain_audio_output();
            match cmd {
                PlayerMessage::Play(path) => {
                    // Truncate any forward history, then append the new track.
                    if !history.is_empty() {
                        history.truncate(history_pos + 1);
                    }
                    history.push(path.clone());
                    history_pos = history.len() - 1;
                    player.load_and_play(path).ok();
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
                PlayerMessage::Seek(to_ms) => {
                    player.seek(to_ms).ok();
                }
                PlayerMessage::SetMode(mode) => {
                    player.set_mode(mode);
                }
                PlayerMessage::Status(reply) => {
                    // Auto-stop when rodio's sink runs dry (track reached EOF).
                    if player.current_track().is_some() && player.is_finished() {
                        match player.get_mode() {
                            PlayerMode::Default => player.stop(),
                            PlayerMode::Replay => {
                                let path = player.current_track().as_ref().unwrap().path.clone();
                                player.load_and_play(path).ok();
                            }
                            PlayerMode::Shuffle => {
                                advance(&mut player, &mut history, &mut history_pos, &library);
                            }
                        }
                    }
                    let status = player.current_track().map(|track| StatusResponse {
                        path: track.path.to_string_lossy().into_owned(),
                        duration_ms: track.duration_ms,
                        position_ms: player.current_position_ms(),
                        paused: player.is_paused(),
                        output_state: player.output_state(),
                        playback_error: player.playback_error().map(str::to_owned),
                        title: track.title.clone(),
                        artist: track.artist.clone(),
                        mode: player.get_mode(),
                    });
                    reply.send(status).ok();
                }
            }
        }
    });

    tx
}

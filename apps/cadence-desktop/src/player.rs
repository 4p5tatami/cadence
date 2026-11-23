use cadence_core::{Library, OutputState, Player, PlayerMode};
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc};
use tokio::sync::oneshot;

/// Reply for commands that load a track: `Err` carries a user-facing message.
pub(crate) type LoadReply = oneshot::Sender<Result<(), String>>;

pub(crate) enum PlayerMessage {
    Play(PathBuf, LoadReply),
    Pause,
    Resume,
    Stop,
    Previous(LoadReply),
    Next(LoadReply),
    Seek(u64),
    /// Volume level in `0.0..=1.0`; see `Player::set_volume`.
    SetVolume(f32),
    SetMode(PlayerMode),
    Status(mpsc::SyncSender<Snapshot>),
}

/// Periodic view of the player for clients. Volume is reported even when stopped
/// so a remote can show and change it before anything plays.
pub(crate) struct Snapshot {
    pub track: Option<StatusResponse>,
    pub volume: f32,
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
    pub file_format: Option<String>,
    pub sample_rate: Option<u32>,
    pub bitrate: Option<u32>,
    pub mode: PlayerMode,
}

impl StatusResponse {
    pub fn is_playing(&self) -> bool {
        !self.paused && self.output_state == OutputState::Ready && self.playback_error.is_none()
    }
}

/// Loads and plays `path`. On failure the current track keeps playing and the
/// error is turned into a short message suitable for showing on the phone.
fn load(player: &mut Player, path: &Path) -> Result<(), String> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());
    if !path.exists() {
        return Err(format!(
            "Couldn't find \u{201c}{name}\u{201d}. Is the drive or folder it's on available?"
        ));
    }
    player
        .load_and_play(path.to_path_buf())
        .map(|_| ())
        .map_err(|error| format!("Couldn't play \u{201c}{name}\u{201d}: {error}"))
}

pub(crate) fn spawn(library: Arc<Library>) -> mpsc::Sender<PlayerMessage> {
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let mut player = Player::new().expect("Failed to create player");
        // history[history_pos] is always the currently playing track (when non-empty).
        // Tracks are only recorded once they load, so history never holds a broken entry.
        let mut history: Vec<PathBuf> = Vec::new();
        let mut history_pos: usize = 0;

        // Advance to the next track: replay forward history or pick a random one.
        let advance = |player: &mut Player,
                       history: &mut Vec<PathBuf>,
                       history_pos: &mut usize,
                       library: &Library|
         -> Result<(), String> {
            if *history_pos + 1 < history.len() {
                load(player, &history[*history_pos + 1])?;
                *history_pos += 1;
                return Ok(());
            }
            let current = player.current_track().map(|t| t.path.clone());
            let Ok(paths) = library.all_track_paths() else {
                return Ok(());
            };
            use rand::seq::SliceRandom;
            let candidates: Vec<&PathBuf> = paths
                .iter()
                .filter(|p| Some(*p) != current.as_ref())
                .collect();
            if let Some(next_path) = candidates.choose(&mut rand::thread_rng()) {
                load(player, next_path)?;
                if !history.is_empty() {
                    history.truncate(*history_pos + 1);
                }
                history.push((*next_path).clone());
                *history_pos = history.len() - 1;
            }
            Ok(())
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
                PlayerMessage::Play(path, reply) => {
                    let result = load(&mut player, &path);
                    if result.is_ok() {
                        // Truncate any forward history, then append the new track.
                        if !history.is_empty() {
                            history.truncate(history_pos + 1);
                        }
                        history.push(path);
                        history_pos = history.len() - 1;
                    }
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
                PlayerMessage::Previous(reply) => {
                    let mut result = Ok(());
                    if history_pos > 0 {
                        result = load(&mut player, &history[history_pos - 1]);
                        if result.is_ok() {
                            history_pos -= 1;
                        }
                    }
                    reply.send(result).ok();
                }
                PlayerMessage::Next(reply) => {
                    let result = advance(&mut player, &mut history, &mut history_pos, &library);
                    reply.send(result).ok();
                }
                PlayerMessage::Seek(to_ms) => {
                    player.seek(to_ms).ok();
                }
                PlayerMessage::SetVolume(level) => {
                    player.set_volume(level);
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
                                advance(&mut player, &mut history, &mut history_pos, &library).ok();
                            }
                        }
                    }
                    let track = player.current_track().map(|track| StatusResponse {
                        path: track.path.to_string_lossy().into_owned(),
                        duration_ms: track.duration_ms,
                        position_ms: player.current_position_ms(),
                        paused: player.is_paused(),
                        output_state: player.output_state(),
                        playback_error: player.playback_error().map(str::to_owned),
                        title: track.title.clone(),
                        artist: track.artist.clone(),
                        file_format: track.file_format.clone(),
                        sample_rate: track.sample_rate,
                        bitrate: track.bitrate,
                        mode: player.get_mode(),
                    });
                    reply
                        .send(Snapshot {
                            track,
                            volume: player.volume(),
                        })
                        .ok();
                }
            }
        }
    });

    tx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_reports_its_name() {
        let Ok(mut player) = Player::new() else {
            return; // No audio backend on this machine (e.g. CI).
        };
        let error = load(&mut player, Path::new("/nonexistent/dir/Alone Again.flac")).unwrap_err();
        assert!(error.contains("Alone Again.flac"), "{error}");
        assert!(!error.contains("/nonexistent"), "{error}");
    }
}

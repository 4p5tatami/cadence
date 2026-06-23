pub mod library;
pub use library::{Library, LibraryRecord, TrackRecord};

use anyhow::{Context, Result};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Instant;
use std::fs::File;
use std::io::{BufReader, Read};
use lofty::file::TaggedFile;

#[derive(Debug, Clone, Serialize)]
pub struct TrackInfo {
    pub path: PathBuf,
    pub duration_ms: u64,
    pub title: Option<String>,
    pub artist: Option<String>,
}

fn get_tagged_file(path: &Path) -> Option<TaggedFile> {
    lofty::probe::Probe::open(path).ok()
        .and_then(|p| p.guess_file_type().ok())
        .and_then(|p| p.read().ok())
}

fn probe_tags(path: &Path) -> (Option<String>, Option<String>) {
    use lofty::prelude::*;
    let Some(tagged) = get_tagged_file(path) else {
        return (None, None);
    };
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag());
    let title = tag.and_then(|t| t.title().as_deref().map(String::from));
    let artist = tag.and_then(|t| t.artist().as_deref().map(String::from));
    (title, artist)
}

/// Represents the current state of a playing track
#[derive(Debug)]
pub struct CurrentTrack {
    /// Information about the track (path, duration)
    pub info: TrackInfo,
    /// Timestamp when playback last started/resumed; None when paused
    pub last_playback_timestamp: Option<Instant>,
    /// Position in ms at the time of last playback start/pause
    pub last_playback_position: u64,
}

impl CurrentTrack {
    fn new(info: TrackInfo) -> Self {
        Self {
            info,
            last_playback_timestamp: Some(Instant::now()),
            last_playback_position: 0,
        }
    }

    pub fn current_position_ms(&self) -> u64 {
        match self.last_playback_timestamp {
            Some(timestamp) => self.last_playback_position + timestamp.elapsed().as_millis() as u64,
            None => self.last_playback_position,
        }
    }

    fn pause(&mut self) {
        self.last_playback_position = self.current_position_ms();
        self.last_playback_timestamp = None;
    }

    fn resume(&mut self) {
        self.last_playback_timestamp = Some(Instant::now());
    }

    fn set_position(&mut self, position_ms: u64, playing: bool) {
        self.last_playback_position = position_ms;
        self.last_playback_timestamp = if playing { Some(Instant::now()) } else { None };
    }
}

/// Fallback duration probe for files where the decoder can't report total_duration()
/// (e.g. VBR MP3s without a Xing/VBRI header).
fn scan_duration_ms(path: &std::path::Path) -> Option<u64> {
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let file = File::open(path).ok()?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let mut probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .ok()?;

    let track = probed.format.default_track()?;

    if let (Some(n_frames), Some(sample_rate)) = (track.codec_params.n_frames, track.codec_params.sample_rate) {
        return Some(n_frames * 1000 / sample_rate as u64);
    }

    let time_base = track.codec_params.time_base?;
    let track_id = track.id;
    let mut end_ts = 0u64;
    loop {
        match probed.format.next_packet() {
            Ok(pkt) if pkt.track_id() == track_id => {
                end_ts = end_ts.max(pkt.ts + pkt.dur);
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }
    if end_ts == 0 {
        return None;
    }
    let secs = end_ts as f64 * time_base.numer as f64 / time_base.denom as f64;
    Some((secs * 1000.0) as u64)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlayerMode {
    Default,
    Shuffle,
    Replay,
}

pub struct Player {
    _stream: OutputStream,
    _handle: OutputStreamHandle,
    sink: Sink,
    current_track: Option<CurrentTrack>,
    _temp_file: Option<tempfile::NamedTempFile>,
    mode: PlayerMode,
}

impl Player {
    pub fn new() -> Result<Self> {
        let (stream, handle) =
            OutputStream::try_default().context("No default output device available")?;
        let sink = Sink::try_new(&handle).context("Failed to create sink")?;
        Ok(Self {
            _stream: stream,
            _handle: handle,
            sink,
            current_track: None,
            _temp_file: None,
            mode: PlayerMode::Default,
        })
    }

    pub fn current_track(&self) -> Option<&CurrentTrack> {
        self.current_track.as_ref()
    }

    pub fn current_position_ms(&self) -> u64 {
        self.current_track
            .as_ref()
            .map(|t| t.current_position_ms())
            .unwrap_or(0)
    }

    pub fn load_and_play(&mut self, path: PathBuf) -> Result<TrackInfo> {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase());

        // For FLAC files: read into memory, patch any non-conformant STREAMINFO
        // (min_block_size=0 from old encoders), write to a temp file, and play
        // from that. Symphonia enforces min_block_size >= 16 strictly.
        // For all other formats: stream directly from disk.
        let play_path: PathBuf;
        let temp_file: Option<tempfile::NamedTempFile>;

        if ext.as_deref() == Some("flac") {
            let mut data = Vec::new();
            File::open(&path)
                .with_context(|| format!("Failed to open {:?}", path))?
                .read_to_end(&mut data)
                .with_context(|| format!("Failed to read {:?}", path))?;

            if data.len() > 10 && data[0..4] == *b"fLaC" {
                let min_bs = u16::from_be_bytes([data[8], data[9]]);
                let max_bs = u16::from_be_bytes([data[10], data[11]]);
                if min_bs < 16 {
                    // Set min_block_size = max_block_size so symphonia treats
                    // the stream as fixed-blocksize (required for ByFrame sequencing).
                    // The spec minimum is 16; use max_block_size if it's valid, else 16.
                    let fixed = if max_bs >= 16 { max_bs } else { 16u16 };
                    let patched = fixed.to_be_bytes();
                    data[8] = patched[0];
                    data[9] = patched[1];

                    // Write patched bytes to a temp file so rodio/symphonia can
                    // open it normally (with a real file descriptor + byte_len).
                    use std::io::Write;
                    let mut tmp = tempfile::Builder::new()
                        .suffix(".flac")
                        .tempfile()
                        .context("Failed to create temp file for FLAC patch")?;
                    tmp.write_all(&data)
                        .context("Failed to write patched FLAC to temp file")?;
                    tmp.flush()
                        .context("Failed to flush patched FLAC temp file")?;
                    play_path = tmp.path().to_path_buf();
                    temp_file = Some(tmp);
                } else {
                    play_path = path.clone();
                    temp_file = None;
                }
            } else {
                play_path = path.clone();
                temp_file = None;
            }
        } else {
            play_path = path.clone();
            temp_file = None;
        }

        let file = File::open(&play_path)
            .with_context(|| format!("Failed to open {:?}", play_path))?;
        let src = match ext.as_deref() {
            Some("flac") => Decoder::new_flac(BufReader::new(file)),
            Some("mp3")  => Decoder::new_mp3(BufReader::new(file)),
            Some("wav")  => Decoder::new_wav(BufReader::new(file)),
            Some("ogg")  => Decoder::new_vorbis(BufReader::new(file)),
            _            => Decoder::new(BufReader::new(file)),
        }
        .with_context(|| format!("Unsupported/invalid audio: {:?}", path))?;

        let duration_ms = src.total_duration()
            .map(|d| d.as_millis() as u64)
            .or_else(|| scan_duration_ms(&play_path))
            .with_context(|| format!("Cannot determine duration for {:?}", path))?;

        let (title, artist) = probe_tags(&path);
        let info = TrackInfo { path, duration_ms, title, artist };
        self.sink.clear();
        self.sink.append(src);
        self.sink.play();
        self._temp_file = temp_file;
        self.current_track = Some(CurrentTrack::new(info.clone()));
        Ok(info)
    }

    pub fn pause(&mut self) {
        if let Some(track) = &mut self.current_track {
            track.pause();
        }
        self.sink.pause();
    }

    pub fn resume(&mut self) {
        if let Some(track) = &mut self.current_track {
            track.resume();
        }
        self.sink.play();
    }

    pub fn stop(&mut self) {
        self.sink.stop();
        self.current_track = None;
        self._temp_file = None;
    }

    pub fn is_finished(&self) -> bool {
        self.sink.empty()
    }

    pub fn seek(&mut self, to_ms: u64) -> Result<()> {
        use std::time::Duration;

        let Some(track) = &self.current_track else { return Ok(()) };
        let to_ms = to_ms.min(track.info.duration_ms.saturating_sub(1));
        let was_playing = track.last_playback_timestamp.is_some();

        if let Some(temp_file) = &self._temp_file {
            let file = temp_file
                .reopen()
                .context("Failed to reopen patched FLAC temp file")?;
            let src = Decoder::new_flac(BufReader::new(file))
                .context("Failed to decode patched FLAC temp file")?
                .skip_duration(Duration::from_millis(to_ms));

            self.sink.clear();
            self.sink.append(src);
            if was_playing {
                self.sink.play();
            } else {
                self.sink.pause();
            }

            if let Some(track) = &mut self.current_track {
                track.set_position(to_ms, was_playing);
            }
            return Ok(());
        }

        self.sink.try_seek(Duration::from_millis(to_ms)).map_err(|e| anyhow::anyhow!("{e}"))?;

        if let Some(track) = &mut self.current_track {
            track.set_position(to_ms, was_playing);
        }
        Ok(())
    }

    pub fn get_mode(&self) -> PlayerMode { self.mode.clone() }

    pub fn set_mode(&mut self, mode: PlayerMode) {
        self.mode = mode;
    }
}

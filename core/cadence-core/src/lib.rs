pub mod library;
pub use library::{Library, LibraryRecord, TrackRecord};

use anyhow::{Context, Result};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use serde::Serialize;
use std::path::PathBuf;
use std::time::Instant;
use std::fs::File;
use std::io::BufReader;

#[derive(Debug, Clone, Serialize)]
pub struct TrackInfo {
    pub path: PathBuf,
    pub duration_ms: u64,
    pub title: Option<String>,
    pub artist: Option<String>,
}

fn probe_tags(path: &std::path::Path) -> (Option<String>, Option<String>) {
    use lofty::prelude::*;
    let Some(tagged) = lofty::probe::Probe::open(path).ok()
        .and_then(|p| p.guess_file_type().ok())
        .and_then(|p| p.read().ok())
    else {
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
    /// Create a new CurrentTrack starting from position 0
    fn new(info: TrackInfo) -> Self {
        Self {
            info,
            last_playback_timestamp: Some(Instant::now()),
            last_playback_position: 0,
        }
    }

    /// Get the current playback position in milliseconds
    pub fn current_position_ms(&self) -> u64 {
        match self.last_playback_timestamp {
            Some(timestamp) => self.last_playback_position + timestamp.elapsed().as_millis() as u64,
            None => self.last_playback_position,
        }
    }

    /// Mark as paused, capturing current position
    fn pause(&mut self) {
        self.last_playback_position = self.current_position_ms();
        self.last_playback_timestamp = None;
    }

    /// Mark as resumed, starting time tracking from now
    fn resume(&mut self) {
        self.last_playback_timestamp = Some(Instant::now());
    }

    /// Update position after seek, preserving playing/paused state
    fn set_position(&mut self, position_ms: u64, playing: bool) {
        self.last_playback_position = position_ms;
        self.last_playback_timestamp = if playing { Some(Instant::now()) } else { None };
    }
}

/// Fallback duration probe for files where the decoder can't report total_duration()
/// (e.g. VBR MP3s without a Xing/VBRI header).
/// Fast path: reads n_frames from codec params. Slow path: walks all packets.
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

    // Fast path: n_frames already known from the container/header.
    if let (Some(n_frames), Some(sample_rate)) = (track.codec_params.n_frames, track.codec_params.sample_rate) {
        return Some(n_frames * 1000 / sample_rate as u64);
    }

    // Slow path: scan every packet and accumulate the highest end-timestamp.
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

pub struct Player {
    _stream: OutputStream,
    _handle: OutputStreamHandle,
    sink: Sink,
    /// Current track state, if any
    current_track: Option<CurrentTrack>,
    current_volume: f32,
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
            current_volume: 1.0,
        })
    }

    /// Get the current track, if any
    pub fn current_track(&self) -> Option<&CurrentTrack> {
        self.current_track.as_ref()
    }

    /// Get the current playback position in milliseconds, or 0 if no track
    pub fn current_position_ms(&self) -> u64 {
        self.current_track
            .as_ref()
            .map(|t| t.current_position_ms())
            .unwrap_or(0)
    }

    pub fn load_and_play(&mut self, path: PathBuf) -> Result<TrackInfo> {
        // Open once for duration using the same decoder we'll use for playback.
        let file = File::open(&path).with_context(|| format!("Failed to open {:?}", path))?;
        let src = Decoder::new(BufReader::new(file))
            .with_context(|| format!("Unsupported/invalid audio: {:?}", path))?;
        let duration_ms = src.total_duration()
            .map(|d| d.as_millis() as u64)
            .or_else(|| scan_duration_ms(&path))
            .unwrap_or_else(|| panic!("Cannot determine duration for {:?}", path));

        let (title, artist) = probe_tags(&path);
        let info = TrackInfo {
            path,
            duration_ms,
            title,
            artist,
        };

        self.sink.clear();
        self.sink.append(src);
        self.sink.play();

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
    }

    /// True when the sink has no more samples — i.e. the track finished playing.
    pub fn is_finished(&self) -> bool {
        self.sink.empty()
    }

    pub fn seek(&mut self, to_ms: u64) -> Result<()> {
        use std::time::Duration;

        let Some(track) = &self.current_track else { return Ok(()) };

        if to_ms >= track.info.duration_ms {
            self.stop();
            return Ok(());
        }

        let was_playing = track.last_playback_timestamp.is_some();
        self.sink.try_seek(Duration::from_millis(to_ms)).map_err(|e| anyhow::anyhow!("{e}"))?;

        if let Some(track) = &mut self.current_track {
            track.set_position(to_ms, was_playing);
        }

        Ok(())
    }

    pub fn set_volume (&mut self, volume: f32) {
        self.current_volume = volume.clamp(0.0, 1.0);
        self.sink.set_volume(self.current_volume);
    }

    pub fn get_volume(&self) -> f32 {
        self.current_volume
    }
}

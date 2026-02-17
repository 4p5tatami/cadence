use anyhow::{Context, Result};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use serde::Serialize;
use std::path::PathBuf;
use std::time::Instant;
use std::{fs::File, io::BufReader};

#[derive(Debug, Clone, Serialize)]
pub struct TrackInfo {
    pub path: PathBuf,
    pub duration_ms: Option<u64>,
}

/// Represents the current state of a playing track
#[derive(Debug)]
pub struct CurrentTrack {
    /// Information about the track (path, duration)
    pub info: TrackInfo,
    /// When playback last started/resumed; None when paused
    pub last_playback_time: Option<Instant>,
    /// Position in ms at the time of last playback start/pause
    pub last_playback_position: u64,
}

impl CurrentTrack {
    /// Create a new CurrentTrack starting from position 0
    fn new(info: TrackInfo) -> Self {
        Self {
            info,
            last_playback_time: Some(Instant::now()),
            last_playback_position: 0,
        }
    }

    /// Get the current playback position in milliseconds
    pub fn current_position_ms(&self) -> u64 {
        match self.last_playback_time {
            Some(instant) => self.last_playback_position + instant.elapsed().as_millis() as u64,
            None => self.last_playback_position,
        }
    }

    /// Mark as paused, capturing current position
    fn pause(&mut self) {
        self.last_playback_position = self.current_position_ms();
        self.last_playback_time = None;
    }

    /// Mark as resumed, starting time tracking from now
    fn resume(&mut self) {
        self.last_playback_time = Some(Instant::now());
    }

    /// Update position and reset time tracking (used after seek)
    fn set_position(&mut self, position_ms: u64) {
        self.last_playback_position = position_ms;
        self.last_playback_time = Some(Instant::now());
    }
}

pub struct Player {
    _stream: OutputStream,
    _handle: OutputStreamHandle,
    sink: Sink,
    /// Current track state, if any
    current_track: Option<CurrentTrack>,
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
        let dur = src.total_duration().map(|d| d.as_millis() as u64);

        let info = TrackInfo {
            path,
            duration_ms: dur,
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

    pub fn seek_approx(&mut self, to_ms: u64) -> Result<()> {
        use std::time::Duration;

        let path = match &self.current_track {
            Some(track) => &track.info.path,
            None => return Ok(()), // No track to seek
        };

        // Open once to query total duration
        let file = File::open(path)?;
        let src = Decoder::new(BufReader::new(file))?;
        let to = Duration::from_millis(to_ms);

        if let Some(total) = src.total_duration() {
            if to >= total {
                // Seeking past EOF: just stop.
                self.stop();
                return Ok(());
            }
        }

        let skipped = src.skip_duration(to); // returns a Source wrapper, not a Duration

        self.sink.clear();
        self.sink.append(skipped);
        self.sink.play();

        if let Some(track) = &mut self.current_track {
            track.set_position(to_ms);
        }

        Ok(())
    }

    pub fn advance_or_rewind(&mut self, delta_ms: i64) -> Result<()> {
        let current = self.current_position_ms() as i64;
        let target = (current + delta_ms).max(0) as u64;
        self.seek_approx(target)
    }
}

mod audio_input;
pub mod library;
mod timeline_source;
pub use library::{Library, LibraryRecord, TrackRecord};

use anyhow::{Context, Result};
use lofty::file::TaggedFile;
use rodio::{OutputStream, OutputStreamBuilder, Sink, Source};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct TrackInfo {
    pub path: PathBuf,
    pub duration_ms: u64,
    pub title: Option<String>,
    pub artist: Option<String>,
}

fn get_tagged_file(path: &Path) -> Option<TaggedFile> {
    lofty::probe::Probe::open(path)
        .ok()
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
}

/// Fallback duration probe for files where the decoder can't report total_duration()
/// (e.g. VBR MP3s without a Xing/VBRI header).
fn scan_duration_ms(path: &std::path::Path) -> Option<u64> {
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let file = audio_input::AudioInput::open(path).ok()?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let mut probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .ok()?;

    let track = probed.format.default_track()?;

    if let (Some(n_frames), Some(sample_rate)) =
        (track.codec_params.n_frames, track.codec_params.sample_rate)
    {
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
    sink: Sink,
    current_track: Option<CurrentTrack>,
    mode: PlayerMode,
}

impl Player {
    pub fn new() -> Result<Self> {
        let stream =
            OutputStreamBuilder::open_default_stream().context("No output device available")?;
        let sink = Sink::connect_new(stream.mixer());
        Ok(Self {
            _stream: stream,
            sink,
            current_track: None,
            mode: PlayerMode::Default,
        })
    }

    pub fn current_track(&self) -> Option<&CurrentTrack> {
        self.current_track.as_ref()
    }

    pub fn current_position_ms(&self) -> u64 {
        self.current_track
            .as_ref()
            .map(|t| (self.sink.get_pos().as_millis() as u64).min(t.info.duration_ms))
            .unwrap_or(0)
    }

    pub fn load_and_play(&mut self, path: PathBuf) -> Result<TrackInfo> {
        let src = audio_input::decoder(&path)?;

        let duration_ms = src
            .total_duration()
            .map(|d| d.as_millis() as u64)
            .or_else(|| scan_duration_ms(&path))
            .with_context(|| format!("Cannot determine duration for {:?}", path))?;

        let (title, artist) = probe_tags(&path);
        let info = TrackInfo {
            path,
            duration_ms,
            title,
            artist,
        };
        // A fresh sink starts its sample clock at zero and avoids waiting for
        // clear() to drain the old source on the audio callback.
        let sink = Sink::connect_new(self._stream.mixer());
        sink.set_volume(self.sink.volume());
        sink.pause();
        sink.append(src);
        self.sink.stop();
        self.sink = sink;
        self.sink.play();
        self.current_track = Some(CurrentTrack { info: info.clone() });
        Ok(info)
    }

    pub fn is_paused(&self) -> bool {
        self.sink.is_paused()
    }

    pub fn pause(&mut self) {
        self.sink.pause();
    }

    pub fn resume(&mut self) {
        self.sink.play();
    }

    pub fn stop(&mut self) {
        self.sink.stop();
        self.current_track = None;
    }

    pub fn is_finished(&self) -> bool {
        self.sink.empty()
    }

    pub fn seek(&mut self, to_ms: u64) -> Result<()> {
        use std::time::Duration;

        let Some(track) = &self.current_track else {
            return Ok(());
        };
        let to_ms = to_ms.min(track.info.duration_ms.saturating_sub(1));
        if let Err(error) = self.sink.try_seek(Duration::from_millis(to_ms)) {
            // Rodio updates its position even on error, and a failed demux seek
            // may invalidate decoder state. Never report that as successful playback.
            self.stop();
            return Err(anyhow::anyhow!(
                "Playback stopped after seek failed: {error:?}"
            ));
        }
        Ok(())
    }

    pub fn get_mode(&self) -> PlayerMode {
        self.mode.clone()
    }

    pub fn set_mode(&mut self, mode: PlayerMode) {
        self.mode = mode;
    }
}

#[cfg(test)]
mod playback_tests {
    use super::*;
    use std::time::{Duration, Instant};

    /// Uses the real output callback, muted. Decoder-only timing excludes this scheduling cost.
    #[test]
    #[ignore = "requires an output device and CADENCE_AUDIO_TEST_FILE"]
    fn external_file_device_seek_timing() {
        let path = PathBuf::from(
            std::env::var_os("CADENCE_AUDIO_TEST_FILE").expect("Set CADENCE_AUDIO_TEST_FILE"),
        );
        let mut player = Player::new().unwrap();
        player.sink.set_volume(0.0);
        let started = Instant::now();
        let info = player.load_and_play(path.clone()).unwrap();
        let startup = started.elapsed();
        let mut timings = Vec::new();
        for paused in [false, true] {
            if paused {
                player.pause();
            }
            for percent in [10, 90, 50, 1, 99, 25, 75, 5, 95, 0] {
                let target = info.duration_ms * percent / 100;
                let started = Instant::now();
                player.seek(target).unwrap();
                timings.push(started.elapsed());
                assert_eq!(player.is_paused(), paused);
                std::thread::sleep(Duration::from_millis(20));
                assert!(player.current_position_ms().abs_diff(target) < 200);
            }
        }
        player.resume();
        std::thread::sleep(Duration::from_millis(80));
        assert!(player.current_position_ms() > 0);
        player.stop();
        assert_eq!(player.current_position_ms(), 0);
        player.load_and_play(path).unwrap();
        assert!(player.current_position_ms() < 200);
        player.stop();
        timings.sort();
        eprintln!(
            "Muted device: load={startup:?}, seek command p95={:?}, max={:?}",
            timings[timings.len() * 95 / 100],
            timings.last().unwrap()
        );
    }
}

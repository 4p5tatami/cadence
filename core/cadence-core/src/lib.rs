mod audio_input;
pub mod library;
mod timeline_source;
pub use library::{Library, LibraryRecord, TrackRecord};

use anyhow::{Context, Result};
use lofty::file::TaggedFile;
use rodio::{OutputStream, OutputStreamBuilder, Sink, Source};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

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
fn scan_duration_ms(path: &Path) -> Option<u64> {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputState {
    Ready,
    Recovering,
}

struct AudioOutput {
    // Keep the OS stream alive as long as its mixer is used.
    _stream: Option<OutputStream>,
    mixer: rodio::mixer::Mixer,
}

fn open_default_audio_output(failed: Arc<AtomicBool>) -> Result<AudioOutput> {
    let stream = OutputStreamBuilder::from_default_device()?
        .with_error_callback(move |_| {
            failed.store(true, Ordering::Release);
        })
        .open_stream_or_fallback()?;
    Ok(AudioOutput {
        mixer: stream.mixer().clone(),
        _stream: Some(stream),
    })
}

/// Owns either a live output and its failure signal, or a connection schedule.
/// Keeping these together prevents a ready state without an output.
enum OutputConnection {
    Ready {
        output: AudioOutput,
        failed: Arc<AtomicBool>,
    },
    AwaitingConnection {
        next_attempt: Instant,
        retry_delay_ms: u64,
    },
}

impl OutputConnection {
    /// Schedules the first connection attempt; no device has been opened yet.
    fn new(now: Instant) -> Self {
        Self::AwaitingConnection {
            next_attempt: now,
            retry_delay_ms: 250,
        }
    }

    fn connection_failed(&self) -> bool {
        matches!(self, OutputConnection::Ready { failed, .. }
            if failed.load(Ordering::Acquire))
    }

    fn output_state(&self) -> OutputState {
        match self {
            OutputConnection::Ready { .. } if !self.connection_failed() => OutputState::Ready,
            _ => OutputState::Recovering,
        }
    }

    fn mixer(&self) -> Option<&rodio::mixer::Mixer> {
        match self {
            OutputConnection::Ready { output, .. } if !self.connection_failed() => {
                Some(&output.mixer)
            }
            _ => None,
        }
    }

    /// Drops the failed output after Player has saved the playback position.
    fn schedule_reconnection(&mut self, now: Instant) {
        *self = Self::AwaitingConnection {
            next_attempt: now,
            retry_delay_ms: 250,
        };
    }

    /// Makes at most one due attempt. Returns true only when a new output opens.
    fn connect_if_due(
        &mut self,
        now: Instant,
        open: impl FnOnce(Arc<AtomicBool>) -> Result<AudioOutput>,
    ) -> bool {
        let OutputConnection::AwaitingConnection {
            next_attempt,
            retry_delay_ms,
        } = self
        else {
            return false;
        };
        if now < *next_attempt {
            return false;
        }
        // Retired attempts retain only their own signal, never the replacement's.
        let failed = Arc::new(AtomicBool::new(false));
        match open(failed.clone()) {
            Ok(output) => {
                *self = Self::Ready { output, failed };
                true
            }
            Err(_) => {
                *next_attempt = now + Duration::from_millis(*retry_delay_ms);
                *retry_delay_ms = (*retry_delay_ms * 2).min(2000);
                false
            }
        }
    }
}

/// Owns track playback, its saved position, user intent, and decoder errors.
struct PlaybackManager {
    sink: Sink,
    current_track: Option<CurrentTrack>,
    paused: bool,
    position_offset_ms: u64,
    playback_error: Option<String>,
}

impl PlaybackManager {
    /// Starts with no track and a placeholder sink that is not attached to a device.
    fn new() -> Self {
        Self {
            sink: Sink::new().0,
            current_track: None,
            paused: true,
            position_offset_ms: 0,
            playback_error: None,
        }
    }

    fn current_position_ms(&self) -> u64 {
        self.current_track
            .as_ref()
            .map(|t| {
                self.position_offset_ms
                    .saturating_add(self.sink.get_pos().as_millis() as u64)
                    .min(t.info.duration_ms)
            })
            .unwrap_or(0)
    }

    /// Replaces the sink without waiting for a device callback; preserves volume.
    fn reset_sink(&mut self) {
        let volume = self.sink.volume();
        self.sink.stop();
        self.sink = Sink::new().0;
        self.sink.set_volume(volume);
    }

    fn freeze_at_current_position(&mut self) {
        self.position_offset_ms = self.current_position_ms();
        self.reset_sink();
    }

    fn attach_source(&mut self, src: Box<dyn Source + Send>, mixer: &rodio::mixer::Mixer) {
        let sink = Sink::connect_new(mixer);
        sink.set_volume(self.sink.volume());
        sink.pause();
        sink.append(src);
        self.sink.stop();
        self.sink = sink;
        if !self.paused {
            self.sink.play();
        }
    }

    /// Reopens and seeks the decoder before attaching it, avoiding callback waits.
    /// A decoder failure pauses the track without changing device availability.
    fn restore_on_output(&mut self, mixer: &rodio::mixer::Mixer) -> Result<()> {
        let result: Result<()> = (|| {
            if let Some(track) = &self.current_track {
                let mut src = audio_input::decoder(&track.info.path)?;
                src.try_seek(Duration::from_millis(self.position_offset_ms))
                    .map_err(|e| anyhow::anyhow!("Cannot restore playback position: {e:?}"))?;
                self.attach_source(src, mixer);
            }
            Ok(())
        })();
        if let Err(error) = &result {
            self.reset_sink();
            self.paused = true;
            self.playback_error = Some(format!("{error}"));
        } else {
            self.playback_error = None;
        }
        result
    }

    fn load_and_play(
        &mut self,
        path: PathBuf,
        mixer: Option<&rodio::mixer::Mixer>,
    ) -> Result<TrackInfo> {
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
        self.reset_sink();
        self.position_offset_ms = 0;
        self.paused = false;
        self.playback_error = None;
        self.current_track = Some(CurrentTrack { info: info.clone() });
        if let Some(mixer) = mixer {
            self.attach_source(src, mixer);
        }
        Ok(info)
    }

    fn pause(&mut self) {
        self.paused = true;
        self.sink.pause();
    }

    fn resume(&mut self, mixer: Option<&rodio::mixer::Mixer>) {
        self.paused = false;
        let retry_track = self.playback_error.take().is_some();
        if let Some(mixer) = mixer {
            if retry_track {
                // restore_on_output records any failure for the next status snapshot.
                let _ = self.restore_on_output(mixer);
            } else {
                self.sink.play();
            }
        }
    }

    fn stop(&mut self) {
        self.reset_sink();
        self.current_track = None;
        self.position_offset_ms = 0;
        self.paused = true;
        self.playback_error = None;
    }

    fn track_reached_end(&self) -> bool {
        self.current_track.is_some() && self.playback_error.is_none() && self.sink.empty()
    }

    fn seek(&mut self, to_ms: u64, mixer: Option<&rodio::mixer::Mixer>) -> Result<()> {
        let Some(track) = &self.current_track else {
            return Ok(());
        };
        self.position_offset_ms = to_ms.min(track.info.duration_ms.saturating_sub(1));
        self.reset_sink();
        self.playback_error = None;
        if let Some(mixer) = mixer {
            self.restore_on_output(mixer)?;
        }
        Ok(())
    }
}

/// Coordinates the output connection and track playback; both remain internal.
pub struct Player {
    output: OutputConnection,
    playback_manager: PlaybackManager,
    mode: PlayerMode,
}

impl Player {
    pub fn new() -> Result<Self> {
        let mut player = Self {
            output: OutputConnection::new(Instant::now()),
            playback_manager: PlaybackManager::new(),
            mode: PlayerMode::Default,
        };
        player.maintain_audio_output();
        Ok(player)
    }

    pub fn output_state(&self) -> OutputState {
        self.output.output_state()
    }
    pub fn playback_error(&self) -> Option<&str> {
        self.playback_manager.playback_error.as_deref()
    }

    /// Checks stream failures and scheduled connection attempts, even without UI polling.
    pub fn maintain_audio_output(&mut self) {
        self.maintain_audio_output_with(Instant::now(), open_default_audio_output);
    }

    fn maintain_audio_output_with(
        &mut self,
        now: Instant,
        open: impl FnOnce(Arc<AtomicBool>) -> Result<AudioOutput>,
    ) {
        if self.output.connection_failed() {
            self.playback_manager.freeze_at_current_position();
            self.output.schedule_reconnection(now);
        }
        if self.output.connect_if_due(now, open) && self.playback_error().is_none() {
            if let Some(mixer) = self.output.mixer() {
                let _ = self.playback_manager.restore_on_output(mixer);
            }
        }
    }

    pub fn current_track(&self) -> Option<&CurrentTrack> {
        self.playback_manager.current_track.as_ref()
    }
    pub fn current_position_ms(&self) -> u64 {
        self.playback_manager.current_position_ms()
    }
    pub fn is_paused(&self) -> bool {
        self.playback_manager.paused
    }
    pub fn pause(&mut self) {
        self.playback_manager.pause();
    }
    pub fn stop(&mut self) {
        self.playback_manager.stop();
    }

    pub fn load_and_play(&mut self, path: PathBuf) -> Result<TrackInfo> {
        self.maintain_audio_output();
        self.playback_manager
            .load_and_play(path, self.output.mixer())
    }

    pub fn resume(&mut self) {
        self.maintain_audio_output();
        self.playback_manager.resume(self.output.mixer());
    }

    pub fn is_finished(&self) -> bool {
        self.output_state() == OutputState::Ready && self.playback_manager.track_reached_end()
    }

    pub fn seek(&mut self, to_ms: u64) -> Result<()> {
        self.maintain_audio_output();
        self.playback_manager.seek(to_ms, self.output.mixer())
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

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            static ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "cadence-recovery-{}-{}.wav",
                std::process::id(),
                ID.fetch_add(1, Ordering::Relaxed)
            ));
            let samples = 44100u32 * 4;
            let bytes = samples * 2;
            let mut wav = Vec::new();
            wav.extend_from_slice(b"RIFF");
            wav.extend_from_slice(&(36 + bytes).to_le_bytes());
            wav.extend_from_slice(b"WAVEfmt ");
            wav.extend_from_slice(&16u32.to_le_bytes());
            wav.extend_from_slice(&1u16.to_le_bytes());
            wav.extend_from_slice(&1u16.to_le_bytes());
            wav.extend_from_slice(&44100u32.to_le_bytes());
            wav.extend_from_slice(&88200u32.to_le_bytes());
            wav.extend_from_slice(&2u16.to_le_bytes());
            wav.extend_from_slice(&16u16.to_le_bytes());
            wav.extend_from_slice(b"data");
            wav.extend_from_slice(&bytes.to_le_bytes());
            for sample in 0..samples {
                wav.extend_from_slice(&((sample % 200) as i16 * 100).to_le_bytes());
            }
            std::fs::write(&path, wav).unwrap();
            Self(path)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    fn fake_output() -> (AudioOutput, rodio::mixer::MixerSource) {
        let (mixer, source) = rodio::mixer::mixer(1, 44100);
        (
            AudioOutput {
                _stream: None,
                mixer,
            },
            source,
        )
    }

    fn player_before_first_connection(now: Instant) -> Player {
        Player {
            output: OutputConnection::new(now),
            playback_manager: PlaybackManager::new(),
            mode: PlayerMode::Default,
        }
    }

    fn output_failure_signal(player: &Player) -> Arc<AtomicBool> {
        match &player.output {
            OutputConnection::Ready { failed, .. } => failed.clone(),
            _ => panic!("test requires an opened output"),
        }
    }

    /// Defers output attempts so command tests never open an OS device.
    fn player_with_deferred_connection(now: Instant) -> Player {
        let mut player = player_before_first_connection(now);
        player.output = OutputConnection::new(now + Duration::from_secs(60));
        player
    }

    #[test]
    fn retries_back_off_and_ignore_retired_callbacks() {
        let now = Instant::now();
        let mut player = player_before_first_connection(now);
        let mut attempts = 0;
        for ms in [0, 250, 750, 1750, 3750, 5750] {
            if ms > 0 {
                player.maintain_audio_output_with(now + Duration::from_millis(ms - 1), |_| {
                    panic!("early retry")
                });
            }
            player.maintain_audio_output_with(now + Duration::from_millis(ms), |_| {
                attempts += 1;
                anyhow::bail!("output absent")
            });
            assert_eq!(player.output_state(), OutputState::Recovering);
            assert!(!player.is_finished());
        }
        assert_eq!(attempts, 6);
        let (output, _source) = fake_output();
        player.maintain_audio_output_with(now + Duration::from_millis(7750), |_| Ok(output));
        let retired = output_failure_signal(&player);
        retired.store(true, Ordering::Release);
        let (replacement, _source2) = fake_output();
        player.maintain_audio_output_with(now + Duration::from_millis(7800), |_| Ok(replacement));
        retired.store(true, Ordering::Release);
        player.maintain_audio_output_with(now + Duration::from_millis(7900), |_| {
            panic!("stale signal")
        });
        assert_eq!(player.output_state(), OutputState::Ready);
        output_failure_signal(&player).store(true, Ordering::Release);
        player.maintain_audio_output_with(now + Duration::from_millis(8000), |_| {
            anyhow::bail!("absent again")
        });
        assert!(matches!(player.output,
            OutputConnection::AwaitingConnection { next_attempt, .. }
                if next_attempt == now + Duration::from_millis(8250)));
    }

    #[test]
    fn recovery_restores_position_volume_and_pause_then_seeks_absolutely() {
        let fixture = Fixture::new();
        let now = Instant::now();
        let mut player = player_with_deferred_connection(now);
        player.load_and_play(fixture.0.clone()).unwrap();
        player.seek(1500).unwrap();
        player.pause();
        player.playback_manager.sink.set_volume(0.4);
        let (output, mut source) = fake_output();
        player.maintain_audio_output_with(now + Duration::from_secs(60), |_| Ok(output));
        for _ in 0..4410 {
            source.next();
        }
        assert!(player.is_paused());
        assert_eq!(player.current_position_ms(), 1500);
        assert_eq!(player.playback_manager.sink.volume(), 0.4);
        player.resume();
        for _ in 0..4410 {
            source.next();
        }
        assert!((1580..=1610).contains(&player.current_position_ms()));
        output_failure_signal(&player).store(true, Ordering::Release);
        let frozen = player.current_position_ms();
        player.maintain_audio_output_with(now, |_| anyhow::bail!("switching"));
        assert_eq!(player.current_position_ms(), frozen);
        assert!(!player.is_finished());
        let (output, mut source) = fake_output();
        player.maintain_audio_output_with(now + Duration::from_millis(250), |_| Ok(output));
        assert!(!player.is_paused());
        for _ in 0..4410 {
            source.next();
        }
        assert!((frozen + 80..=frozen + 110).contains(&player.current_position_ms()));
        for target in [3000, 200, 2000, 0] {
            player.seek(target).unwrap();
            assert_eq!(player.current_position_ms(), target);
            for _ in 0..4410 {
                source.next();
            }
            assert!((target + 80..=target + 110).contains(&player.current_position_ms()));
        }
    }

    #[test]
    fn commands_during_outage_replace_or_cancel_recovery_target() {
        let first = Fixture::new();
        let second = Fixture::new();
        let now = Instant::now();
        let mut player = player_with_deferred_connection(now);
        player.load_and_play(first.0.clone()).unwrap();
        player.seek(3000).unwrap();
        player.pause();
        assert!(player.is_paused());
        player.load_and_play(second.0.clone()).unwrap();
        assert_eq!(player.current_track().unwrap().info.path, second.0);
        assert_eq!(player.current_position_ms(), 0);
        assert!(!player.is_paused());
        player.seek(99999).unwrap();
        assert_eq!(player.current_position_ms(), 3999);
        player.set_mode(PlayerMode::Replay);
        assert!(!player.is_finished());
        player.stop();
        let (output, mut source) = fake_output();
        player.maintain_audio_output_with(now + Duration::from_secs(60), |_| Ok(output));
        assert!(player.current_track().is_none());
        assert_eq!(player.current_position_ms(), 0);
        assert!(player.is_paused());
        assert!(!player.is_finished());
        assert_eq!(source.next(), None);
    }

    #[test]
    fn restoration_error_preserves_track_and_does_not_advance() {
        let fixture = Fixture::new();
        let now = Instant::now();
        let mut player = player_with_deferred_connection(now);
        player.load_and_play(fixture.0.clone()).unwrap();
        player.seek(1700).unwrap();
        let original_audio = std::fs::read(&fixture.0).unwrap();
        std::fs::remove_file(&fixture.0).unwrap();
        let (output, _source) = fake_output();
        player.maintain_audio_output_with(now + Duration::from_secs(60), |_| Ok(output));
        assert_eq!(player.output_state(), OutputState::Ready);
        assert!(player.playback_error().is_some());
        assert_eq!(player.current_position_ms(), 1700);
        assert!(player.current_track().is_some());
        assert!(player.is_paused());
        assert!(!player.is_finished());
        player.maintain_audio_output_with(now + Duration::from_secs(120), |_| {
            panic!("retrying decoder error")
        });

        // Track errors must not prevent independent output recovery or cause EOF.
        output_failure_signal(&player).store(true, Ordering::Release);
        player.maintain_audio_output_with(now + Duration::from_secs(121), |_| {
            anyhow::bail!("output also lost")
        });
        assert_eq!(player.output_state(), OutputState::Recovering);
        assert!(player.playback_error().is_some());
        let (output, mut source) = fake_output();
        player.maintain_audio_output_with(now + Duration::from_millis(121250), |_| Ok(output));
        assert_eq!(player.output_state(), OutputState::Ready);
        assert!(player.playback_error().is_some());
        assert!(!player.is_finished());

        // An explicit retry restores only the track, reusing the healthy output.
        std::fs::write(&fixture.0, original_audio).unwrap();
        let signal = output_failure_signal(&player);
        player.resume();
        assert!(Arc::ptr_eq(&signal, &output_failure_signal(&player)));
        assert!(player.playback_error().is_none());
        assert!(!player.is_paused());
        assert_eq!(player.current_position_ms(), 1700);
        for _ in 0..4410 {
            source.next();
        }
        assert!((1780..=1810).contains(&player.current_position_ms()));
    }

    /// Uses the real output callback, muted. Decoder-only timing excludes this scheduling cost.
    #[test]
    #[ignore = "requires an output device and CADENCE_AUDIO_TEST_FILE"]
    fn external_file_device_seek_timing() {
        let path = PathBuf::from(
            std::env::var_os("CADENCE_AUDIO_TEST_FILE").expect("Set CADENCE_AUDIO_TEST_FILE"),
        );
        let mut player = Player::new().unwrap();
        player.playback_manager.sink.set_volume(0.0);
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

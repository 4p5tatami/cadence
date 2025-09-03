use anyhow::{Context, Result};
use parking_lot::Mutex;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::{fs::File, io::BufReader, path::Path, sync::Arc, time::Duration};

#[derive(Debug, Clone)]
pub struct TrackInfo {
    pub path: String,
    pub duration_ms: Option<u64>,
}

pub struct Player {
    _stream: OutputStream,
    handle: OutputStreamHandle,
    sink: Arc<Mutex<Sink>>,
}

impl Player {
    pub fn new_default() -> Result<Self> {
        let (_stream, handle) = OutputStream::try_default()
            .context("No default output device available")?;
        let sink = Sink::try_new(&handle).context("Failed to create sink")?;
        Ok(Self { _stream, handle, sink: Arc::new(Mutex::new(sink)) })
    }

    pub fn load_and_play<P: AsRef<Path>>(&self, path: P) -> Result<TrackInfo> {
        let file = File::open(path.as_ref())
            .with_context(|| format!("Failed to open {:?}", path.as_ref()))?;
        let source = Decoder::new(BufReader::new(file))
            .with_context(|| format!("Unsupported/invalid audio: {:?}", path.as_ref()))?;

        // Best-effort: report duration if known (many formats expose it).
        let dur = source.total_duration().map(|d| d.as_millis() as u64);

        let sink = self.sink.lock();
        sink.stop(); // stop anything currently queued
        let file = File::open(path.as_ref())?;
        let source = Decoder::new(BufReader::new(file))?;
        sink.append(source);
        sink.play();

        Ok(TrackInfo {
            path: path.as_ref().to_string_lossy().into_owned(),
            duration_ms: dur,
        })
    }

    pub fn pause(&self) { self.sink.lock().pause(); }
    pub fn resume(&self) { self.sink.lock().play(); }
    pub fn stop(&self) { self.sink.lock().stop(); }

    /// Naive “seek”: stops + re-queues from an offset by skipping samples (approx).
    /// This is placeholder until we switch to a decoder with random access control.
    pub fn seek_approx<P: AsRef<Path>>(&self, path: P, to_ms: u64) -> Result<()> {
        let file = File::open(path.as_ref())?;
        let mut src = Decoder::new(BufReader::new(file))?;

        // Rough skip; trades CPU for simplicity in MVP.
        let to = Duration::from_millis(to_ms);
        let skipped = src.skip_duration(to);
        if skipped < to {
            // reached EOF; just stop
            self.stop();
            return Ok(());
        }
        let sink = self.sink.lock();
        sink.stop();
        sink.append(src);
        sink.play();
        Ok(())
    }

    pub fn is_paused(&self) -> bool { self.sink.lock().is_paused() }
    pub fn empty(&self) -> bool { self.sink.lock().empty() }
}

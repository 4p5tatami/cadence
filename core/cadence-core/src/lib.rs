use anyhow::{Context, Result};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use serde::Serialize;
use std::{fs::File, io::BufReader, path::Path};

#[derive(Debug, Clone, Serialize)]
pub struct TrackInfo {
    pub path: String,
    pub duration_ms: Option<u64>,
}

pub struct Player {
    sink: Sink,
}

impl Player {
    pub fn new_default() -> Result<Self> {
        let (_, handle) =
            OutputStream::try_default().context("No default output device available")?;
        let sink = Sink::try_new(&handle).context("Failed to create sink")?;
        Ok(Self { sink })
    }

    pub fn load_and_play<P: AsRef<Path>>(&self, path: P) -> Result<TrackInfo> {
        let p = path.as_ref();

        // Open once for duration using the same decoder we’ll use for playback.
        let f1 = File::open(p).with_context(|| format!("Failed to open {:?}", p))?;
        let src = Decoder::new(BufReader::new(f1))
            .with_context(|| format!("Unsupported/invalid audio: {:?}", p))?;
        let dur = src.total_duration().map(|d| d.as_millis() as u64);

        self.sink.stop();
        self.sink.append(src);
        self.sink.play();

        Ok(TrackInfo {
            path: p.to_string_lossy().into_owned(),
            duration_ms: dur,
        })
    }

    pub fn pause(&self) {
        self.sink.pause();
    }
    pub fn resume(&self) {
        self.sink.play();
    }
    pub fn stop(&self) {
        self.sink.stop();
    }

    /// Naive “seek”: stops + re-queues from an offset by skipping samples (approx).
    /// This is placeholder until we switch to a decoder with random access control.
    pub fn seek_approx<P: AsRef<Path>>(&self, path: P, to_ms: u64) -> Result<()> {
        use std::time::Duration;

        let path = path.as_ref();

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

        self.sink.stop();
        self.sink.append(skipped);
        self.sink.play();

        Ok(())
    }

    pub fn sleep_until_end(&self) {
        self.sink.sleep_until_end();
    }
}

//! Correct exceptional container timelines while retaining Symphonia's native
//! seeking and decoding: legacy Flake sample numbering and nonzero Ogg origins.
use crate::audio_input::AudioInput;
use anyhow::{ensure, Context, Result};
use rodio::{source::SeekError, Source};
use std::{path::Path, time::Duration};
use symphonia::core::{
    audio::{SampleBuffer, SignalSpec},
    codecs::Decoder,
    formats::{FormatReader, SeekMode, SeekTo},
    io::MediaSourceStream,
};

pub(crate) struct TimelineSource {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    buffer: SampleBuffer<f32>,
    offset: usize,
    rate: u32,
    channels: u16,
    block_size: u64,
    scale: u64,
    start_ts: u64,
    track_id: u32,
    total_samples: u64,
}

fn probe(input: AudioInput) -> Result<Box<dyn FormatReader>> {
    Ok(symphonia::default::get_probe()
        .format(
            &Default::default(),
            MediaSourceStream::new(Box::new(input), Default::default()),
            &symphonia::core::formats::FormatOptions {
                enable_gapless: true,
                ..Default::default()
            },
            &Default::default(),
        )?
        .format)
}

impl TimelineSource {
    pub(crate) fn detect(path: &Path, legacy_block_size: Option<u16>) -> Result<Option<Self>> {
        let mut format = probe(AudioInput::open(path)?)?;
        let track_id = format.default_track().context("Missing audio track")?.id;
        let params = format
            .default_track()
            .context("Missing audio track")?
            .codec_params
            .clone();
        let (scale, block_size) = if let Some(block_size) = legacy_block_size {
            let first = format.next_packet()?;
            let second = match format.next_packet() {
                Ok(packet) => packet,
                Err(symphonia::core::errors::Error::IoError(error))
                    if error.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    return Ok(None)
                }
                Err(error) => return Err(error.into()),
            };
            if second.ts == first.dur {
                return Ok(None);
            }
            let block_size = block_size as u64;
            ensure!(
                first.ts == 0 && first.dur == block_size && second.ts == block_size * block_size,
                "Cannot repair FLAC: inconsistent frame numbering"
            );
            (block_size, block_size)
        } else {
            if params.start_ts == 0 || params.codec != symphonia::core::codecs::CODEC_TYPE_VORBIS {
                return Ok(None);
            }
            // Vorbis overlap history needs preroll. 8192 samples cover its maximum block.
            (1, 8192)
        };
        let total_samples = params
            .n_frames
            .filter(|n| *n > 0)
            .context("Cannot translate audio timeline without a total sample count")?;
        ensure!(
            params.start_ts.checked_add(total_samples).is_some(),
            "Audio timeline overflows"
        );
        let mut input = AudioInput::open(path)?;
        if scale != 1 {
            input.prepare_legacy()?;
        }
        let rate = params.sample_rate.context("Missing sample rate")?;
        let channels = params.channels.context("Missing channels")?;
        let mut source = Self {
            format: probe(input)?,
            decoder: symphonia::default::get_codecs().make(&params, &Default::default())?,
            buffer: SampleBuffer::new(block_size, SignalSpec::new(rate, channels)),
            offset: 0,
            rate,
            channels: channels.count() as u16,
            block_size,
            scale,
            start_ts: params.start_ts,
            track_id,
            total_samples,
        };
        source.read_packet()?;
        Ok(Some(source))
    }

    fn read_packet(&mut self) -> Result<(u64, u64)> {
        let packet = loop {
            let packet = self.format.next_packet()?;
            if packet.track_id() == self.track_id {
                break packet;
            }
        };
        ensure!(packet.ts % self.scale == 0, "Invalid legacy FLAC timestamp");
        let start = (packet.ts / self.scale).saturating_sub(self.start_ts);
        ensure!(
            packet.dur <= self.block_size
                && start
                    .checked_add(packet.dur)
                    .is_some_and(|end| end <= self.total_samples),
            "Invalid audio packet duration"
        );
        let decoded = self.decoder.decode(&packet)?;
        ensure!(
            (self.scale == 1 || decoded.frames() as u64 == packet.dur)
                && decoded.spec().rate == self.rate
                && decoded.spec().channels.count() == self.channels as usize,
            "Invalid decoded audio format"
        );
        self.buffer.copy_interleaved_ref(decoded);
        self.offset = 0;
        Ok((start, self.buffer.len() as u64 / self.channels as u64))
    }

    fn seek(&mut self, position: Duration) -> Result<()> {
        let channel = self.offset % self.channels as usize;
        let sample = (position.as_nanos() * self.rate as u128 / 1_000_000_000)
            .min(self.total_samples.saturating_sub(1) as u128) as u64;
        // Packet timestamps are scaled but durations aren't. Native seeking can
        // land after the requested timestamp, so start one maximum block earlier.
        // This also handles Flake's variable-size blocks with the fixed flag set.
        let search_start = sample.saturating_sub(self.block_size);
        self.format.seek(
            SeekMode::Accurate,
            SeekTo::TimeStamp {
                ts: (search_start + self.start_ts) * self.scale,
                track_id: self.track_id,
            },
        )?;
        self.decoder.reset();
        self.buffer.clear();
        self.offset = 0;
        let mut decoded_samples = 0;
        let actual = loop {
            let (start, duration) = self.read_packet()?;
            decoded_samples += duration;
            ensure!(
                start <= sample && decoded_samples <= 2 * self.block_size,
                "Audio seek landed outside the refinement window"
            );
            if sample < start + duration {
                break start;
            }
        };
        self.offset = (sample - actual) as usize * self.channels as usize + channel;
        ensure!(
            self.offset < self.buffer.len(),
            "Audio seek exceeds decoded frame"
        );
        Ok(())
    }
}

impl Iterator for TimelineSource {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        while self.offset >= self.buffer.len() {
            self.read_packet().ok()?;
        }
        let sample = *self.buffer.samples().get(self.offset)?;
        self.offset += 1;
        Some(sample)
    }
}

impl Source for TimelineSource {
    // Sample rate and channel count are constant across the entire stream.
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        self.channels
    }
    fn sample_rate(&self) -> u32 {
        self.rate
    }
    fn total_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs_f64(
            self.total_samples as f64 / self.rate as f64,
        ))
    }
    fn try_seek(&mut self, position: Duration) -> Result<(), SeekError> {
        self.seek(position)
            .map_err(|error| SeekError::Other(error.into_boxed_dyn_error()))
    }
}

//! A streaming, read-only compatibility fix for old fixed-block FLAC encoders.
use anyhow::{ensure, Context, Result};
use rodio::{Decoder, Source};
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

pub(crate) struct AudioInput<R = File> {
    inner: R,
    len: u64,
    position: u64,
    block_size_patch: Option<[u8; 2]>,
    legacy_metadata: Vec<(u64, u8)>,
    is_ogg: bool,
}

impl AudioInput<File> {
    pub(crate) fn open(path: &Path) -> Result<Self> {
        let file = File::open(path).with_context(|| format!("Failed to open {path:?}"))?;
        let len = file.metadata()?.len();
        Self::new(file, len).with_context(|| format!("Invalid audio header: {path:?}"))
    }
}

impl<R: Read + Seek> AudioInput<R> {
    fn new(mut inner: R, len: u64) -> Result<Self> {
        let block_size_patch = inspect_flac(&mut inner, len)?;
        inner.rewind()?;
        let mut magic = [0; 4];
        let is_ogg = len >= 4 && {
            inner.read_exact(&mut magic)?;
            magic == *b"OggS"
        };
        inner.rewind()?;
        Ok(Self {
            inner,
            len,
            position: 0,
            block_size_patch,
            legacy_metadata: Vec::new(),
            is_ogg,
        })
    }

    /// The legacy adapter owns duration/range checks. Hide the unscaled total
    /// and seek table from the native demuxer, whose timestamps are scaled.
    pub(super) fn prepare_legacy(&mut self) -> Result<()> {
        self.inner.seek(SeekFrom::Start(21))?;
        let mut byte = [0];
        self.inner.read_exact(&mut byte)?;
        self.legacy_metadata.push((21, byte[0] & 0xf0));
        self.legacy_metadata
            .extend((22..26).map(|offset| (offset, 0)));
        let mut offset = 4;
        loop {
            self.inner.seek(SeekFrom::Start(offset))?;
            let mut header = [0; 4];
            self.inner.read_exact(&mut header)?;
            if header[0] & 0x7f == 3 {
                self.legacy_metadata.push((offset, (header[0] & 0x80) | 1));
            }
            if header[0] & 0x80 != 0 {
                break;
            }
            offset += 4 + u32::from_be_bytes([0, header[1], header[2], header[3]]) as u64;
        }
        self.rewind()?;
        Ok(())
    }
}

impl<R: Read> Read for AudioInput<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let count = self.inner.read(buf)?;
        if let Some(patch) = self.block_size_patch {
            for (i, value) in patch.into_iter().enumerate() {
                let offset = 8 + i as u64;
                if offset >= self.position && offset - self.position < count as u64 {
                    buf[(offset - self.position) as usize] = value;
                }
            }
        }
        for &(offset, value) in &self.legacy_metadata {
            if offset >= self.position && offset - self.position < count as u64 {
                buf[(offset - self.position) as usize] = value;
            }
        }
        self.position += count as u64;
        Ok(count)
    }
}

impl<R: Seek> Seek for AudioInput<R> {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        self.position = self.inner.seek(from)?;
        Ok(self.position)
    }
}

impl symphonia::core::io::MediaSource for AudioInput<File> {
    fn is_seekable(&self) -> bool {
        true
    }
    fn byte_len(&self) -> Option<u64> {
        Some(self.len)
    }
}

pub(crate) fn decoder(path: &Path) -> Result<Box<dyn Source + Send>> {
    let input = AudioInput::open(path)?;
    if input.block_size_patch.is_some() || input.is_ogg {
        if let Some(legacy) = crate::timeline_source::TimelineSource::detect(
            path,
            input.block_size_patch.map(u16::from_be_bytes),
        )? {
            return Ok(Box::new(legacy));
        }
    }
    let len = input.len;
    let mut builder = Decoder::builder()
        .with_data(input)
        .with_byte_len(len)
        .with_seekable(true)
        .with_coarse_seek(false);
    if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
        builder = builder.with_hint(&ext.to_ascii_lowercase());
    }
    builder
        .build()
        .map(|source| Box::new(source) as Box<dyn Source + Send>)
        .with_context(|| format!("Unsupported/invalid audio: {path:?}"))
}

fn inspect_flac<R: Read + Seek>(input: &mut R, len: u64) -> Result<Option<[u8; 2]>> {
    let mut magic = [0; 4];
    if len < 4 {
        return Ok(None);
    }
    input.read_exact(&mut magic)?;
    if &magic != b"fLaC" {
        return Ok(None);
    }

    let mut header = [0; 4];
    input
        .read_exact(&mut header)
        .context("Truncated FLAC metadata header")?;
    ensure!(
        header[0] & 0x7f == 0 && header[1..] == [0, 0, 34],
        "Invalid FLAC STREAMINFO block"
    );
    let mut info = [0; 34];
    input
        .read_exact(&mut info)
        .context("Truncated FLAC STREAMINFO")?;
    let min = u16::from_be_bytes([info[0], info[1]]);
    if min >= 16 {
        return Ok(None);
    }
    let max = u16::from_be_bytes([info[2], info[3]]);
    ensure!(max >= 16, "Cannot repair FLAC: invalid maximum block size");

    // Metadata can contain large pictures/padding: seek over it, never allocate it.
    let mut offset = 42u64;
    while header[0] & 0x80 == 0 {
        ensure!(len.saturating_sub(offset) >= 4, "Truncated FLAC metadata");
        input.read_exact(&mut header)?;
        ensure!(
            header[0] & 0x7f != 0 && header[0] & 0x7f != 127,
            "Invalid FLAC metadata block type"
        );
        let size = u32::from_be_bytes([0, header[1], header[2], header[3]]) as u64;
        offset += 4;
        ensure!(
            size <= len.saturating_sub(offset),
            "FLAC metadata extends past end of file"
        );
        offset += size;
        input.seek(SeekFrom::Start(offset))?;
    }

    // A first fixed-block frame has frame number zero (one byte). Including
    // optional block-size/sample-rate fields and CRC, its header is at most 10 bytes.
    let mut frame = [0u8; 10];
    input
        .read_exact(&mut frame[..5])
        .context("Truncated FLAC frame header")?;
    ensure!(
        frame[0] == 0xff && frame[1] == 0xf8 && frame[4] == 0,
        "Cannot repair FLAC: expected first fixed-block frame"
    );
    ensure!(
        frame[3] & 1 == 0 && frame[3] >> 4 <= 10 && (frame[3] >> 1) & 7 != 3,
        "Invalid FLAC frame header"
    );
    let block_code = frame[2] >> 4;
    let rate_code = frame[2] & 15;
    ensure!(
        block_code != 0 && rate_code != 15,
        "Reserved FLAC frame header value"
    );
    let block_bytes = match block_code {
        6 => 1,
        7 => 2,
        _ => 0,
    };
    let rate_bytes = match rate_code {
        12 => 1,
        13 | 14 => 2,
        _ => 0,
    };
    let header_len = 5 + block_bytes + rate_bytes + 1;
    input
        .read_exact(&mut frame[5..header_len])
        .context("Truncated FLAC frame header")?;
    ensure!(
        crc8(&frame[..header_len]) == 0,
        "Invalid FLAC frame header CRC"
    );
    let block_size = match block_code {
        1 => 192,
        2..=5 => 576u32 << (block_code - 2),
        6 => frame[5] as u32 + 1,
        7 => u16::from_be_bytes([frame[5], frame[6]]) as u32 + 1,
        8..=15 => 256u32 << (block_code - 8),
        _ => unreachable!(),
    };
    ensure!(
        block_size == max as u32,
        "Cannot repair FLAC: frame and STREAMINFO block sizes disagree"
    );
    Ok(Some(max.to_be_bytes()))
}

fn crc8(bytes: &[u8]) -> u8 {
    bytes.iter().fold(0u8, |mut crc, byte| {
        crc ^= byte;
        for _ in 0..8 {
            crc = if crc & 0x80 != 0 {
                (crc << 1) ^ 7
            } else {
                crc << 1
            };
        }
        crc
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rodio::Source;
    use std::io::Cursor;
    use std::time::{Duration, Instant};

    fn fixture() -> Vec<u8> {
        let mut data = b"fLaC\x80\0\0\x22".to_vec();
        data.extend([0; 34]);
        data[10..12].copy_from_slice(&4608u16.to_be_bytes());
        let header = [0xff, 0xf8, 0x79, 0x18, 0, 0x11, 0xff];
        data.extend(header);
        data.push(crc8(&header));
        data
    }

    fn reader(data: Vec<u8>) -> Result<AudioInput<Cursor<Vec<u8>>>> {
        let len = data.len() as u64;
        AudioInput::new(Cursor::new(data), len)
    }

    #[test]
    fn overlay_preserves_bytes_offsets_and_underlying_file() {
        let original = fixture();
        let mut input = reader(original.clone()).unwrap();
        let mut patched = original.clone();
        patched[8..10].copy_from_slice(&4608u16.to_be_bytes());
        // Every boundary, including single-byte reads through each patched byte.
        for start in 0..=original.len() {
            for size in 0..=12 {
                input.seek(SeekFrom::Start(start as u64)).unwrap();
                let mut buf = vec![0; size];
                let count = input.read(&mut buf).unwrap();
                assert_eq!(&buf[..count], &patched[start..start + count]);
            }
        }
        input.seek(SeekFrom::End(-1)).unwrap();
        assert_eq!(
            input.seek(SeekFrom::Current(-1)).unwrap(),
            original.len() as u64 - 2
        );
        assert_eq!(input.inner.into_inner(), original);
    }

    #[test]
    fn valid_and_non_flac_inputs_pass_through() {
        for data in [b"RIFFanything".to_vec(), vec![1, 2], {
            let mut data = fixture();
            // Valid variable-block STREAMINFO must not be normalized to fixed.
            data[8..10].copy_from_slice(&16u16.to_be_bytes());
            data[43] = 0xf9;
            data
        }] {
            let mut input = reader(data.clone()).unwrap();
            assert!(input.block_size_patch.is_none());
            let mut read = Vec::new();
            input.read_to_end(&mut read).unwrap();
            assert_eq!(read, data);
        }
    }

    #[test]
    fn rejects_truncation_and_unproven_repairs() {
        let data = fixture();
        for len in 4..data.len() {
            assert!(
                reader(data[..len].to_vec()).is_err(),
                "accepted length {len}"
            );
        }
        for (offset, value) in [(7, 33), (10, 0), (43, 0xf9), (46, 1), (49, 0)] {
            let mut bad = data.clone();
            bad[offset] = value;
            assert!(reader(bad).is_err(), "accepted corruption at {offset}");
        }
        let mut bad = data;
        bad[4] = 0; // Claimed additional metadata extends beyond the file.
        bad[42..46].copy_from_slice(&[0x81, 0xff, 0xff, 0xff]);
        assert!(reader(bad).is_err());
    }

    #[test]
    fn skips_metadata_without_copying_it() {
        let mut data = fixture();
        data[4] = 0;
        data.splice(42..42, [0x81, 0, 0, 3, 1, 2, 3]);
        assert_eq!(
            reader(data).unwrap().block_size_patch,
            Some(4608u16.to_be_bytes())
        );
    }

    /// Run with CADENCE_AUDIO_TEST_FILE pointing at a local test file. No audio device needed.
    #[test]
    #[ignore = "requires an external audio file; decodes a reference into memory"]
    fn external_file_seek_matches_sequential_decode() {
        let path =
            std::env::var_os("CADENCE_AUDIO_TEST_FILE").expect("Set CADENCE_AUDIO_TEST_FILE");
        let path = Path::new(&path);
        verify_seeks(path);
    }

    fn verify_seeks(path: &Path) {
        let original = std::fs::read(path).unwrap();
        let started = Instant::now();
        let source = decoder(path).unwrap();
        let startup = started.elapsed();
        let rate = source.sample_rate() as usize;
        let channels = source.channels() as usize;
        let declared_duration = source.total_duration();
        let reference: Vec<f32> = source.collect();
        assert!(!reference.is_empty());
        let duration_ms = (reference.len() / channels) as u64 * 1000 / rate as u64;
        if let Some(declared) = declared_duration {
            // MP3 Xing counts may include a non-audio header frame or encoder padding.
            let duration_tolerance = if path.extension().is_some_and(|ext| ext == "mp3") {
                50
            } else {
                2
            };
            assert!(
                duration_ms.abs_diff(declared.as_millis() as u64) <= duration_tolerance,
                "decoded only {duration_ms}ms, declared {declared:?}"
            );
        }
        let mut seeking = decoder(path).unwrap();
        let mut timings = Vec::new();
        for percent in [0, 10, 50, 90, 25, 99, 1, 75, 5, 95].repeat(3) {
            let target_ms = duration_ms * percent / 100;
            // Lossy codecs need overlap/filter history after reset. Compare after
            // 4096 frames of warm-up; the timeline tolerance is checked below.
            let warmup = match path.extension().and_then(|ext| ext.to_str()) {
                Some("mp3" | "m4a" | "aac" | "ogg") => 4096 * channels,
                _ => 0,
            };
            let expected = target_ms as usize * rate / 1000 * channels + warmup;
            let started = Instant::now();
            seeking
                .try_seek(Duration::from_millis(target_ms))
                .unwrap_or_else(|error| {
                    panic!("seek to {target_ms}ms ({percent}%) failed: {error:?}")
                });
            let actual: Vec<_> = seeking.by_ref().skip(warmup).take(256 * channels).collect();
            timings.push(started.elapsed());
            assert!(!actual.is_empty(), "seek to {target_ms}ms returned EOF");
            // Lossless seeking must agree within one sample. Lossy files can
            // have rounded container timestamps; allow at most 10ms after preroll.
            let tolerance = if warmup == 0 {
                1
            } else {
                (rate / 100) as isize
            };
            let matches = (-tolerance..=tolerance).any(|delta| {
                let offset = expected as isize + delta * channels as isize;
                offset >= 0
                    && reference
                        .get(offset as usize..offset as usize + actual.len())
                        .is_some_and(|slice| {
                            slice
                                .iter()
                                .zip(&actual)
                                .all(|(a, b)| (a - b).abs() <= 1e-6)
                        })
            });
            assert!(matches, "sample mismatch at {target_ms}ms ({percent}%)");
        }
        timings.sort();
        eprintln!(
            "{path:?}: startup={startup:?}, seek p95={:?}, max={:?}",
            timings[(timings.len() * 95 / 100).min(timings.len() - 1)],
            timings.last().unwrap()
        );
        assert_eq!(
            std::fs::read(path).unwrap(),
            original,
            "source file changed"
        );
    }

    struct TestFile(std::path::PathBuf);
    impl TestFile {
        fn new(data: &[u8]) -> Self {
            let unique = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "cadence-audio-{}-{unique}.flac",
                std::process::id()
            ));
            std::fs::write(&path, data).unwrap();
            Self(path)
        }
    }
    impl Drop for TestFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    #[test]
    fn ogg_with_nonzero_origin_rewinds_and_seeks_accurately() {
        let file = TestFile::new(include_bytes!("../tests/fixtures/nonzero-origin.ogg"));
        // The .flac suffix also verifies that content sniffing selects the adapter.
        // Its codec preroll permits the strict lossless-style comparison here.
        verify_seeks(&file.0);
    }

    // Original synthetic PCM encoded as verbatim FLAC subframes, with CRCs.
    // Covers the legacy variable-size/sample-number combination without bundling music.
    fn encoded_flac(legacy: bool, variable: bool, seek_table: bool) -> Vec<u8> {
        let sizes: Vec<u16> = if variable {
            vec![512, 128, 256, 512, 100]
        } else {
            vec![512; 5]
        };
        let total: u64 = sizes.iter().map(|n| *n as u64).sum();
        let mut data = b"fLaC\x80\0\0\x22".to_vec();
        data.extend([0; 34]);
        let min = if legacy {
            0
        } else {
            *sizes.iter().min().unwrap()
        };
        data[8..10].copy_from_slice(&min.to_be_bytes());
        data[10..12].copy_from_slice(&512u16.to_be_bytes());
        let packed = (44100u64 << 44) | (1 << 41) | (15 << 36) | total;
        data[18..26].copy_from_slice(&packed.to_be_bytes());
        if seek_table {
            data[4] = 0;
            data.extend([0x83, 0, 0, 18]);
            data.extend(0u64.to_be_bytes()); // sample zero, offset zero
            data.extend(0u64.to_be_bytes());
            data.extend(512u16.to_be_bytes());
        }
        let mut sample = 0u64;
        for (frame_number, &size) in sizes.iter().enumerate() {
            let mut frame = vec![
                0xff,
                if variable && !legacy { 0xf9 } else { 0xf8 },
                0x79,
                0x18,
            ];
            let sequence = if legacy || variable {
                sample
            } else {
                frame_number as u64
            };
            // Test sequences fit in Unicode's canonical UTF-8 range.
            let mut utf8 = [0; 4];
            frame.extend(
                char::from_u32(sequence as u32)
                    .unwrap()
                    .encode_utf8(&mut utf8)
                    .as_bytes(),
            );
            frame.extend((size - 1).to_be_bytes());
            frame.push(crc8(&frame));
            for channel in 0..2 {
                frame.push(2); // Verbatim subframe, no wasted bits.
                for index in sample..sample + size as u64 {
                    frame.extend(((index * 31337 + channel * 976) as i16).to_be_bytes());
                }
            }
            let mut crc = 0u16;
            for &byte in &frame {
                crc ^= (byte as u16) << 8;
                for _ in 0..8 {
                    crc = if crc & 0x8000 != 0 {
                        (crc << 1) ^ 0x8005
                    } else {
                        crc << 1
                    };
                }
            }
            frame.extend(crc.to_be_bytes());
            data.extend(frame);
            sample += size as u64;
        }
        data
    }

    #[test]
    fn generated_flac_seek_matrix() {
        for legacy in [false, true] {
            for variable in [false, true] {
                for seek_table in [false, true] {
                    let file = TestFile::new(&encoded_flac(legacy, variable, seek_table));
                    verify_seeks(&file.0);
                    // Exercise the final short block and saturating end behavior too.
                    let mut source = decoder(&file.0).unwrap();
                    let end = source
                        .total_duration()
                        .unwrap()
                        .saturating_sub(Duration::from_millis(1));
                    source.try_seek(end).unwrap();
                    assert!(source.next().is_some());
                }
            }
        }
        // Malformed minimum with otherwise normal frame numbering uses Rodio.
        let mut ordinary = encoded_flac(false, false, false);
        ordinary[8..10].fill(0);
        verify_seeks(&TestFile::new(&ordinary).0);
    }

    #[test]
    fn sink_seeks_while_paused_and_resumes_without_a_device() {
        use std::sync::{mpsc, Arc};
        let file = TestFile::new(&encoded_flac(true, true, true));
        let (sink, mut output) = rodio::Sink::new();
        let sink = Arc::new(sink);
        sink.append(decoder(&file.0).unwrap());
        sink.pause();
        for target in [15, 2, 25, 0] {
            let (reply, result) = mpsc::channel();
            let worker_sink = Arc::clone(&sink);
            let worker = std::thread::spawn(move || {
                reply
                    .send(worker_sink.try_seek(Duration::from_millis(target)))
                    .unwrap();
            });
            let started = Instant::now();
            loop {
                if let Ok(result) = result.try_recv() {
                    result.unwrap();
                    break;
                }
                assert!(
                    started.elapsed() < Duration::from_secs(2),
                    "paused seek blocked"
                );
                assert_eq!(output.next(), Some(0.0));
                std::thread::yield_now();
            }
            worker.join().unwrap();
            assert!(sink.is_paused());
            assert_eq!(sink.get_pos(), Duration::from_millis(target));
        }
        sink.play();
        assert!(output.by_ref().take(4000).any(|sample| sample != 0.0));
        sink.stop();
        // Drain the stop control, then append again to verify reuse after stop.
        for _ in 0..4000 {
            output.next();
        }
        sink.append(decoder(&file.0).unwrap());
        sink.play();
        assert!(output.take(4000).any(|sample| sample != 0.0));
    }
}

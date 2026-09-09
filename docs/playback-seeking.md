# Playback seeking

Playback uses Rodio 0.21.1 with an explicit file length, accurate seeking, and
sink-reported progress plus the restored decoder position. The player no longer
copies FLAC files or decodes from the beginning on each seek. Seeking prepares
a fresh decoder before attaching it to a fresh sink, so it cannot block waiting
for acknowledgment from a failed audio callback.

## Compatibility cases verified during implementation

The reported Flake 0.11 file has more than a zero minimum block size: its frames
use sample numbers with the fixed-block flag set, and their sizes vary. Merely
setting the minimum to the maximum makes Symphonia multiply sample positions
by 4608. For example, the second packet gets timestamp 21,233,664 instead of 4608.

`AudioInput` validates the metadata and first frame header, then overlays the
minimum block size during reads. `TimelineSource` detects legacy numbering from
the first two packets, translates seek timestamps, and lets Symphonia perform
its native binary search. It decodes at most two maximum blocks for refinement.
The adapter hides the unscaled total and seek table from the native demuxer and
owns duration/range checking itself. Audio frames and the original file are never
modified. No frame index, full-file buffer, or temporary playback file is used.

The same adapter handles Vorbis streams with a nonzero container origin, using
bounded preroll for codec overlap history. Other streams retain Rodio decoding.

Seek failures pause playback and are displayed, preserving the track for retry.
The desktop serializes seeks, retains only the latest pending target, and holds
the cursor until an authoritative post-seek status arrives.

## Validation

- Generated FLAC matrix: normal and legacy numbering, fixed and variable blocks,
  with and without seek tables; header truncation, CRC, bounds, and overlay reads.
- Committed synthetic Vorbis fixture: nonzero origin, rewind, repeated seeks.
- Sink tests: paused seeks, resume, stop, and reuse without an output device.
- Desktop tests: stale/out-of-order polls, replacement seeks, failed seeks,
  failed status refreshes, and local/remote track changes.
- External file checks: reported FLAC, ordinary FLAC, PCM WAV, MP3 including Xing
  VBR, M4A, and synthetic AAC/M4A and Vorbis encoded with the installed VLC library.
- Lossless sample comparisons allow one sample of rounding; external lossy
  comparisons allow 10 ms of container timing variation after codec warm-up.
  MP3 duration checks allow 50 ms for header-frame counts/padding.

Historical debug-build measurements before output recovery: reported FLAC
decoder seeks approximately 10 ms p95; muted real-device seek commands
approximately 12 ms p95. This does **not** measure acoustic latency. A listening
comparison with VLC and a physical mobile-client check remain manual validation.
Headerless files may still require the existing synchronous duration scan.

## Audio output recovery

The stream error callback only sets a flag belonging to that output attempt.
The player thread checks it every 25 ms independently of status requests,
freezes the sample position, and replaces the failed sink and stream. It opens
the current default output immediately, then retries after 250 ms, 500 ms,
1 second, and every 2 seconds. Late callbacks from retired attempts are ignored.

Recovery restores the track through the same decoder compatibility layer,
including its position, volume, mode, and playing/paused intent. Pause and Stop
cancel automatic resumption; a new track or seek changes the recovery target.
Decoder restoration failures remain visible and paused until the user retries
Play or selects another track. Output absence never counts as track completion.

Tauri and WebSocket track status include `output_state` (`ready`, `recovering`)
and nullable `playback_error`. Device availability is independent of decoder
errors: a failed track can have a ready output. `paused` retains user intent;
WebSocket `playing` is false unless the output is ready, playback is requested,
and there is no playback error. This
freezes existing mobile clients without a protocol migration. The desktop uses
the same condition for its clock and displays “Reconnecting audio output…”
while waiting. No output picker or silent default-device-change detection is
included.

`Player` coordinates an output connection and playback manager. `OutputConnection::new` schedules
the first connection attempt; `PlaybackManager::new` starts without a track.
`Player::new` calls `maintain_audio_output` immediately to attempt opening the
default device. The connection enum owns its connection and recovery methods and contains either
the live output and failure flag or the retry schedule. The playback manager
owns sink replacement, the saved position, pause intent, and decoder errors.
The player freezes playback before replacing a failed output and restores it
after reconnection. A decoder error requires an explicit playback retry but
does not prevent independent device recovery.

Automated recovery tests use generated WAV audio and a manually consumed Rodio
mixer, without an OS output device. They cover backoff, retired callbacks,
position offsets, commands during outages, restoration errors, and remote and
desktop clocks. Hardware acceptance remains manual: switch SoundBlaster outputs
while playing and paused, switch repeatedly, then disconnect/reconnect the
device. Check that the track resumes at the saved position only when requested,
the slider stays still during the outage, and no shuffle/replay occurs.

Run `cargo test --workspace --locked` and `npm test` / `npm run build` in
`apps/cadence-desktop`. External tests are opt-in:

```powershell
$env:CADENCE_AUDIO_TEST_FILE = 'absolute path to an audio file'
cargo test -p cadence-core --locked external_file -- --ignored --nocapture --test-threads=1
```

The device test is muted. External music files are not committed; FLAC fixtures
are generated by the tests and the Ogg fixture contains an original test tone.

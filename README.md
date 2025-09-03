# Cadence 🎶

**Cross-platform high-resolution music player**
Syncs playback across Windows & Android — like Spotify Connect, but with support for lossless formats (FLAC/WAV/etc).

## Features (MVP)
- Play/Pause/Resume/Stop local audio
- Support for FLAC/WAV/OGG/MP3
- Simple CLI: `cadence play <file>`

## Roadmap
- Cross-device sync (Windows ↔ Android)
- High-resolution playback (24-bit FLAC, etc.)
- Rich UI (desktop + mobile)
- Playlist & library management

## Development
### Requirements
- Rust (latest stable)
- Cargo
- Visual Studio Build Tools (Windows)

### Run locally
```bash
cargo run -p cadence-cli -- play "path/to/music.flac"

# Cadence

Cross-platform high-resolution music player.
Syncs playback across Windows & Android — like Spotify Connect, but with support for lossless formats (FLAC/WAV/etc).

## Features
- Play/Pause/Resume/Stop local audio
- Seek forward/rewind by seconds
- Supports FLAC, WAV, OGG, MP3, M4A, AAC, Opus, WMA
- Desktop app (Tauri + React)
- CLI for terminal use

## Project structure

```
core/cadence-core        # Audio engine (rodio-based), shared library
tools/cadence-cli        # Terminal interface
apps/cadence-desktop     # Desktop app (Tauri + React + TypeScript)
```

## Development

### Requirements
- Rust (latest stable)
- Node.js + npm
- Visual Studio Build Tools (Windows)
- WebView2 (Windows, usually pre-installed)

### Run the desktop app
```bash
cd apps/cadence-desktop
npm install
npm run tauri dev
```

### Run the CLI
```bash
cargo run -p cadence-cli -- "path/to/music.flac"
```

## Roadmap
- Cross-device sync (Windows & Android)
- High-resolution playback (24-bit FLAC, etc.)
- Playlist & library management
- Polished UI

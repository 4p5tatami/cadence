# Cadence

Cross-platform high-resolution music player.
Syncs playback across Windows & Android — like Spotify Connect, but with support for lossless formats (FLAC/WAV/etc).

## Features
- Play/Pause/Resume/Stop local audio
- Seek to any position (frame-accurate, via symphonia)
- Supports FLAC, WAV, OGG, MP3, M4A, AAC, Opus, WMA
- Desktop app (Tauri + React)

## Project structure

```
core/cadence-core        # Audio engine (rodio + symphonia), shared library
apps/cadence-desktop     # Desktop app (Tauri + React + TypeScript)
```

## Development

### Quick Setup (Automated)

**Linux/macOS:**
```bash
./setup.sh
```

**Windows (PowerShell as Admin):**
```powershell
.\setup.ps1
```

This automatically installs all dependencies and sets up the project.

### Manual Setup

**Requirements:**
- **Node.js 20.19+ or 22.12+**
- **Rust** (latest stable)
- **System dependencies** (Linux: WebKit2GTK, ALSA, etc. | Windows: Visual Studio Build Tools, WebView2)

> **📋 See [REQUIREMENTS.md](REQUIREMENTS.md) for detailed installation instructions**

### Run the desktop app
```bash
cd apps/cadence-desktop
npm run tauri dev
```

## Roadmap
- Cross-device sync (Windows & Android)
- High-resolution playback (24-bit FLAC, etc.)
- Playlist & library management
- Polished UI

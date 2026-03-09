# Cadence

Cross-platform music player with Android remote control.
Index your local library on the desktop, then search and control playback from your phone — like Spotify Connect, but for local lossless files.

## Features

- Plays FLAC, WAV, OGG, MP3, M4A, AAC, Opus, WMA
- Frame-accurate seek (via symphonia)
- SQLite music library with full-text search
- Android companion app — search, play, pause, seek over Wi-Fi
- Real-time sync between desktop and phone (WebSocket, ~100ms latency)

## Project structure

```
core/cadence-core        # Audio engine (rodio + symphonia) + library DB (SQLite/FTS5)
apps/cadence-desktop     # Desktop app (Tauri 2 + React + TypeScript)
apps/cadence-mobile      # Android companion app (React Native + Expo)
```

## Desktop app

### Requirements

- **Node.js** 20.19+ or 22.12+
- **Rust** (latest stable)
- **Windows**: Visual Studio Build Tools + WebView2 (usually pre-installed)
- **Linux**: WebKit2GTK, ALSA/PipeWire — see [REQUIREMENTS.md](REQUIREMENTS.md)

### Quick setup

**Windows (PowerShell as Admin):**
```powershell
.\setup.ps1
```

**Linux/macOS:**
```bash
./setup.sh
```

### Run

```bash
cd apps/cadence-desktop
npm run tauri dev
```

### Usage

1. Click **☰** (top-right) → **Index libraries** to open the library manager
2. Click **Add Folder** and select a folder containing music files — Cadence scans it recursively
3. Back on the main screen, use the search bar to find tracks
4. Click a track to play it
5. Use the progress bar to seek, and the Pause/Resume/Stop buttons to control playback
6. The WebSocket address (shown bottom-right) is what you enter in the Android app

## Android app

### Requirements

- [Expo Go](https://expo.dev/go) installed on your Android device (SDK 54)
- Phone and PC on the **same Wi-Fi network**
- Desktop app running

### Run

```bash
cd apps/cadence-mobile
npm install --legacy-peer-deps
npx expo start --clear
```

Scan the QR code with Expo Go.

### Usage

1. Enter the WebSocket address shown in the desktop app (e.g. `ws://192.168.1.x:7878`)
2. Tap **Connect**
3. Search for tracks — results come from the desktop's indexed library
4. Tap a track to play it on the desktop
5. Pause/Resume/Stop and seek from the phone's now-playing bar
6. Tap the progress bar to seek to any position

## Development

### Automated setup

```powershell
# Windows (PowerShell as Admin)
.\setup.ps1
```

```bash
# Linux/macOS
./setup.sh
```

### Manual setup

See [REQUIREMENTS.md](REQUIREMENTS.md) for detailed platform-specific instructions.

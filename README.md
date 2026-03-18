# Cadence

Cross-platform music player with Android remote control.
Index your local library on the desktop, then search and control playback from your phone — like Spotify Connect, but for local lossless files.

## Features

- Plays FLAC, WAV, OGG, MP3, M4A, AAC, Opus, WMA
- Frame-accurate seek (via symphonia)
- SQLite music library with full-text search
- Previous / Next track with shuffle and navigable history
- Android companion app — search, play, pause, seek, skip over Wi-Fi
- Auto-discovery of desktop app via mDNS
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
5. Use the progress bar to seek, and the Pause/Resume/Stop/Prev/Next buttons to control playback
6. The desktop app advertises itself on your local network automatically

## Android app

### Requirements

- Phone and PC on the **same Wi-Fi network**
- Desktop app running
- A development build installed on your device (see below) — Expo Go is **not** supported due to native modules (`react-native-zeroconf`)

### Building the Android app

The mobile app uses native modules and requires a custom development build via EAS:

```bash
cd apps/cadence-mobile
npm install --legacy-peer-deps
eas build --profile development --platform android
```

Install the resulting `.apk` on your device, then start the dev server:

```bash
npx expo start --clear
```

Open the Cadence app on your device and scan the QR code, or connect via the dev server URL.

### Usage

1. Open the app — it automatically scans for Cadence desktop instances on your network
2. Tap a discovered device to connect, or enter a WebSocket address manually (e.g. `ws://192.168.1.x:7878`)
3. Search for tracks — results come from the desktop's indexed library
4. Tap a track to play it on the desktop
5. Control playback from the now-playing bar: Pause/Resume/Stop, Prev/Next, and tap the progress bar to seek
6. If the connection drops, the app reconnects automatically

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

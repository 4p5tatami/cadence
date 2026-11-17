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
apps/cadence-desktop     # Rust and FLTK desktop indexer and playback host
apps/cadence-mobile      # Android companion app (React Native, TypeScript, Expo)
```

## Desktop app

### Requirements

- Rust stable, CMake 3.28+, and a C++ compiler
- Windows with Visual Studio Build Tools, or Linux with X11 or XWayland and ALSA development libraries

See [REQUIREMENTS.md](REQUIREMENTS.md) for the platform-specific setup.

### Quick setup

**Windows (PowerShell)**
```powershell
.\setup.ps1
```

**Linux (Debian or Ubuntu)**
```bash
./setup.sh
```

### Run

```bash
cargo run --locked -p cadence-desktop
```

### Usage

1. Open **Music Library Indexer** and select **Add Folder**. Cadence scans the selected folder and its subfolders in the background.
2. Use the trash button to remove a folder from the index. The music files stay on disk.
3. Leave the desktop app open and connect from the Android app to search the library and control playback.

Cadence reads the existing library database at `%APPDATA%\com.cadence.desktop\cadence.db` on Windows and `${XDG_DATA_HOME:-$HOME/.local/share}/com.cadence.desktop/cadence.db` on Linux.

## Android app

### Requirements

- Phone and PC on the **same Wi-Fi network**
- Desktop app running
- Node.js 22.12+ and npm for Android development
- A development build installed on your device (see below) — Expo Go is **not** supported due to native modules (`react-native-zeroconf`)

### Building the Android app

The mobile app uses native modules and requires a custom development build via EAS:

```bash
cd apps/cadence-mobile
npm ci --legacy-peer-deps
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

Run `cargo test --workspace --locked` from the repository root. For the Android TypeScript check, run `npx --no-install tsc --noEmit` from `apps/cadence-mobile` after installing its npm dependencies.

## GitHub releases

CI runs the Rust workspace tests and the Android TypeScript check on pull requests to `main` and pushes to `main`. A new `vMAJOR.MINOR.PATCH` tag runs those checks again and publishes one GitHub Release with Windows x64 `.exe` and `.msi` installers, Linux x64 `.deb`, `.rpm`, and AppImage packages, and a signed Android ARM64 APK.

### One-time setup

1. Use Cadence's existing Android release keystore if you have one. Android updates require the same signing key. For a first release, generate one outside the repository with JDK 17. Use the alias `cadence` and the same password for the keystore and the key. Keep a backup of the keystore and its password.

   ```bash
   keytool -genkeypair -v -storetype PKCS12 -keystore "$HOME/cadence-release.jks" \
     -alias cadence -keyalg RSA -keysize 2048 -validity 10000
   ```

2. In **Settings → Secrets and variables → Actions**, set `ANDROID_KEYSTORE_BASE64` to the base64-encoded keystore and `ANDROID_KEYSTORE_PASSWORD` to its password. With an authenticated GitHub CLI, run these commands from the repository and enter the password when prompted. Adjust the keystore path if you are reusing one.

   ```bash
   base64 < "$HOME/cadence-release.jks" | tr -d '\n' | gh secret set ANDROID_KEYSTORE_BASE64
   gh secret set ANDROID_KEYSTORE_PASSWORD
   ```

### Publish a version

1. Merge the changes to `main` and wait for **Cadence checks** to pass.
2. Tag the latest `main` commit with an unused version. The tag sets the installer and APK versions. Use minor and patch numbers from 0 to 99 and an Android version newer than the last distributed APK.

   ```bash
   git switch main &&
   git pull --ff-only origin main &&
   read -r -p 'Release version (MAJOR.MINOR.PATCH): ' version &&
   git tag -a "v$version" -m "Cadence v$version" && git push origin "v$version"
   ```

3. Watch the **Release** workflow in the **Actions** tab. Once it passes, the installers, APK, and generated release notes appear under **Releases**.

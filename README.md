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

## Download

Get the latest version from [Releases](https://github.com/4p5tatami/cadence/releases/latest):

| Platform | File | How to install |
|----------|------|----------------|
| Windows | `cadence-desktop_X.Y.Z_x64-setup.exe` (or `.msi`) | Run it. SmartScreen may warn about an unrecognised app: *More info → Run anyway*. |
| Linux (Debian/Ubuntu) | `cadence-desktop_X.Y.Z_amd64.deb` | `sudo apt install ./cadence-desktop_X.Y.Z_amd64.deb` |
| Linux (Fedora/openSUSE) | `cadence-desktop-X.Y.Z-1.x86_64.rpm` | `sudo dnf install ./cadence-desktop-X.Y.Z-1.x86_64.rpm` |
| Linux (any distro) | `cadence-desktop_X.Y.Z_amd64.AppImage` | `chmod +x` it, then run it. |
| Android | `cadence-vX.Y.Z.apk` | Open it on the phone and allow installing from unknown sources. For automatic updates, add this repo's URL to [Obtainium](https://github.com/ImranR98/Obtainium). |

The desktop app and the phone must be on the same Wi-Fi network.

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

**Windows (regular PowerShell, admin not needed):**
```powershell
powershell -ExecutionPolicy Bypass -File .\setup.ps1
```

**Linux (Debian/Ubuntu):**
```bash
./setup.sh
```

macOS and non-apt Linux distros aren't covered by the scripts; see [REQUIREMENTS.md](REQUIREMENTS.md).

### Run

```bash
cd apps/cadence-desktop
npm run tauri dev
```

The app opens in its own window. Don't open the `http://localhost:1420` URL from the terminal in a browser: that's the dev server feeding the window, and it can't reach the Rust backend.

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
- The app installed from [Download](#download), or for development a development build (see below) — Expo Go is **not** supported due to native modules (`react-native-zeroconf`)

### Building the Android app

The app uses native modules, so it's built locally as a development build (an `.apk` containing the native code; your JS is served live from the dev server). This needs JDK 17 and the Android SDK, which the setup scripts install when run with `--android` / `-Android`:

```bash
./setup.sh --android                                                  # Linux
powershell -ExecutionPolicy Bypass -File .\setup.ps1 -Android         # Windows
```

Then open a new terminal (or run `source ~/.bashrc`) so `JAVA_HOME` and `ANDROID_HOME` are set. Terminals that were already open won't have them, and Gradle fails with "SDK location not found". Manual install steps are in [REQUIREMENTS.md](REQUIREMENTS.md#android-app-build).

**Option A: build and install over USB** (simplest). Enable *Developer options → USB debugging* on the phone, connect it, accept the prompt on the phone, then:

```bash
cd apps/cadence-mobile
npm run android        # = npx expo run:android
```

This generates the native `android/` project, builds a debug `.apk` for your phone's CPU, installs it, and starts the dev server. The first build takes several minutes; later builds are incremental.

**Option B: build an `.apk` file** to copy to the phone yourself:

```bash
cd apps/cadence-mobile
npx expo prebuild --platform android
cd android
./gradlew assembleDebug -PreactNativeArchitectures=arm64-v8a      # Windows: .\gradlew.bat ...
```

The APK is written to `android/app/build/outputs/apk/debug/app-debug.apk`. Drop `-PreactNativeArchitectures=...` to build for every CPU type (slower, larger file). After installing it, start the dev server from `apps/cadence-mobile`:

```bash
npx expo start --clear
```

Open the Cadence app on your device; it lists dev servers on your network, or scan the QR code.

You only need to rebuild the `.apk` when native code changes (adding or upgrading a package with native code, or editing `app.json`). JS/TS changes just need the dev server.

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
# Windows (regular PowerShell)
powershell -ExecutionPolicy Bypass -File .\setup.ps1
```

```bash
# Linux (Debian/Ubuntu)
./setup.sh
```

### Manual setup

See [REQUIREMENTS.md](REQUIREMENTS.md) for detailed platform-specific instructions.

## Releases

Pushing a version tag runs the [Release workflow](.github/workflows/release.yml) on GitHub Actions. It builds the Windows installers (`.exe`, `.msi`), the Linux packages (`.deb`, `.rpm`, `.AppImage`) and a signed Android APK, and publishes them together as a GitHub Release. The release stays a draft until every build has succeeded.

### One-time setup

The only setup is the Android signing key. Android only installs an update if it's signed with the same key as the installed app, so **back up the key file and its password outside this PC**. If they're lost, everyone has to uninstall and reinstall the app.

1. Create the key (skip if you already have `~/keys/cadence-upload.jks`). Everything in `-dname` ends up in the certificate and is readable from the APK by anyone:
   ```bash
   mkdir -p ~/keys
   keytool -genkeypair -storetype PKCS12 -keystore ~/keys/cadence-upload.jks \
     -alias cadence -keyalg RSA -keysize 4096 -validity 36500 -dname "CN=Cadence"
   ```
2. Encode it for GitHub:
   ```bash
   base64 -w0 ~/keys/cadence-upload.jks > ~/keys/cadence-upload.jks.b64
   ```
3. On GitHub, open the repo's *Settings → Secrets and variables → Actions → New repository secret* and add:

   | Name | Value |
   |------|-------|
   | `ANDROID_KEYSTORE_BASE64` | the contents of `~/keys/cadence-upload.jks.b64` |
   | `ANDROID_KEYSTORE_PASSWORD` | the keystore password |

4. Delete `~/keys/cadence-upload.jks.b64` (keep the `.jks`).

Desktop builds need no secrets. The installers aren't code-signed, so Windows shows a SmartScreen warning on first run.

### Making a release

```bash
git checkout main && git pull
git tag v0.2.0
git push origin v0.2.0
```

The tag sets the version of every build, so there's nothing to bump in the source. It must look like `vMAJOR.MINOR.PATCH`, with minor and patch at most 99. The builds take about 15–25 minutes; follow them in the repo's **Actions** tab. When all jobs pass, the release appears under **Releases** with every file attached.

If a job fails, fix the cause and use **Re-run failed jobs** on the workflow run. Builds upload into the existing draft, so nothing is duplicated.

### Building release files locally

- **Desktop:** `cd apps/cadence-desktop && npx tauri build`. Installers are written to `target/release/bundle/` at the repo root. Each OS builds only its own installers.
- **Android:** in `apps/cadence-mobile/android` (after `npx expo prebuild --platform android`), run `./gradlew assembleRelease` with the Gradle properties `CADENCE_UPLOAD_STORE_FILE`, `CADENCE_UPLOAD_STORE_PASSWORD`, `CADENCE_UPLOAD_KEY_ALIAS=cadence` and `CADENCE_UPLOAD_KEY_PASSWORD` set (for example in `~/.gradle/gradle.properties`). Without them the APK is signed with the public debug key: fine for testing, never for sharing.

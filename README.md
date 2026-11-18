# Cadence

Cross-platform music player with Android remote control. Index your local music library on a Windows or Linux computer, then search and control playback from your phone. Audio plays through the computer’s speakers or headphones.

Cadence indexes common music formats including FLAC, WAV, MP3, AAC, M4A, and Ogg Vorbis.

## Download and install

Open the [latest GitHub release](https://github.com/4p5tatami/cadence/releases/latest) and expand **Assets**. Download the desktop package for your computer and the Android APK from the same release.

| Platform | Release asset | Installation |
| --- | --- | --- |
| Windows x64 | `.exe` or `.msi` | Run either installer, then open the installed app from the Start menu. |
| Linux x64, Debian or Ubuntu | `.deb` | Open the package with your system’s package installer. |
| Linux x64, Fedora | `.rpm` | Open the package with your system’s package installer. |
| Linux x64, AppImage | `.AppImage` | Mark the file as executable, then open it. |
| Android ARM64 | `cadence-v<version>-android-arm64.apk` | Open the APK on your phone and tap **Install**. |

If Android prompts you to allow installation, grant **Allow from this source** to the browser or file manager that opened the APK. [Android installation help](https://support.google.com/pixelphone/answer/7391672?hl=en)

## Index and play music

1. Open **Cadence** on your computer and select **Add Folder**. Cadence scans the selected folder and its subfolders. Select **Cancel** in the popup to discard the current scan. The trash button removes a folder from the index and keeps the music files on disk.
2. Connect the phone and computer to the **same local network** and leave the desktop app open.
3. Open Cadence on Android and tap your computer under **Available devices**.
4. Search the indexed library and tap a track to play it on your computer. Use the now-playing bar to pause, resume, stop, skip, or seek.

If your computer doesn’t appear, check that the indexer is open and both devices are on the same network. If the desktop firewall asks about Cadence, allow it on your private network.

## Development

See [REQUIREMENTS.md](REQUIREMENTS.md) for desktop build dependencies and setup. Run these commands from the repository root.

```bash
cargo run --locked -p cadence-desktop
cargo test --workspace --locked
```

### Android app

The Android app needs a custom development build because it uses native network discovery, so Expo Go won't work. Build it locally with JDK 17 and the Android SDK (see [REQUIREMENTS.md](REQUIREMENTS.md#android-development)).

1. Connect your phone over USB with USB debugging enabled, and check that `adb devices` lists it.
2. Uninstall any release build of Cadence first. Release and development builds share the package name `com.cadence.app` but are signed with different keys, so one cannot install over the other.

   ```bash
   adb uninstall com.cadence.app
   ```

3. Install dependencies, then build and install the development build. This also starts Metro.

   ```bash
   cd apps/cadence-mobile
   npm ci --legacy-peer-deps
   npx expo run:android
   ```

The first build takes several minutes. After that, while the development build is installed, run `npx expo start --clear` and open the app on your phone. If the build fails because the generated `android/` folder is stale, regenerate it with `npx expo prebuild --clean --platform android`.

Cloud builds were deliberately ditched in order to avoid expensive third-party build dependencies. 
# System Requirements

## Linux (Ubuntu/Pop!_OS/Debian)

### Required Packages

Install all required system dependencies with this single command:

```bash
sudo apt install -y \
  libwebkit2gtk-4.1-dev \
  build-essential \
  libasound2-dev \
  pkg-config \
  curl \
  file
```

### Node.js

**Required version:** 20.19+ or 22.12+

Install Node.js 22 (recommended):

```bash
# Add Node.js 22 repository
curl -fsSL https://deb.nodesource.com/setup_22.x | sudo -E bash -

# Install Node.js
sudo apt-get install -y nodejs

# Verify installation
node --version  # Should be v22.x or higher
npm --version
```

### Rust

**Required version:** Latest stable

Install Rust via rustup:

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Load Rust into current shell
source $HOME/.cargo/env

# Verify installation
rustc --version
cargo --version
```

---

## Windows

### Required Software

1. **Node.js** (20.19+ or 22.12+)
   - Download from: https://nodejs.org/
   - The installer includes npm

2. **Rust**
   - Download from: https://rustup.rs/
   - Follow the installer instructions

3. **Visual Studio Build Tools**
   - Download: https://visualstudio.microsoft.com/downloads/
   - Install "Desktop development with C++" workload
   - Or use the Visual Studio Installer to add C++ build tools

4. **WebView2** (usually pre-installed on Windows 10/11)
   - If missing, download from: https://developer.microsoft.com/en-us/microsoft-edge/webview2/

---

## Android app build

Building the Android app locally needs **JDK 17** and the **Android SDK**. `./setup.sh --android` (Linux) and `setup.ps1 -Android` (Windows) install both; this section is the manual equivalent. Android Studio is not required.

Required SDK packages (these match Expo SDK 54 / React Native 0.81 and change when Expo is upgraded):

| Package | Purpose |
|---------|---------|
| `platform-tools` | `adb`, to install the app on a USB-connected phone |
| `platforms;android-36` | Android API the app compiles against |
| `build-tools;36.0.0` | Packages and signs the `.apk` |
| `ndk;27.1.12297006` | Compiles the native (C++) parts of React Native modules |
| `cmake;3.22.1` | Build system for those native parts |

Disk space: about 3 GB for the SDK, plus a few GB of Gradle caches after the first build.

### Linux

```bash
sudo apt install -y openjdk-17-jdk-headless unzip android-sdk-platform-tools-common

export JAVA_HOME=/usr/lib/jvm/java-17-openjdk-amd64
export ANDROID_HOME=$HOME/Android/Sdk

# Command-line tools: https://developer.android.com/studio#command-line-tools-only
mkdir -p $ANDROID_HOME/cmdline-tools
curl -fLO https://dl.google.com/android/repository/commandlinetools-linux-16111833_latest.zip
unzip -q commandlinetools-linux-16111833_latest.zip -d $ANDROID_HOME/cmdline-tools
mv $ANDROID_HOME/cmdline-tools/cmdline-tools $ANDROID_HOME/cmdline-tools/latest

yes | $ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager --licenses
$ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager \
  "platform-tools" "platforms;android-36" "build-tools;36.0.0" "ndk;27.1.12297006" "cmake;3.22.1"
```

Add `JAVA_HOME`, `ANDROID_HOME` and `$ANDROID_HOME/platform-tools` (on `PATH`) to your shell config. `android-sdk-platform-tools-common` installs the udev rules that let `adb` see USB-connected phones (your user must be in the `plugdev` group).

### Windows

1. Install JDK 17: `winget install Microsoft.OpenJDK.17`
2. Download the command-line tools from https://developer.android.com/studio#command-line-tools-only and extract them so that `%LOCALAPPDATA%\Android\Sdk\cmdline-tools\latest\bin\sdkmanager.bat` exists.
3. Set user environment variables `JAVA_HOME` (the JDK folder) and `ANDROID_HOME` (`%LOCALAPPDATA%\Android\Sdk`), and add `%ANDROID_HOME%\platform-tools` to `Path`.
4. In a new terminal, accept the licenses and install the packages listed above:
   ```powershell
   & "$env:ANDROID_HOME\cmdline-tools\latest\bin\sdkmanager.bat" --licenses
   & "$env:ANDROID_HOME\cmdline-tools\latest\bin\sdkmanager.bat" "platform-tools" "platforms;android-36" "build-tools;36.0.0" "ndk;27.1.12297006" "cmake;3.22.1"
   ```
5. For USB installs, your phone may need its manufacturer's USB driver (Pixel: the [Google USB Driver](https://developer.android.com/studio/run/win-usb)).

---

## Package Breakdown (Linux)

| Package | Purpose |
|---------|---------|
| `libwebkit2gtk-4.1-dev` | WebView rendering engine for Tauri UI |
| `build-essential` | GCC, g++, make - C/C++ toolchain for native crates (SQLite is compiled from source) |
| `libasound2-dev` | ALSA audio library (required for audio playback) |
| `pkg-config` | Locates the system libraries above at build time |
| `curl` | Installs Node.js and Rust |
| `file` | Used by the Tauri bundler when packaging |

Tauri's generic prerequisites also list `libssl-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev` and `wget`. Cadence doesn't need them: nothing in the dependency tree uses OpenSSL, and the last two are only for the system tray icon, which isn't enabled. Install them if you turn on Tauri's `tray-icon` feature.

---

## Running the App

After installing all requirements:

```bash
cd apps/cadence-desktop

# Install JavaScript dependencies
npm install

# Run the development server
npm run tauri dev
```

---

## Troubleshooting

### Linux: "Cannot find native binding" error

Reinstall dependencies from the lockfile (don't delete `package-lock.json`; that unpins every dependency):
```bash
rm -rf node_modules
npm ci
```

### Linux: "alsa-sys" build error

Install ALSA development library:
```bash
sudo apt install -y libasound2-dev
```

### Node.js version too old

The project requires Node.js 20.19+ or 22.12+. Upgrade using the instructions above.

### "Cannot read properties of undefined (reading 'invoke')"

The UI was opened in a regular browser. `npm run tauri dev` opens the app in its own window; the `http://localhost:1420` URL it prints is the dev server feeding that window and has no access to the Rust backend.

### Linux: blank or missing app window (NVIDIA, Wayland)

WebKitGTK's DMA-BUF renderer misbehaves on some NVIDIA/Wayland setups. Disable it:
```bash
WEBKIT_DISABLE_DMABUF_RENDERER=1 npm run tauri dev
```
If the window is still blank, also prefix `GDK_BACKEND=x11`.

### Windows: "running scripts is disabled on this system"

PowerShell's default execution policy blocks `.ps1` files. Run the setup script with:
```powershell
powershell -ExecutionPolicy Bypass -File .\setup.ps1
```

### Android: "SDK location not found" or wrong Java version

Gradle reads `ANDROID_HOME` and `JAVA_HOME` from the environment. Open a new terminal after running the setup script, and check `echo $JAVA_HOME $ANDROID_HOME` (PowerShell: `$env:JAVA_HOME; $env:ANDROID_HOME`). `JAVA_HOME` must point at JDK 17.

### Android: `adb devices` shows "unauthorized" or nothing

Unlock the phone and accept the "Allow USB debugging?" prompt. On Linux, make sure `android-sdk-platform-tools-common` is installed and you're in the `plugdev` group, then replug the cable. Set the USB mode to "File transfer" if the phone still isn't listed.

### Windows: Android build fails with path or CMake errors

The NDK build can hit Windows' 260-character path limit. Clone the repo to a short path such as `C:\dev\cadence`.

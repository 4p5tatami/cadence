# System requirements

## Desktop on Linux

Install Rust stable, a C compiler, pkg-config, and ALSA development headers. The `index` command uses Zenity for the folder picker. On Debian or Ubuntu, `./setup.sh` installs the required packages and Rust if needed.

```bash
sudo apt-get install -y build-essential pkg-config libasound2-dev zenity
```

## Desktop on Windows

Install [Rust stable](https://rustup.rs/) and [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with **Desktop development with C++** and the Windows SDK. Open a new terminal after installation and run `./setup.ps1` to check your tools.

## Run the desktop app

From the repository root, run:

```bash
cargo run --locked -p cadence-desktop
```

## Android development

Install [Node.js](https://nodejs.org/) 22.12 or newer and npm. The Android app uses a custom development build because it includes native discovery. See the [Android setup in the README](README.md#android-app) for build steps. Local builds require JDK 17 and the Android SDK, with `ANDROID_HOME` set and `platform-tools` (for `adb`) on your `PATH`.

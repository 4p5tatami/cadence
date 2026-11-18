# System requirements

## Desktop on Linux

Install Rust stable and a C++ compiler, CMake 3.28 or newer, ALSA development headers, and the FLTK build dependencies. On Debian or Ubuntu, `./setup.sh` installs these packages and Rust if needed.

```bash
sudo apt-get install -y build-essential cmake pkg-config libasound2-dev libpango1.0-dev \
  libx11-dev libxcursor-dev libxext-dev libxfixes-dev libxft-dev libxinerama-dev libxrender-dev
```

If your distribution ships an older CMake, install a newer version and make sure `cmake --version` finds it on `PATH`. FLTK uses X11, including XWayland on Wayland desktops.

## Desktop on Windows

Install [Rust stable](https://rustup.rs/), [CMake](https://cmake.org/download/) 3.28 or newer, and [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with **Desktop development with C++** and the Windows SDK. Open a new terminal after installation and run `./setup.ps1` to check your tools.

## Run the desktop app

From the repository root, run:

```bash
cargo run --locked -p cadence-desktop
```

## Android development

Install [Node.js](https://nodejs.org/) 22.12 or newer and npm. The Android app uses a custom development build because it includes native discovery. See the [Android setup in the README](README.md#android-app) for build steps. Local builds require JDK 17 and the Android SDK, with `ANDROID_HOME` set and `platform-tools` (for `adb`) on your `PATH`.

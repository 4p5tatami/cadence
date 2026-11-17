#!/bin/bash
# Cadence Development Environment Setup Script for Linux (Debian/Ubuntu)
#
# Installs everything needed to build and run the desktop app, plus the mobile
# app's npm dependencies.
#
#   ./setup.sh             desktop + mobile npm dependencies
#   ./setup.sh --android   also JDK 17 + Android SDK, to build the Android app locally

set -euo pipefail

# Always run from the repo root, wherever the script was invoked from.
cd "$(dirname "$(readlink -f "$0")")"
ROOT_DIR=$(pwd)

WITH_ANDROID=false
for arg in "$@"; do
    case "$arg" in
        --android) WITH_ANDROID=true ;;
        -h|--help)
            sed -n '2,8p' "$0" | sed 's/^# \{0,1\}//'
            exit 0 ;;
        *)
            echo "Unknown option: $arg (see --help)"
            exit 1 ;;
    esac
done

TOTAL_STEPS=5
if $WITH_ANDROID; then TOTAL_STEPS=6; fi

# Android build requirements. These track Expo SDK 54 / React Native 0.81
# (see node_modules/react-native/gradle/libs.versions.toml); update them together
# when upgrading Expo.
ANDROID_HOME="${ANDROID_HOME:-$HOME/Android/Sdk}"
CMDLINE_TOOLS_ZIP="commandlinetools-linux-16111833_latest.zip"   # cmdline-tools 23.0
CMDLINE_TOOLS_SHA1="e025545c62a8e64c7559119566a569fb1dec5f60"
ANDROID_PACKAGES=(
    "platform-tools"
    "platforms;android-36"
    "build-tools;36.0.0"
    "ndk;27.1.12297006"
    "cmake;3.22.1"
)

echo "=================================="
echo "  Cadence Setup Script (Linux)    "
echo "=================================="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

if ! command -v apt-get &> /dev/null; then
    echo -e "${RED}Error: This script is for Debian/Ubuntu-based systems (uses apt).${NC}"
    echo "On other distros, install the equivalent packages listed in REQUIREMENTS.md."
    exit 1
fi

command_exists() {
    command -v "$1" &> /dev/null
}

# Vite 7 requires Node ^20.19.0 || >=22.12.0 (Node 21 is not supported).
node_version_ok() {
    local major minor
    IFS=. read -r major minor _ <<< "${1#v}"
    if [ "$major" -eq 20 ] && [ "$minor" -ge 19 ]; then return 0; fi
    if [ "$major" -eq 22 ] && [ "$minor" -ge 12 ]; then return 0; fi
    [ "$major" -ge 23 ]
}

install_node_22() {
    curl -fsSL https://deb.nodesource.com/setup_22.x | sudo -E bash -
    sudo apt-get install -y nodejs
    hash -r
}

# Step 1: System dependencies
echo -e "${YELLOW}[1/$TOTAL_STEPS] Installing system dependencies...${NC}"

# libwebkit2gtk-4.1-dev  WebView used by Tauri
# build-essential        C/C++ toolchain for native crates (SQLite is compiled from source)
# libasound2-dev         ALSA headers for audio output
# pkg-config             Locates the system libraries above at build time
# curl                   Used below to install Node.js, Rust and the Android SDK
# file                   Used by the Tauri bundler when packaging
#
# Not needed by the current config (add them if you enable these Tauri features):
#   libayatana-appindicator3-dev, librsvg2-dev  -> system tray icon ("tray-icon")
APT_PACKAGES=(libwebkit2gtk-4.1-dev build-essential libasound2-dev pkg-config curl file)

# openjdk-17-jdk-headless           JDK for Gradle (React Native 0.81 targets JDK 17)
# unzip                             Extracts the Android command-line tools
# android-sdk-platform-tools-common udev rules so adb can see USB-connected phones
if $WITH_ANDROID; then
    APT_PACKAGES+=(openjdk-17-jdk-headless unzip android-sdk-platform-tools-common)
fi

echo "This will install: ${APT_PACKAGES[*]}"
echo ""
sudo apt-get update
sudo apt-get install -y "${APT_PACKAGES[@]}"

echo -e "${GREEN}OK: System dependencies installed${NC}"
echo ""

# Step 2: Node.js
echo -e "${YELLOW}[2/$TOTAL_STEPS] Checking Node.js...${NC}"

if command_exists node; then
    if node_version_ok "$(node --version)"; then
        echo -e "${GREEN}OK: Node.js $(node --version) is installed${NC}"
    else
        echo -e "${RED}! Node.js $(node --version) is not supported (need 20.19+ or 22.12+)${NC}"
        if [ "$(command -v node)" != "/usr/bin/node" ]; then
            echo -e "${YELLOW}Your node is at $(command -v node), probably from nvm/fnm/volta.${NC}"
            echo "Install Node 22 with that tool (e.g. 'nvm install 22'), then re-run this script."
            exit 1
        fi
        echo "Upgrading to Node.js 22..."
        install_node_22
        echo -e "${GREEN}OK: Node.js upgraded to $(node --version)${NC}"
    fi
else
    echo "Node.js not found. Installing Node.js 22..."
    install_node_22
    echo -e "${GREEN}OK: Node.js $(node --version) installed${NC}"
fi

echo -e "${GREEN}OK: npm $(npm --version) is available${NC}"
echo ""

# Step 3: Rust
echo -e "${YELLOW}[3/$TOTAL_STEPS] Checking Rust...${NC}"

if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    source "$HOME/.cargo/env"
fi

if command_exists rustup; then
    echo "Updating the stable toolchain via rustup..."
    rustup update stable --no-self-update
    echo -e "${GREEN}OK: $(rustc --version)${NC}"
elif command_exists rustc; then
    echo -e "${YELLOW}! Found $(rustc --version) without rustup (probably the distro package).${NC}"
    echo "  Distro Rust is often too old for Tauri's dependencies. If the build fails, remove it"
    echo "  ('sudo apt remove rustc cargo') and re-run this script to install Rust via rustup."
else
    echo "Rust not found. Installing Rust via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    # shellcheck disable=SC1091
    source "$HOME/.cargo/env"
    echo -e "${GREEN}OK: $(rustc --version) installed${NC}"
    echo -e "${YELLOW}Note: open a new terminal (or run 'source \$HOME/.cargo/env') to use Rust.${NC}"
fi
echo ""

# Step 4: Desktop npm dependencies
echo -e "${YELLOW}[4/$TOTAL_STEPS] Installing desktop npm dependencies...${NC}"
cd "$ROOT_DIR/apps/cadence-desktop"
npm ci
echo -e "${GREEN}OK: Desktop dependencies installed${NC}"
echo ""

# Step 5: Mobile npm dependencies
echo -e "${YELLOW}[5/$TOTAL_STEPS] Installing mobile app dependencies...${NC}"
cd "$ROOT_DIR/apps/cadence-mobile"
npm ci --legacy-peer-deps
echo -e "${GREEN}OK: Mobile dependencies installed${NC}"
echo ""
cd "$ROOT_DIR"

# Step 6 (optional): Android SDK
if $WITH_ANDROID; then
    echo -e "${YELLOW}[6/$TOTAL_STEPS] Installing Android SDK into $ANDROID_HOME...${NC}"

    JAVA_HOME=$(find /usr/lib/jvm -maxdepth 1 -type d -name 'java-17-openjdk-*' | head -n 1)
    if [ -z "$JAVA_HOME" ]; then
        echo -e "${RED}Error: JDK 17 not found under /usr/lib/jvm after installing openjdk-17-jdk-headless.${NC}"
        exit 1
    fi
    export JAVA_HOME ANDROID_HOME
    echo "Using JDK at $JAVA_HOME"

    SDKMANAGER="$ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager"
    if [ ! -x "$SDKMANAGER" ]; then
        echo "Downloading Android command-line tools..."
        TMP_DIR=$(mktemp -d)
        trap 'rm -rf "$TMP_DIR"' EXIT
        curl -fL --progress-bar -o "$TMP_DIR/cmdline-tools.zip" \
            "https://dl.google.com/android/repository/$CMDLINE_TOOLS_ZIP"
        echo "$CMDLINE_TOOLS_SHA1  $TMP_DIR/cmdline-tools.zip" | sha1sum -c --quiet -
        unzip -q "$TMP_DIR/cmdline-tools.zip" -d "$TMP_DIR"
        mkdir -p "$ANDROID_HOME/cmdline-tools"
        rm -rf "$ANDROID_HOME/cmdline-tools/latest"
        mv "$TMP_DIR/cmdline-tools" "$ANDROID_HOME/cmdline-tools/latest"
    fi

    echo "Accepting Android SDK licenses..."
    (yes || true) | "$SDKMANAGER" --licenses > /dev/null

    echo "Installing SDK packages (the NDK alone is ~650 MB; this can take a while)..."
    "$SDKMANAGER" --install "${ANDROID_PACKAGES[@]}"

    # Persist JAVA_HOME / ANDROID_HOME for new shells (Gradle and the Expo CLI read them).
    BASHRC="$HOME/.bashrc"
    MARKER_START="# >>> cadence android >>>"
    MARKER_END="# <<< cadence android <<<"
    if [ -f "$BASHRC" ]; then
        sed -i "/^$MARKER_START\$/,/^$MARKER_END\$/d" "$BASHRC"
    fi
    {
        echo "$MARKER_START"
        echo "export JAVA_HOME=\"$JAVA_HOME\""
        echo "export ANDROID_HOME=\"$ANDROID_HOME\""
        echo "export PATH=\"\$ANDROID_HOME/platform-tools:\$ANDROID_HOME/cmdline-tools/latest/bin:\$PATH\""
        echo "$MARKER_END"
    } >> "$BASHRC"

    echo -e "${GREEN}OK: Android SDK installed; JAVA_HOME and ANDROID_HOME added to ~/.bashrc${NC}"
    echo -e "${YELLOW}Note: open a new terminal to pick them up (zsh/fish users: copy that block to your shell's config).${NC}"
    echo ""
fi

echo "=================================="
echo -e "${GREEN}Setup complete!${NC}"
echo "=================================="
echo ""
echo "To run the desktop app:"
echo "  cd apps/cadence-desktop"
echo "  npm run tauri dev"
echo ""
echo "  The app opens in its own window. The http://localhost:1420 URL printed in the"
echo "  terminal is the dev server for that window; opening it in a browser won't work."
echo ""
if $WITH_ANDROID; then
    echo -e "${YELLOW}Before building the Android app, load JAVA_HOME/ANDROID_HOME into your shell:${NC}"
    echo "  source ~/.bashrc        (or open a new terminal; already-open terminals don't have them)"
    echo ""
    echo "To build and run the Android app (phone connected over USB, USB debugging on):"
    echo "  cd apps/cadence-mobile"
    echo "  npm run android"
    echo ""
    echo "  Or build an .apk to install by hand: see 'Building the Android app' in README.md."
else
    echo "To build the Android app locally, re-run with: ./setup.sh --android"
fi
echo ""

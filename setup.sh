#!/usr/bin/env bash
set -euo pipefail

if ! command -v apt-get >/dev/null; then
    echo "This script uses apt. See REQUIREMENTS.md for manual setup." >&2
    exit 1
fi

sudo apt-get update
sudo apt-get install -y build-essential cmake curl pkg-config libasound2-dev libpango1.0-dev \
    libx11-dev libxcursor-dev libxext-dev libxfixes-dev libxft-dev libxinerama-dev libxrender-dev

cmake_version=$(cmake --version | awk 'NR == 1 {print $3}')
if ! dpkg --compare-versions "$cmake_version" ge 3.28; then
    echo "CMake 3.28 or newer is required. See REQUIREMENTS.md." >&2
    exit 1
fi

if ! command -v cargo >/dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    echo 'Run source "$HOME/.cargo/env" to make Cargo available in this shell.'
fi

echo "Run the desktop app with cargo run --locked -p cadence-desktop."

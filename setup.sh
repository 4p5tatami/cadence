#!/usr/bin/env bash
set -euo pipefail

if ! command -v apt-get >/dev/null; then
    echo "This script uses apt. See REQUIREMENTS.md for manual setup." >&2
    exit 1
fi

sudo apt-get update
sudo apt-get install -y build-essential curl pkg-config libasound2-dev zenity

if ! command -v cargo >/dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    echo 'Run source "$HOME/.cargo/env" to make Cargo available in this shell.'
fi

echo "Run the desktop app with cargo run --locked -p cadence-desktop."

# System Requirements

## Linux (Ubuntu/Pop!_OS/Debian)

### Required Packages

Install all required system dependencies with this single command:

```bash
sudo apt install -y \
  libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libasound2-dev \
  pkg-config
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

## Package Breakdown (Linux)

| Package | Purpose |
|---------|---------|
| `libwebkit2gtk-4.1-dev` | WebView rendering engine for Tauri UI |
| `build-essential` | GCC, g++, make - essential build tools |
| `libssl-dev` | SSL/TLS library for secure connections |
| `libayatana-appindicator3-dev` | System tray support |
| `librsvg2-dev` | SVG rendering support |
| `libasound2-dev` | ALSA audio library (required for audio playback) |
| `pkg-config` | Helper tool for build configuration |
| `curl`, `wget`, `file` | Utility tools for downloads and file operations |

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

Remove dependencies and reinstall:
```bash
rm -rf node_modules package-lock.json
npm install
```

### Linux: "alsa-sys" build error

Install ALSA development library:
```bash
sudo apt install -y libasound2-dev
```

### Node.js version too old

The project requires Node.js 20.19+ or 22.12+. Upgrade using the instructions above.

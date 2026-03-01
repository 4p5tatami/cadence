#!/bin/bash
# Cadence Development Environment Setup Script for Linux
# This script installs all required dependencies for building and running Cadence

set -e  # Exit on error

echo "=================================="
echo "  Cadence Setup Script (Linux)    "
echo "=================================="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if running on a Debian-based system
if ! command -v apt &> /dev/null; then
    echo -e "${RED}Error: This script is for Debian/Ubuntu-based systems (uses apt)${NC}"
    exit 1
fi

# Function to check if a command exists
command_exists() {
    command -v "$1" &> /dev/null
}

# Step 1: Install system dependencies
echo -e "${YELLOW}[1/4] Installing system dependencies...${NC}"
echo "This will install: libwebkit2gtk-4.1-dev, build-essential, libasound2-dev, and more"
echo ""

sudo apt update
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

echo -e "${GREEN}✓ System dependencies installed${NC}"
echo ""

# Step 2: Check/Install Node.js
echo -e "${YELLOW}[2/4] Checking Node.js...${NC}"

if command_exists node; then
    NODE_VERSION=$(node --version | sed 's/v//' | cut -d. -f1)
    if [ "$NODE_VERSION" -ge 20 ]; then
        echo -e "${GREEN}✓ Node.js $(node --version) is already installed${NC}"
    else
        echo -e "${RED}! Node.js $(node --version) is too old (need 20.19+ or 22.12+)${NC}"
        echo "Upgrading to Node.js 22..."
        curl -fsSL https://deb.nodesource.com/setup_22.x | sudo -E bash -
        sudo apt-get install -y nodejs
        echo -e "${GREEN}✓ Node.js upgraded to $(node --version)${NC}"
    fi
else
    echo "Node.js not found. Installing Node.js 22..."
    curl -fsSL https://deb.nodesource.com/setup_22.x | sudo -E bash -
    sudo apt-get install -y nodejs
    echo -e "${GREEN}✓ Node.js $(node --version) installed${NC}"
fi

echo -e "${GREEN}✓ npm $(npm --version) is available${NC}"
echo ""

# Step 3: Check/Install Rust
echo -e "${YELLOW}[3/4] Checking Rust...${NC}"

if command_exists rustc; then
    echo -e "${GREEN}✓ Rust $(rustc --version) is already installed${NC}"
else
    echo "Rust not found. Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    echo -e "${GREEN}✓ Rust $(rustc --version) installed${NC}"
    echo -e "${YELLOW}Note: Run 'source \$HOME/.cargo/env' to use Rust in your current shell${NC}"
fi
echo ""

# Step 4: Install npm dependencies
echo -e "${YELLOW}[4/4] Installing npm dependencies...${NC}"

cd apps/cadence-desktop
npm install

echo -e "${GREEN}✓ npm dependencies installed${NC}"
echo ""

# All done!
echo "=================================="
echo -e "${GREEN}✓ Setup complete!${NC}"
echo "=================================="
echo ""
echo "To run the app:"
echo "  cd apps/cadence-desktop"
echo "  npm run tauri dev"
echo ""

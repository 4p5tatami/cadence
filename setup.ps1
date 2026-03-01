# Cadence Development Environment Setup Script for Windows
# Run this in PowerShell as Administrator

Write-Host "==================================" -ForegroundColor Cyan
Write-Host "  Cadence Setup Script (Windows)  " -ForegroundColor Cyan
Write-Host "==================================" -ForegroundColor Cyan
Write-Host ""

# Check if running as Administrator
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Host "Warning: Not running as Administrator. Some installations may fail." -ForegroundColor Yellow
    Write-Host "Consider right-clicking PowerShell and selecting 'Run as Administrator'" -ForegroundColor Yellow
    Write-Host ""
}

# Function to check if a command exists
function Test-Command($command) {
    $null -ne (Get-Command $command -ErrorAction SilentlyContinue)
}

# Step 1: Check Node.js
Write-Host "[1/3] Checking Node.js..." -ForegroundColor Yellow

if (Test-Command node) {
    $nodeVersion = (node --version).TrimStart('v').Split('.')[0]
    if ([int]$nodeVersion -ge 20) {
        Write-Host "✓ Node.js $(node --version) is already installed" -ForegroundColor Green
    } else {
        Write-Host "! Node.js $(node --version) is too old (need 20.19+ or 22.12+)" -ForegroundColor Red
        Write-Host "Please download and install Node.js 22 from: https://nodejs.org/" -ForegroundColor Yellow
        Start-Process "https://nodejs.org/"
        exit 1
    }
} else {
    Write-Host "Node.js not found!" -ForegroundColor Red
    Write-Host "Please download and install Node.js 22 from: https://nodejs.org/" -ForegroundColor Yellow
    Start-Process "https://nodejs.org/"
    exit 1
}

Write-Host "✓ npm $(npm --version) is available" -ForegroundColor Green
Write-Host ""

# Step 2: Check Rust
Write-Host "[2/3] Checking Rust..." -ForegroundColor Yellow

if (Test-Command rustc) {
    Write-Host "✓ Rust $(rustc --version) is already installed" -ForegroundColor Green
} else {
    Write-Host "Rust not found!" -ForegroundColor Red
    Write-Host "Please download and install Rust from: https://rustup.rs/" -ForegroundColor Yellow
    Start-Process "https://rustup.rs/"
    exit 1
}
Write-Host ""

# Step 3: Install npm dependencies
Write-Host "[3/3] Installing npm dependencies..." -ForegroundColor Yellow

Set-Location apps\cadence-desktop
npm install

Write-Host "✓ npm dependencies installed" -ForegroundColor Green
Write-Host ""

# All done!
Write-Host "==================================" -ForegroundColor Green
Write-Host "✓ Setup complete!" -ForegroundColor Green
Write-Host "==================================" -ForegroundColor Green
Write-Host ""
Write-Host "To run the app:"
Write-Host "  cd apps\cadence-desktop"
Write-Host "  npm run tauri dev"
Write-Host ""
Write-Host "Note: Make sure you have:" -ForegroundColor Yellow
Write-Host "  - Visual Studio Build Tools (C++ workload)" -ForegroundColor Yellow
Write-Host "  - WebView2 (usually pre-installed on Windows 10/11)" -ForegroundColor Yellow
Write-Host ""

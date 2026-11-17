# Cadence Development Environment Setup Script for Windows
#
# Run from a normal (non-admin) PowerShell window:
#   powershell -ExecutionPolicy Bypass -File .\setup.ps1             desktop + mobile npm deps
#   powershell -ExecutionPolicy Bypass -File .\setup.ps1 -Android    also JDK 17 + Android SDK
#
# Keep this file ASCII-only: Windows PowerShell 5.1 reads BOM-less files in the
# legacy ANSI code page, where some UTF-8 bytes turn into quote characters and
# break parsing.

param(
    [switch]$Android
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'   # Invoke-WebRequest is very slow with the progress bar
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

# Always run from the repo root, wherever the script was invoked from.
Set-Location $PSScriptRoot
$rootDir = Get-Location

$totalSteps = if ($Android) { 6 } else { 5 }

# Android build requirements. These track Expo SDK 54 / React Native 0.81
# (see node_modules\react-native\gradle\libs.versions.toml); update them together
# when upgrading Expo.
$androidHome = if ($env:ANDROID_HOME) { $env:ANDROID_HOME } else { Join-Path $env:LOCALAPPDATA "Android\Sdk" }
$cmdlineToolsZip = "commandlinetools-win-16111833_latest.zip"   # cmdline-tools 23.0
$cmdlineToolsSha1 = "57d04f2d75eb8e8fffc5000a987e5de4b5a63e9d"
$androidPackages = @(
    "platform-tools",
    "platforms;android-36",
    "build-tools;36.0.0",
    "ndk;27.1.12297006",
    "cmake;3.22.1"
)

Write-Host "==================================" -ForegroundColor Cyan
Write-Host "  Cadence Setup Script (Windows)  " -ForegroundColor Cyan
Write-Host "==================================" -ForegroundColor Cyan
Write-Host ""

function Test-Command($command) {
    $null -ne (Get-Command $command -ErrorAction SilentlyContinue)
}

# Native commands don't throw on failure; stop the script if one fails.
function Invoke-Checked([string]$description, [scriptblock]$command) {
    & $command
    if ($LASTEXITCODE -ne 0) {
        Write-Host "X $description failed (exit code $LASTEXITCODE)" -ForegroundColor Red
        exit $LASTEXITCODE
    }
}

# Vite 7 requires Node ^20.19.0 || >=22.12.0 (Node 21 is not supported).
function Test-NodeVersion([version]$v) {
    if ($v.Major -eq 20 -and $v.Minor -ge 19) { return $true }
    if ($v.Major -eq 22 -and $v.Minor -ge 12) { return $true }
    return $v.Major -ge 23
}

# Returns the first JDK 17 directory found (reads each JDK's 'release' file
# instead of running java.exe, which writes its version to stderr).
function Find-Jdk17 {
    $candidates = @()
    if ($env:JAVA_HOME) { $candidates += $env:JAVA_HOME }
    foreach ($vendorDir in @("Microsoft", "Eclipse Adoptium", "Java", "Zulu")) {
        $base = Join-Path $env:ProgramFiles $vendorDir
        if (Test-Path $base) {
            $candidates += Get-ChildItem $base -Directory | ForEach-Object { $_.FullName }
        }
    }
    foreach ($dir in $candidates) {
        $release = Join-Path $dir "release"
        if ((Test-Path $release) -and (Select-String -Path $release -Pattern 'JAVA_VERSION="17\.' -Quiet)) {
            return $dir
        }
    }
    return $null
}

# Step 1: Node.js
Write-Host "[1/$totalSteps] Checking Node.js..." -ForegroundColor Yellow

if (-not (Test-Command node)) {
    Write-Host "X Node.js not found." -ForegroundColor Red
    Write-Host "  Install Node.js 22 LTS, then open a new terminal and re-run this script:" -ForegroundColor Yellow
    Write-Host "    winget install OpenJS.NodeJS.LTS"
    Write-Host "  or download it from https://nodejs.org/"
    exit 1
}

$nodeVersion = [version]((node --version).TrimStart('v'))
if (-not (Test-NodeVersion $nodeVersion)) {
    Write-Host "X Node.js v$nodeVersion is not supported (need 20.19+ or 22.12+)." -ForegroundColor Red
    Write-Host "  Upgrade to Node.js 22 LTS, then open a new terminal and re-run this script:" -ForegroundColor Yellow
    Write-Host "    winget upgrade OpenJS.NodeJS.LTS"
    Write-Host "  or download it from https://nodejs.org/"
    exit 1
}
Write-Host "OK: Node.js v$nodeVersion, npm $(npm.cmd --version)" -ForegroundColor Green
Write-Host ""

# Step 2: Rust
Write-Host "[2/$totalSteps] Checking Rust..." -ForegroundColor Yellow

if (-not (Test-Command rustc)) {
    Write-Host "X Rust not found." -ForegroundColor Red
    Write-Host "  Install Rust, then open a new terminal and re-run this script:" -ForegroundColor Yellow
    Write-Host "    winget install Rustlang.Rustup"
    Write-Host "  or download it from https://rustup.rs/"
    exit 1
}

if (Test-Command rustup) {
    Write-Host "Updating the stable toolchain via rustup..."
    Invoke-Checked "rustup update" { rustup update stable --no-self-update }
}

$rustHost = (rustc -vV | Select-String '^host:').ToString()
if ($rustHost -notmatch 'msvc') {
    Write-Host "! Rust's default toolchain is '$rustHost'. Tauri on Windows needs the MSVC toolchain:" -ForegroundColor Yellow
    Write-Host "    rustup default stable-msvc"
}
Write-Host "OK: $(rustc --version)" -ForegroundColor Green
Write-Host ""

# Step 3: Visual Studio C++ Build Tools and WebView2
Write-Host "[3/$totalSteps] Checking C++ Build Tools and WebView2..." -ForegroundColor Yellow

$vswhere = Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\Installer\vswhere.exe"
$vcTools = $null
if (Test-Path $vswhere) {
    $vcTools = & $vswhere -latest -products * `
        -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
        -property installationPath
}
if (-not $vcTools) {
    Write-Host "X Visual Studio C++ Build Tools not found (needed to compile Rust on Windows)." -ForegroundColor Red
    Write-Host "  Install them, then re-run this script:" -ForegroundColor Yellow
    Write-Host '    winget install Microsoft.VisualStudio.2022.BuildTools --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"'
    Write-Host "  or install the 'Desktop development with C++' workload from"
    Write-Host "  https://visualstudio.microsoft.com/downloads/"
    exit 1
}
Write-Host "OK: C++ Build Tools at $vcTools" -ForegroundColor Green

$webView2Key = "Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"
$webView2Version = @(
    "HKLM:\SOFTWARE\WOW6432Node\$webView2Key",
    "HKLM:\SOFTWARE\$webView2Key",
    "HKCU:\Software\$webView2Key"
) | ForEach-Object {
    (Get-ItemProperty -Path $_ -Name pv -ErrorAction SilentlyContinue).pv
} | Where-Object { $_ -and $_ -ne "0.0.0.0" } | Select-Object -First 1

if ($webView2Version) {
    Write-Host "OK: WebView2 Runtime $webView2Version" -ForegroundColor Green
} else {
    Write-Host "! WebView2 Runtime not detected. The app window needs it to render." -ForegroundColor Yellow
    Write-Host "  Install it from https://developer.microsoft.com/en-us/microsoft-edge/webview2/"
}
Write-Host ""

# Step 4: Desktop npm dependencies
Write-Host "[4/$totalSteps] Installing desktop npm dependencies..." -ForegroundColor Yellow
Set-Location (Join-Path $rootDir "apps\cadence-desktop")
Invoke-Checked "npm ci (desktop)" { npm.cmd ci }
Write-Host "OK: Desktop dependencies installed" -ForegroundColor Green
Write-Host ""

# Step 5: Mobile npm dependencies
Write-Host "[5/$totalSteps] Installing mobile app dependencies..." -ForegroundColor Yellow
Set-Location (Join-Path $rootDir "apps\cadence-mobile")
Invoke-Checked "npm ci (mobile)" { npm.cmd ci --legacy-peer-deps }
Write-Host "OK: Mobile dependencies installed" -ForegroundColor Green
Write-Host ""
Set-Location $rootDir

# Step 6 (optional): JDK 17 + Android SDK
if ($Android) {
    Write-Host "[6/$totalSteps] Installing JDK 17 and Android SDK into $androidHome..." -ForegroundColor Yellow

    $jdk = Find-Jdk17
    if (-not $jdk) {
        if (-not (Test-Command winget)) {
            Write-Host "X JDK 17 not found and winget is unavailable." -ForegroundColor Red
            Write-Host "  Install Microsoft Build of OpenJDK 17 from https://learn.microsoft.com/java/openjdk/download"
            Write-Host "  then re-run this script."
            exit 1
        }
        Write-Host "Installing Microsoft Build of OpenJDK 17 via winget..."
        Invoke-Checked "winget install OpenJDK 17" {
            winget install --id Microsoft.OpenJDK.17 --exact --accept-source-agreements --accept-package-agreements
        }
        $jdk = Find-Jdk17
        if (-not $jdk) {
            Write-Host "X JDK 17 was installed but not found under $env:ProgramFiles. Set JAVA_HOME and re-run." -ForegroundColor Red
            exit 1
        }
    }
    $env:JAVA_HOME = $jdk
    $env:ANDROID_HOME = $androidHome
    Write-Host "OK: JDK 17 at $jdk" -ForegroundColor Green

    $sdkmanager = Join-Path $androidHome "cmdline-tools\latest\bin\sdkmanager.bat"
    if (-not (Test-Path $sdkmanager)) {
        Write-Host "Downloading Android command-line tools..."
        $tmpDir = Join-Path ([IO.Path]::GetTempPath()) ("cadence-android-" + [guid]::NewGuid())
        New-Item -ItemType Directory -Path $tmpDir | Out-Null
        try {
            $zip = Join-Path $tmpDir "cmdline-tools.zip"
            Invoke-WebRequest -UseBasicParsing -OutFile $zip `
                -Uri "https://dl.google.com/android/repository/$cmdlineToolsZip"
            $actualSha1 = (Get-FileHash -Algorithm SHA1 -Path $zip).Hash
            if ($actualSha1 -ne $cmdlineToolsSha1) {
                Write-Host "X Checksum mismatch for $cmdlineToolsZip (got $actualSha1)." -ForegroundColor Red
                exit 1
            }
            Expand-Archive -Path $zip -DestinationPath $tmpDir
            $latestDir = Join-Path $androidHome "cmdline-tools\latest"
            New-Item -ItemType Directory -Force -Path (Split-Path $latestDir) | Out-Null
            if (Test-Path $latestDir) { Remove-Item -Recurse -Force $latestDir }
            Move-Item (Join-Path $tmpDir "cmdline-tools") $latestDir
        } finally {
            Remove-Item -Recurse -Force $tmpDir -ErrorAction SilentlyContinue
        }
    }

    Write-Host "Accepting Android SDK licenses..."
    Invoke-Checked "sdkmanager --licenses" { @('y') * 50 | & $sdkmanager --licenses | Out-Null }

    # Pass packages via a file: sdkmanager is a .bat, and cmd.exe can split
    # unquoted arguments like "platforms;android-36" at the semicolon.
    Write-Host "Installing SDK packages (the NDK alone is ~750 MB; this can take a while)..."
    $packageFile = Join-Path ([IO.Path]::GetTempPath()) "cadence-android-packages.txt"
    Set-Content -Path $packageFile -Value $androidPackages -Encoding ASCII
    try {
        Invoke-Checked "sdkmanager --install" { & $sdkmanager "--package_file=$packageFile" }
    } finally {
        Remove-Item $packageFile -ErrorAction SilentlyContinue
    }

    # Persist for new terminals (Gradle and the Expo CLI read these).
    [Environment]::SetEnvironmentVariable("JAVA_HOME", $jdk, "User")
    [Environment]::SetEnvironmentVariable("ANDROID_HOME", $androidHome, "User")
    $platformTools = Join-Path $androidHome "platform-tools"
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if (-not $userPath) { $userPath = "" }
    if (($userPath -split ';') -notcontains $platformTools) {
        $newPath = (@($userPath.TrimEnd(';'), $platformTools) | Where-Object { $_ }) -join ';'
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    }

    Write-Host "OK: Android SDK installed; JAVA_HOME, ANDROID_HOME and PATH set for your user" -ForegroundColor Green
    Write-Host "Note: open a new terminal to pick them up." -ForegroundColor Yellow
    Write-Host ""
}

Write-Host "==================================" -ForegroundColor Green
Write-Host "Setup complete!" -ForegroundColor Green
Write-Host "==================================" -ForegroundColor Green
Write-Host ""
Write-Host "To run the desktop app:"
Write-Host "  cd apps\cadence-desktop"
Write-Host "  npm run tauri dev"
Write-Host ""
Write-Host "  The app opens in its own window. The http://localhost:1420 URL printed in the"
Write-Host "  terminal is the dev server for that window; opening it in a browser won't work."
Write-Host ""
if ($Android) {
    Write-Host "To build and run the Android app (phone connected over USB, USB debugging on):"
    Write-Host "  cd apps\cadence-mobile"
    Write-Host "  npm run android"
    Write-Host ""
    Write-Host "  Or build an .apk to install by hand: see 'Building the Android app' in README.md."
} else {
    Write-Host "To build the Android app locally, re-run with: .\setup.ps1 -Android"
}
Write-Host ""

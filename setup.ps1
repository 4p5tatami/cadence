$ErrorActionPreference = 'Stop'

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Error 'Install Rust stable from https://rustup.rs/ and open a new terminal.'
}

if (-not (Get-Command cmake -ErrorAction SilentlyContinue)) {
    Write-Error 'Install CMake 3.28 or newer and add it to PATH. See REQUIREMENTS.md.'
}

$cmakeVersion = (cmake --version | Select-Object -First 1) -replace '^cmake version ', ''
if ($cmakeVersion -notmatch '^\d+\.\d+\.\d+' -or [version]$Matches[0] -lt [version]'3.28') {
    Write-Error 'Install CMake 3.28 or newer. See REQUIREMENTS.md.'
}

Write-Host 'With Visual Studio Build Tools and the Desktop development with C++ workload installed, run:'
Write-Host '  cargo run --locked -p cadence-desktop'

$ErrorActionPreference = 'Stop'

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Error 'Install Rust stable from https://rustup.rs/ and open a new terminal.'
}

Write-Host 'With Visual Studio Build Tools and the Desktop development with C++ workload installed, run:'
Write-Host '  cargo run --locked -p cadence-desktop'

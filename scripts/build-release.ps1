# Build de release sem caminhos da maquina dentro do exe (registry do cargo, pasta do repo, PDB).
# Uso: powershell -File scripts/build-release.ps1   (gera app/target/release/eavely.exe)
$repo = (Resolve-Path "$PSScriptRoot\..").Path
$cargo = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $env:USERPROFILE '.cargo' }
$rustup = if ($env:RUSTUP_HOME) { $env:RUSTUP_HOME } else { Join-Path $env:USERPROFILE '.rustup' }
$env:RUSTFLAGS = "--remap-path-prefix=$cargo=/cargo --remap-path-prefix=$rustup=/rustup --remap-path-prefix=$repo=/eavely -C target-feature=+crt-static"
Set-Location "$repo\app"
cargo build --release

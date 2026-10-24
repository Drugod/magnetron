param([ValidateSet('dev','build','test','live')][string]$Action = 'dev')
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $projectRoot
$localCargo = Join-Path $projectRoot '.toolchain/cargo'
if (Test-Path -LiteralPath (Join-Path $localCargo 'bin/cargo.exe')) {
    $env:CARGO_HOME = $localCargo
    $env:RUSTUP_HOME = Join-Path $projectRoot '.toolchain/rustup'
    $env:PATH = "$localCargo\bin;$env:PATH"
}
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw 'Falta Rust. Instala Rust estable MSVC desde https://rustup.rs y las herramientas C++ de Visual Studio.'
}
Copy-Item -LiteralPath src-tauri/v2/worker.py -Destination vendor/v2-runtime/worker.py -Force
& (Join-Path $PSScriptRoot 'prepare-licenses.ps1')
if ($Action -eq 'test') { & cargo test --manifest-path src-tauri/Cargo.toml }
elseif ($Action -eq 'live') { & cargo test --manifest-path src-tauri/Cargo.toml live_catalogs -- --ignored --nocapture }
else { & npm.cmd run tauri -- $Action }
if ($Action -eq 'build' -and $LASTEXITCODE -eq 0) { & (Join-Path $PSScriptRoot 'package-portable.ps1') }
exit $LASTEXITCODE

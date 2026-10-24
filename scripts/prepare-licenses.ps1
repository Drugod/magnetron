$ErrorActionPreference = 'Stop'
Set-Location -LiteralPath (Split-Path -Parent $PSScriptRoot)
New-Item -ItemType Directory -Force artifacts/licenses,src-tauri/target | Out-Null
$metadataFile = Join-Path (Get-Location) 'src-tauri/target/license-metadata.json'
& cargo metadata --manifest-path src-tauri/Cargo.toml --locked --format-version 1 --filter-platform x86_64-pc-windows-msvc | Set-Content -LiteralPath $metadataFile -Encoding utf8
if ($LASTEXITCODE -ne 0) { throw 'No se pudieron resolver las licencias de las dependencias.' }
& node scripts/generate-notices.mjs $metadataFile
if ($LASTEXITCODE -ne 0) { throw 'No se pudieron generar los avisos de licencia.' }
Copy-Item -LiteralPath docs/licenses/librqbit-LICENSE.txt -Destination artifacts/licenses/librqbit-LICENSE.txt -Force

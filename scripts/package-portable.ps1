$ErrorActionPreference = 'Stop'
function Get-Sha256([string]$Path) {
    $stream = [IO.File]::Open($Path,[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::ReadWrite)
    $algorithm = [Security.Cryptography.SHA256]::Create()
    try { return [BitConverter]::ToString($algorithm.ComputeHash($stream)).Replace('-','').ToLowerInvariant() }
    finally { $algorithm.Dispose(); $stream.Dispose() }
}
$projectRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $projectRoot
$version = (Get-Content -Raw -LiteralPath src-tauri/tauri.conf.json | ConvertFrom-Json).version
$binary = Join-Path $projectRoot 'src-tauri/target/release/magnet.exe'
$installer = Join-Path $projectRoot "src-tauri/target/release/bundle/nsis/MAGNETRON_${version}_x64-setup.exe"
if (-not (Test-Path -LiteralPath $binary) -or -not (Test-Path -LiteralPath $installer)) { throw 'Completa primero npm run bundle.' }
New-Item -ItemType Directory -Force artifacts | Out-Null
Copy-Item -LiteralPath $binary -Destination "artifacts/MAGNETRON-$version.exe" -Force
Copy-Item -LiteralPath $binary -Destination artifacts/MAGNETRON.exe -Force
Copy-Item -LiteralPath $installer -Destination "artifacts/MAGNETRON_${version}_x64-setup.exe" -Force
$resourceSets = @{
    'v2' = 'vendor/v2-runtime'
    'engine' = 'vendor/jackett/Jackett'
    'amule' = 'vendor/amule-3.1.0/amule-portable-x64'
}
foreach ($destinationRoot in @($projectRoot, (Join-Path $projectRoot 'artifacts'))) {
    foreach ($folder in $resourceSets.Keys) {
        $destination = Join-Path $destinationRoot $folder
        New-Item -ItemType Directory -Force $destination | Out-Null
        $resourceRoot = (Resolve-Path -LiteralPath $resourceSets[$folder]).Path
        foreach ($item in Get-ChildItem -LiteralPath $resourceRoot -Recurse -Force) {
            $relative = $item.FullName.Substring($resourceRoot.Length).TrimStart([char]'\',[char]'/')
            $target = Join-Path $destination $relative
            if ($item.PSIsContainer) {
                New-Item -ItemType Directory -Force -Path $target | Out-Null
                continue
            }
            if (Test-Path -LiteralPath $target -PathType Leaf) {
                if ((Get-Sha256 $item.FullName) -eq (Get-Sha256 $target)) { continue }
            }
            Copy-Item -LiteralPath $item.FullName -Destination $target -Force
        }
    }
    $licenses = Join-Path $destinationRoot 'licenses'
    New-Item -ItemType Directory -Force $licenses | Out-Null
    Copy-Item -LiteralPath (Join-Path $projectRoot 'LICENSE') -Destination (Join-Path $licenses 'MAGNETRON-GPL-3.0.txt') -Force
    foreach ($file in @('Jackett-v0.24.2900-source.zip','aMule-3.1.0-source.zip','librqbit-LICENSE.txt','THIRD-PARTY-NOTICES.txt')) {
        $source = Join-Path $projectRoot "artifacts/licenses/$file"
        $target = Join-Path $licenses $file
        if ([IO.Path]::GetFullPath($source) -ne [IO.Path]::GetFullPath($target)) { Copy-Item -LiteralPath $source -Destination $target -Force }
    }
}
$portable = Join-Path $projectRoot 'MAGNETRON.exe'
Copy-Item -LiteralPath $binary -Destination (Join-Path $projectRoot "MAGNETRON-$version.exe") -Force
try {
    Copy-Item -LiteralPath $binary -Destination $portable -Force -ErrorAction Stop
} catch [System.IO.IOException] {
    $portable = Join-Path $projectRoot "MAGNETRON-$version.exe"
    Copy-Item -LiteralPath $binary -Destination $portable -Force
    Write-Warning "MAGNETRON.exe está abierto. Nueva versión disponible en $portable; cierra la anterior antes de abrirla."
}
$entries = @("MAGNETRON-$version.exe","MAGNETRON_${version}_x64-setup.exe",'MAGNETRON.exe','engine/JackettConsole.exe','amule/bin/amuled.exe')
$sums = foreach ($entry in $entries) {
    $hash = Get-Sha256 (Join-Path $projectRoot "artifacts/$entry")
    "$hash *$entry"
}
[IO.File]::WriteAllLines((Join-Path $projectRoot 'artifacts/SHA256SUMS.txt'),$sums,[Text.UTF8Encoding]::new($false))
Write-Output "MAGNETRON $version disponible en $portable"

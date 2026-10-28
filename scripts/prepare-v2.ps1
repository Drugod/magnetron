$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$projectRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $projectRoot
function Fetch-Verified($Url,$Path,$Sha) {
    if (-not (Test-Path -LiteralPath $Path)) { Invoke-WebRequest -UseBasicParsing -Uri $Url -OutFile $Path }
    $stream=[IO.File]::OpenRead((Join-Path $projectRoot $Path));$hash=[Security.Cryptography.SHA256]::Create()
    try { $actual=[BitConverter]::ToString($hash.ComputeHash($stream)).Replace('-','').ToLowerInvariant() } finally { $stream.Dispose();$hash.Dispose() }
    if ($actual -ne $Sha) { throw "SHA-256 incorrecto: $Path" }
}
New-Item -ItemType Directory -Force vendor | Out-Null
Fetch-Verified 'https://www.python.org/ftp/python/3.13.16/python-3.13.16-embed-amd64.zip' 'vendor/python-3.13.16-embed-amd64.zip' '97dae5274cc54867065e8d5a3226e48c35017ed332a0fdb0e27d5b5821961297'
Fetch-Verified 'https://files.pythonhosted.org/packages/f0/2b/b843874837440dadb6d6cde2374a311367efdb40f86375de46cf9a0e59ea/libtorrent-2.1.1-cp313-cp313-win_amd64.whl' 'vendor/libtorrent-2.1.1-cp313-cp313-win_amd64.whl' 'fa11aa801d5b9184f067f559c465d3074a4e878d9c671b00dd4930d6dfce2b46'
Expand-Archive -LiteralPath vendor/python-3.13.16-embed-amd64.zip -DestinationPath vendor/v2-runtime -Force
Copy-Item -LiteralPath vendor/libtorrent-2.1.1-cp313-cp313-win_amd64.whl -Destination vendor/libtorrent-wheel.zip -Force
Expand-Archive -LiteralPath vendor/libtorrent-wheel.zip -DestinationPath vendor/v2-runtime -Force
Copy-Item -LiteralPath docs/licenses/libtorrent-2.1.1-LICENSE.txt -Destination vendor/v2-runtime/LICENSE-libtorrent.txt -Force
$licensePath=Join-Path $projectRoot 'vendor/v2-runtime/LICENSE.txt'
$notice=[IO.File]::ReadAllText((Join-Path $projectRoot 'docs/licenses/libtorrent-2.1.1-LICENSE.txt'))
[IO.File]::AppendAllText($licensePath,"`n`n===== libtorrent 2.1.1 =====`n"+$notice,[Text.UTF8Encoding]::new($false))
Copy-Item -LiteralPath src-tauri/v2/worker.py -Destination vendor/v2-runtime/worker.py -Force
& ./vendor/v2-runtime/python.exe -I -c 'import libtorrent; print(libtorrent.version)'
if ($LASTEXITCODE -ne 0) { throw 'El motor v2 no arranca' }

Copy-Item -LiteralPath src-tauri/v2/engine_guard.py -Destination vendor/v2-runtime/engine_guard.py -Force

& ./vendor/v2-runtime/python.exe scripts/prepare-video.py
if ($LASTEXITCODE -ne 0) { throw "No se pudieron preparar las herramientas de vídeo" }

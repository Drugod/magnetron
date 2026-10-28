param([Parameter(Mandatory=$true)][string]$Installer,[Parameter(Mandatory=$true)][string]$Version)
$ErrorActionPreference = 'Stop'
if (-not $env:GITHUB_ACTIONS -or $env:RUNNER_OS -ne 'Windows') { throw 'Run only on a disposable Windows CI runner' }
$workspace = Join-Path $env:RUNNER_TEMP 'MAGNETRON-upgrade-validation'
New-Item -ItemType Directory -Force -Path $workspace | Out-Null
$baseline = Join-Path $workspace 'baseline'
New-Item -ItemType Directory -Force -Path $baseline | Out-Null
gh release download v0.8.0 --repo Drugod/magnetron --pattern MAGNETRON_0.8.0_x64-setup.exe --pattern SHA256SUMS.txt --dir $baseline
if ($LASTEXITCODE -ne 0) { throw 'Baseline download failed' }
$previous = Join-Path $baseline 'MAGNETRON_0.8.0_x64-setup.exe'
$checksum = Get-Content (Join-Path $baseline 'SHA256SUMS.txt') | Where-Object { $_ -match ' \*MAGNETRON_0.8.0_x64-setup\.exe$' }
if (-not $checksum -or (Get-FileHash -LiteralPath $previous -Algorithm SHA256).Hash -ne $checksum.Substring(0,64)) { throw 'Baseline digest mismatch' }
$destination = Join-Path $workspace 'installed app'
$data = Join-Path $env:LOCALAPPDATA 'local.magnetforge.desktop'
New-Item -ItemType Directory -Force -Path $data | Out-Null
$marker = Join-Path $data 'update-validation-marker.txt'
$token = [Guid]::NewGuid().ToString()
Set-Content -LiteralPath $marker -Value $token
$old = Start-Process -FilePath $previous -ArgumentList @('/S',"/D=$destination") -WindowStyle Hidden -PassThru -Wait
if ($old.ExitCode -ne 0) { throw "Baseline installer failed: $($old.ExitCode)" }
function Assert-Version([string]$expected) {
    $apps = @(Get-ChildItem -LiteralPath $destination -Filter '*.exe' -File | Where-Object { $_.Name -notmatch 'uninstall' -and $_.VersionInfo.ProductVersion -match ('^' + [regex]::Escape($expected) + '($|[.+-])') })
    if ($apps.Count -ne 1) { throw "Expected one installed app with product version $expected" }
}
Assert-Version '0.8.0'
$new = Start-Process -FilePath (Resolve-Path -LiteralPath $Installer).Path -ArgumentList @('/S',"/D=$destination") -WindowStyle Hidden -PassThru -Wait
if ($new.ExitCode -ne 0) { throw "Upgrade installer failed: $($new.ExitCode)" }
Assert-Version $Version
if ((Get-Content -LiteralPath $marker -Raw).Trim() -ne $token) { throw 'Upgrade changed existing application data' }
Write-Output "Installer upgrade from 0.8.0 to $Version preserves the application-data marker"

$runtime = Join-Path $destination 'v2/python.exe'
& $runtime -I (Join-Path $PSScriptRoot 'test-package-contents.py') $destination
if ($LASTEXITCODE -ne 0) { throw 'Installed package content checks failed' }
$previousVideoRuntime=$env:MAGNETRON_VIDEO_RUNTIME
try {
    $env:MAGNETRON_VIDEO_RUNTIME=Join-Path $destination 'v2'
    & $runtime -I (Join-Path $PSScriptRoot 'test-video.py')
    if ($LASTEXITCODE -ne 0) { throw 'Installed video runtime checks failed' }
} finally { $env:MAGNETRON_VIDEO_RUNTIME=$previousVideoRuntime }
if ($LASTEXITCODE -ne 0) { throw 'Installed package content validation failed' }

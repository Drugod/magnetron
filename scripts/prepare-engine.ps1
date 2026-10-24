$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
Set-Location -LiteralPath (Split-Path -Parent $PSScriptRoot)
$releaseTag = 'v0.24.2900'
$expectedHash = '624BE07C2E3328D04A944E741F5DBBC039E63D4009A123F9F7C61DC25BBB3CBE'
New-Item -ItemType Directory -Force vendor,artifacts/licenses | Out-Null
$archive = Join-Path (Get-Location) 'vendor/Jackett.Windows.zip'
if (-not (Test-Path -LiteralPath $archive)) {
    Invoke-WebRequest -UseBasicParsing -Uri "https://github.com/Jackett/Jackett/releases/download/$releaseTag/Jackett.Binaries.Windows.zip" -OutFile $archive
}
if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -ne $expectedHash) { throw 'La distribución de Jackett no coincide con el hash fijado.' }
if (-not (Test-Path -LiteralPath 'vendor/jackett/Jackett/JackettConsole.exe')) {
    Expand-Archive -LiteralPath $archive -DestinationPath vendor/jackett -Force
}
$sourceArchive = 'artifacts/licenses/Jackett-v0.24.2900-source.zip'
if (-not (Test-Path -LiteralPath $sourceArchive)) {
    Invoke-WebRequest -UseBasicParsing -Uri "https://github.com/Jackett/Jackett/archive/refs/tags/$releaseTag.zip" -OutFile $sourceArchive
}
if ((Get-FileHash -LiteralPath $sourceArchive -Algorithm SHA256).Hash -ne '30BDAC79721FF3F28034FD3B2EE90216250E149CD633E8992976B63082CE4149') { throw 'El código fuente de Jackett no coincide con el archivo fijado.' }
Write-Output 'Motor integrado verificado. No se instala ningún servicio ni se modifica el PATH.'

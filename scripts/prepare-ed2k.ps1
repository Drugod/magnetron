$ErrorActionPreference = 'Stop'
Set-Location -LiteralPath (Split-Path -Parent $PSScriptRoot)
New-Item -ItemType Directory -Force vendor,artifacts/licenses | Out-Null
$archive = 'vendor/amule-3.1.0-Windows-x64.zip'
if (-not (Test-Path -LiteralPath $archive)) {
    Invoke-WebRequest -Uri 'https://github.com/amule-org/amule/releases/download/3.1.0/aMule-3.1.0-Windows-x64.zip' -OutFile $archive
}
if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -ne '81D3A024207C272524E4D992BFAF352A86256B00AFAC414F0405CE465642CFB7') { throw 'El motor aMule no coincide con el hash fijado.' }
if (-not (Test-Path -LiteralPath 'vendor/amule-3.1.0/amule-portable-x64/bin/amuled.exe')) {
    Expand-Archive -LiteralPath $archive -DestinationPath vendor/amule-3.1.0 -Force
}
$sourceArchive = 'artifacts/licenses/aMule-3.1.0-source.zip'
if (-not (Test-Path -LiteralPath $sourceArchive)) {
    Invoke-WebRequest -Uri 'https://api.github.com/repos/amule-org/amule/zipball/3.1.0' -OutFile $sourceArchive
}
if ((Get-FileHash -LiteralPath $sourceArchive -Algorithm SHA256).Hash -ne 'A1C3A81E91DD23CEB6B2E8B52D05BCEFAAD6E57E7611F76812B31719C694B287') { throw 'El código fuente de aMule no coincide con el archivo fijado.' }
Write-Output 'Motor eD2k verificado. Se ejecuta como componente privado de MAGNETRON, sin instalar servicios.'

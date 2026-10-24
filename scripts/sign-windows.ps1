param([Parameter(Mandatory=$true)][string]$CertificateThumbprint,[Parameter(Mandatory=$true)][string[]]$Files,[string]$SignTool='signtool.exe')
$ErrorActionPreference='Stop'
foreach ($file in $Files) {
    $resolved=(Resolve-Path -LiteralPath $file).Path
    & $SignTool sign /sha1 $CertificateThumbprint /fd SHA256 /tr https://timestamp.digicert.com /td SHA256 $resolved
    if ($LASTEXITCODE -ne 0) { throw "No se pudo firmar $resolved" }
    & $SignTool verify /pa /all $resolved
    if ($LASTEXITCODE -ne 0) { throw "Firma inválida: $resolved" }
}

param(
    [Parameter(Mandatory)][string]$PackageDirectory,
    [Parameter(Mandatory)][string]$Version,
    [string]$OutputDirectory = "artifacts/windows-installer"
)
$ErrorActionPreference = "Stop"
if ($Version -notmatch '^\d+\.\d+\.\d+$') { throw "Expected a numeric release version" }
$Payload = (Resolve-Path $PackageDirectory).Path
if (-not (Test-Path (Join-Path $Payload "RasterDrop.exe"))) { throw "Release executable is missing" }
if (-not (Test-Path (Join-Path $Payload "vcruntime140.dll"))) { throw "C++ runtime files are missing" }
New-Item -ItemType Directory -Force $OutputDirectory | Out-Null
$Output = (Resolve-Path $OutputDirectory).Path
$Bootstrapper = Join-Path $Output "MicrosoftEdgeWebview2Setup.exe"
Invoke-WebRequest "https://go.microsoft.com/fwlink/p/?LinkId=2124703" -OutFile $Bootstrapper
$Signature = Get-AuthenticodeSignature $Bootstrapper
if ($Signature.Status -ne "Valid" -or $Signature.SignerCertificate.Subject -notmatch 'O=Microsoft Corporation') {
    throw "WebView2 bootstrapper does not have a valid Microsoft signature"
}
$Compiler = Get-Command ISCC.exe -ErrorAction SilentlyContinue
if ($Compiler) {
    $Iscc = $Compiler.Source
} else {
    $Iscc = "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe"
    if (-not (Test-Path $Iscc)) { throw "Inno Setup 6.3+ is required" }
}
$Script = Join-Path $PSScriptRoot "../installers/windows/RasterDrop.iss"
& $Iscc "/DAppVersion=$Version" "/DPayloadDir=$Payload" "/DBootstrapperPath=$Bootstrapper" "/O$Output" $Script
if ($LASTEXITCODE -ne 0) { throw "Installer compilation failed" }
$Installer = Join-Path $Output "RasterDrop-$Version-windows-x64-setup.exe"
if (-not (Test-Path $Installer)) { throw "Installer was not produced" }
Remove-Item $Bootstrapper
$Hash = (Get-FileHash $Installer -Algorithm SHA256).Hash.ToLowerInvariant()
"$Hash  $(Split-Path -Leaf $Installer)" | Set-Content -Encoding ascii (Join-Path $Output "SHA256SUMS-installer.txt")
Write-Host "Built installer: $Installer"

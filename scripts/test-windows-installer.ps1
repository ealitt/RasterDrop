param(
    [Parameter(Mandatory)][string]$Installer,
    [Parameter(Mandatory)][string]$PackageDirectory
)
$ErrorActionPreference = "Stop"
$Installer = (Resolve-Path $Installer).Path
$Payload = (Resolve-Path $PackageDirectory).Path
$InstallDir = Join-Path $env:RUNNER_TEMP "RasterDrop-installer-smoke"
$Log = Join-Path $env:RUNNER_TEMP "RasterDrop-install.log"
function Install-Release {
    $Process = Start-Process $Installer -ArgumentList "/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /DIR=`"$InstallDir`" /LOG=`"$Log`"" -Wait -PassThru
    if ($Process.ExitCode -ne 0) {
        Get-Content $Log -Tail 40 -ErrorAction SilentlyContinue
        throw "Installer failed with exit code $($Process.ExitCode)"
    }
}
Install-Release
foreach ($File in Get-ChildItem $Payload -File) {
    $Installed = Join-Path $InstallDir $File.Name
    if (-not (Test-Path $Installed)) { throw "Installed file missing: $($File.Name)" }
    if ((Get-FileHash $Installed).Hash -ne (Get-FileHash $File.FullName).Hash) {
        throw "Installed file differs from verified release: $($File.Name)"
    }
}
$Shortcut = Join-Path $env:APPDATA "Microsoft/Windows/Start Menu/Programs/RasterDrop/RasterDrop.lnk"
if (-not (Test-Path $Shortcut)) { throw "Start menu shortcut missing" }
$UninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\{B13B3B77-639F-4B2A-9A83-34492A6A7B13}_is1"
if (-not (Test-Path $UninstallKey)) { throw "Windows uninstall registration missing" }
$App = Start-Process (Join-Path $InstallDir "RasterDrop.exe") -PassThru
Start-Sleep -Seconds 5
if ($App.HasExited) { throw "Installed app exited at startup with code $($App.ExitCode)" }
Stop-Process -Id $App.Id -Force
Install-Release
$Uninstaller = Join-Path $InstallDir "unins000.exe"
$Process = Start-Process $Uninstaller -ArgumentList '/VERYSILENT /SUPPRESSMSGBOXES /NORESTART' -Wait -PassThru
if ($Process.ExitCode -ne 0) { throw "Uninstaller failed" }
Start-Sleep -Seconds 2
if (Test-Path (Join-Path $InstallDir "RasterDrop.exe")) { throw "Uninstall left application files behind" }
if (Test-Path $Shortcut) { throw "Uninstall left shortcut behind" }
if (Test-Path $UninstallKey) { throw "Uninstall registration was not removed" }
Write-Host "Installer smoke test passed: exact payload, shortcuts, registration, launch, reinstall, uninstall."

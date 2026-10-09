param([switch]$ExecutableOnly)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { throw "Rust/Cargo is required" }
if (-not $ExecutableOnly -and -not (Get-Command dx -ErrorAction SilentlyContinue)) { throw "Install Dioxus CLI 0.7.9: cargo install dioxus-cli --version 0.7.9 --locked" }
if (-not $env:VCPKG_ROOT) { throw "Set VCPKG_ROOT to a vcpkg checkout" }

$env:VCPKG_DEFAULT_TRIPLET = "x64-windows-static-md"
$env:VCPKGRS_TRIPLET = "x64-windows-static-md"
& "$env:VCPKG_ROOT\vcpkg.exe" install --triplet x64-windows-static-md "--x-install-root=$env:VCPKG_ROOT\installed"
if ($LASTEXITCODE -ne 0) { throw "vcpkg install failed" }

cargo test --all-features
if ($LASTEXITCODE -ne 0) { throw "Tests failed" }
if ($ExecutableOnly) {
    cargo clippy --all-targets --all-features -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw "Clippy failed" }
    cargo build --release --locked --bin raster-drop
    if ($LASTEXITCODE -ne 0) { throw "Release build failed" }
    $ArtifactDir = Join-Path $Root "artifacts/windows-x64"
    New-Item -ItemType Directory -Force $ArtifactDir | Out-Null
    Copy-Item "target/release/raster-drop.exe" (Join-Path $ArtifactDir "RasterDrop.exe") -Force
    # Bundle the C++ runtime used by the static-md native image codecs.
    $VsWhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
    $VsRoot = & $VsWhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    if ($LASTEXITCODE -ne 0 -or -not $VsRoot) { throw "Could not locate Visual C++ runtime" }
    $CrtDir = Get-ChildItem "$VsRoot\VC\Redist\MSVC\*\x64\Microsoft.VC*.CRT" -Directory |
        Sort-Object FullName -Descending | Select-Object -First 1
    if (-not $CrtDir) { throw "Could not locate redistributable x64 C++ runtime" }
    Copy-Item (Join-Path $CrtDir.FullName "*.dll") $ArtifactDir -Force
    Copy-Item "LICENSE", "THIRD_PARTY_NOTICES.md" $ArtifactDir -Force
    "RasterDrop 0.1.1 for Windows 10/11 x64. Keep the included DLL files beside RasterDrop.exe and run RasterDrop.exe. Microsoft Edge WebView2 Runtime must be installed (normally present on Windows 11). Images are converted locally." | Set-Content (Join-Path $ArtifactDir "README.txt")
    Write-Host "Built Windows executable: $ArtifactDir/RasterDrop.exe"
    exit 0
}
dx bundle --desktop --release --package-types msi,nsis
if ($LASTEXITCODE -ne 0) { throw "Dioxus bundle failed" }

$Exe = Get-ChildItem -Path target -Recurse -Filter raster-drop.exe | Where-Object { $_.FullName -match "release" } | Select-Object -First 1
if (-not $Exe) { throw "Release executable was not produced" }
Write-Host "Built Windows release without a console subsystem: $($Exe.FullName)"
Write-Host "Installer artifacts are under target/dx. Test them on a clean Windows 10/11 VM with no vcpkg or developer tools."

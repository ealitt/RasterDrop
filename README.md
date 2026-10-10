# RasterDrop

A compact cross-platform Dioxus Desktop utility for converting images locally. It never uploads files, contacts a server, requires an account, or needs a network connection after installation.

> **Branding placeholders:** replace `com.example.rasterdrop`, `REPLACE_ME`, and the files under `assets/icon.*` before distribution.

## Features

- Content-based detection for HEIC/HEIF, JPEG, PNG, WebP, TIFF, BMP, and AVIF.
- Output to JPEG, PNG, WebP, TIFF, BMP, or AVIF.
- Native drag-and-drop and multi-file picker.
- Saves beside each source by default, or to a persisted fixed folder/Downloads.
- Skips files already in the selected format without decoding or touching them.
- Bounded native worker pool, per-file status, numerical progress, cancellation, retry, and reveal.
- Collision-safe numbered filenames by default; optional atomic overwrite.
- Correct HEIF container transformations and EXIF orientation for ordinary formats.
- White alpha compositing for JPEG/BMP.

Files added while a batch is active are held for the immediately following batch. The active batch's locked format/settings are used, so a format change cannot split a running batch.

## Development prerequisites

### All platforms

1. Current stable Rust (`rustup toolchain install stable`). Rust 1.88+ is required by `image 0.25.10`.
2. Dioxus CLI 0.7.9:
   ```bash
   cargo install dioxus-cli --version 0.7.9 --locked
   ```
3. Native `libheif` 1.17+ with HEVC decoding and AV1 decoding enabled.

### macOS

```bash
xcode-select --install
brew install libheif pkg-config
pkg-config --modversion libheif
```

The project supports Apple Silicon and Intel macOS 12+. Local development may link Homebrew libraries. Distribution bundles all non-system dylibs into the `.app`; end users must not need Homebrew.

### Windows 10/11 x64

Install Visual Studio Build Tools with the C++ workload, CMake/Ninja, and vcpkg. Set:

```powershell
$env:VCPKG_ROOT = "C:\src\vcpkg"
$env:VCPKG_DEFAULT_TRIPLET = "x64-windows-static-md"
$env:VCPKGRS_TRIPLET = "x64-windows-static-md"
& "$env:VCPKG_ROOT\vcpkg.exe" install --triplet x64-windows-static-md
```

`vcpkg.json` disables libheif's encoder-oriented defaults and enables AOM. libde265 supplies HEIC decode; static CRT-compatible native libraries avoid shipping developer-machine DLL paths. Dioxus packaging uses the WebView2 bootstrapper strategy configured in `Dioxus.toml`.

## Run in development

```bash
cargo run
# Dioxus hot reload:
dx serve --desktop
# On CLIs requiring the explicit platform spelling:
dx serve --platform desktop
```

Drop files from Finder/File Explorer or use **Add Images**. Processing is local native Rust and is performed off the UI thread.

## Tests and quality gates

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

The HEIC integration fixture is `tests/fixtures/representative.heic`, from libheif-rs's MIT-licensed test data. Tests verify HEIC-to-JPEG/PNG, standard codecs, alpha behavior, content-based skip, naming/collisions, settings recovery, cleanup, mixed failures, bounded workers, and monotonic progress.

## macOS bundle and DMG

```bash
chmod +x scripts/build-macos.sh
./scripts/build-macos.sh
```

The script runs tests, invokes `dx bundle --platform desktop --release`, recursively copies non-system `libheif` codec dylibs into `Contents/Frameworks`, normalizes install names to `@rpath`, checks for leaked Homebrew paths, and verifies the bundle. Without a signing identity it applies an ad-hoc signature for local testing.

### Signing and notarization

Set `APPLE_SIGNING_IDENTITY` to a Developer ID Application identity before running the script. Then create/locate the DMG and submit it:

```bash
xcrun notarytool submit path/to/RasterDrop.dmg \
  --apple-id "$APPLE_ID" --team-id "$APPLE_TEAM_ID" \
  --password "$APPLE_APP_PASSWORD" --wait
xcrun stapler staple path/to/RasterDrop.dmg
spctl --assess --type execute --verbose "RasterDrop.app"
```

Use hardened runtime (`--options runtime`). Sign nested dylibs before the app. Add entitlements only if future features require them; this release does not request network, camera, microphone, or broad file entitlements.

## Windows installer

Release EXE installers are packaged from the verified Windows ZIP using the `Windows installer` GitHub Actions workflow. The installer adds Start menu shortcuts and an uninstaller, bundles the C++ runtime DLLs, and installs WebView2 when it is missing (internet access is needed for that prerequisite). Run the workflow with the existing release tag to package a release without recompiling the app.

On a Windows machine with Inno Setup 6.3+ installed, an extracted release ZIP can also be packaged locally:

```powershell
.\scripts\package-windows-installer.ps1 -PackageDirectory path\to\extracted-release -Version 0.1.1
```

To build only a Windows x64 executable on a Windows machine with the prerequisites above:

```powershell
.\scripts\build-windows.ps1 -ExecutableOnly
```

The executable and notices are placed in `artifacts/windows-x64`. The executable requires Microsoft Edge WebView2 Runtime. The `Windows executable` GitHub Actions workflow builds and tests this package on Windows Server 2022 and uploads it as `RasterDrop-0.1.1-windows-x64`.

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\scripts\build-windows.ps1
```

The script builds against `x64-windows-static-md`, runs tests, and invokes Dioxus bundling. Installer artifacts are produced under `target/dx`. The release binary has the Windows GUI subsystem and therefore opens no console window. Validate the MSI/installer on a clean Windows 10 and Windows 11 VM without Visual Studio, vcpkg, Rust, Python, or other developer tools.

For code signing, sign the executable and installer with your organization's Authenticode certificate after bundling, for example with `signtool sign /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 ...`.

## Native codec packaging

- `image` provides native PNG, JPEG, TIFF, BMP, and AVIF codecs. WebP output uses bundled libwebp with quality 82 and maximum compression effort to avoid the large files produced by lossless WebP for photographic inputs.
- `libheif-rs 2.7.0` binds to libheif. The adapter requests interleaved RGBA8, copies rows respecting native stride, and lets libheif apply crop/rotation/mirroring.
- macOS releases bundle `libheif` and every non-system transitive dylib with normalized `@rpath` paths.
- Windows releases use vcpkg's static-md triplet. Do not mix triplets.
- The `embedded-libheif` Cargo feature is intentionally not used: it does not automatically make all codec dependencies self-contained.

CI builds macOS Apple Silicon, macOS Intel, and Windows x64 independently. A macOS build is not treated as validation of Windows packaging.

## Architecture

- `src/app.rs`, `src/components/`: Dioxus UI and explicit signal state.
- `src/settings.rs`: versioned Serde settings with atomic persistence and safe defaults.
- `src/conversion/formats.rs`: lightweight content detection and format-family rules.
- `src/conversion/naming.rs`: output resolution and process-wide collision reservations.
- `src/conversion/heif.rs`: stride-safe libheif RGBA decoder.
- `src/conversion/worker.rs`: testable single-file conversion/temporary-file pipeline.
- `src/conversion/coordinator.rs`: bounded Rayon pool and channel events.
- `src/platform.rs`: Downloads and reveal integration.

Workers never mutate Dioxus signals. Each worker decodes and encodes one image at a time; complete batches of decoded images are never retained. Worker count is `min(8, max(1, logical CPUs - 1))`.

## Data and privacy

The executable has no conversion network path, account code, telemetry, local server, Python sidecar, cloud API, or shell-based codec. File picker access and paths selected/dropped by the user are the only image inputs. Diagnostic logs are stored in the platform application-data directory and contain errors/paths needed for troubleshooting.

## Metadata and color limitations

Visible orientation is normalized. Output pixels are RGB/RGBA and transparency is retained where supported. EXIF, GPS, ICC profiles, and creation metadata are not copied in this release; consequently GPS metadata is not propagated. Full professional ICC color management is not implemented. WebP output uses high-quality lossy color compression with compressed alpha. GIF and animated WebP inputs are rejected rather than silently flattened. HEIC output is intentionally absent.

## Manual release checklist

See [`docs/MANUAL_TEST_CHECKLIST.md`](docs/MANUAL_TEST_CHECKLIST.md). Every packaged target must be tested on its native OS and again on a clean machine without developer tools.

## License and notices

Application code is MIT licensed. See [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md) before distribution. Native codec license obligations depend on the exact CI-produced dependency graph; inspect each packaged artifact and have distribution counsel review static-linking obligations.

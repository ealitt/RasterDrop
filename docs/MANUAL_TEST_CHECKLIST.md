# Manual release test checklist

Record OS version, architecture, artifact checksum, tester, and date for every run.

## macOS (Apple Silicon and Intel)

- [ ] Launch the signed packaged `.app`; confirm no terminal or developer environment is needed.
- [ ] Verify Gatekeeper assessment and notarization/stapling when applicable.
- [ ] Change each output format in the main window.
- [ ] Drop one image from Finder and confirm drag highlighting plus automatic conversion.
- [ ] Drop 100 ordinary phone images; verify UI responsiveness and monotonic percentage.
- [ ] Drop files from multiple folders and verify source-adjacent outputs.
- [ ] Drop `.jpg`/`.jpeg`, `.tif`/`.tiff`, and misleading-extension same-format files; verify no rewrite/timestamp change.
- [ ] Drop a text file, empty image, corrupted file, folder, and removed queued source; verify isolated failures.
- [ ] Use **Add Images** with multiple files.
- [ ] Select **Use Downloads**, restart, and verify persistence.
- [ ] Remove/lock the fixed output folder and verify a clear settings and batch error without fallback.
- [ ] Verify numbered collisions and atomic overwrite while originals remain unchanged.
- [ ] Cancel a large batch; verify completed outputs remain and temporary files are absent.
- [ ] Retry failed files and clear the list.
- [ ] Reveal the most recent output in Finder.
- [ ] Convert representative HEIC/HEIF images with rotations, alpha, Unicode names, and Spaces paths.
- [ ] Test PNG alpha to JPEG/BMP and verify a white background.
- [ ] Verify animated GIF/WebP rejection.
- [ ] Test light mode, dark mode, keyboard operation, VoiceOver labels, Retina rendering, and reduced motion.
- [ ] Inspect `otool -L` for the executable and all Frameworks; verify no Homebrew paths.
- [ ] Repeat on a clean Mac without Homebrew, Rust, Python, or developer tools.

## Windows 10 and Windows 11 x64

- [ ] Install using the packaged installer; confirm WebView2 installation/bootstrap behavior.
- [ ] Launch from Start Menu and confirm no console window.
- [ ] Change each output format in the main window.
- [ ] Drop one image from File Explorer and confirm native paths, highlighting, and conversion.
- [ ] Repeat all batch, multi-folder, skip, invalid-input, picker, Downloads, persistence, collision, cancel, retry, reveal, HEIC orientation, alpha, animation, accessibility, dark-mode, high-DPI, Unicode, and long-path checks above.
- [ ] Confirm **Reveal Most Recent** selects the file in File Explorer.
- [ ] Inspect imports/dependencies and verify no DLL resolves into vcpkg/build directories.
- [ ] Repeat on clean Windows 10 and Windows 11 VMs without Visual Studio, vcpkg, Rust, Python, or developer tools.

## Stress and resource checks

- [ ] Convert 1,000 ordinary phone photos; sample memory and verify only bounded workers decode concurrently.
- [ ] Convert several very large images and confirm UI responsiveness.
- [ ] Mix valid, unsupported, corrupted, same-format, read-denied, and write-denied entries.
- [ ] Confirm progress never moves backward and reaches 100% with all terminal states counted.
- [ ] Confirm one failure never aborts unrelated files.
- [ ] Search all destination folders for `.rasterdrop-*.tmp` after success, failure, and cancellation.

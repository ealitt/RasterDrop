# Third-party notices

This file is a packaging checklist, not legal advice. Preserve license texts for the exact versions included in each release artifact.

| Component | Purpose | License summary |
|---|---|---|
| Dioxus / Dioxus Desktop | Desktop UI | MIT OR Apache-2.0 |
| image and image codec crates | Standard image decode/encode | MIT OR Apache-2.0 and codec-specific permissive licenses |
| libheif / libheif-rs | HEIF container and Rust binding | LGPL-3.0-or-later (libheif), MIT (binding) |
| libde265 | HEVC decoding | LGPL-3.0-or-later |
| dav1d and/or AOM | AV1 decoding | BSD-style |
| rav1e/ravif | AVIF encoding | BSD-2-Clause / permissive |
| WebView2 Runtime | Windows WebView | Microsoft license |
| WebKit (system) | macOS WebView | platform system component |

The release configuration does not intentionally enable HEIC encoding. The project-level vcpkg manifest disables libheif default features to avoid adding x265 solely for HEVC encoding. Verify the resolved vcpkg and Homebrew graphs for every release; package all corresponding license files.

Dynamic macOS distribution must comply with LGPL relinking/replacement requirements for bundled libheif/libde265. Static Windows distribution requires a specific LGPL compliance review and an appropriate method for recipients to relink or replace covered libraries. If those obligations do not fit the intended distribution, switch Windows packaging to dynamic DLL bundling and provide the required notices/source offer.

The HEIC test fixture under `tests/fixtures/representative.heic` is copied from the MIT-licensed `libheif-rs` crate test data.

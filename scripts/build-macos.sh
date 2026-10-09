#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-12.0}"

command -v cargo >/dev/null || { echo "Rust/Cargo is required" >&2; exit 1; }
command -v dx >/dev/null || { echo "Install Dioxus CLI 0.7.9: cargo install dioxus-cli --version 0.7.9 --locked" >&2; exit 1; }
command -v pkg-config >/dev/null || { echo "pkg-config is required" >&2; exit 1; }
pkg-config --atleast-version=1.17 libheif || { echo "libheif 1.17+ is required (brew install libheif pkg-config)" >&2; exit 1; }

cargo test --all-features
dx bundle --desktop --release --package-types macos

APP="$(find target -type d -name 'RasterDrop.app' -print -quit)"
[[ -n "$APP" ]] || { echo "Dioxus did not produce RasterDrop.app" >&2; exit 1; }
FRAMEWORKS="$APP/Contents/Frameworks"
EXECUTABLE="$APP/Contents/MacOS/raster-drop"
mkdir -p "$FRAMEWORKS"

is_system_library() {
  [[ "$1" == /System/* || "$1" == /usr/lib/* || "$1" == @* ]]
}

copy_dependencies() {
  local binary="$1" dependency target
  while IFS= read -r dependency; do
    [[ -n "$dependency" ]] || continue
    is_system_library "$dependency" && continue
    [[ -f "$dependency" ]] || continue
    target="$FRAMEWORKS/$(basename "$dependency")"
    if [[ ! -f "$target" ]]; then
      cp -L "$dependency" "$target"
      chmod u+w "$target"
      copy_dependencies "$target"
    fi
  done < <(otool -L "$binary" | tail -n +2 | sed -E 's/^[[:space:]]*([^[:space:]]+).*/\1/')
}

copy_dependencies "$EXECUTABLE"
for dylib in "$FRAMEWORKS"/*.dylib; do
  [[ -e "$dylib" ]] || continue
  install_name_tool -id "@rpath/$(basename "$dylib")" "$dylib"
done
for binary in "$EXECUTABLE" "$FRAMEWORKS"/*.dylib; do
  [[ -e "$binary" ]] || continue
  while IFS= read -r dependency; do
    is_system_library "$dependency" && continue
    [[ -n "$dependency" ]] || continue
    install_name_tool -change "$dependency" "@rpath/$(basename "$dependency")" "$binary"
  done < <(otool -L "$binary" | tail -n +2 | sed -E 's/^[[:space:]]*([^[:space:]]+).*/\1/')
done
install_name_tool -add_rpath '@executable_path/../Frameworks' "$EXECUTABLE" 2>/dev/null || true

# Strip non-exported symbols after linkage rewriting and before signing.
strip -x "$EXECUTABLE"
for dylib in "$FRAMEWORKS"/*.dylib; do
  [[ -e "$dylib" ]] || continue
  strip -x "$dylib"
done

if [[ -n "${APPLE_SIGNING_IDENTITY:-}" ]]; then
  for dylib in "$FRAMEWORKS"/*.dylib; do codesign --force --options runtime --timestamp --sign "$APPLE_SIGNING_IDENTITY" "$dylib"; done
  codesign --force --deep --options runtime --timestamp --sign "$APPLE_SIGNING_IDENTITY" "$APP"
else
  codesign --force --deep --sign - "$APP"
fi

if otool -L "$EXECUTABLE" | grep -E '/opt/homebrew|/usr/local/Cellar'; then
  echo "ERROR: bundle still references build-machine Homebrew paths" >&2
  exit 1
fi
codesign --verify --deep --strict "$APP"
ARTIFACT_DIR="$ROOT/artifacts/local-macos"
ARTIFACT_APP="$ARTIFACT_DIR/RasterDrop.app"
mkdir -p "$ARTIFACT_DIR"
rm -rf "$ARTIFACT_APP"
ditto "$APP" "$ARTIFACT_APP"
DMG="$ROOT/artifacts/RasterDrop.dmg"
mkdir -p "$(dirname "$DMG")"
rm -f "$DMG"
hdiutil create -volname "RasterDrop" -srcfolder "$ARTIFACT_APP" -ov -format UDZO "$DMG" >/dev/null
echo "Built and verified: $ARTIFACT_APP"
echo "Created DMG: $DMG"

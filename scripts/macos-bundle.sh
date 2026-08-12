#!/usr/bin/env bash
# Build, bundle, and ad-hoc codesign League Director.app
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

cargo bundle --release --format osx
APP="$ROOT/target/release/bundle/osx/League Director.app"

if [[ ! -d "$APP" ]]; then
  echo "bundle missing: $APP" >&2
  exit 1
fi

# Bundle ffmpeg next to the binary so remux works without Homebrew.
if command -v ffmpeg >/dev/null 2>&1; then
  cp "$(command -v ffmpeg)" "$APP/Contents/MacOS/ffmpeg"
  chmod +x "$APP/Contents/MacOS/ffmpeg"
fi

# cargo-bundle already merges assets/Info.plist.ext. Re-sign so TCC sees a stable identity.
codesign --force --deep --sign - "$APP"
codesign --verify --verbose=2 "$APP" || true

echo
echo "App: $APP"
echo "Identifier: $(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$APP/Contents/Info.plist")"
echo "Version:    $(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$APP/Contents/Info.plist")"
echo
echo "Grant Accessibility + Input Monitoring + Files and Folders to this .app, not to cargo."

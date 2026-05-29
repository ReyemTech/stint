#!/usr/bin/env bash
# Build Stint.app and relocate the embedded StintWidget.appex into
# Contents/PlugIns/ where macOS's WidgetKit looks for widget extensions.
#
# Why this script exists:
#   Tauri's bundle.resources places files under Contents/Resources/. Apple
#   requires .appex extensions at Contents/PlugIns/<name>.appex. We let
#   build.rs produce the .appex into crates/stint-app/PlugIns/ (gitignored)
#   and this wrapper moves it into the right place inside the bundled .app
#   post-build, then re-signs.
#
# Usage:
#   scripts/build-app-with-widget.sh            # ad-hoc sign (dev / local install)
#   scripts/build-app-with-widget.sh "Developer ID Application: ..."  # release sign

set -euo pipefail

readonly REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

readonly SIGN_IDENTITY="${1:--}"  # default: "-" = ad-hoc
readonly APP="target/release/bundle/macos/Stint.app"
readonly SRC_APPEX="crates/stint-app/PlugIns/StintWidget.appex"
readonly DEST_APPEX="$APP/Contents/PlugIns/StintWidget.appex"

echo "==> Building Stint.app"
cargo tauri build --bundles app

if [[ ! -d "$SRC_APPEX" ]]; then
  echo "ERROR: $SRC_APPEX missing — build.rs did not produce the widget appex"
  exit 1
fi

echo "==> Relocating StintWidget.appex into Contents/PlugIns/"
mkdir -p "$(dirname "$DEST_APPEX")"
rm -rf "$DEST_APPEX"
cp -R "$SRC_APPEX" "$DEST_APPEX"

# Strip the Resources/PlugIns duplicate that Tauri's bundle step would
# otherwise leave behind (harmless but doubles the dylib).
rm -rf "$APP/Contents/Resources/PlugIns"

echo "==> Re-signing embedded StintIntents framework (build.rs ad-hoc only)"
codesign --force --options runtime --timestamp --sign "$SIGN_IDENTITY" \
  "$APP/Contents/Frameworks/StintIntents.framework"

echo "==> Signing $DEST_APPEX with $SIGN_IDENTITY (sandboxed)"
codesign --force --options runtime --timestamp --sign "$SIGN_IDENTITY" \
  --entitlements crates/stint-app/swift/StintWidget/StintWidget.entitlements \
  "$DEST_APPEX"

echo "==> Re-signing main bundle to seal the new PlugIns/ + Frameworks/"
codesign --force --options runtime --timestamp --sign "$SIGN_IDENTITY" \
  --entitlements crates/stint-app/entitlements.plist \
  "$APP/Contents/MacOS/stint-app"
codesign --force --options runtime --timestamp --sign "$SIGN_IDENTITY" \
  --entitlements crates/stint-app/entitlements.plist \
  "$APP"

echo "==> Verifying signature"
codesign --verify --deep --strict --verbose=2 "$APP" 2>&1 | tail -3

echo "==> Done. Bundle at $APP"

#!/usr/bin/env bash
# scripts/release/test-cask-locally.sh
# Build the .dmg from the latest workflow_dispatch smoke artifacts and verify
# the cask formula installs/uninstalls cleanly. Does not touch the real tap.

set -euo pipefail

readonly TAP_REPO="${TAP_REPO:-../homebrew-tap}"
readonly CASK="${TAP_REPO}/Casks/stint.rb"
readonly DMG="${1:?path to local Stint-X.Y.Z.dmg required}"
readonly VERSION="${2:?version (matching the dmg) required}"

[[ -f "$CASK" ]] || { echo "error: $CASK not found" >&2; exit 1; }
[[ -f "$DMG" ]]  || { echo "error: $DMG not found" >&2; exit 1; }

SHA=$(shasum -a 256 "$DMG" | awk '{print $1}')

# Temp tap that points the URL at a local file:// URL.
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"; brew uninstall --cask stint-local 2>/dev/null || true' EXIT

mkdir -p "$WORK/Casks"
sed \
  -e "s|^cask \"stint\"|cask \"stint-local\"|" \
  -e "s|^\([[:space:]]*version[[:space:]]*\)\"[^\"]*\"|\1\"$VERSION\"|" \
  -e "s|^\([[:space:]]*sha256[[:space:]]*\)\"[^\"]*\"|\1\"$SHA\"|" \
  -e "s|https://github.com[^\"]*|file://$DMG|" \
  "$CASK" > "$WORK/Casks/stint-local.rb"

brew audit --cask "$WORK/Casks/stint-local.rb"
brew install --cask --no-quarantine "$WORK/Casks/stint-local.rb"
test -d /Applications/Stint.app
brew uninstall --cask stint-local

echo "✓ cask install/uninstall round-trip OK"

#!/usr/bin/env bash
# scripts/release/sync-beta-latest.sh
# Mirror artifacts from v<version> to a moving beta-latest GitHub Release.
#
# GitHub does not expose /releases/latest-prerelease, so we maintain a
# moving `beta-latest` tag/release that always points at the newest beta.
# The cask and updater can then link to stable URLs.
#
# Usage: sync-beta-latest.sh <version>
#   e.g. sync-beta-latest.sh 0.2.0-beta.1

set -euo pipefail

readonly VERSION="${1:?version required (e.g. 0.2.0-beta.1)}"
readonly ARTIFACTS_DIR="${ARTIFACTS_DIR:-target/release-artifacts}"
readonly REPO="reyemtech/stint"

# Rename Stint-VERSION.dmg → Stint-Beta-latest.dmg for the stable URL.
DMG_SRC=""
for f in "$ARTIFACTS_DIR"/Stint-*.dmg; do
  DMG_SRC="$f"
  break
done
[[ -f "$DMG_SRC" ]] || { echo "error: no DMG matching Stint-*.dmg in $ARTIFACTS_DIR" >&2; exit 1; }

DMG_FIXED_NAME="$ARTIFACTS_DIR/Stint-Beta-latest.dmg"
cp "$DMG_SRC" "$DMG_FIXED_NAME"

# Delete existing beta-latest release if present (also removes the tag).
gh release delete beta-latest --repo "$REPO" --yes --cleanup-tag 2>/dev/null || true

# Recreate as a fresh prerelease pointing at the current commit.
gh release create beta-latest \
  --repo "$REPO" \
  --prerelease \
  --title "Latest beta (${VERSION})" \
  --notes "Moving tag. Currently mirrors v${VERSION}. Do not link directly to /releases/tag/beta-latest for archaeology — use /releases/tag/v${VERSION} instead." \
  --target "$(git rev-parse HEAD)" \
  "$DMG_FIXED_NAME" \
  "$ARTIFACTS_DIR/Stint.app.tar.gz" \
  "$ARTIFACTS_DIR/Stint.app.tar.gz.sig" \
  "$ARTIFACTS_DIR/latest.json"

echo "✓ beta-latest mirrors v${VERSION}"

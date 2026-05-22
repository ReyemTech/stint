#!/usr/bin/env bash
# scripts/release/render-install-script.sh
# Substitute @@…@@ placeholders in scripts/install.sh.tpl.
#
# Usage: render-install-script.sh \
#   --version 0.1.0 \
#   --tarball-sha256 abc... \
#   --dmg-sha256 def... \
#   --output /path/to/install.sh

set -euo pipefail

VERSION=""
TARBALL_SHA=""
DMG_SHA=""
OUTPUT=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version)        VERSION="$2"; shift 2 ;;
    --tarball-sha256) TARBALL_SHA="$2"; shift 2 ;;
    --dmg-sha256)     DMG_SHA="$2"; shift 2 ;;
    --output)         OUTPUT="$2"; shift 2 ;;
    *) echo "unknown arg: $1" >&2; exit 1 ;;
  esac
done

: "${VERSION:?--version required}"
: "${TARBALL_SHA:?--tarball-sha256 required}"
: "${DMG_SHA:?--dmg-sha256 required}"
: "${OUTPUT:?--output required}"

TPL="$(dirname "$0")/../install.sh.tpl"
sed \
  -e "s|@@STINT_VERSION@@|$VERSION|g" \
  -e "s|@@TARBALL_SHA256@@|$TARBALL_SHA|g" \
  -e "s|@@DMG_SHA256@@|$DMG_SHA|g" \
  "$TPL" > "$OUTPUT"

chmod +x "$OUTPUT"
shasum -a 256 "$OUTPUT" | awk '{print $1}' > "${OUTPUT}.sha256"

echo "✓ wrote $OUTPUT (+ .sha256 sibling)"

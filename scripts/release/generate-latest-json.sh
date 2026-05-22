#!/usr/bin/env bash
# scripts/release/generate-latest-json.sh
# Compose a tauri-plugin-updater manifest from a signed .app.tar.gz.
# Universal bundle: same URL + signature for both arch keys.

set -euo pipefail

VERSION=""
SIG_PATH=""
BUNDLE_URL=""
NOTES=""
OUTPUT=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version)        VERSION="$2"; shift 2 ;;
    --signature-path) SIG_PATH="$2"; shift 2 ;;
    --bundle-url)     BUNDLE_URL="$2"; shift 2 ;;
    --notes)          NOTES="$2"; shift 2 ;;
    --output)         OUTPUT="$2"; shift 2 ;;
    *) echo "unknown arg: $1" >&2; exit 1 ;;
  esac
done

: "${VERSION:?--version required}"
: "${SIG_PATH:?--signature-path required}"
: "${BUNDLE_URL:?--bundle-url required}"
: "${OUTPUT:?--output required}"

SIGNATURE="$(cat "$SIG_PATH")"
NOTES="${NOTES:-see CHANGELOG.md}"
PUB_DATE="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

jq -n \
  --arg version "$VERSION" \
  --arg notes "$NOTES" \
  --arg pub_date "$PUB_DATE" \
  --arg sig "$SIGNATURE" \
  --arg url "$BUNDLE_URL" \
  '{
    version: $version,
    notes: $notes,
    pub_date: $pub_date,
    platforms: {
      "darwin-x86_64":  { signature: $sig, url: $url },
      "darwin-aarch64": { signature: $sig, url: $url }
    }
  }' > "$OUTPUT"

echo "✓ wrote $OUTPUT"

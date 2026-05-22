#!/usr/bin/env bash
# scripts/release/update-cask.sh
# In-place edit of `version` and `sha256` in a Homebrew cask formula.
#
# Usage: update-cask.sh <path/to/cask.rb> <version> <sha256>

set -euo pipefail

readonly CASK="${1:?cask file required}"
readonly VERSION="${2:?version required}"
readonly SHA256="${3:?sha256 required}"

[[ -f "$CASK" ]] || { echo "error: $CASK not found" >&2; exit 1; }
[[ "$SHA256" =~ ^[a-f0-9]{64}$ ]] || { echo "error: sha256 must be 64 hex chars" >&2; exit 1; }

# Portable sed -i form (works on BSD/macOS and GNU): backup file we then delete.
# Use POSIX [[:space:]] class — BSD sed BRE does not support GNU's \s.
sed -i.bak \
  -e "s/^\([[:space:]]*version[[:space:]]*\)\"[^\"]*\"/\1\"$VERSION\"/" \
  -e "s/^\([[:space:]]*sha256[[:space:]]*\)\"[^\"]*\"/\1\"$SHA256\"/" \
  "$CASK"
rm -f "${CASK}.bak"

echo "✓ $CASK → version $VERSION"

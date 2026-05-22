#!/usr/bin/env bash
# tests/release/render-install-script.test.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
readonly REPO_ROOT
SANDBOX="$(mktemp -d)"
readonly SANDBOX
trap 'rm -rf "$SANDBOX"' EXIT

"$REPO_ROOT/scripts/release/render-install-script.sh" \
  --version "0.1.0" \
  --tarball-sha256 "deadbeef1234567890" \
  --dmg-sha256 "cafef00d1234567890" \
  --output "$SANDBOX/install.sh"

# Substitutions happened.
grep -q 'STINT_VERSION="0.1.0"' "$SANDBOX/install.sh" || { echo "FAIL: version"; exit 1; }
grep -q 'STINT_TARBALL_SHA256="deadbeef1234567890"' "$SANDBOX/install.sh" || { echo "FAIL: tarball"; exit 1; }
grep -q 'STINT_DMG_SHA256="cafef00d1234567890"' "$SANDBOX/install.sh" || { echo "FAIL: dmg"; exit 1; }
# No leftover placeholders.
! grep -q '@@' "$SANDBOX/install.sh" || { echo "FAIL: leftover placeholder"; exit 1; }
# Rendered output must parse as valid POSIX sh
sh -n "$SANDBOX/install.sh" || { echo "FAIL: rendered install.sh has syntax errors"; exit 1; }

# Rendered output must be shellcheck-clean (POSIX mode)
shellcheck -s sh "$SANDBOX/install.sh" || { echo "FAIL: rendered install.sh has shellcheck warnings"; exit 1; }
# Sibling checksum file written.
[[ -f "$SANDBOX/install.sh.sha256" ]] || { echo "FAIL: missing sha256 sibling"; exit 1; }

echo PASS

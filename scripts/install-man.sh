#!/usr/bin/env bash
# Generate and install the stint(1) man page to a standard location.
# Usage: ./scripts/install-man.sh [PREFIX]
#   PREFIX defaults to /usr/local
#   Installs to $PREFIX/share/man/man1/stint.1
#
# Requires `stint` to be built or installed in PATH.

set -euo pipefail
PREFIX="${1:-/usr/local}"
DEST="$PREFIX/share/man/man1"
TMP="$(mktemp -d)"
trap "rm -rf $TMP" EXIT

if ! command -v stint >/dev/null 2>&1; then
    echo "error: stint not found in PATH" >&2
    echo "       install it first: cargo install --path crates/stint-cli" >&2
    exit 1
fi

stint generate-man "$TMP"
sudo install -d -m 755 "$DEST"
sudo install -m 644 "$TMP/stint.1" "$DEST/stint.1"
echo "installed $DEST/stint.1"
echo "verify: man stint"

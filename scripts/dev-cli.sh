#!/usr/bin/env bash
# Build, codesign with stint-dev, and run the CLI binary. Replaces
# `cargo run -p stint-cli -- ARGS` for dev work where you want Keychain
# "Always Allow" to persist across rebuilds.
#
# First-time setup: run `scripts/setup-dev-cert.sh` once.
#
# Usage: scripts/dev-cli.sh <stint subcommand and args>
#   e.g. scripts/dev-cli.sh config login
#        scripts/dev-cli.sh today
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

if [[ -f .env.local ]]; then
  # shellcheck disable=SC1091
  source .env.local
fi

if ! security find-identity -v -p codesigning | grep -q '"stint-dev"'; then
  echo "stint-dev code-signing identity not found." >&2
  echo "Run scripts/setup-dev-cert.sh once to create it." >&2
  exit 1
fi

cargo build -p stint-cli --quiet
codesign -f -s "stint-dev" target/debug/stint 2>/dev/null
exec target/debug/stint "$@"

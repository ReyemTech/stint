#!/usr/bin/env bash
# Sweep test-only Keychain entries (`tech.reyem.stint.test.*`) that
# accumulate from cargo test runs.
#
# Background: integration tests scope their Keychain writes to a unique
# per-test prefix via STINT_SECRET_PREFIX (see crates/stint-cli/tests/*).
# Each entry is created by the `stint` test subprocess and never deleted
# from the test-process Drop (cross-cdhash delete prompts).
#
# This script reads the keychain dump, finds every `tech.reyem.stint.test.`
# service, and deletes it via Apple-signed `security` (no prompts).
set -euo pipefail

KEYCHAIN="$HOME/Library/Keychains/login.keychain-db"

mapfile -t SERVICES < <(
  security dump-keychain "$KEYCHAIN" 2>/dev/null \
    | grep -oE '"svce"<blob>="tech\.reyem\.stint\.test\.[^"]+"' \
    | sed -E 's/^"svce"<blob>="//; s/"$//' \
    | sort -u
)

if [[ ${#SERVICES[@]} -eq 0 ]]; then
  echo "No tech.reyem.stint.test.* entries to clean."
  exit 0
fi

echo "Deleting ${#SERVICES[@]} test Keychain entries:"
for svc in "${SERVICES[@]}"; do
  if security delete-generic-password -s "$svc" -a stint "$KEYCHAIN" >/dev/null 2>&1; then
    echo "  ✓ $svc"
  else
    echo "  ✗ $svc (failed)"
  fi
done

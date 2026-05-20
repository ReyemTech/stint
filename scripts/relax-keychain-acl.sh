#!/usr/bin/env bash
# One-time helper. Relaxes the keychain ACL on the tech.reyem.stint.*
# generic-password entries so any binary signed by your stint-dev cert
# (or any Apple-signed tool) can read them without re-prompting after
# each rebuild.
#
# Background: macOS "Always Allow" stores the running binary's exact
# cdhash in the keychain entry's ACL, NOT its designated requirement.
# Every `cargo build` changes the cdhash, so each rebuild re-prompts.
# This script applies a partition-list relaxation that whitelists
# codesigned binaries (matching the cert) regardless of cdhash.
#
# Covered entries:
#   - tech.reyem.stint.solidtime.token  (PAT auth)
#   - tech.reyem.stint.solidtime.oauth  (OAuth blob)
#   - tech.reyem.stint.calendar.<uuid>  (per-account calendar OAuth blobs;
#     discovered automatically by querying calendar_accounts in the local
#     SQLite DB — no manual update needed when accounts are added)
#
# Run this AFTER:
#   1. scripts/setup-dev-cert.sh (creates stint-dev cert)
#   2. The keychain entries exist (e.g. you've run `stint config set
#      solidtime.token <PAT>`, `scripts/dev-cli.sh config login`, and/or
#      `stint calendar add google` to create calendar account entries)
#
# You'll be asked for your login keychain password once.
set -euo pipefail

KEYCHAIN="$HOME/Library/Keychains/login.keychain-db"
ACCOUNT="stint"
SERVICES=(
  "tech.reyem.stint.solidtime.token"
  "tech.reyem.stint.solidtime.oauth"
)

# Discover per-account calendar Keychain entries (Phase 3b). Each
# row in calendar_accounts has a UUID; the matching Keychain entry
# is tech.reyem.stint.calendar.<uuid>. We read the DB rather than
# scraping the keychain itself because security(1) doesn't support
# globbing service names.
STINT_DB="$HOME/Library/Application Support/stint/stint.db"
if [[ -f "$STINT_DB" ]] && command -v sqlite3 >/dev/null 2>&1; then
  while IFS= read -r account_uuid; do
    [[ -n "$account_uuid" ]] && SERVICES+=("tech.reyem.stint.calendar.${account_uuid}")
  done < <(sqlite3 "$STINT_DB" "SELECT id FROM calendar_accounts" 2>/dev/null || true)
fi
PARTITIONS="apple-tool:,apple:,codesign:"

echo "Relaxing ACL on tech.reyem.stint.* keychain entries so codesigned"
echo "dev builds (via scripts/dev-cli.sh) won't keep re-prompting."
echo

read -r -s -p "Login keychain password: " KC_PASS
echo
echo

EXIT=0
for svc in "${SERVICES[@]}"; do
  if ! security find-generic-password -s "$svc" -a "$ACCOUNT" "$KEYCHAIN" >/dev/null 2>&1; then
    echo "  - $svc: not in keychain (skipping; create it first if you need it)"
    continue
  fi

  if security set-generic-password-partition-list \
       -S "$PARTITIONS" \
       -s "$svc" \
       -a "$ACCOUNT" \
       -k "$KC_PASS" \
       "$KEYCHAIN" >/dev/null 2>&1; then
    echo "  ✓ $svc: partition list updated"
  else
    echo "  ✗ $svc: failed (wrong keychain password, or entry exists but ACL update rejected)"
    EXIT=1
  fi
done

echo
if [[ $EXIT -eq 0 ]]; then
  echo "Done. Next rebuild via scripts/dev-cli.sh should not prompt."
else
  echo "Some entries failed. Re-run with the correct password, or use Keychain Access GUI."
fi
exit $EXIT

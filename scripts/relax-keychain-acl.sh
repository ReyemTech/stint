#!/usr/bin/env bash
# Recreate the tech.reyem.stint.* keychain entries with "allow any app"
# access (security add-generic-password -A) so dev rebuilds don't keep
# re-prompting on every cdhash change.
#
# Background: macOS Keychain entries created via the standard SecItem
# APIs (e.g. Rust's keyring crate) bind their ACL to the creating
# binary's cdhash. "Always Allow" stores the current cdhash, but each
# `cargo build` produces a fresh one, so the grant invalidates on
# every rebuild. The partition-list mechanism (`codesign:`) was supposed
# to fix this for any code-signed binary, but on macOS Sonoma+ that
# partition apparently requires a team-ID (Apple Developer Program)
# signature — our self-signed stint-dev cert doesn't qualify.
#
# This script's workaround: read the existing password, delete the
# entry, re-add it with `-A` (allow all apps). The `-A` flag removes
# the cdhash check entirely. The `security` CLI itself is Apple-signed
# and can read the existing password without a prompt.
#
# Run this AFTER:
#   1. scripts/setup-dev-cert.sh
#   2. The keychain entries exist (e.g. `stint config set
#      solidtime.token <PAT>`, `scripts/dev-cli.sh config login`, or
#      `stint calendar add google`)
#
# Re-run after any new keychain entry is created (e.g. rotating the
# Solidtime PAT, connecting a new calendar account). The Rust keyring
# crate's set_password preserves ACL on update, so refreshes don't
# reset the -A flag.
set -euo pipefail

ACCOUNT="stint"
KEYCHAIN="$HOME/Library/Keychains/login.keychain-db"

SERVICES=(
  "tech.reyem.stint.solidtime.token"
  "tech.reyem.stint.solidtime.oauth"
)

# Discover per-account calendar Keychain entries (Phase 3b). Each row
# in calendar_accounts has a UUID; the matching Keychain entry is
# tech.reyem.stint.calendar.<uuid>. We read the DB rather than scraping
# the keychain itself because security(1) doesn't support globbing.
STINT_DB="$HOME/Library/Application Support/stint/stint.db"
if [[ -f "$STINT_DB" ]] && command -v sqlite3 >/dev/null 2>&1; then
  while IFS= read -r account_uuid; do
    [[ -n "$account_uuid" ]] && SERVICES+=("tech.reyem.stint.calendar.${account_uuid}")
  done < <(sqlite3 "$STINT_DB" "SELECT id FROM calendar_accounts" 2>/dev/null || true)
fi

echo "Recreating tech.reyem.stint.* keychain entries with allow-all-apps"
echo "so dev rebuilds (any cdhash) don't re-prompt for access."
echo

EXIT=0
for svc in "${SERVICES[@]}"; do
  if ! security find-generic-password -s "$svc" -a "$ACCOUNT" "$KEYCHAIN" >/dev/null 2>&1; then
    echo "  - $svc: not in keychain (skipping; create it first if you need it)"
    continue
  fi

  # Read the existing password. The `security` CLI is Apple-signed, so
  # this typically doesn't prompt.
  pass=$(security find-generic-password -s "$svc" -a "$ACCOUNT" -w "$KEYCHAIN" 2>/dev/null) || {
    echo "  ✗ $svc: could not read existing password (skipping)"
    EXIT=1
    continue
  }

  # Delete and re-add with -A (allow any app).
  if ! security delete-generic-password -s "$svc" -a "$ACCOUNT" "$KEYCHAIN" >/dev/null 2>&1; then
    echo "  ✗ $svc: could not delete existing entry"
    EXIT=1
    continue
  fi

  if security add-generic-password \
       -A \
       -s "$svc" \
       -a "$ACCOUNT" \
       -w "$pass" \
       "$KEYCHAIN" >/dev/null 2>&1; then
    echo "  ✓ $svc: recreated with allow-all-apps"
  else
    echo "  ✗ $svc: re-add failed (password not restored — see Keychain Access.app)"
    EXIT=1
  fi
done

echo
if [[ $EXIT -eq 0 ]]; then
  echo "Done. Subsequent dev rebuilds should not re-prompt for these entries."
else
  echo "Some entries failed. Use Keychain Access.app to inspect or recover."
fi
exit $EXIT

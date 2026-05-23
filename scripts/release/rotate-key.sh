#!/usr/bin/env bash
# scripts/release/rotate-key.sh
# Guided credential rotation for the release pipeline.
#
# Each subcommand walks the human through any unavoidable manual steps
# (App Store Connect / Xcode UI) with explicit checkpoints, then delegates
# the actual GitHub-secret upload to bootstrap-secrets.sh, then verifies the
# secret's updatedAt timestamp changed and optionally triggers a smoke
# release.
#
# Subcommands:
#   app-store-connect-key   Rotate the App Store Connect API key used for
#                           notarization. Creating an API key is web-UI-only
#                           (Apple exposes no API for that); everything else
#                           is scripted.
#   apple-cert              Rotate the Developer ID Application cert
#                           (5-year cadence). Xcode generates the cert; the
#                           script handles export, upload, and verify.
#   tauri-key               Rotate the Tauri updater signing key.
#                           DANGEROUS — breaks auto-update for every
#                           existing install until they manually reinstall.
#                           Use only on suspected compromise.

set -euo pipefail

readonly REPO="reyemtech/stint"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly SCRIPT_DIR
readonly BOOTSTRAP="$SCRIPT_DIR/bootstrap-secrets.sh"

# ── helpers ───────────────────────────────────────────────────────────────────

bold()  { printf '\033[1m%s\033[0m\n' "$*"; }
step()  { printf '\n\033[36m── %s ──\033[0m %s\n' "$1" "$2"; }
warn()  { printf '\033[33m! %s\033[0m\n' "$*" >&2; }
ok()    { printf '\033[32m✓ %s\033[0m\n' "$*"; }
abort() { printf '\033[31m✗ %s\033[0m\n' "$*" >&2; exit 1; }

require_tools() {
  command -v gh >/dev/null || abort "gh not installed. brew install gh"
  gh auth status >/dev/null 2>&1 || abort "gh not authenticated. gh auth login"
  [[ -x "$BOOTSTRAP" ]] || abort "$BOOTSTRAP not executable"
}

pause() {
  local msg="${1:-Press ENTER to continue, Ctrl-C to abort}"
  read -r -p "$msg: " _
}

secret_updated_at() {
  gh secret list --repo "$REPO" --json name,updatedAt \
    -q ".[] | select(.name==\"$1\") | .updatedAt" 2>/dev/null || echo ""
}

verify_secret_rotated() {
  local name="$1" before="$2"
  local after
  after=$(secret_updated_at "$name")
  if [[ -z "$after" ]]; then
    abort "$name not found on $REPO after rotation"
  fi
  if [[ "$after" == "$before" ]]; then
    warn "$name updatedAt did not change — secret may not have been re-set"
    return 1
  fi
  ok "$name updated on $REPO ($after)"
}

offer_smoke_release() {
  cat <<EOF

A smoke release confirms the new credential works end-to-end. The fastest
path is to push a tiny chore: commit (no-op CHANGELOG, README typo fix,
etc.) and watch the release workflow.
EOF
  read -r -p "Open the Actions page now? [y/N] " resp
  if [[ "$resp" =~ ^[Yy]$ ]]; then
    open "https://github.com/$REPO/actions/workflows/release.yml" 2>/dev/null \
      || echo "  https://github.com/$REPO/actions/workflows/release.yml"
  fi
}

# ── app-store-connect-key ─────────────────────────────────────────────────────

cmd_app_store_connect_key() {
  bold "Rotate App Store Connect API key (notarization auth)"
  cat <<'EOF'

Rotates APP_STORE_CONNECT_KEY_ID + APP_STORE_CONNECT_ISSUER_ID +
APP_STORE_CONNECT_PRIVATE_KEY in one pass. Used for `xcrun notarytool
--key` authentication.

Trigger: suspected compromise, or "cycling for hygiene every N months."
Blast radius: releases fail at notarization step until the new key is
              in place. Existing shipped releases unaffected (the key
              authenticates submission, not the eventual signature).

EOF
  step 1 "Create a new API key in App Store Connect"
  cat <<'EOF'
  1. Open https://appstoreconnect.apple.com/access/api
  2. Click the "+" next to "Active" (or "Keys" header)
  3. Name: stint-ci-YYYY-MM (or similar — helps identify in rotation history)
  4. Access: Developer  (the minimum role for notarytool to work)
  5. Click "Generate"
  6. Click "Download API Key" — Apple offers the .p8 file exactly once.
     Save it somewhere temporary, e.g. ~/Downloads/AuthKey_XXXXXXXXXX.p8
  7. Note the Key ID (10-char alphanumeric) in the table.
  8. Note the Issuer ID (UUID) at the top of the keys page.

After bootstrap-secrets.sh finishes, REVOKE the previous key on the same
page so an exposed old .p8 stops being usable. The new key remains the
sole active credential.
EOF
  pause "Press ENTER once the .p8 is downloaded and you've noted Key ID + Issuer ID"

  local before_kid before_iid before_key
  before_kid=$(secret_updated_at APP_STORE_CONNECT_KEY_ID)
  before_iid=$(secret_updated_at APP_STORE_CONNECT_ISSUER_ID)
  before_key=$(secret_updated_at APP_STORE_CONNECT_PRIVATE_KEY)

  step 2 "Upload to GitHub secrets"
  "$BOOTSTRAP" APP_STORE_CONNECT_KEY_ID APP_STORE_CONNECT_ISSUER_ID APP_STORE_CONNECT_PRIVATE_KEY

  step 3 "Verify rotation"
  verify_secret_rotated APP_STORE_CONNECT_KEY_ID "$before_kid" || true
  verify_secret_rotated APP_STORE_CONNECT_ISSUER_ID "$before_iid" || true
  verify_secret_rotated APP_STORE_CONNECT_PRIVATE_KEY "$before_key" || true

  step 4 "Revoke the old key in App Store Connect"
  cat <<'EOF'
  Back at https://appstoreconnect.apple.com/access/api, click the old key
  in the Active table → "Revoke Key" → confirm. Revoked keys cannot be
  un-revoked, but if you skip this step and the old .p8 leaks, anyone
  with it can notarize binaries under your team's name until expiry.
EOF
  pause "Press ENTER when the old key is revoked"

  step 5 "Smoke test"
  offer_smoke_release
}

# ── apple-cert ────────────────────────────────────────────────────────────────

cmd_apple_cert() {
  bold "Rotate Developer ID Application cert"
  cat <<'EOF'

Rotates APPLE_CERTIFICATE + APPLE_CERTIFICATE_PASSWORD +
APPLE_SIGNING_IDENTITY in one pass. Used for codesigning the .app + .dmg
+ embedded CLI.

Trigger: annual / 5-year expiry, or revocation.
Blast radius: releases fail at codesign until the new cert is in place.
              Existing shipped releases continue to work — Gatekeeper
              trusts the cert's signature, not its current validity.

EOF
  step 1 "Generate a new cert in Xcode"
  cat <<'EOF'
  1. Open Xcode
  2. Settings (⌘,) → Accounts
  3. Select your Apple Developer account
  4. Click "Manage Certificates…"
  5. Click "+" → "Developer ID Application"
  6. The new cert appears in your login Keychain alongside the old one.
EOF
  pause "Press ENTER when the new cert is in Keychain"

  step 2 "Export cert + private key to .p12"
  cat <<'EOF'
  1. Open Keychain Access
  2. Find the new "Developer ID Application: …" cert
  3. Right-click → Export…
  4. File Format: Personal Information Exchange (.p12)
  5. Save to e.g. /tmp/dev-id-new.p12
  6. Choose a strong export password — bootstrap-secrets.sh will ask for it.
EOF
  pause "Press ENTER when the .p12 is exported"

  step 3 "Note the new Common Name"
  cat <<'EOF'
  In Keychain Access, double-click the new cert. The "Common Name" field
  reads:  Developer ID Application: Your Name (TEAMID)
  Copy it — bootstrap-secrets.sh will ask for it.
EOF

  local before_cert before_pwd before_id
  before_cert=$(secret_updated_at APPLE_CERTIFICATE)
  before_pwd=$(secret_updated_at APPLE_CERTIFICATE_PASSWORD)
  before_id=$(secret_updated_at APPLE_SIGNING_IDENTITY)

  step 4 "Upload all three secrets"
  "$BOOTSTRAP" APPLE_CERTIFICATE APPLE_CERTIFICATE_PASSWORD APPLE_SIGNING_IDENTITY

  step 5 "Verify rotation"
  verify_secret_rotated APPLE_CERTIFICATE "$before_cert" || true
  verify_secret_rotated APPLE_CERTIFICATE_PASSWORD "$before_pwd" || true
  verify_secret_rotated APPLE_SIGNING_IDENTITY "$before_id" || true

  step 6 "Optional: delete the old cert from Keychain"
  cat <<'EOF'
  In Keychain Access, you can delete the old "Developer ID Application"
  cert after the next successful release confirms the new one works.
EOF

  step 7 "Smoke test"
  offer_smoke_release
}

# ── tauri-key ─────────────────────────────────────────────────────────────────

cmd_tauri_key() {
  bold "Rotate TAURI_SIGNING_PRIVATE_KEY (updater signing key)"
  cat <<'EOF'

⚠️  DANGER ⚠️

This rotation BREAKS auto-update for EVERY existing install of stint.
Existing installs verify update manifests against the OLD public key
baked into their binary; the new release's manifest is signed with the
NEW key, which existing installs will REJECT.

Recovery requires every user to manually reinstall:
  brew reinstall --cask stint
  # or
  curl -fsSL https://stint.reyem.tech/install.sh | sh

ONLY rotate this key if you have credible reason to believe the current
key has been compromised. Never rotate proactively.

EOF
  read -r -p "Type 'I understand auto-update will break' to proceed: " confirm
  if [[ "$confirm" != "I understand auto-update will break" ]]; then
    abort "rotation aborted"
  fi

  local before_key before_pwd
  before_key=$(secret_updated_at TAURI_SIGNING_PRIVATE_KEY)
  before_pwd=$(secret_updated_at TAURI_SIGNING_PRIVATE_KEY_PASSWORD)

  step 1 "Generate new key + patch tauri.conf.json"
  warn "bootstrap-secrets.sh will overwrite ~/.tauri/stint.key and edit crates/stint-app/tauri.conf.json"
  "$BOOTSTRAP" TAURI_SIGNING_PRIVATE_KEY TAURI_SIGNING_PRIVATE_KEY_PASSWORD

  step 2 "Verify secrets rotated"
  verify_secret_rotated TAURI_SIGNING_PRIVATE_KEY "$before_key" || true
  verify_secret_rotated TAURI_SIGNING_PRIVATE_KEY_PASSWORD "$before_pwd" || true

  step 3 "Commit + ship the pubkey change"
  cat <<'EOF'
  bootstrap-secrets.sh edited crates/stint-app/tauri.conf.json in place
  with the new public key. Commit + push so the next release embeds it:

    git status crates/stint-app/tauri.conf.json
    git add crates/stint-app/tauri.conf.json
    git commit -m "fix(updater): rotate signing key (incident response)"
    git push

  CI cuts a release with the new pubkey embedded. Existing installs run
  the OLD binary with the OLD pubkey — they will reject this new release.

EOF

  step 4 "Notify users"
  cat <<'EOF'
  After the recovery-key release ships:
    1. Push a notice to https://stint.reyem.tech/recovery.html with
       reinstall instructions.
    2. Update README + release notes prominently.
    3. Existing installs cannot auto-update — they must reinstall manually.

EOF

  step 5 "Do NOT trigger a smoke release until you've committed the pubkey change."
}

# ── dispatcher ────────────────────────────────────────────────────────────────

usage() {
  cat <<EOF
Usage: $(basename "$0") <subcommand>

  app-store-connect-key   Rotate notarization API key
  apple-cert              Rotate Developer ID Application cert
  tauri-key               Rotate Tauri updater signing key (DANGEROUS)
  help                    This message

Each subcommand walks the manual prep with checkpoints, delegates secret
upload to bootstrap-secrets.sh, and verifies the secret's updatedAt
timestamp changed.
EOF
}

main() {
  require_tools
  case "${1:-help}" in
    app-store-connect-key) cmd_app_store_connect_key ;;
    apple-cert)            cmd_apple_cert ;;
    tauri-key)             cmd_tauri_key ;;
    help|-h|--help)        usage ;;
    *) usage; exit 1 ;;
  esac
}

main "$@"

#!/usr/bin/env bash
# scripts/release/bootstrap-secrets.sh
# Interactive walkthrough for setting up the twelve GitHub secrets that the
# release pipeline needs. Idempotent: detects existing secrets and prompts
# before overwriting.
#
# Usage:
#   ./bootstrap-secrets.sh             # set up all secrets
#   ./bootstrap-secrets.sh <NAME>      # rotate a single secret

set -euo pipefail

if (( BASH_VERSINFO[0] < 4 )); then
  echo "error: bash 4+ required (you have $BASH_VERSION)." >&2
  echo "On macOS, install via: brew install bash" >&2
  exit 1
fi

readonly REPO="reyemtech/stint"
readonly SECRETS=(
  APPLE_CERTIFICATE
  APPLE_CERTIFICATE_PASSWORD
  APPLE_SIGNING_IDENTITY
  APPLE_ID
  APPLE_PASSWORD
  APPLE_TEAM_ID
  KEYCHAIN_PASSWORD
  TAURI_SIGNING_PRIVATE_KEY
  TAURI_SIGNING_PRIVATE_KEY_PASSWORD
  HOMEBREW_TAP_TOKEN
  STINT_GOOGLE_CLIENT_ID
  STINT_GOOGLE_CLIENT_SECRET
)

require_gh() {
  if ! command -v gh >/dev/null; then
    echo "error: gh CLI not installed. brew install gh" >&2
    exit 1
  fi
  if ! gh auth status >/dev/null 2>&1; then
    echo "error: gh not authenticated. Run: gh auth login" >&2
    exit 1
  fi
}

require_tauri() {
  if ! command -v cargo >/dev/null || ! cargo tauri --version >/dev/null 2>&1; then
    echo "warn: cargo-tauri not found; TAURI_SIGNING_PRIVATE_KEY prompts will fail." >&2
    echo "      Install via: cargo install tauri-cli --version '^2.0' --locked" >&2
    echo "      Continuing — skip the TAURI_* secrets if not needed yet." >&2
  fi
}

secret_exists() {
  gh secret list --repo "$REPO" --json name -q '.[].name' | grep -qx "$1"
}

confirm_overwrite() {
  local name="$1"
  if secret_exists "$name"; then
    read -r -p "Secret $name already exists. Overwrite? [y/N] " resp
    [[ "${resp:-N}" =~ ^[Yy]$ ]]
  fi
}

set_secret() {
  local name="$1" value="$2"
  printf '%s' "$value" | gh secret set "$name" --repo "$REPO"
  echo "✓ $name set"
}

prompt_apple_certificate() {
  read -r -e -p "Path to developer-id.p12: " p12_path
  [[ -f "$p12_path" ]] || { echo "error: file not found" >&2; return 1; }
  set_secret APPLE_CERTIFICATE "$(base64 < "$p12_path")"
  read -r -p "Securely delete $p12_path now? [y/N] " resp
  if [[ "$resp" =~ ^[Yy]$ ]]; then
    rm -P "$p12_path" 2>/dev/null || rm -f "$p12_path"
    echo "✓ removed $p12_path"
  else
    echo "  reminder: $p12_path contains private key material; delete manually when done"
  fi
}

prompt_apple_certificate_password() {
  read -r -s -p "p12 export password: " pwd; echo
  set_secret APPLE_CERTIFICATE_PASSWORD "$pwd"
}

prompt_apple_signing_identity() {
  cat <<'TIP'
The signing identity string is the certificate's Common Name. In Keychain
Access, double-click the Developer ID Application cert and copy the
"Common Name" field. Format: Developer ID Application: Name (TEAMID)
TIP
  read -r -p "Signing identity: " identity
  [[ "$identity" =~ ^Developer\ ID\ Application: ]] || {
    echo "error: must start with 'Developer ID Application:'" >&2
    return 1
  }
  set_secret APPLE_SIGNING_IDENTITY "$identity"
}

prompt_apple_id() {
  read -r -p "Apple ID email: " email
  set_secret APPLE_ID "$email"
}

prompt_apple_password() {
  cat <<'TIP'
Create an app-specific password at appleid.apple.com → Sign-In and Security
→ App-Specific Passwords. Apple shows it once; do not lose it.
TIP
  read -r -s -p "App-specific password: " pwd; echo
  set_secret APPLE_PASSWORD "$pwd"
}

prompt_apple_team_id() {
  read -r -p "Apple Team ID (10 chars): " team
  [[ "$team" =~ ^[A-Z0-9]{10}$ ]] || {
    echo "error: must be 10 uppercase alphanumerics" >&2
    return 1
  }
  set_secret APPLE_TEAM_ID "$team"
}

prompt_keychain_password() {
  set_secret KEYCHAIN_PASSWORD "$(openssl rand -base64 24)"
  echo "  (auto-generated 24-byte random string)"
}

substitute_tauri_pubkey() {
  local pubkey_path="$1"
  local conf_path="crates/stint-app/tauri.conf.json"
  if [[ ! -f "$conf_path" ]]; then
    echo "warn: $conf_path not found; skipping in-file substitution" >&2
    echo "      manually paste the contents of $pubkey_path into plugins.updater.pubkey" >&2
    return 0
  fi
  if [[ ! -f "$pubkey_path" ]]; then
    echo "error: pubkey file $pubkey_path not found" >&2
    return 1
  fi
  # The pubkey is a base64-ish blob on one or more lines; collapse to one.
  local pubkey
  pubkey=$(tr -d '\n' < "$pubkey_path")
  if [[ -z "$pubkey" ]]; then
    echo "error: pubkey file is empty" >&2
    return 1
  fi
  # The placeholder is unique enough that a plain sed substitution is safe.
  # Use a sentinel delimiter (`|`) to avoid escaping slashes in the pubkey.
  local tmp
  tmp=$(mktemp)
  sed "s|REPLACE_ME_WITH_OUTPUT_OF_TAURI_SIGNER_GENERATE|${pubkey}|" "$conf_path" > "$tmp"
  mv "$tmp" "$conf_path"
  if grep -q 'REPLACE_ME_WITH_OUTPUT_OF_TAURI_SIGNER_GENERATE' "$conf_path"; then
    echo "warn: placeholder still present in $conf_path — substitution may have failed" >&2
  else
    echo "✓ substituted pubkey into $conf_path"
  fi
}

prompt_tauri_signing_private_key() {
  if [[ -f "$HOME/.tauri/stint.key" ]]; then
    read -r -p "Existing key at ~/.tauri/stint.key — regenerate? [y/N] " resp
    if [[ "$resp" =~ ^[Yy]$ ]]; then
      cargo tauri signer generate -w "$HOME/.tauri/stint.key" --force
      substitute_tauri_pubkey "$HOME/.tauri/stint.key.pub"
    fi
  else
    echo "Generating Tauri updater key pair…"
    mkdir -p "$HOME/.tauri"
    cargo tauri signer generate -w "$HOME/.tauri/stint.key"
    substitute_tauri_pubkey "$HOME/.tauri/stint.key.pub"
  fi
  set_secret TAURI_SIGNING_PRIVATE_KEY "$(base64 < "$HOME/.tauri/stint.key")"
}

prompt_tauri_signing_private_key_password() {
  read -r -s -p "Tauri key passphrase (chosen during generate): " pwd; echo
  set_secret TAURI_SIGNING_PRIVATE_KEY_PASSWORD "$pwd"
}

prompt_homebrew_tap_token() {
  cat <<'TIP'
Create a fine-grained PAT at github.com/settings/personal-access-tokens/new:
  - Resource owner: reyemtech
  - Repository access: select reyemtech/homebrew-tap only
  - Permissions: Contents (RW), Pull requests (RW), Metadata (RO)
TIP
  read -r -s -p "PAT: " pat; echo
  set_secret HOMEBREW_TAP_TOKEN "$pat"
}

prompt_stint_google_client_id() {
  read -r -p "Google OAuth client ID (same as .env.local for now): " cid
  set_secret STINT_GOOGLE_CLIENT_ID "$cid"
}

prompt_stint_google_client_secret() {
  read -r -s -p "Google OAuth client secret: " csecret; echo
  set_secret STINT_GOOGLE_CLIENT_SECRET "$csecret"
}

print_manual_steps() {
  cat <<'EOF'

Done with secrets. Remaining manual steps:

  □ Create the public repo github.com/reyemtech/homebrew-tap (Task 6.1).
  □ Add DNS record: stint.reyem.tech CNAME reyemtech.github.io (Task 5.4).
  □ Commit the tauri.conf.json pubkey substitution if you rotated TAURI_SIGNING_PRIVATE_KEY
    (bootstrap-secrets.sh substitutes in-place; you still need to commit the change).

EOF
}

main() {
  require_gh
  require_tauri
  local targets=()
  if (( $# == 0 )); then
    targets=("${SECRETS[@]}")
  else
    targets=("$@")
  fi
  for name in "${targets[@]}"; do
    local fn="prompt_${name,,}"
    if declare -F "$fn" >/dev/null; then
      confirm_overwrite "$name" || { echo "skipped $name"; continue; }
      "$fn"
    else
      echo "warn: no prompt function for $name; skipping" >&2
    fi
  done
  print_manual_steps
}

main "$@"

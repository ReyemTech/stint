#!/usr/bin/env bash
# scripts/release/notarize.sh
# Submit a signed artifact to Apple's notary service and staple the ticket.
# Retries up to 3 times on transient (5xx, timeout) failures; bails immediately
# on authentication or content failures.
#
# Usage: notarize.sh <signed-artifact.{app,dmg}>
#
# Required env:
#   APP_STORE_CONNECT_KEY_ID       — 10-char alphanumeric key ID
#   APP_STORE_CONNECT_ISSUER_ID    — UUID of the App Store Connect team
#   APP_STORE_CONNECT_PRIVATE_KEY  — base64-encoded contents of the .p8 file
#
# Authenticates via App Store Connect API key (rather than the legacy
# app-specific-password mode) so credentials can be rotated without humans
# generating new passwords at appleid.apple.com.

set -euo pipefail

readonly ARTIFACT="${1:?signed artifact path required}"
readonly MAX_ATTEMPTS=3

: "${APP_STORE_CONNECT_KEY_ID:?must be set}"
: "${APP_STORE_CONNECT_ISSUER_ID:?must be set}"
: "${APP_STORE_CONNECT_PRIVATE_KEY:?must be set}"

# Materialize the .p8 to a temp file — notarytool only accepts a path, not a
# string. Clean up on any exit.
KEY_FILE="$(mktemp -t notary-key.XXXXXX.p8)"
chmod 600 "$KEY_FILE"
echo "$APP_STORE_CONNECT_PRIVATE_KEY" | base64 -d > "$KEY_FILE"
trap 'rm -f "$KEY_FILE"' EXIT

# notarytool wants a zip for .app submissions.
SUBMIT_PATH="$ARTIFACT"
case "$ARTIFACT" in
  *.app)
    SUBMIT_PATH="${ARTIFACT}.zip"
    ditto -c -k --keepParent "$ARTIFACT" "$SUBMIT_PATH"
    ;;
esac

for attempt in $(seq 1 $MAX_ATTEMPTS); do
  echo "→ notarize attempt $attempt/$MAX_ATTEMPTS"
  set +e
  output=$(xcrun notarytool submit "$SUBMIT_PATH" \
    --key "$KEY_FILE" \
    --key-id "$APP_STORE_CONNECT_KEY_ID" \
    --issuer "$APP_STORE_CONNECT_ISSUER_ID" \
    --wait \
    --output-format json 2>&1)
  rc=$?
  set -e

  echo "$output"

  if [[ $rc -eq 0 ]]; then
    # Defensive parse: notarytool's --output-format json emits the final
    # JSON object at the end of stdout, but progress lines can interleave
    # depending on buffering. Pull the LAST {...} block containing a
    # "status" field rather than relying on tail-line position.
    parsed=$(python3 - <<'PY' "$output"
import json, re, sys
text = sys.argv[1]
# Find balanced single-level JSON objects (no nesting in notarytool output).
matches = re.findall(r'\{[^{}]*"status"[^{}]*\}', text, re.DOTALL)
if not matches:
    sys.exit("no JSON object containing 'status' found")
obj = json.loads(matches[-1])
print(obj.get("status", ""))
print(obj.get("id", ""))
PY
)
    status=$(echo "$parsed" | sed -n '1p')
    submission_id=$(echo "$parsed" | sed -n '2p')
    if [[ "$status" == "Accepted" ]]; then
      echo "✓ notarized"
      xcrun stapler staple "$ARTIFACT"
      [[ "$SUBMIT_PATH" != "$ARTIFACT" ]] && rm -f "$SUBMIT_PATH"
      exit 0
    fi
    echo "error: notarization status: $status" >&2
    if [[ -n "$submission_id" ]]; then
      xcrun notarytool log "$submission_id" \
        --key "$KEY_FILE" \
        --key-id "$APP_STORE_CONNECT_KEY_ID" \
        --issuer "$APP_STORE_CONNECT_ISSUER_ID"
    fi
    exit 1
  fi

  # Transient? Retry. Permanent (auth, format)? Bail.
  if echo "$output" | grep -qE '(timeout|temporar|5[0-9][0-9])'; then
    echo "→ transient failure; retrying in 30s" >&2
    sleep 30
    continue
  fi
  echo "error: notarytool failed (non-retryable)" >&2
  exit 1
done

echo "error: notarization failed after $MAX_ATTEMPTS attempts" >&2
exit 1

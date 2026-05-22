#!/usr/bin/env bash
# scripts/release/notarize.sh
# Submit a signed artifact to Apple's notary service and staple the ticket.
# Retries up to 3 times on transient (5xx, timeout) failures; bails immediately
# on authentication or content failures.
#
# Usage: notarize.sh <signed-artifact.{app,dmg}>
#
# Required env: APPLE_ID, APPLE_PASSWORD, APPLE_TEAM_ID

set -euo pipefail

readonly ARTIFACT="${1:?signed artifact path required}"
readonly MAX_ATTEMPTS=3

: "${APPLE_ID:?must be set}"
: "${APPLE_PASSWORD:?must be set}"
: "${APPLE_TEAM_ID:?must be set}"

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
    --apple-id "$APPLE_ID" \
    --password "$APPLE_PASSWORD" \
    --team-id "$APPLE_TEAM_ID" \
    --wait \
    --output-format json 2>&1)
  rc=$?
  set -e

  echo "$output"

  if [[ $rc -eq 0 ]]; then
    status=$(echo "$output" | python3 -c 'import json,sys; print(json.loads(sys.stdin.read().splitlines()[-1])["status"])')
    if [[ "$status" == "Accepted" ]]; then
      echo "✓ notarized"
      xcrun stapler staple "$ARTIFACT"
      [[ "$SUBMIT_PATH" != "$ARTIFACT" ]] && rm -f "$SUBMIT_PATH"
      exit 0
    fi
    echo "error: notarization status: $status" >&2
    submission_id=$(echo "$output" | python3 -c 'import json,sys; print(json.loads(sys.stdin.read().splitlines()[-1])["id"])')
    xcrun notarytool log "$submission_id" \
      --apple-id "$APPLE_ID" --password "$APPLE_PASSWORD" --team-id "$APPLE_TEAM_ID"
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

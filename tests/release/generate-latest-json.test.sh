#!/usr/bin/env bash
# tests/release/generate-latest-json.test.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
readonly REPO_ROOT
SANDBOX="$(mktemp -d)"
readonly SANDBOX
trap 'rm -rf "$SANDBOX"' EXIT

# Fake a signature file and an app bundle tarball.
mkdir -p "$SANDBOX/artifacts"
echo "FAKE_SIGNATURE_BASE64" > "$SANDBOX/artifacts/Stint.app.tar.gz.sig"
touch "$SANDBOX/artifacts/Stint.app.tar.gz"

cd "$SANDBOX"
"$REPO_ROOT/scripts/release/generate-latest-json.sh" \
  --version "0.1.0" \
  --signature-path "artifacts/Stint.app.tar.gz.sig" \
  --bundle-url "https://example.com/Stint.app.tar.gz" \
  --notes "see CHANGELOG.md" \
  --output "artifacts/latest.json"

# Validate JSON shape.
jq -e '.version == "0.1.0"' artifacts/latest.json >/dev/null || { echo "FAIL: version"; exit 1; }
jq -e '.platforms["darwin-x86_64"].signature == "FAKE_SIGNATURE_BASE64"' artifacts/latest.json >/dev/null || { echo "FAIL: x86 signature"; exit 1; }
jq -e '.platforms["darwin-aarch64"].url == "https://example.com/Stint.app.tar.gz"' artifacts/latest.json >/dev/null || { echo "FAIL: aarch64 url"; exit 1; }
jq -e '.pub_date' artifacts/latest.json >/dev/null || { echo "FAIL: missing pub_date"; exit 1; }

echo PASS

#!/usr/bin/env bash
# tests/release/update-cask.test.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
readonly REPO_ROOT
SANDBOX="$(mktemp -d)"
readonly SANDBOX
trap 'rm -rf "$SANDBOX"' EXIT

cat > "$SANDBOX/stint.rb" <<'EOF'
cask "stint" do
  version "0.0.0"
  sha256 "0000000000000000000000000000000000000000000000000000000000000000"
  url "https://example.com/Stint-#{version}.dmg"
  name "Stint"
end
EOF

"$REPO_ROOT/scripts/release/update-cask.sh" "$SANDBOX/stint.rb" "1.2.3" "abc123def456abc123def456abc123def456abc123def456abc123def456abc1"

grep -q 'version "1.2.3"' "$SANDBOX/stint.rb" || { echo FAIL: version; exit 1; }
grep -q 'sha256 "abc123def456abc123def456abc123def456abc123def456abc123def456abc1"' "$SANDBOX/stint.rb" || { echo FAIL: sha256; exit 1; }
echo PASS

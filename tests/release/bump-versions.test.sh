#!/usr/bin/env bash
# tests/release/bump-versions.test.sh
# Verify bump-versions.sh updates all target files to the given version.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
readonly REPO_ROOT
SANDBOX="$(mktemp -d)"
readonly SANDBOX
trap 'rm -rf "$SANDBOX"' EXIT

# Mirror the files bump-versions.sh touches into a sandbox.
mkdir -p "$SANDBOX/crates/stint-app" "$SANDBOX/ui"
cat > "$SANDBOX/Cargo.toml" <<'EOF'
[workspace]
members = ["crates/*"]

[workspace.package]
version = "0.0.0"
edition = "2021"
EOF
cat > "$SANDBOX/crates/stint-app/tauri.conf.json" <<'EOF'
{ "version": "0.0.0", "productName": "Stint" }
EOF
cat > "$SANDBOX/ui/package.json" <<'EOF'
{ "name": "stint-ui", "version": "0.0.0", "private": true }
EOF

# Run the script against the sandbox.
cd "$SANDBOX"
bash "$REPO_ROOT/scripts/release/bump-versions.sh" "1.2.3"

# Assert each file was updated.
fail=0
grep -q 'version = "1.2.3"' "$SANDBOX/Cargo.toml"               || { echo "FAIL: Cargo.toml not bumped"; fail=1; }
grep -q '"version": "1.2.3"' "$SANDBOX/crates/stint-app/tauri.conf.json" || { echo "FAIL: tauri.conf.json not bumped"; fail=1; }
grep -q '"version": "1.2.3"' "$SANDBOX/ui/package.json"          || { echo "FAIL: ui/package.json not bumped"; fail=1; }

[[ $fail -eq 0 ]] && echo "PASS"
exit $fail

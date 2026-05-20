#!/usr/bin/env bash
# Build, codesign with stint-dev, and launch the Stint.app GUI binary.
# The GUI equivalent of scripts/dev-cli.sh — replaces `cargo tauri dev`
# for dev work where you want macOS Keychain "Always Allow" to persist
# across rebuilds.
#
# Why this script exists:
#   `cargo tauri dev` rebuilds target/debug/stint-app on every change and
#   produces a binary with only the linker's ad-hoc signature. macOS
#   Keychain ACL is bound to the binary cdhash, and the partition-list
#   relaxation (scripts/relax-keychain-acl.sh) only honours signatures
#   from a real code-signing identity. Result: every rebuild re-prompts.
#   This wrapper builds, then codesigns with the stable stint-dev cert
#   before launching — partition check passes, Always-Allow persists.
#
# Trade-off:
#   Drops Tauri's Rust HMR. For Rust changes, Ctrl+C and re-run. UI HMR
#   via Vite continues to work (this script starts Vite in the background
#   if it isn't already running).
#
# First-time setup (one-time, in order):
#   1. scripts/setup-dev-cert.sh        — creates the stint-dev cert
#   2. scripts/dev-cli.sh config login  — or `stint config set solidtime.token`
#                                          to populate the Keychain entries
#   3. scripts/relax-keychain-acl.sh    — relaxes ACL on those entries
#
# Usage: scripts/dev-app.sh

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

# Load build-time secrets (STINT_GOOGLE_CLIENT_ID + SECRET) so option_env!
# in stint-core picks them up. The .env.local file is gitignored; copy
# .env.local.example to get started.
if [[ -f .env.local ]]; then
  # shellcheck disable=SC1091
  source .env.local
else
  echo "Warning: .env.local not found. Google OAuth will be disabled in this build."
  echo "         See .env.local.example to enable it."
fi

if ! security find-identity -v -p codesigning | grep -q '"stint-dev"'; then
  echo "stint-dev code-signing identity not found." >&2
  echo "Run scripts/setup-dev-cert.sh once to create it." >&2
  exit 1
fi

UI_BG_PID=""
cleanup() {
  if [[ -n "$UI_BG_PID" ]]; then
    kill "$UI_BG_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT INT TERM

if ! lsof -i :5173 -sTCP:LISTEN >/dev/null 2>&1; then
  echo "Starting UI dev server on :5173..."
  pnpm --filter stint-ui dev >/tmp/stint-ui-dev.log 2>&1 &
  UI_BG_PID=$!
  printf "  waiting for :5173"
  for _ in $(seq 1 50); do
    if curl -s -o /dev/null http://localhost:5173 2>/dev/null; then
      echo " ready."
      break
    fi
    printf "."
    sleep 0.2
  done
else
  echo "UI dev server already running on :5173 (using existing instance)."
fi

cargo build -p stint-app --quiet
codesign -f -s stint-dev target/debug/stint-app 2>/dev/null

# Re-apply allow-all-apps ACL to stint Keychain entries before launch.
# Idempotent and cheap. Covers the case where a previous stint-app run
# (or any keyring::Entry::set_password call) reset the ACL.
if [[ -x scripts/relax-keychain-acl.sh ]]; then
  scripts/relax-keychain-acl.sh || true
fi

echo "Launching stint-app (signed by stint-dev)..."
target/debug/stint-app

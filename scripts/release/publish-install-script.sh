#!/usr/bin/env bash
# scripts/release/publish-install-script.sh
# Push the rendered install.sh + install.sh.sha256 to the docs-pages branch.

set -euo pipefail

readonly ARTIFACTS_DIR="${ARTIFACTS_DIR:-target/release-artifacts}"

[[ -f "$ARTIFACTS_DIR/install.sh" ]] || { echo "error: install.sh missing" >&2; exit 1; }

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

git config --global user.email "release@reyem.tech"
git config --global user.name  "stint-release-bot"

git clone --branch docs-pages --depth 1 "https://x-access-token:${GITHUB_TOKEN}@github.com/reyemtech/stint.git" "$WORK"

cp "$ARTIFACTS_DIR/install.sh" "$WORK/install.sh"
cp "$ARTIFACTS_DIR/install.sh.sha256" "$WORK/install.sh.sha256"

cd "$WORK"
git add install.sh install.sh.sha256
if git diff --staged --quiet; then
  echo "→ install.sh unchanged; nothing to publish"
  exit 0
fi
git commit -m "chore(release): publish install.sh"
git push origin docs-pages

echo "✓ install.sh published"

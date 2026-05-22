#!/usr/bin/env bash
# scripts/release/push-tap-pr.sh
# Open a PR (auto-merge) against reyemtech/homebrew-tap with the new version.
#
# Usage: push-tap-pr.sh <version> <channel>
# Env:   GH_TOKEN     — fine-grained PAT scoped to reyemtech/homebrew-tap
#        ARTIFACTS_DIR — directory containing the signed/notarized DMG
#                       (default: target/release-artifacts)
#        STINT_REPO    — absolute path to the stint repo checkout
#                       (default: current working directory)

set -euo pipefail

readonly VERSION="${1:?version required}"
readonly CHANNEL="${2:?channel required (stable|beta)}"
readonly ARTIFACTS_DIR="${ARTIFACTS_DIR:-target/release-artifacts}"

# Capture an absolute anchor to the stint repo BEFORE we `cd` into the
# tap clone. The plan's original `../../stint/...` relative path doesn't
# resolve from a mktemp dir.
STINT_REPO="${STINT_REPO:-$(pwd)}"
readonly STINT_REPO
readonly UPDATE_CASK="$STINT_REPO/scripts/release/update-cask.sh"

[[ -x "$UPDATE_CASK" ]] || {
  echo "error: $UPDATE_CASK not found or not executable" >&2
  exit 1
}

CASK_NAME="stint"
DMG_GLOB="Stint-*.dmg"
if [[ "$CHANNEL" == "beta" ]]; then
  CASK_NAME="stint-beta"
  DMG_GLOB="Stint-*.dmg"
fi
readonly CASK_NAME
readonly DMG_GLOB

# Resolve the DMG without relying on `ls` (SC2010). The glob expands
# inside the array literal; if nothing matches, the literal pattern
# survives and the [[ -f ]] check below fails cleanly.
DMG_PATH=""
for candidate in "$ARTIFACTS_DIR"/$DMG_GLOB; do
  DMG_PATH="$candidate"
  break
done

[[ -f "$DMG_PATH" ]] || {
  echo "error: no DMG in $ARTIFACTS_DIR matching $DMG_GLOB" >&2
  exit 1
}
readonly DMG_PATH

SHA="$(shasum -a 256 "$DMG_PATH" | awk '{print $1}')"
readonly SHA

WORK="$(mktemp -d)"
readonly WORK
trap 'rm -rf "$WORK"' EXIT

git config --global user.email "release@reyem.tech"
git config --global user.name  "stint-release-bot"

git clone --depth 1 \
  "https://x-access-token:${GH_TOKEN}@github.com/reyemtech/homebrew-tap.git" \
  "$WORK"
cd "$WORK"

BRANCH="update-${CASK_NAME}-${VERSION//[^A-Za-z0-9.-]/-}"
readonly BRANCH
git checkout -b "$BRANCH"

"$UPDATE_CASK" "Casks/${CASK_NAME}.rb" "$VERSION" "$SHA"

git add "Casks/${CASK_NAME}.rb"
git commit -m "feat: ${CASK_NAME} ${VERSION}"
git push origin "$BRANCH"

PR_URL="$(gh pr create \
  --title "${CASK_NAME} ${VERSION}" \
  --body "Automated bump from stint release v${VERSION}. Auto-merged after brew audit passes." \
  --head "$BRANCH" --base main)"

gh pr merge --auto --squash "$PR_URL"
echo "✓ opened + auto-merge: $PR_URL"

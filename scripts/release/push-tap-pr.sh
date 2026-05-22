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

git clone --depth 1 \
  "https://x-access-token:${GH_TOKEN}@github.com/reyemtech/homebrew-tap.git" \
  "$WORK"
cd "$WORK"
# Scope identity to this clone — avoid leaking into a shared runner config.
git config user.email "release@reyem.tech"
git config user.name  "stint-release-bot"

BRANCH="update-${CASK_NAME}-${VERSION//[^A-Za-z0-9.-]/-}"
readonly BRANCH

# Idempotence: if a stale branch exists on the tap remote from a prior failed
# run (e.g., post-merge cleanup didn't fire, or this is a re-run), delete it
# first. Otherwise `git push` rejects with "remote contains work". Same idea
# for any existing PR — close it before opening a fresh one.
if gh api "/repos/reyemtech/homebrew-tap/branches/$BRANCH" >/dev/null 2>&1; then
  echo "→ stale remote branch $BRANCH detected; deleting"
  gh api -X DELETE "/repos/reyemtech/homebrew-tap/git/refs/heads/$BRANCH" || true
fi
existing_pr="$(gh pr list --repo reyemtech/homebrew-tap --head "$BRANCH" --state open --json number -q '.[0].number')"
if [[ -n "$existing_pr" ]]; then
  echo "→ closing stale PR #$existing_pr"
  gh pr close "$existing_pr" --repo reyemtech/homebrew-tap || true
fi

git checkout -b "$BRANCH"

"$UPDATE_CASK" "Casks/${CASK_NAME}.rb" "$VERSION" "$SHA"

git add "Casks/${CASK_NAME}.rb"
git commit -m "feat: ${CASK_NAME} ${VERSION}"
git push origin "$BRANCH"

PR_URL="$(gh pr create \
  --title "${CASK_NAME} ${VERSION}" \
  --body "Automated bump from stint release v${VERSION}. Auto-merged after brew audit passes." \
  --head "$BRANCH" --base main)"

# Try auto-merge first (waits for required checks). If the PR is already
# in clean status (no required checks configured, or all already green),
# gh refuses `--auto` with "Pull request is in clean status" because
# there is nothing to wait on — in that case merge directly so the tap
# doesn't sit on an open PR that will never auto-resolve.
MERGE_LOG="$(mktemp)"
if gh pr merge --auto --squash "$PR_URL" >"$MERGE_LOG" 2>&1; then
  cat "$MERGE_LOG"
  echo "✓ opened + auto-merge enabled: $PR_URL"
elif grep -qi "clean status" "$MERGE_LOG"; then
  cat "$MERGE_LOG"
  echo "→ auto-merge n/a (PR already mergeable); merging directly"
  gh pr merge --squash "$PR_URL"
  echo "✓ opened + merged: $PR_URL"
else
  cat "$MERGE_LOG" >&2
  echo "error: auto-merge failed for $PR_URL" >&2
  exit 1
fi
rm -f "$MERGE_LOG"

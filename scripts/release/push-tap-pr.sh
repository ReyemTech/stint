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

# Idempotence: close every open tap-update PR for this cask, regardless of
# version. A previous release that failed mid-publish (e.g., auto-merge
# rejected) can leave a stale PR pointing at the old version; without this
# cleanup that PR lingers indefinitely and confuses tap reviewers. Each
# release supersedes any pending update — we always want exactly zero open
# PRs before opening this run's PR.
mapfile -t stale_prs < <(gh pr list --repo reyemtech/homebrew-tap \
  --state open --json number,headRefName \
  --jq ".[] | select(.headRefName | startswith(\"update-${CASK_NAME}-\")) | .number")
for pr in "${stale_prs[@]:-}"; do
  [[ -n "$pr" ]] || continue
  echo "→ closing stale tap-update PR #$pr"
  gh pr close "$pr" --delete-branch --repo reyemtech/homebrew-tap || true
done

# Catch any orphan branches that have no open PR (PR closed without branch
# deletion, or branch pushed without PR ever opened). Same prefix scope.
mapfile -t stale_branches < <(gh api "/repos/reyemtech/homebrew-tap/branches?per_page=100" \
  --jq ".[] | select(.name | startswith(\"update-${CASK_NAME}-\")) | .name")
for br in "${stale_branches[@]:-}"; do
  [[ -n "$br" ]] || continue
  echo "→ deleting orphan tap-update branch $br"
  gh api -X DELETE "/repos/reyemtech/homebrew-tap/git/refs/heads/$br" || true
done

git checkout -b "$BRANCH"

"$UPDATE_CASK" "Casks/${CASK_NAME}.rb" "$VERSION" "$SHA"

git add "Casks/${CASK_NAME}.rb"
git commit -m "feat: ${CASK_NAME} ${VERSION}"
git push origin "$BRANCH"

PR_URL="$(gh pr create \
  --title "${CASK_NAME} ${VERSION}" \
  --body "Automated bump from stint release v${VERSION}. Auto-merged after brew audit passes." \
  --head "$BRANCH" --base main)"

# Try auto-merge first (waits for required checks). `gh pr merge --auto`
# can refuse for several reasons that all reduce to "auto-merge is
# unnecessary or unavailable — merge directly instead":
#
#   - "Pull request is in clean status"  (no required checks pending)
#   - "Protected branch rules not configured"  (tap repo has no rules at all)
#   - "Auto-merge is not allowed for this repository"  (org-level setting off)
#
# In all three cases the right move is a direct `gh pr merge --squash`; the
# PR is otherwise mergeable, just not eligible for the auto-merge queue.
# Any other failure is real and we should bubble it up.
MERGE_LOG="$(mktemp)"
if gh pr merge --auto --squash "$PR_URL" >"$MERGE_LOG" 2>&1; then
  cat "$MERGE_LOG"
  echo "✓ opened + auto-merge enabled: $PR_URL"
elif grep -qiE 'clean status|Protected branch rules not configured|Auto-merge is not allowed' "$MERGE_LOG"; then
  cat "$MERGE_LOG"
  echo "→ auto-merge n/a; merging directly"
  gh pr merge --squash "$PR_URL"
  echo "✓ opened + merged: $PR_URL"
else
  cat "$MERGE_LOG" >&2
  echo "error: auto-merge failed for $PR_URL" >&2
  exit 1
fi
rm -f "$MERGE_LOG"

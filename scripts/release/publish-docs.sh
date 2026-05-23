#!/usr/bin/env bash
# scripts/release/publish-docs.sh
# Push the Starlight build at site/dist/ to the docs-pages branch,
# preserving install.sh + install.sh.sha256 + CNAME (which are owned by
# publish-install-script.sh and the manual one-time DNS setup).
#
# Race condition: this script and publish-install-script.sh both push to
# the same branch. Their writes target disjoint file sets, so a non-fast-
# forward push retries cleanly. Retry once before giving up.

set -euo pipefail

readonly SITE_DIST="${SITE_DIST:-site/dist}"
readonly REPO="reyemtech/stint"

[[ -d "$SITE_DIST" ]] || { echo "error: $SITE_DIST not found — run \`pnpm --filter stint-docs build\` first" >&2; exit 1; }
[[ -f "$SITE_DIST/index.html" ]] || { echo "error: $SITE_DIST/index.html missing — build incomplete" >&2; exit 1; }
[[ -n "${GITHUB_TOKEN:-}" ]] || { echo "error: GITHUB_TOKEN required" >&2; exit 1; }

# Files / directories owned by other deploy scripts or by manual setup.
# publish-docs.sh must not touch these — they live alongside the
# Starlight output at the repo root of docs-pages.
#
# Critically includes `.github/` — the docs-pages branch has its own
# `.github/workflows/deploy-pages.yml` which is what actually deploys
# pushed content to GitHub Pages. Wiping it kills the deploy trigger
# silently (the push to docs-pages succeeds, but Pages never updates).
readonly -a PRESERVE=(
  "install.sh"
  "install.sh.sha256"
  "CNAME"
  ".github"
)

push_attempt() {
  local work
  work="$(mktemp -d)"
  trap 'rm -rf "$work"' RETURN

  git clone --branch docs-pages --depth 1 \
    "https://x-access-token:${GITHUB_TOKEN}@github.com/${REPO}.git" "$work"

  cd "$work"
  # Identity scoped to this clone so it doesn't leak into any shared
  # runner-level git config.
  git config user.email "release@reyem.tech"
  git config user.name  "stint-release-bot"

  # Stash the files / dirs we're not allowed to touch. -a preserves
  # mode and recurses into directories (cp -p alone fails on dirs).
  local stash
  stash="$(mktemp -d)"
  for f in "${PRESERVE[@]}"; do
    [[ -e "$f" ]] && cp -a "$f" "$stash/"
  done

  # Wipe everything *except* .git/, then restore the preserved entries
  # and copy the fresh Starlight output on top. Avoids stale-file
  # accumulation from prior deploys.
  find . -mindepth 1 -maxdepth 1 ! -name ".git" -exec rm -rf {} +
  cp -R "$OLDPWD/$SITE_DIST"/. .
  for f in "${PRESERVE[@]}"; do
    [[ -e "$stash/$f" ]] && cp -a "$stash/$f" "$f"
  done
  rm -rf "$stash"

  git add -A
  if git diff --staged --quiet; then
    echo "→ no doc changes; nothing to publish"
    return 0
  fi
  git commit -m "chore(docs): publish Starlight site"
  git push origin docs-pages
}

if push_attempt; then
  echo "✓ docs published to docs-pages"
  exit 0
fi

echo "→ push failed (likely race with publish-install-script.sh); retrying once"
sleep 5
push_attempt
echo "✓ docs published to docs-pages (on retry)"

#!/usr/bin/env bash
# scripts/release/bump-versions.sh
# Bump version strings across the workspace. Called by @semantic-release/exec
# during the prepare phase.
#
# Usage: bump-versions.sh <version>

set -euo pipefail

readonly VERSION="${1:?version required, e.g. 1.2.3}"

if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$ ]]; then
  echo "error: invalid semver: $VERSION" >&2
  exit 1
fi

echo "→ bumping workspace to $VERSION"

# Cargo.toml — replace [workspace.package].version
python3 - "$VERSION" <<'PY'
import re, sys, pathlib
version = sys.argv[1]
p = pathlib.Path("Cargo.toml")
text = p.read_text()
new = re.sub(
    r'(\[workspace\.package\][^\[]*?\nversion\s*=\s*")[^"]+(")',
    rf'\g<1>{version}\g<2>',
    text,
    count=1,
    flags=re.DOTALL,
)
if new == text:
    raise SystemExit("error: Cargo.toml [workspace.package].version not found")
p.write_text(new)
PY

# tauri.conf.json — top-level version
python3 - "$VERSION" <<'PY'
import json, sys, pathlib
version = sys.argv[1]
p = pathlib.Path("crates/stint-app/tauri.conf.json")
obj = json.loads(p.read_text())
obj["version"] = version
p.write_text(json.dumps(obj, indent=2) + "\n")
PY

# ui/package.json — top-level version
python3 - "$VERSION" <<'PY'
import json, sys, pathlib
version = sys.argv[1]
p = pathlib.Path("ui/package.json")
obj = json.loads(p.read_text())
obj["version"] = version
p.write_text(json.dumps(obj, indent=2) + "\n")
PY

# Re-lockfile Cargo and pnpm to pick up the new versions.
if [[ -f "Cargo.lock" ]]; then
  cargo update -w --offline >/dev/null 2>&1 || cargo update -w
fi
if [[ -f "ui/pnpm-lock.yaml" ]]; then
  ( cd ui && pnpm install --lockfile-only --silent )
fi

echo "✓ bumped to $VERSION"

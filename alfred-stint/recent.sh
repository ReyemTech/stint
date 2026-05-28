#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"

BIN="$(resolve_bin)" || {
  echo '{"items":[{"title":"Stint binary not found","valid":false}]}'
  exit 0
}

JSON="$("$BIN" --json list --limit 20 2>/dev/null || echo "[]")"
python3 - <<PY
import json
items = []
for e in json.loads('''$JSON'''):
    items.append({
        "uid": e["local_uuid"],
        "title": e.get("description","(no description)"),
        "subtitle": e.get("start_at",""),
        "arg": e["local_uuid"],
        "mods": {
            "alt": {"arg": f"stint://entry/{e['local_uuid']}", "subtitle": "Open in Stint"},
        }
    })
print(json.dumps({"items": items}))
PY

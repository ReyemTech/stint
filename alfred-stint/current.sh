#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"

BIN="$(resolve_bin)" || {
  cat <<EOF
{"items":[{"title":"Stint binary not found","subtitle":"Set STINT_BIN in workflow env","valid":false}]}
EOF
  exit 0
}

JSON="$("$BIN" --json current 2>/dev/null || echo "null")"
if [[ "$JSON" == "null" ]] || [[ -z "$JSON" ]]; then
  echo '{"items":[{"title":"No active timer","valid":false}]}'
  exit 0
fi
python3 - <<PY
import json, sys
e = json.loads('''$JSON''')
print(json.dumps({"items":[{
    "uid": e["local_uuid"],
    "title": e.get("description","(no description)"),
    "subtitle": "Open in Stint",
    "arg": f"stint://entry/{e['local_uuid']}",
}]}))
PY

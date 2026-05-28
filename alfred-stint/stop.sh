#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"

BIN="$(resolve_bin)" || { echo "Stint binary not found"; exit 1; }

ENTRY="$("$BIN" --json stop)"
DESC="$(echo "$ENTRY" | python3 -c 'import sys,json; print(json.load(sys.stdin).get("description",""))')"
echo "Stopped: $DESC"

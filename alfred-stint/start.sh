#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"

DESC="${1:?usage: start.sh <description>}"
BIN="$(resolve_bin)" || { echo "Stint binary not found"; exit 1; }

"$BIN" --json start --description "$DESC" | head -1

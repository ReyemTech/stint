#!/usr/bin/env bash
# Shared helpers for Stint Alfred workflow scripts.

resolve_bin() {
  if [[ -n "$STINT_BIN" ]] && [[ -x "$STINT_BIN" ]]; then
    echo "$STINT_BIN"
    return
  fi
  if command -v stint >/dev/null 2>&1; then
    command -v stint
    return
  fi
  for candidate in "$HOME/.cargo/bin/stint" "/Applications/Stint.app/Contents/MacOS/stint"; do
    [[ -x "$candidate" ]] && { echo "$candidate"; return; }
  done
  return 1
}

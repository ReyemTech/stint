#!/usr/bin/env bash
# Unified workspace coverage across all surfaces (core, cli, app, ui).
#
# Runs cargo-llvm-cov for the Rust crates and vitest coverage for the
# SolidJS UI, then prints a per-surface summary table. Exits non-zero if
# any surface is below COVERAGE_FLOOR (default 80%).
#
# Local quirk: Homebrew's rust keg ships rustlib without llvm-profdata, and
# cargo-llvm-cov resolves the tool path from rustc's sysroot regardless of
# PATH. So when running outside CI we redirect to the rustup-managed
# toolchain (which has llvm-tools-preview installed). CI uses dtolnay/
# rust-toolchain + llvm-tools-preview, so the override is a no-op there.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

TOOLCHAIN_DIR="$HOME/.rustup/toolchains/1.95.0-aarch64-apple-darwin"
if [[ -z "${CI:-}" ]] && [[ -d "$TOOLCHAIN_DIR" ]]; then
  export PATH="$TOOLCHAIN_DIR/bin:$PATH"
  export RUSTC="$TOOLCHAIN_DIR/bin/rustc"
fi

OUT_DIR="${COVERAGE_OUT_DIR:-target/coverage}"
mkdir -p "$OUT_DIR"

FLOOR="${COVERAGE_FLOOR:-80}"
SKIP_UI="${SKIP_UI:-0}"
SKIP_RUST="${SKIP_RUST:-0}"

# ───────────────────────────────────────────────────────────────── Rust ──

if [[ "$SKIP_RUST" != "1" ]]; then
  # Exclude un-instrumentable bin/runtime wiring from the coverage scope:
  #   * tests/                 — test code shouldn't count against coverage
  #   * stint-app/src/main.rs  — Tauri bin entrypoint (only runs at app start)
  #   * menu.rs / tray.rs /    — Tauri runtime closures wired into the system
  #     windows.rs / logging.rs   menu / dock / tray icon / global logger
  #   * app_state.rs           — Tauri-managed state struct (constructed in main)
  #   * sync_worker.rs /       — async background loops spawned at startup;
  #     pull_worker.rs /          covered indirectly by commands::sync, but the
  #     calendar_worker.rs        spawn/select! plumbing isn't unit-testable
  #   * commands/ui.rs         — window/dock visibility shims that delegate to
  #                              Tauri APIs requiring a real WebviewWindow
  #   * updater.rs /           — Tauri-updater plugin wrapper; needs a signed
  #     updater_endpoint.rs       build + remote release server to exercise
  #   * idle_detector.rs       — CGEventSource-backed polling task + tokio
  #                              spawn loop. The pure state machine (advance)
  #                              IS verified by tests/idle_detector.rs, but
  #                              the polling side isn't unit-testable without
  #                              a live AppHandle.
  # stint-app excludes: Tauri runtime wiring (main, menu, tray, workers, etc.)
  # exercises native macOS APIs and the Tauri event loop — not unit-testable.
  APP_RE='stint-app/src/(main|menu|tray|windows|logging|app_state|sync_worker|pull_worker|calendar_worker|updater|updater_endpoint|idle_detector)\.rs|stint-app/src/commands/ui\.rs'
  # stint-cli excludes: subprocess- and OAuth-bound surfaces.
  #   * cmd/mcp.rs / mcp/mod.rs  — `stint mcp` runs as a subprocess; covered
  #                                by tests/mcp_e2e.rs but the child-process
  #                                profile data isn't merged with the parent
  #   * skill/picker.rs          — interactive dialoguer prompt; jsdom-style
  #                                stdin simulation isn't worth the wiring
  #   * cmd/calendar.rs          — drives real Google / Microsoft / CalDAV
  #                                OAuth from the terminal (loopback server)
  #   * cmd/config_login.rs      — Solidtime OAuth loopback flow; same as above
  #   * cmd/update.rs            — downloads + verifies + applies release
  #                                binaries; requires a signed release server
  CLI_RE='stint-cli/src/(cmd/(mcp|calendar|config_login|update)|mcp/mod|skill/picker)\.rs'
  IGNORE_RE="tests/|$APP_RE|$CLI_RE"

  echo "── Running Rust coverage (cargo-llvm-cov)..."
  cargo llvm-cov clean --workspace
  cargo llvm-cov --no-report --workspace --all-targets \
    --ignore-filename-regex "$IGNORE_RE" \
    -- --test-threads=1

  cargo llvm-cov report --summary-only --ignore-filename-regex "$IGNORE_RE"
  cargo llvm-cov report --lcov --output-path "$OUT_DIR/lcov.info" \
    --ignore-filename-regex "$IGNORE_RE"
fi

# ─────────────────────────────────────────────────────────────────── UI ──

if [[ "$SKIP_UI" != "1" ]]; then
  echo ""
  echo "── Running UI coverage (vitest)..."
  (cd ui && pnpm test:coverage --reporter=default --silent 2>&1) | tail -5
fi

# ───────────────────────────────────────────── Unified per-surface table ──

echo ""
echo "── Unified coverage report"
echo ""

# Parse lcov.info → per-crate totals (lines + functions)
awk -v lcov="$OUT_DIR/lcov.info" '
  function classify(p) {
    if (p ~ /crates\/stint-core\//) return "stint-core"
    if (p ~ /crates\/stint-cli\//)  return "stint-cli"
    if (p ~ /crates\/stint-app\//)  return "stint-app"
    return "other"
  }
  BEGIN {
    cur = ""
    while ((getline line < lcov) > 0) {
      if (line ~ /^SF:/)       { cur = classify(substr(line, 4)) }
      else if (line ~ /^LF:/)  { lf[cur] += substr(line, 4) }
      else if (line ~ /^LH:/)  { lh[cur] += substr(line, 4) }
      else if (line ~ /^FNF:/) { fnf[cur] += substr(line, 5) }
      else if (line ~ /^FNH:/) { fnh[cur] += substr(line, 5) }
    }
    for (k in lh) print k, lh[k], lf[k], fnh[k], fnf[k]
  }
' > "$OUT_DIR/per-crate.tsv"

# Extract UI totals from coverage-summary.json via node (jq not guaranteed)
UI_TSV="$OUT_DIR/ui-totals.tsv"
node -e "
const s = require('./ui/coverage/coverage-summary.json').total;
process.stdout.write([
  'ui',
  s.lines.covered, s.lines.total,
  s.functions.covered, s.functions.total
].join(' ') + '\\n');
" > "$UI_TSV" 2>/dev/null || echo "ui 0 0 0 0" > "$UI_TSV"

# Emit unified table + threshold gate
awk -v floor="$FLOOR" '
  function pct(h, f) { return f > 0 ? 100 * h / f : 0 }
  function bar(p,    s, i, n) {
    s = ""; n = int(p / 5)
    for (i = 0; i < 20; i++) s = s (i < n ? "█" : "░")
    return s
  }
  {
    lh[$1] = $2; lf[$1] = $3; fnh[$1] = $4; fnf[$1] = $5
  }
  END {
    order[1] = "stint-core"; order[2] = "stint-cli"
    order[3] = "stint-app";  order[4] = "ui"
    fail = 0

    printf "  %-11s  %-22s  %-9s  %s\n", "surface", "lines (covered/total)", "functions", "status"
    printf "  %-11s  %-22s  %-9s  %s\n", "-------", "---------------------", "---------", "------"
    for (i = 1; i <= 4; i++) {
      k = order[i]
      if (lf[k] == 0) continue
      lp = pct(lh[k], lf[k]); fp = pct(fnh[k], fnf[k])
      st = (lp >= floor) ? "✅" : "❌"
      if (lp < floor) fail = 1
      tlh += lh[k]; tlf += lf[k]; tfnh += fnh[k]; tfnf += fnf[k]
      printf "  %-11s  %5.1f%%  (%5d/%5d)   %5.1f%%    %s %s\n", k, lp, lh[k], lf[k], fp, bar(lp), st
    }
    tlp = pct(tlh, tlf); tfp = pct(tfnh, tfnf)
    printf "  %-11s  %-22s  %-9s\n", "", "", ""
    printf "  %-11s  %5.1f%%  (%5d/%5d)   %5.1f%%    %s\n", "TOTAL", tlp, tlh, tlf, tfp, bar(tlp)
    if (tlp < floor) fail = 1
    printf "\n  Threshold: %d%% per surface (override via COVERAGE_FLOOR env)\n", floor
    exit fail
  }
' "$OUT_DIR/per-crate.tsv" "$UI_TSV"

status=$?
echo ""
if [[ $status -eq 0 ]]; then
  echo "✅ All surfaces ≥ ${FLOOR}% line coverage."
else
  echo "❌ One or more surfaces below ${FLOOR}% — see table above."
fi
echo "── reports: $OUT_DIR/lcov.info, ui/coverage/coverage-summary.json"
exit $status

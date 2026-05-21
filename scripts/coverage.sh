#!/usr/bin/env bash
# Workspace coverage via cargo-llvm-cov.
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

# Two-step pattern: collect raw profdata once, then format reports from it.
# Cheaper than re-running tests per output format.
cargo llvm-cov clean --workspace
cargo llvm-cov --no-report --workspace --all-targets \
  --ignore-filename-regex 'tests/' \
  -- --test-threads=1

cargo llvm-cov report --summary-only
cargo llvm-cov report --lcov --output-path "$OUT_DIR/lcov.info"

# cargo-llvm-cov's TOTAL row under-counts when both lib and bin crates are
# in scope (only the lib-test target shows up in the totals line). Compute
# the accurate workspace totals from the lcov file we just wrote.
awk '
  /^LF:/ { lf += substr($0, 4) }
  /^LH:/ { lh += substr($0, 4) }
  /^FNF:/ { fnf += substr($0, 5) }
  /^FNH:/ { fnh += substr($0, 5) }
  /^BRF:/ { brf += substr($0, 5) }
  /^BRH:/ { brh += substr($0, 5) }
  END {
    printf "\n── workspace totals (from lcov.info):\n"
    if (lf  > 0) printf "  lines:     %6.2f%%  (%d / %d)\n", 100*lh/lf,   lh,  lf
    if (fnf > 0) printf "  functions: %6.2f%%  (%d / %d)\n", 100*fnh/fnf, fnh, fnf
    if (brf > 0) printf "  branches:  %6.2f%%  (%d / %d)\n", 100*brh/brf, brh, brf
  }
' "$OUT_DIR/lcov.info"

echo "── coverage written to $OUT_DIR/lcov.info"

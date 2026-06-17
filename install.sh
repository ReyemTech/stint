#!/bin/sh
# install.sh — generated for stint vSTINT_VERSION at release time.
# DO NOT EDIT manually. Source: scripts/install.sh.tpl.

set -eu

STINT_VERSION="0.5.1"
STINT_TARBALL_SHA256="5a8acb6ddaa0e4eb9229ed884f8b271388b9f1809573bdec866b90d6a49d7888"
STINT_DMG_SHA256="936313779a6ca6611a1ff8cec75fdce19a278e7d7dbc68550454ae59fd88417b"
STINT_RELEASE_URL="https://github.com/reyemtech/stint/releases/download/v${STINT_VERSION}"

# ── helpers ───────────────────────────────────────────────────────────────────

bold()    { printf '\033[1m%s\033[0m\n' "$1"; }
warn()    { printf '\033[33m! %s\033[0m\n' "$1" >&2; }
err()     { printf '\033[31m✗ %s\033[0m\n' "$1" >&2; exit 1; }
ok()      { printf '\033[32m✓ %s\033[0m\n' "$1"; }

require() {
  command -v "$1" >/dev/null 2>&1 || err "required command not found: $1"
}

verify_sha256() {
  _vs_file="$1"
  _vs_expected="$2"
  _vs_actual="$(shasum -a 256 "$_vs_file" | awk '{print $1}')"
  if [ "$_vs_actual" != "$_vs_expected" ]; then
    err "checksum mismatch on $_vs_file: expected $_vs_expected, got $_vs_actual"
  fi
}

# ── argument parsing ──────────────────────────────────────────────────────────

INSTALL_GUI=0
UNINSTALL=0
FORCE=0
VERSION_OVERRIDE=""

while [ $# -gt 0 ]; do
  case "$1" in
    --gui)       INSTALL_GUI=1 ;;
    --uninstall) UNINSTALL=1 ;;
    --force)     FORCE=1 ;;
    --version)
      [ $# -ge 2 ] || err "--version requires a value (e.g. --version v0.1.2)"
      VERSION_OVERRIDE="$2"; shift
      ;;
    --help)
      cat <<'HELP'
Usage: install.sh [options]
  --gui                Install the GUI (Stint.app) in addition to the CLI.
  --uninstall          Remove stint from this machine (CLI; prompts for GUI).
  --force              Skip overwrite prompts (required for non-TTY).
  --version vX.Y.Z     Pin to a specific version (default: latest stable).
  --help               This message.
HELP
      exit 0
      ;;
    *) err "unknown argument: $1" ;;
  esac
  shift
done

# Non-TTY safety: refuse interactive prompts.
if [ ! -t 0 ] && [ "$FORCE" -eq 0 ] && { [ "$UNINSTALL" -eq 1 ] || [ "$INSTALL_GUI" -eq 1 ]; }; then
  warn "stdin is not a TTY; pass --force to skip interactive prompts."
  FORCE=1
fi

# ── platform check ────────────────────────────────────────────────────────────

[ "$(uname -s)" = "Darwin" ] || err "stint installer only supports macOS."
ARCH="$(uname -m)"
case "$ARCH" in
  arm64|x86_64) ;;
  *) err "unsupported architecture: $ARCH" ;;
esac

# Stint.app requires macOS 13 (Ventura) or later. Fail fast before any
# download — without this guard the DMG installs and launch then fails with
# kLSIncompatibleSystemVersionErr, which is harder to diagnose.
if [ "$INSTALL_GUI" -eq 1 ]; then
  MACOS_VER="$(sw_vers -productVersion 2>/dev/null || echo "")"
  MACOS_MAJOR="${MACOS_VER%%.*}"
  if [ -n "$MACOS_MAJOR" ] && [ "$MACOS_MAJOR" -lt 13 ] 2>/dev/null; then
    err "Stint.app requires macOS 13 (Ventura) or later — detected macOS $MACOS_VER. Rerun without --gui to install just the CLI."
  fi
fi

require curl
require shasum

# ── version pinning ───────────────────────────────────────────────────────────

if [ -n "$VERSION_OVERRIDE" ]; then
  STINT_VERSION="${VERSION_OVERRIDE#v}"
  STINT_RELEASE_URL="https://github.com/reyemtech/stint/releases/download/v${STINT_VERSION}"
  warn "overriding to v${STINT_VERSION}; checksum baked into this script may not match."
  warn "for tamper-evidence, install with curl ... | sh (no --version) and that version's bundled checksums."
  STINT_TARBALL_SHA256=""   # disable checksum verify for explicit version override
  STINT_DMG_SHA256=""
fi

# ── uninstall path ────────────────────────────────────────────────────────────

if [ "$UNINSTALL" -eq 1 ]; then
  bold "stint uninstaller"
  for prefix in "${STINT_INSTALL_DIR:-}" /usr/local/bin "$HOME/.local/bin"; do
    [ -n "$prefix" ] && [ -x "$prefix/stint" ] || continue
    rm -f "$prefix/stint" && ok "removed $prefix/stint"
  done
  if [ -d "/Applications/Stint.app" ]; then
    if [ "$FORCE" -eq 1 ]; then
      rm -rf "/Applications/Stint.app"
      ok "removed /Applications/Stint.app"
    else
      printf 'Remove /Applications/Stint.app? [y/N] '
      read -r resp
      case "$resp" in y|Y) rm -rf "/Applications/Stint.app"; ok "removed Stint.app" ;; esac
    fi
  fi
  cat <<'EOF'

User data was NOT removed. To delete it manually:
  rm -rf ~/Library/Application\ Support/stint
  security delete-generic-password -s tech.reyem.stint.solidtime.token
  security delete-generic-password -s tech.reyem.stint.solidtime.oauth
  # Calendar accounts each have their own Keychain entry:
  security find-generic-password -s tech.reyem.stint.calendar
  # (one for each Google/MS/CalDAV account that was added)

EOF
  exit 0
fi

# ── CLI install ───────────────────────────────────────────────────────────────

bold "stint v${STINT_VERSION} installer"

INSTALL_DIR="${STINT_INSTALL_DIR:-}"
if [ -z "$INSTALL_DIR" ]; then
  if [ -w "/usr/local/bin" ]; then
    INSTALL_DIR="/usr/local/bin"
  else
    INSTALL_DIR="$HOME/.local/bin"
    mkdir -p "$INSTALL_DIR"
  fi
fi

TARBALL="stint-${STINT_VERSION}-universal-apple-darwin.tar.gz"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

echo "→ downloading $TARBALL"
curl -fsSL "${STINT_RELEASE_URL}/${TARBALL}" -o "$TMPDIR/$TARBALL"

if [ -n "$STINT_TARBALL_SHA256" ]; then
  verify_sha256 "$TMPDIR/$TARBALL" "$STINT_TARBALL_SHA256"
  ok "checksum verified"
fi

tar -xzf "$TMPDIR/$TARBALL" -C "$TMPDIR"
chmod +x "$TMPDIR/stint"
xattr -dr com.apple.quarantine "$TMPDIR/stint" 2>/dev/null || true

mv "$TMPDIR/stint" "$INSTALL_DIR/stint"
ok "installed $INSTALL_DIR/stint"

if ! "$INSTALL_DIR/stint" --version >/dev/null 2>&1; then
  err "installed binary failed to run; check codesign and quarantine."
fi

case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *)
    warn "$INSTALL_DIR is not in your PATH. Add to ~/.zshrc or ~/.bashrc:"
    # shellcheck disable=SC2016 # $PATH is intended to appear literally in the printed shell snippet.
    printf '  export PATH="%s:$PATH"\n' "$INSTALL_DIR"
    ;;
esac

# ── GUI install (optional) ────────────────────────────────────────────────────

if [ "$INSTALL_GUI" -eq 1 ]; then
  if [ -e "/Applications/Stint.app" ] && [ "$FORCE" -eq 0 ]; then
    printf '/Applications/Stint.app exists. Overwrite? [y/N] '
    read -r resp
    case "$resp" in y|Y) ;; *) warn "GUI install skipped"; exit 0 ;; esac
  fi

  DMG="Stint-${STINT_VERSION}.dmg"
  echo "→ downloading $DMG"
  curl -fsSL "${STINT_RELEASE_URL}/${DMG}" -o "$TMPDIR/$DMG"
  if [ -n "$STINT_DMG_SHA256" ]; then
    verify_sha256 "$TMPDIR/$DMG" "$STINT_DMG_SHA256"
    ok "dmg checksum verified"
  fi

  # `hdiutil attach -quiet` suppresses ALL stdout including the volume path; omit -quiet so we can parse.
  MOUNT="$(hdiutil attach -nobrowse "$TMPDIR/$DMG" 2>/dev/null | grep -o '/Volumes/[^[:cntrl:]]*' | tail -1)"
  [ -n "$MOUNT" ] && [ -d "$MOUNT" ] || err "failed to mount $DMG"
  trap 'hdiutil detach "$MOUNT" -quiet 2>/dev/null || true; rm -rf "$TMPDIR"' EXIT
  cp -R "$MOUNT/Stint.app" /Applications/
  xattr -dr com.apple.quarantine /Applications/Stint.app 2>/dev/null || true
  hdiutil detach "$MOUNT" -quiet
  echo "→ launching Stint.app in background (menu-bar icon will appear)"
  open -g /Applications/Stint.app
  ok "installed /Applications/Stint.app"
fi

bold "done. run: stint --help"

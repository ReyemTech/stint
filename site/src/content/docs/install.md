---
title: Install
description: Install stint via Homebrew, direct DMG download, or the curl|sh installer. macOS 13+ required for the GUI.
---

:::note[macOS 13 (Ventura) or later required]
The Stint.app GUI requires macOS 13. The CLI may run on macOS 12 in practice
but only macOS 13+ is officially supported. All channels are signed and
notarized — no Gatekeeper warnings on a fresh install.
:::

stint ships through four channels. Pick whichever fits your workflow.

## Homebrew (recommended)

```bash
brew tap reyemtech/tap
brew install --cask stint
```

Installs `Stint.app` to `/Applications` and symlinks the CLI at
`/opt/homebrew/bin/stint` (Apple Silicon) or `/usr/local/bin/stint` (Intel).
Auto-updates happen via Homebrew (`brew upgrade --cask stint`) *and* via the
in-app updater — both keep you current; whichever runs first wins.

## Direct DMG download

Grab the latest `.dmg` from the
[GitHub releases page](https://github.com/reyemtech/stint/releases/latest) and
drag `Stint.app` into `/Applications`.

The DMG includes only the GUI; for the CLI use the Homebrew cask or the
`curl | sh` installer below.

## `curl | sh` — CLI only

```bash
curl -fsSL https://stint.reyem.tech/install.sh | sh
```

Installs the standalone `stint` binary to `/usr/local/bin/stint` (if
writable) or `~/.local/bin/stint`. The standalone CLI self-updates via
`stint update`.

## `curl | sh` — CLI + GUI

```bash
curl -fsSL https://stint.reyem.tech/install.sh | sh -s -- --gui
```

Installs the CLI plus copies `Stint.app` to `/Applications`. The script
verifies macOS 13+ before downloading the DMG so older systems fail fast
with a clear message.

### Pinning to a specific version

```bash
curl -fsSL https://stint.reyem.tech/install.sh | sh -s -- --version v0.1.5
```

Useful for reproducing an issue against a known release.

## Uninstall

```bash
# CLI (any install method):
curl -fsSL https://stint.reyem.tech/install.sh | sh -s -- --uninstall

# GUI:
rm -rf /Applications/Stint.app
```

User data (timer database, Solidtime credentials, calendar OAuth tokens)
is preserved on uninstall. To wipe it:

```bash
rm -rf ~/Library/Application\ Support/stint
security delete-generic-password -s tech.reyem.stint.solidtime.token
security delete-generic-password -s tech.reyem.stint.solidtime.oauth
# Each calendar account has its own Keychain entry:
security find-generic-password -s tech.reyem.stint.calendar
```

## Next steps

- [Quickstart](/getting-started/quickstart/) — start your first timer
- [Solidtime setup](/getting-started/solidtime/) — connect to your Solidtime instance
- [Calendar setup](/getting-started/calendar/) — surface meetings as one-click entries

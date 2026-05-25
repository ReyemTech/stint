---
title: Quickstart
description: Connect stint to your Solidtime instance and start your first timer in five minutes.
---

Five minutes from `brew install` to a synced first entry.

## 1. Install

```bash
brew tap reyemtech/tap
brew install --cask stint
```

Or pick another method from the [Install](/install/) page.

## 2. Connect to Solidtime

```bash
stint config set solidtime.url https://your-solidtime-host.example.com
stint config set solidtime.token
# stint prompts; paste your Personal Access Token. Stored in macOS Keychain.
stint config set solidtime.org <organization-uuid>
stint config test
# → "✓ connected as you@example.com to <Your Org>"
```

The GUI's Settings → Solidtime panel does the same thing with a UI. PATs
are the quickest path; OAuth (requires a one-time Passport client
registration on your Solidtime host) is documented under
[Solidtime setup](/getting-started/solidtime/).

## 3. Start a timer

In the terminal:

```bash
stint start "writing the Phase 5 docs"
```

Or in the GUI:

1. Click the menu-bar icon to open the popover
2. Type a description, optionally pick a project
3. Press **Start timer**

Both surfaces write to the same database. Started in the terminal, you'll
see it in the popover within a second.

## 4. Stop and sync

```bash
stint stop
stint today    # see the day's entries
stint sync     # force sync now (otherwise drains every 30s in the background)
```

The entry appears in Solidtime under the configured organization.

## What's next

- [Calendar setup](/getting-started/calendar/) — log meetings as time entries with one click
- [Keyboard shortcuts](/reference/shortcuts/) — ⌘1 / ⌘2 / ⌘3 for routes, Esc to dismiss the popover
- [CLI commands](/reference/cli/) — complete command reference
- [Scripting stint](/scripting/) — `--json` output, the loopback HTTP API, and `stint://` URLs for Raycast / Alfred / shell pipelines
- [AI integration](/ai-integration/) — wire stint into Claude Code, Codex, or OpenCode with one `stint skill install`

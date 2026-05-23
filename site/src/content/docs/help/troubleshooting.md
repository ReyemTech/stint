---
title: Troubleshooting
description: Solutions for common stint issues — sync failures, OAuth problems, update flow.
---

## "Update available" appeared but clicking Install did nothing

Fixed in v0.2.0. Earlier versions had a single "Install & restart" button
that did install the new bundle to `/Applications` but failed to relaunch
the running process, leaving you on the old version silently.

If you're still on a pre-v0.2.0 install: **Quit stint from the menu bar
and reopen it.** The new bundle is already on disk; relaunching picks it
up. From v0.2.0 onward the panel has separate **Install** and
**Restart Stint** buttons, and either restarting via the button or
quit-and-reopen works.

## Sync failures and the SyncErrorBanner

stint shows a banner at the top of the main window when an entry failed
to sync. Common causes:

- **422 Unprocessable Entity** — usually a missing `member_id`. Open
  Settings → Solidtime, pick your organization from the dropdown, and
  the `member_id` is auto-backfilled. Re-trigger sync via tray menu's
  "Sync now".
- **401 Unauthorized** — your PAT was revoked or your OAuth refresh
  failed. Re-authenticate via Settings → Solidtime.
- **4xx other** — stint abandons the entry after a few retries to avoid
  retry storms. To retry manually: `stint sync retry-abandoned`.
- **5xx / network** — auto-retried with exponential backoff. The banner
  clears when the entry eventually syncs.

The banner shows the conflicting entry in Solidtime via a deep link when
possible.

## Google Calendar shows "OAuth not configured"

You're running a build of stint (likely a local fork) where
`STINT_GOOGLE_CLIENT_ID` and `STINT_GOOGLE_CLIENT_SECRET` weren't set at
compile time. See [Calendar setup → For forks](/getting-started/calendar/#for-forks-registering-your-own-google-oauth-client).

## stint doesn't appear in macOS Spotlight after installing

This is almost always a macOS Spotlight issue, not a stint issue. New
apps installed via brew, DMG drag-and-drop, or auto-update sometimes
skip Spotlight's normal indexing trigger.

Check whether Spotlight knows about it:

```bash
mdfind "kMDItemCFBundleIdentifier == 'tech.reyem.stint'"
```

If empty, force a re-import:

```bash
mdimport /Applications/Stint.app
```

If that still doesn't work, your Spotlight index may be backed up. A
common culprit is a heavily-syncing folder (Dropbox, OneDrive) saturating
`mds`'s queue. Exclude such folders from Spotlight via System Settings →
Spotlight → Search Privacy.

Nuclear option: rebuild the entire root volume's Spotlight index.

```bash
sudo mdutil -E /System/Volumes/Data
```

Takes 10–30 minutes in the background.

## Keychain prompts on every launch (development only)

If you're running stint from source via `cargo build` or `cargo tauri
dev`, macOS re-prompts for Keychain access on every rebuild because
clicking "Always Allow" stores the binary's exact cdhash and rebuilds
produce a new cdhash.

The fix is documented in the repo's `CLAUDE.md`:

```bash
scripts/setup-dev-cert.sh        # one-time: create a stable self-signed cert
scripts/dev-cli.sh <subcommand>  # use instead of `cargo run -p stint-cli`
scripts/dev-app.sh               # use instead of `cargo tauri dev`
scripts/relax-keychain-acl.sh    # after the keychain entries exist
```

Released builds (signed with a Developer ID cert) don't have this
problem.

## "Apple cannot check it for malicious software" on first launch

This shouldn't happen for stint — all release builds are notarized. If
you see it:

1. Check you downloaded from a trusted source
   (`stint.reyem.tech/install.sh`, `github.com/reyemtech/stint/releases`,
   or `brew install --cask reyemtech/tap/stint`).
2. Check the binary's signature:

   ```bash
   codesign --verify --strict --verbose=2 /Applications/Stint.app
   spctl --assess --type execute --verbose /Applications/Stint.app
   ```

   Both should say "accepted".

3. If you genuinely downloaded a tampered build, reinstall via brew (the
   cask is verified by Homebrew's tap).

## "stint update" says "no update available" when there clearly is one

Likely one of:

- The auto-update channel mismatches (Settings → Updates → Channel). The
  stable channel doesn't see prereleases by design.
- Your network blocks `github.com/reyemtech/stint/releases/...` — corporate
  proxies sometimes do this.
- The release was published less than 30 seconds ago and you're hitting a
  CDN cache; retry shortly.

## Where to find logs

```
~/Library/Logs/tech.reyem.stint/stint.log.<date>
```

Rolling daily. Look here when something silently misbehaves — useful for
investigating sync issues, OAuth flow failures, or updater problems.

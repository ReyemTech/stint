# Runbook — Release rollback

When a stable release ships broken code, follow this in order.

## 1. Confirm the regression

- Reproduce the bug locally. Don't roll back on rumor.
- Identify the last-known-good version. Check `git log v0.1.5...HEAD` to see
  what changed between it and the current latest.

## 2. Stop the bleeding (updater rollback)

```bash
gh workflow run release-revert.yml -f version=0.1.5
gh run watch
```

This replaces `latest.json` on the current latest stable Release with v0.1.5's
manifest. Auto-update users get rolled back on their next check (~24h
default).

## 3. Annotate the bad release

The workflow auto-annotates the bad release's notes with a "reverted" banner.
Verify on github.com:

    https://github.com/reyemtech/stint/releases/latest

## 4. Repair affected users

Cut a forward-fix release as soon as the root cause is understood. Auto-update
users get it on their next check.

Brew users have to wait until they `brew upgrade` (or until tauri-updater
picks up the new release inside the app).

## 5. Catastrophic case (updater itself broken)

If the auto-updater in shipped builds can't apply updates at all:

1. Pin both casks to the last-known-good version (manual PR to
   reyemtech/homebrew-tap).
2. Update `latest.json` to the last-known-good (the rollback workflow above).
3. Push a `recovery.html` to docs-pages explaining the manual reinstall path:
   `brew reinstall --cask stint` or rerun the install script.
4. Communicate via README + GitHub Releases page.

## 6. Post-mortem

Required for every reverted release. Lives at
`docs/incidents/YYYY-MM-DD-vX.Y.Z.md`. Minimum content:

- What broke (symptoms, scope).
- How we found out.
- Who was affected.
- Root cause.
- Fix.
- Guardrail added to prevent recurrence (test, CI gate, code-review rule).

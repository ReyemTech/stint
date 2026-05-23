---
title: Solidtime setup
description: Connect stint to a Solidtime instance via Personal Access Token or OAuth.
---

stint requires a [Solidtime](https://www.solidtime.io) instance to sync with.
Two authentication modes are supported.

## Option 1 — Personal Access Token (quickest)

Best for: quick setup, single-user instances.

1. In Solidtime: **Profile → API Tokens → New token**. Name it `stint`. Copy
   the token — Solidtime shows it once.
2. In stint:

   ```bash
   stint config set solidtime.url https://your-solidtime-host.example.com
   stint config set solidtime.token
   # paste token at the prompt; stored in macOS Keychain
   stint config set solidtime.org <organization-uuid>
   stint config test
   ```

   Or use the GUI: **Settings → Solidtime → Personal Access Token** with the
   "Use PAT" toggle on.

PATs don't expire automatically. Rotate them by repeating step 1 with a new
token; the old token can be revoked from the Solidtime UI.

## Option 2 — OAuth (recommended for multi-user / shared installs)

Best for: anyone managing multiple stint installs or running a hosted
Solidtime that other users connect to.

### One-time server setup

stint uses Solidtime's Laravel Passport OAuth server. A public client must
exist on your Solidtime host before any stint install can use OAuth:

```bash
# On the Solidtime host:
php artisan passport:client \
  --public \
  --name=stint \
  --redirect_uri=http://127.0.0.1/callback
```

Passport prints a `Client ID`. Hold onto it — every stint install
connecting to this Solidtime needs it.

### Per-install setup

1. In stint Settings → Solidtime, toggle to **OAuth**.
2. Paste the `Client ID` from the server setup.
3. Click **Sign in with Solidtime**. A browser tab opens, you authorize,
   the tab redirects back to stint via the local callback. Access and
   refresh tokens are stored in the macOS Keychain.
4. Pick your organization from the dropdown. stint backfills your
   `member_id` from the membership list automatically (required for time
   entry writes).
5. Click **Test connection** — should show your user + org.

OAuth tokens refresh automatically. If a refresh fails (revoked,
network), you'll see a banner prompting re-authentication.

## Switching between PAT and OAuth

Both can coexist in the Keychain. The `solidtime.auth_mode` setting picks
which is active. You can switch modes any time without losing the other
credential.

## Where credentials live

| Item | Location |
|---|---|
| `solidtime.url`, `solidtime.org`, `solidtime.member_id` | SQLite settings table |
| PAT | macOS Keychain — `tech.reyem.stint.solidtime.token` |
| OAuth tokens (access, refresh, expiry, client_id) | macOS Keychain — `tech.reyem.stint.solidtime.oauth` (single JSON blob) |

## Verifying

```bash
stint config show              # non-secret values
stint config test              # round-trip API call
```

The GUI's Settings → Solidtime panel shows the same status with a green/red
indicator.

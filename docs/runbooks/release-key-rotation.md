# Runbook — Release credential rotation

The release pipeline holds four rotatable credentials. All four are rotated
via `scripts/release/rotate-key.sh <subcommand>`, which walks any
unavoidable manual steps (App Store Connect / Xcode UI) with explicit
checkpoints, then handles the GitHub-secret upload and verification.

This document is the why-and-when reference; the script is the how.

| Credential | Trigger | Cadence | Subcommand |
|---|---|---|---|
| App Store Connect API key | suspected compromise, hygiene rotation | flexible | `rotate-key.sh app-store-connect-key` |
| Developer ID Application cert | expiry, revocation | 5 years | `rotate-key.sh apple-cert` |
| Tauri updater signing key | suspected compromise ONLY | never proactively | `rotate-key.sh tauri-key` |

Plus one auto-managed credential (`KEYCHAIN_PASSWORD`) that doesn't rotate
manually — `bootstrap-secrets.sh` regenerates it whenever called.

## 1. App Store Connect API key

Authenticates `xcrun notarytool` against Apple's notary service. Used on
every release.

**Why it exists:** stint previously used `APPLE_ID` + `APPLE_PASSWORD`
(app-specific password) + `APPLE_TEAM_ID`. App-specific passwords expire
annually and can only be generated via the appleid.apple.com web UI — no
API. Migrating to App Store Connect API key auth eliminates that recurring
manual chore: the key is created once via the App Store Connect web UI and
afterwards every rotation is scripted.

**Blast radius:** releases fail at notarization until the new key is in
place. Existing shipped releases unaffected (the key authenticates
submission, not the resulting signature).

**Rotation:**

```bash
scripts/release/rotate-key.sh app-store-connect-key
```

The script walks you through creating a new key in App Store Connect,
downloading the `.p8`, then handles upload of all three secrets
(`APP_STORE_CONNECT_KEY_ID`, `APP_STORE_CONNECT_ISSUER_ID`,
`APP_STORE_CONNECT_PRIVATE_KEY`). At the end it prompts you to revoke the
old key in App Store Connect so a leaked `.p8` is no longer usable.

## 2. Developer ID Application cert

Codesigns the `.app`, `.dmg`, and embedded CLI binary. Used on every
release.

**Trigger:** Apple's Developer ID certs are valid for 5 years from
issuance. Watch for expiry warnings in build logs roughly 90 days out.

**Blast radius:** releases fail at codesign until the new cert is in
place. Existing shipped releases continue to work — Gatekeeper trusts the
cert's signature at the time of signing, not its current validity.

**Rotation:**

```bash
scripts/release/rotate-key.sh apple-cert
```

The script walks you through generating a new cert in Xcode → Keychain
Access, exporting to `.p12`, then handles upload of all three secrets
(`APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`).

Cert generation can't be fully scripted — Apple gates `.p12` export of
private keys through Keychain Access. (Generating certs via App Store
Connect API is possible but the JWT + CSR + key-pair-stitching machinery
isn't worth writing for a 5-year cadence.)

## 3. Tauri updater signing key

Signs `latest.json` so `tauri-plugin-updater` in existing installs can
verify update payloads. Every shipped stint binary embeds the *public*
half; rotating the *private* half means existing binaries reject all
future updates as signature-mismatched.

**Trigger:** suspected compromise ONLY. Never rotate proactively.

**Blast radius:** total auto-update outage until users manually reinstall.
The runbook for that recovery — pinning the cask, pushing a
`recovery.html`, surfacing a notice in the README — is at
`docs/runbooks/release-rollback.md`.

**Rotation:**

```bash
scripts/release/rotate-key.sh tauri-key
```

The script requires typing `I understand auto-update will break` literally
before proceeding. It generates a new keypair, edits
`crates/stint-app/tauri.conf.json` with the new pubkey, uploads the new
private key, and reminds you to commit + ship the pubkey change.

## When the script can't help

If `gh auth status` fails, fix authentication before running rotation:
the script aborts up front rather than getting halfway through.

If `bootstrap-secrets.sh` doesn't exist or isn't executable, the script
aborts. (`chmod +x scripts/release/bootstrap-secrets.sh` if needed.)

For first-time setup (no existing secrets), run `bootstrap-secrets.sh`
directly with no arguments — it walks every secret in sequence.
`rotate-key.sh` only handles re-credentialing of already-set secrets.

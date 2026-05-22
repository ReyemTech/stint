# Runbook — Release credential rotation

Procedures for rotating each of the credentials Phase 4 introduced.
None of these are scheduled tasks — they're triggered by external events
(annual cert expiry, suspected compromise, Apple-mandated password reset).

## 1. APPLE_PASSWORD (app-specific password)

**Trigger:** Apple invalidates the password (typically annually, or on
account-security events).

**Procedure:**

1. Visit appleid.apple.com → Sign-In and Security → App-Specific Passwords.
2. Revoke the old "stint notary" password.
3. Generate a new one. Apple shows it once.
4. `scripts/release/bootstrap-secrets.sh APPLE_PASSWORD`
5. Trigger a smoke release to verify (or wait for the next legitimate
   `feat:`/`fix:` push).

**Blast radius:** Releases fail at the notarization step with an
authentication error until the new password is in place.

## 2. APPLE_CERTIFICATE (Developer ID Application)

**Trigger:** Annual expiry, or cert revocation.

**Procedure:**

1. In Xcode → Settings → Accounts → Manage Certificates → "+" → "Developer ID
   Application".
2. The new cert appears in your login keychain alongside the old one.
3. Export both cert + private key as `.p12` (same procedure as Task 1.1).
4. Record the new "Common Name" — likely identical to the old one with a
   different expiry date.
5. `scripts/release/bootstrap-secrets.sh APPLE_CERTIFICATE APPLE_CERTIFICATE_PASSWORD APPLE_SIGNING_IDENTITY`
6. Optionally delete the old cert from keychain after the next successful
   release.

**Blast radius:** Releases fail at the codesign step until the new cert is
in place. Existing signed-and-shipped releases continue to work — Gatekeeper
trusts the cert's signature, not the cert's current validity.

## 3. TAURI_SIGNING_PRIVATE_KEY (updater key)

**Trigger:** Only suspected compromise. Never rotate proactively.

**Procedure:**

This is the dangerous one. Rotating breaks auto-update for every existing
install of stint.

1. Generate a new key: `cargo tauri signer generate -w ~/.tauri/stint-new.key`.
2. Update `crates/stint-app/tauri.conf.json` `plugins.updater.pubkey` with
   the new public key.
3. `scripts/release/bootstrap-secrets.sh TAURI_SIGNING_PRIVATE_KEY TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
4. Cut a release containing **only** the pubkey change (it gets a `fix:`
   commit, triggers a patch release).
5. Existing installs run the old build, which verifies updates against the
   *old* public key. The new release's `latest.json` is signed with the new
   key — existing installs reject the update.
6. Push a notice via the docs site instructing users to reinstall manually
   (`brew reinstall --cask stint` or rerun the install script).
7. Future releases follow the normal path; only the gap-installs (users who
   manually reinstall) bridge to the new key.

**Blast radius:** Total auto-update outage until users manually reinstall.
Last resort.

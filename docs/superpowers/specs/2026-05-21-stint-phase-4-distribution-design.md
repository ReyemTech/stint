# stint — Phase 4: Distribution (spec)

Sign, notarize, and distribute stint through three channels (Homebrew cask, direct DMG, `curl | sh`), with automated semantic-release versioning and `tauri-plugin-updater` for in-app updates. Mac App Store is deferred to Phase 4.5.

- **Status:** Confirmed 2026-05-21
- **Predecessors:** Phase 3.5 (test coverage uplift — shipped), Phase 3d (UX polish — shipped)
- **Target placement:** Phase 4. Phase 4.5 (Mac App Store) follows separately.

## 1. Goal

Make `stint` installable through signed, notarized, auto-updating channels driven by a single CI/CD pipeline. No user should encounter the "Cannot be opened because the developer cannot be verified" trap.

Concretely, after Phase 4 a user can:

- `brew tap reyemtech/tap && brew install --cask stint`
- Download `Stint-X.Y.Z.dmg` from the GitHub Releases page and drag to `/Applications`
- `curl -fsSL https://stint.reyem.tech/install.sh | sh` (CLI only) or with `-s -- --gui` (CLI + GUI)
- Opt into pre-releases via `brew install --cask stint-beta`
- Receive in-app update notifications driven by `tauri-plugin-updater`

## 2. Channel architecture

One signed/notarized `.dmg` is the master artifact. Every channel consumes it:

| Channel | Target | Artifact | Mechanism |
|---|---|---|---|
| Homebrew cask `stint` | GUI + CLI | `Stint-X.Y.Z.dmg` | `brew install --cask reyemtech/tap/stint` |
| Homebrew cask `stint-beta` | GUI + CLI | same `.dmg` from beta release | `brew install --cask reyemtech/tap/stint-beta` |
| Direct DMG | GUI + CLI | same `.dmg` | GitHub Releases download page |
| `curl \| sh` (CLI only) | CLI | `stint-X.Y.Z-universal-apple-darwin.tar.gz` | `curl … \| sh` |
| `curl \| sh --gui` | GUI + CLI | same `.dmg` mounted/copied | `curl … \| sh -s -- --gui` |
| Auto-update (in-app) | GUI + CLI | same `.app.tar.gz` | `tauri-plugin-updater` reads `latest.json` from latest Release |

**Two release tracks share one build.**

The bytes inside the `.app` are identical for stable and beta. Channel switching is a *runtime* setting; the cask name only determines which GitHub Release the first install pulls from. After install, the app's "Channel" setting controls subsequent updates. This means:

- One build artifact per release commit, not two.
- Same bundle ID (`tech.reyem.stint`), same data directory, same Keychain prefix.
- Beta cask installs *over* a stable install (and vice versa) — there is no side-by-side install model.
- The two GitHub Releases (`vX.Y.Z` and the moving `beta-latest`) differ only in their tag and prerelease flag, not in their assets.

## 3. Release pipeline

Driven by `semantic-release` reading Conventional Commits. Every push to `main` runs CI; if green and the commit history contains a `feat:` or `fix:` since the last tag, semantic-release cuts a release.

### Flow

```
Conventional commits on main / beta
        │
        ▼
   CI green (fmt / clippy / test / typecheck / build)
        │
        ▼
   semantic-release ───────► decides version from commit history
        │
        ├─► bumps Cargo.toml + tauri.conf.json + ui/package.json
        ├─► regenerates CHANGELOG.md
        ├─► commits "chore(release): vX.Y.Z [skip ci]" back to branch
        ├─► tags vX.Y.Z
        │
        ▼
   Build job (universal x86_64 + aarch64 lipo) on macos-14
        │
        ├─► CLI binary (universal)
        ├─► Stint.app with embedded CLI in Contents/MacOS/stint
        ├─► sign with Developer ID Application
        ├─► notarize + staple .app
        ├─► package .dmg → sign + notarize + staple .dmg
        ├─► tar.gz the CLI universal binary
        ├─► tauri signer sign on .app.tar.gz → .sig
        ├─► generate latest.json with signature
        │
        ▼
   Publish job
        ├─► create GitHub Release with .dmg, .tar.gz, latest.json, .sig as assets
        ├─► if beta: also delete + recreate beta-latest Release with same assets
        ├─► open PR to reyemtech/homebrew-tap with new cask version + sha256
        ├─► auto-merge tap PR after brew audit passes
        └─► publish install.sh with embedded checksums to docs-pages branch
```

### Workflows

- `.github/workflows/ci.yml` — existing Phase 2.5 workflow. Unchanged.
- `.github/workflows/release.yml` — new. Triggers on push to `main` and `beta`. Runs semantic-release driver.
- `.github/workflows/release-artifacts.yml` — new reusable workflow. Builds, signs, notarizes, packages.
- `.github/workflows/release-revert.yml` — new. Manually triggered to roll back `latest.json` to a prior version.
- `.github/workflows/deploy-pages.yml` — new. Deploys `docs-pages/` to GitHub Pages.

### Tooling

- `semantic-release` + plugins (Node devDeps at repo root):
  - `@semantic-release/commit-analyzer`
  - `@semantic-release/release-notes-generator`
  - `@semantic-release/changelog`
  - `@semantic-release/exec` — runs `scripts/release/bump-versions.sh`
  - `@semantic-release/git` — commits version bump + CHANGELOG with `[skip ci]`
  - `@semantic-release/github` — creates Release + uploads assets
- New scripts under `scripts/release/`:
  - `bump-versions.sh` — bumps `Cargo.toml` (workspace), `crates/stint-app/tauri.conf.json`, `ui/package.json`, then `cargo update -w`
  - `notarize.sh` — wraps `xcrun notarytool submit --wait` with retry on transient 5xx
  - `generate-latest-json.sh` — composes the updater manifest
  - `render-install-script.sh` — substitutes `@@…@@` placeholders in `scripts/install.sh.tpl`
  - `update-cask.sh` — in-place edit of cask `version` + `sha256` in tap repo
- Initial version: `0.1.0` (via `.releaserc.json` `initialVersion`).
- Wall-clock budget per release: 12–18 min (notarization is the slowest step).

### Concurrency

`concurrency: { group: release-${{ github.ref }}, cancel-in-progress: false }` — never cancel an in-flight release; queue the next one.

## 4. Signing, secrets, key management

Three independent signing identities, none can be conflated:

| Identity | Purpose | Storage |
|---|---|---|
| Developer ID Application | Sign `.app` + embedded CLI so Gatekeeper trusts them | Secret `APPLE_CERTIFICATE` (base64 .p12) + `APPLE_CERTIFICATE_PASSWORD` |
| Apple Notary credentials | Submit signed binaries to Apple's notary service | Secrets `APPLE_ID`, `APPLE_PASSWORD` (app-specific), `APPLE_TEAM_ID` |
| Tauri updater key | Sign update bundles so the in-app updater verifies authenticity | Private: secret `TAURI_SIGNING_PRIVATE_KEY` + `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. Public: baked into `tauri.conf.json` (committed) |

The Tauri updater key is **never rotated** unless compromise is suspected. Rotation breaks auto-update for every existing install.

### Required GitHub secrets (full inventory)

```
APPLE_CERTIFICATE
APPLE_CERTIFICATE_PASSWORD
APPLE_SIGNING_IDENTITY            # "Developer ID Application: Mario Meyer (TEAMID)"
APPLE_ID
APPLE_PASSWORD
APPLE_TEAM_ID
KEYCHAIN_PASSWORD                 # arbitrary; for ephemeral keychain unlock
TAURI_SIGNING_PRIVATE_KEY
TAURI_SIGNING_PRIVATE_KEY_PASSWORD
HOMEBREW_TAP_TOKEN                # fine-grained PAT, scoped read+write to reyemtech/homebrew-tap only
STINT_GOOGLE_CLIENT_ID            # reuses the same client as dev .env.local (deliberate; see §11)
STINT_GOOGLE_CLIENT_SECRET
```

### Ephemeral keychain in CI

CI creates a fresh keychain per run, imports the cert, signs, and deletes the keychain on cleanup (even on failure). Never touches the runner's default keychain. Sign with `--options runtime` for hardened-runtime compliance.

### Entitlements

`crates/stint-app/entitlements.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>com.apple.security.cs.allow-jit</key>            <true/>
  <key>com.apple.security.cs.allow-unsigned-executable-memory</key> <true/>
  <key>com.apple.security.cs.disable-library-validation</key>       <true/>
  <key>com.apple.security.network.client</key>          <true/>
  <key>keychain-access-groups</key>
  <array>
    <string>$(AppIdentifierPrefix)tech.reyem.stint</string>
  </array>
</dict>
</plist>
```

The three `cs.*` entitlements are required by WebView2/wry under hardened runtime. No App Sandbox — that's Phase 4.5 territory.

### Bootstrap script

Setting up twelve secrets by hand is error-prone — the most likely first-release failure mode is "I typo'd a base64-encoded cert." Phase 4 includes `scripts/release/bootstrap-secrets.sh`, an interactive walkthrough that:

1. Verifies `gh` CLI is authenticated and has write access to the repo.
2. Generates the Tauri updater key pair via `tauri signer generate` (interactive prompt for passphrase), prints the public key for manual paste into `tauri.conf.json`, and pushes the private key + passphrase to GitHub secrets.
3. For each Apple secret, walks the user through the manual step (e.g., "open Keychain Access, find your Developer ID Application cert, export as .p12"), waits for the resulting file, base64-encodes, and pushes to GitHub.
4. Detects existing secrets and prompts before overwriting (idempotent re-runs are safe).
5. Verifies each secret was set successfully via `gh secret list`.
6. Prints a final checklist of manual one-time steps that can't be scripted (DNS record for `stint.reyem.tech`, creating the empty `reyemtech/homebrew-tap` repo, registering Apple Notary credentials at appleid.apple.com).

Not scripted because they're too security-sensitive to automate:

- Generating the Apple Developer ID Application cert (must happen in Xcode → Settings → Accounts → Manage Certificates with explicit user action).
- Creating the app-specific password at appleid.apple.com (Apple does not expose an API for this).
- Reviewing the public key against the committed `tauri.conf.json` before merging the first release.

The script is run once per maintainer setup, not per release. It's also the recovery tool when rotating any of the credentials (re-run with the specific secret name as an arg: `./bootstrap-secrets.sh APPLE_PASSWORD`).

### Key rotation runbook

Lives at `docs/runbooks/release-key-rotation.md`. Covers:

- **`APPLE_PASSWORD` (annual)** — generate new app-specific password at appleid.apple.com, swap secret.
- **`APPLE_CERTIFICATE` (annual)** — generate new Developer ID cert in Xcode → Keychain Access, export as `.p12`, base64-encode, swap secret + `APPLE_SIGNING_IDENTITY`.
- **`TAURI_SIGNING_PRIVATE_KEY` (only if compromised)** — generate new key, ship a brew-only release telling users to reinstall manually, then rotate. Public key in `tauri.conf.json` changes; existing installs lose auto-update until manual reinstall.

## 5. Homebrew cask + tap repo

### Tap repo

New public repo `github.com/reyemtech/homebrew-tap` with:

```
homebrew-tap/
├─ README.md
├─ .github/workflows/test-casks.yml    # brew audit --strict --cask on PR
└─ Casks/
   ├─ stint.rb
   └─ stint-beta.rb
```

Users tap once with `brew tap reyemtech/tap`, then `brew install --cask stint` or `brew install --cask stint-beta`.

### Cask `stint.rb` (stable)

```ruby
cask "stint" do
  version "0.1.0"
  sha256 "<dmg sha256 — updated by CD>"

  url "https://github.com/reyemtech/stint/releases/download/v#{version}/Stint-#{version}.dmg",
      verified: "github.com/reyemtech/stint/"
  name "Stint"
  desc "Time tracker that syncs with Solidtime (CLI + menu bar app)"
  homepage "https://github.com/reyemtech/stint"

  livecheck do
    url :url
    strategy :github_latest
  end

  auto_updates true
  depends_on macos: ">= :ventura"

  app "Stint.app"
  binary "#{appdir}/Stint.app/Contents/MacOS/stint"

  uninstall quit:      "tech.reyem.stint",
            launchctl: "tech.reyem.stint",
            delete:    "/Applications/Stint.app"

  zap trash: [
    "~/Library/Application Support/stint",
    "~/Library/Preferences/tech.reyem.stint.plist",
    "~/Library/Caches/tech.reyem.stint",
    "~/Library/Logs/stint",
    "~/Library/WebKit/tech.reyem.stint",
  ]
end
```

### Cask `stint-beta.rb`

Identical structure, but:

- `version` is a prerelease string like `0.2.0-beta.1`
- `url` points at the moving `beta-latest` tag (see §7)
- `name "Stint"` (same bundle/app name — same-install model)
- Same `app`, `binary`, `uninstall`, `zap` keys as `stint`
- `livecheck` uses the `expanded_assets` strategy against the `beta-latest` Release

The casks **do not** declare `conflicts_with` against each other because they ship the same bundle ID. Installing one over the other just upgrades or downgrades the running app.

### CD updates the tap

After the GitHub Release is created, CD:

1. Clones `reyemtech/homebrew-tap` using `HOMEBREW_TAP_TOKEN`.
2. Runs `scripts/release/update-cask.sh` to in-place edit `version` and `sha256`.
3. Commits with a Conventional message: `feat: stint <version>`.
4. Pushes to a branch, opens a PR, auto-merges after the tap's own `brew audit` workflow passes.

## 6. `curl | sh` installer

### Hosting

`https://stint.reyem.tech/install.sh` from a new `docs-pages` branch served by GitHub Pages. Phase 5 will replace `index.html` with real docs; Phase 4 just stands up the bare minimum (the install script + a redirect index).

Custom domain config:

- `CNAME` file in `docs-pages/` containing `stint.reyem.tech`
- DNS CNAME record `stint.reyem.tech → reyemtech.github.io` (one manual setup task; not in CI)
- GitHub Pages auto-provisions a Let's Encrypt certificate

Fallback URL: `https://raw.githubusercontent.com/reyemtech/stint/main/scripts/install.sh`. Same template, also rendered with checksums on release.

### Script behavior

```bash
curl -fsSL https://stint.reyem.tech/install.sh | sh                          # CLI only
curl -fsSL https://stint.reyem.tech/install.sh | sh -s -- --gui              # CLI + GUI
curl -fsSL https://stint.reyem.tech/install.sh | sh -s -- --uninstall        # remove (interactive)
curl -fsSL https://stint.reyem.tech/install.sh | sh -s -- --version v0.1.0   # pin
```

**CLI-only path:**

1. Detect arch (`uname -m`) and OS (`uname -s` must be `Darwin`; bail otherwise).
2. Resolve target version (default: latest stable; `--version vX.Y.Z` pins to any tag, including beta tags like `v0.2.0-beta.1`).
3. Pick install prefix: `/usr/local/bin` if writable, else `$HOME/.local/bin` (create if missing). Honor `STINT_INSTALL_DIR` env override.
4. Download `stint-${VERSION}-universal-apple-darwin.tar.gz` from the release.
5. Verify SHA256 against the script-embedded checksum.
6. Extract, `chmod +x`, move to install dir, `xattr -dr com.apple.quarantine`.
7. Verify `stint --version` exits 0.
8. If install dir is not in `$PATH`, print a one-line "Add this to your shell rc:" hint.

**`--gui` adds:**

1. Download `Stint-${VERSION}.dmg`.
2. Verify SHA256.
3. `hdiutil attach -nobrowse -quiet`.
4. Overwrite check on `/Applications/Stint.app` (skip with `--force`).
5. `cp -R` to `/Applications` (prompts for sudo only if needed).
6. `xattr -dr com.apple.quarantine /Applications/Stint.app`.
7. `hdiutil detach`.
8. `open -g /Applications/Stint.app` to register the bundle with Launch Services.

**`--uninstall` path:**

- Remove the installed CLI binary.
- If `/Applications/Stint.app` exists, prompt before removing.
- Print but do not execute commands for cleaning up `~/Library/Application Support/stint` and Keychain entries. Data deletion stays explicit and manual.

### Provenance

Three guards layered:

1. **Checksums baked at release time.** `scripts/install.sh.tpl` has `@@TARBALL_SHA256@@` and `@@DMG_SHA256@@` placeholders. CD substitutes them before publishing. v0.1.0's `install.sh` only ever installs v0.1.0's artifacts.
2. **Verification before extraction.** Script computes SHA256 of the downloaded file and compares to the embedded value. Mismatch aborts.
3. **TLS only.** `curl -fsSL` with no `-k`. Both reyem.tech and github.com use HSTS in practice.

We also publish `install.sh.sha256` as a sibling for users who want to verify the script itself:

```bash
curl -fsSL https://stint.reyem.tech/install.sh > install.sh
curl -fsSL https://stint.reyem.tech/install.sh.sha256 | sha256 -c
sh install.sh
```

### TTY detection

When `stdin` is not a TTY (the standard `curl | sh` case), interactive prompts deadlock. Script detects with `[ -t 0 ]` and:

- Defaults to non-interactive behavior (e.g., `--force` for overwrite).
- Refuses destructive operations without an explicit flag.

## 7. Beta channel via moving tag

GitHub does not expose a `releases/latest-prerelease` endpoint. We simulate one with a force-moved `beta-latest` tag.

### Release shape per beta cut

For each beta release, CD creates **two** GitHub Releases on the same commit:

1. **`v0.2.0-beta.1`** — versioned, marked prerelease, full release notes. Permanent record.
2. **`beta-latest`** — moving tag, marked prerelease, same artifacts. CD deletes any existing `beta-latest` Release first, then re-creates. Provides the stable URL.

Stable URLs:

| URL | Use |
|---|---|
| `https://github.com/reyemtech/stint/releases/download/beta-latest/latest.json` | Tauri updater endpoint for beta |
| `https://github.com/reyemtech/stint/releases/download/beta-latest/Stint-Beta-latest.dmg` | Cask + curl installer (fixed filename, copied/renamed in CD) |
| `https://github.com/reyemtech/stint/releases/tag/v0.2.0-beta.1` | Archaeology, per-beta CHANGELOG anchor |

The artifact double-store (~30 MB extra per beta) is acceptable at our cadence.

### Tag-rewrite race

The window between "delete `beta-latest` Release" and "create new `beta-latest` Release" is microseconds, and GitHub serves asset content via per-asset CDN URLs that complete a download in flight even if the parent Release moves. We do not design around this.

## 8. `tauri-plugin-updater` integration

### Dependencies

```toml
# crates/stint-app/Cargo.toml
[features]
default = ["updater"]
updater = ["dep:tauri-plugin-updater"]

[dependencies]
tauri-plugin-updater = { version = "2", optional = true }
```

The feature flag is off in the Phase 4.5 MAS build (Apple forbids self-updating App Store apps). Plugin registration in `main.rs` is `#[cfg(feature = "updater")]` gated.

```json
// ui/package.json
"@tauri-apps/plugin-updater": "^2.0.0"
```

Plugin registered in `crates/stint-app/src/main.rs` alongside existing plugins.

### Configuration

```json
// crates/stint-app/tauri.conf.json (fragment)
"plugins": {
  "updater": {
    "active": true,
    "endpoints": [
      "https://github.com/reyemtech/stint/releases/latest/download/latest.json"
    ],
    "pubkey": "<Ed25519 public key>",
    "dialog": false
  }
}
```

`"dialog": false` so we render our own UI inside SolidJS.

### `latest.json` shape

```json
{
  "version": "0.1.0",
  "notes": "see CHANGELOG.md",
  "pub_date": "2026-05-21T18:00:00Z",
  "platforms": {
    "darwin-x86_64":  { "signature": "...", "url": "https://github.com/reyemtech/stint/releases/download/v0.1.0/Stint.app.tar.gz" },
    "darwin-aarch64": { "signature": "...", "url": "https://github.com/reyemtech/stint/releases/download/v0.1.0/Stint.app.tar.gz" }
  }
}
```

Single bundle URL for both archs because we ship universal binaries.

### UI surface

Three new touchpoints in the existing SolidJS app:

**Settings → "Updates" panel.** New section in the Settings route showing current version, channel selector, last-check timestamp, "Check automatically" toggle, "Install in background when available" toggle, and a "Check now" button.

**Popover footer indicator.** Existing `StatusDot` gains an "Update ready · Restart to install" state. Tap opens the relaunch confirmation.

**Info banner.** Reuses the Phase 3d `SyncErrorBanner` slot as info-style: "stint v0.2.0 is available. [Install now]".

### Settings defaults

- `update.channel = "stable"`
- `update.check_interval_hours = 24`
- `update.auto_install = true` — when an update is downloaded, apply it on the next idle moment without requiring a "Install" click. User still has to relaunch (or we relaunch on next idle period).
- `update.last_check_at` — ISO timestamp, written after each check.

The default for `auto_install` is **on**, matching Chrome and VS Code. Faster patch propagation is worth the trade-off in user control; the channel toggle lets paranoid users opt out.

### Channel switching

- **Stable → Beta:** changes endpoint URL to `beta-latest`. On next check, if beta version is higher than current, offers install.
- **Beta → Stable:** changes endpoint back. Doesn't auto-downgrade. UI explicitly warns: "Switching to Stable won't downgrade you. Reinstall via Homebrew (or DMG) to return to the current stable release."

### Check timing

- 5 seconds after app launch (don't block startup).
- Every `update.check_interval_hours` while running.
- On user demand via "Check now".

### Failure handling

All updater errors route through the Phase 3d `SyncErrorBanner` infrastructure. Add a new `UpdateError` variant alongside the existing `SyncError`. Same UI affordances (dismissible, details modal, retry).

| Scenario | Behavior |
|---|---|
| Endpoint unreachable | Silent on automatic checks; visible failure with retry on manual check. |
| Signature verification fails | Update rejected. Error surfaced with "Contact support" link. Never silently applied. |
| Download interrupted | Plugin resumes on next check; only surfaced after 3 consecutive failures. |
| Disk full | Plugin restores from temp; original app stays runnable. Surfaces "insufficient disk space." |
| `minimumSystemVersion` not met | Update refused with clear "requires macOS X+." |

### CLI doesn't self-update

`stint update` prints upgrade instructions tailored to the install method but never modifies itself. macOS exclusive file locks on running binaries make self-replacement fragile. Users update via `brew upgrade`, re-running `curl | sh`, or letting the GUI updater replace the bundle (which also updates the embedded CLI).

## 9. Rollback procedure

When a release ships broken code, the recovery path is manual but well-defined.

### Step 1 — stop the bleeding

Manually trigger `.github/workflows/release-revert.yml` with the last-known-good version:

```bash
gh workflow run release-revert.yml -f version=0.1.5
```

The workflow:

1. Fetches `latest.json` from release `v0.1.5`.
2. Re-uploads it as an asset on the current `releases/latest` Release, replacing the bad manifest.
3. Posts a notice to the bad release's GitHub Release page.

Users who haven't auto-updated yet stop receiving the bad build. Users who already updated stay broken until step 2.

### Step 2 — repair affected users

Cut `v0.1.6` with a fix. Auto-updater pulls it on the next 24-hour check window.

**Catastrophic case** (the updater itself broken):

1. Pin both casks to last-known-good (manual PR to tap).
2. Update `latest.json` to last-known-good (existing-install users auto-roll-back on next check).
3. Push a `recovery.html` page to `stint.reyem.tech` with manual reinstall instructions.

### Step 3 — post-mortem

Required for any reverted release. Lives at `docs/incidents/YYYY-MM-DD-vX.Y.Z.md`. Minimum content: what broke, how we found out, who was affected, what the fix was, what guardrails we add to prevent recurrence.

## 10. Forward compatibility with Phase 4.5 (MAS)

Phase 4 makes choices that Phase 4.5 will reinherit. To avoid painting Phase 4.5 into a corner:

- **Keep all `~/Library/Application Support/stint/` lookups behind `stint-core`'s config module.** No new code in Phase 4 bypasses it. MAS containers will replace the path; the surface area touched by that change must stay small.
- **Don't hardcode the bundle ID or Keychain prefix** in any new release script. The version bumper, install script, and CD all parameterize these from `tauri.conf.json` so MAS can override.
- **`tauri-plugin-updater` registration is conditional on a Cargo feature flag** (`features = ["updater"]`, default on). The MAS build flips this off — Apple forbids self-updating App Store apps.

Phase 4.5 itself will need its own design pass covering App Sandbox entitlements, Mac App Distribution signing, the Solidtime/Google/MS OAuth redirect audit, and the migration story from Developer ID install to MAS install (separate Keychain access scope; user re-authenticates).

## 11. Out of scope (Phase 4 only)

Explicitly deferred:

- **Linux/Windows distribution.** macOS only per the project design spec (`2026-05-17-stint-design.md` §12).
- **Mac App Store.** Phase 4.5.
- **Production Google OAuth client.** Releases reuse the dev `STINT_GOOGLE_CLIENT_ID`/`SECRET`. Known trade-off: dev experimentation and production traffic share the same Google quota. Acceptable for early Phase 4 traffic; revisit when usage grows.
- **Crash reporting / telemetry.** No Sentry, no analytics. Separate phase later.
- **Update verification UI for paranoid users** (e.g., "show me the SHA256 of what was downloaded").
- **Delta updates.** Tauri plugin downloads full bundles each release (~20 MB). Acceptable.
- **Standalone CLI self-update.** CLI prints instructions, never modifies itself.
- **Side-by-side stable + beta install.** Same-install model only. Beta installs over stable; channel is a runtime setting.
- **Per-version GitHub Release notes for beta.** The `beta-latest` Release shows only the most recent beta's notes; archaeology lives at the `vX.Y.Z-beta.N` tag.
- **`brew install --formula stint`.** CLI-only installs go through `curl | sh`. No formula maintained.
- **Notarization of the install script.** macOS doesn't notarize shell scripts; SHA256 pinning and TLS are what we get.
- **Apple Developer ID renewal automation.** Annual manual swap, documented in the runbook.
- **Independent per-crate versioning.** Workspace-root version applies to all crates. Not publishing to crates.io.
- **macOS pre-Ventura support.** Floor is 13.0. Older users stay on the last pre-Phase-4 build.

## 12. Verification

Each task lands with tests where they're meaningful. The Phase 4 verification plan in PLAN.md will detail per-task verification commands. High-level:

- **Local cask testing** via `scripts/release/test-cask-locally.sh` before the first real release. Builds a `.dmg` against a faked v0.0.0 release, points the cask at the local file, runs `brew install` and `brew uninstall`.
- **Manual smoke test of the updater** before tagging v0.1.0: cut a throwaway v0.0.0-test build, publish a v0.0.1-test `latest.json`, verify the app downloads and applies. Documented as a release-pipeline runbook check.
- **Updater logic unit tests** in `stint-core` (parsing `latest.json`, comparing versions, deciding to fetch). wiremock for fake manifests.
- **Tauri command integration tests** for `check_for_updates`, `apply_update`, `set_channel`.
- **Install-script tests** — bash test harness invoking the rendered script against a local fake GitHub release server. Covers `--gui`, `--uninstall`, `--version`, TTY-detection paths.
- **End-to-end notarized release test** is *not* automated — too heavy for PR CI. The first real release on `main` is the smoke test, gated behind the local manual checks above.

## 13. Known trade-offs

Decisions worth being honest about in case future maintainers wonder why:

- **One Google OAuth client for dev and prod.** Quota and verification status shared. Will outgrow this eventually.
- **`HOMEBREW_TAP_TOKEN` is a PAT, not a GitHub App.** Faster to set up; less revocable. Acceptable for v1.
- **Auto-merge of tap PRs.** Removes a manual checkpoint. If `brew audit` is broken by an upstream Homebrew change, releases get blocked at the tap merge. Tradeoff for low-friction releases.
- **Tauri plugin downloads full bundles, no delta updates.** ~20 MB per update. Bandwidth is cheap; engineering time is not.
- **`auto_install = true` by default.** More aggressive than Slack/Discord defaults. Bet on faster fixes for users; user can flip off.
- **`v0.1.0` is a fresh start.** Pre-Phase-4 dev users re-authenticate (Solidtime, calendars) on first signed install because Keychain access groups change between unsigned-cdhash and signed-cert builds. Documented in release notes.

## 14. Repository additions

New files landing in Phase 4 (rough inventory; the plan will enumerate them precisely):

```
package.json                              # root, only for semrelease devDeps
pnpm-lock.yaml                            # already exists at root; semrelease deps will land here
.releaserc.json                           # semantic-release config
scripts/release/
  bootstrap-secrets.sh                    # interactive walkthrough for §4 secret setup
  bump-versions.sh
  notarize.sh
  generate-latest-json.sh
  render-install-script.sh
  update-cask.sh
  test-cask-locally.sh
scripts/install.sh.tpl                    # source for the rendered install script
crates/stint-app/entitlements.plist
crates/stint-app/src/updater.rs           # Tauri commands for the updater
ui/src/routes/Settings/UpdatesPanel.tsx
ui/src/lib/updates.ts                     # signals + Tauri IPC for the updater
docs-pages/                               # new branch; minimal Pages stub
.github/workflows/release.yml
.github/workflows/release-artifacts.yml
.github/workflows/release-revert.yml
.github/workflows/deploy-pages.yml
docs/runbooks/release-key-rotation.md
docs/runbooks/release-rollback.md
```

External (not in this repo):

- `github.com/reyemtech/homebrew-tap` — new repo with `Casks/stint.rb`, `Casks/stint-beta.rb`, and a `brew audit` PR check.
- DNS record `stint.reyem.tech → reyemtech.github.io`.
- One-time manual setup of all GitHub secrets listed in §4.

## 15. Acceptance criteria

Phase 4 ships when all of the following are true:

1. `git tag phase-4-complete` is reachable from a green CI/CD run on `main`.
2. `brew tap reyemtech/tap && brew install --cask stint` installs a working, signed, notarized stint on a fresh macOS Ventura+ machine with no Gatekeeper prompt.
3. Direct DMG download from the GitHub Releases page works the same way.
4. `curl -fsSL https://stint.reyem.tech/install.sh | sh` installs the CLI in `~/.local/bin` (or `/usr/local/bin`) and verifies it.
5. `curl -fsSL https://stint.reyem.tech/install.sh | sh -s -- --gui` installs the GUI to `/Applications` with no Gatekeeper prompt.
6. The v0.1.0 release (the first to ship through this pipeline) auto-publishes a `latest.json` that an existing v0.0.0-test install picks up and applies cleanly via `tauri-plugin-updater`. The v0.0.0-test build is a throwaway cut during Phase 4 plan execution explicitly to verify this.
7. `brew install --cask stint-beta` installs the latest prerelease.
8. `gh workflow run release-revert.yml -f version=<prior>` rolls `latest.json` back to a prior version's manifest.
9. The key-rotation runbook has been walked through end-to-end at least once (even if no rotation was performed).
10. `scripts/release/bootstrap-secrets.sh` has been run successfully to set up all twelve GitHub secrets, and a second dry-run confirms it's idempotent.
11. README updated with all four install methods.

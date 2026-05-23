## [0.2.0](https://github.com/ReyemTech/stint/compare/v0.1.5...v0.2.0) (2026-05-23)

### Features

* **about:** show update-available badge next to version ([82f9929](https://github.com/ReyemTech/stint/commit/82f99296a67ccddc534c39939a9b8e04242bfbb8))
* **menu:** "Check for Updates…" in menus + CHANGELOG link ([726091d](https://github.com/ReyemTech/stint/commit/726091dbea7fb35dcc3fefd9990d69f35060bfdd))
* **popover:** Settings cogwheel + update-available indicator ([59e15b7](https://github.com/ReyemTech/stint/commit/59e15b7e734786daae1b45be7cabe1f8648eede1))

## [0.1.5](https://github.com/ReyemTech/stint/compare/v0.1.4...v0.1.5) (2026-05-23)

### Bug Fixes

* **notarize:** switch to App Store Connect API key auth ([fef649b](https://github.com/ReyemTech/stint/commit/fef649bb393dd39534337e2055fe97cbe91bb1e8))
* **release:** swap Apple ID secrets for App Store Connect API key secrets ([9241cbd](https://github.com/ReyemTech/stint/commit/9241cbd0b7c3f07d4f694274e64fa21957a08bb0))

## [0.1.4](https://github.com/ReyemTech/stint/compare/v0.1.3...v0.1.4) (2026-05-23)

### Bug Fixes

* **release-revert:** self-healing manifest backup so restore actually works ([9dcce93](https://github.com/ReyemTech/stint/commit/9dcce93af3cccd2eb07b64670b8a4aa8dc465ff2))
* **release:** broaden tap-PR auto-merge fallback + clean stale PRs ([348fe17](https://github.com/ReyemTech/stint/commit/348fe1726943c7f80536322bdc03e5d4be6e9db0)), closes [#15](https://github.com/ReyemTech/stint/issues/15) [#6](https://github.com/ReyemTech/stint/issues/6)

## [0.1.3](https://github.com/ReyemTech/stint/compare/v0.1.2...v0.1.3) (2026-05-22)

### Bug Fixes

* **app:** write tracing logs to ~/Library/Logs/tech.reyem.stint/ ([ace0f07](https://github.com/ReyemTech/stint/commit/ace0f070944d3732debe18f389b5a1d449226811))
* **updater-ui:** two-step Install → Restart Stint button flow ([3ebf887](https://github.com/ReyemTech/stint/commit/3ebf887d808f5575f20006c84dadf30265a726ee))
* **updater:** split install + restart into separate Tauri commands ([1f4dc9e](https://github.com/ReyemTech/stint/commit/1f4dc9eb6f33216e3de1c3653199c73e26f2885f))

## [0.1.2](https://github.com/ReyemTech/stint/compare/v0.1.1...v0.1.2) (2026-05-22)

### Bug Fixes

* **install:** fail fast on --gui when macOS <13 ([72b4133](https://github.com/ReyemTech/stint/commit/72b41330efffb3d9922036440c367f0c7de26da7))
* **release:** fall back to direct merge when tap PR is in clean status ([a8a8811](https://github.com/ReyemTech/stint/commit/a8a8811892762502edaf953174452b25c65e260e))

## [0.1.1](https://github.com/ReyemTech/stint/compare/v0.1.0...v0.1.1) (2026-05-22)

### Bug Fixes

* **release:** proper macOS signing for embedded CLI + remove broken entitlement ([9b7a5a0](https://github.com/ReyemTech/stint/commit/9b7a5a06ed2d6de0a81304251b85b2da5c38bb15))
* **release:** push-tap-pr idempotent — delete stale branch/PR before retry ([65b6a21](https://github.com/ReyemTech/stint/commit/65b6a214f3c0b595639e6304c51b1532d856402a))

## [0.1.0](https://github.com/ReyemTech/stint/compare/v0.0.0...v0.1.0) (2026-05-22)

### Features

* **app:** hardened-runtime entitlements for signed release builds ([0b6466b](https://github.com/ReyemTech/stint/commit/0b6466bff87621284fcf8c1dc9893c12c348c29e))
* **app:** settings_get + settings_set Tauri commands for per-key access ([8183a4c](https://github.com/ReyemTech/stint/commit/8183a4c1aa846b50d1581da40cd59f6a93f56ad7))
* **app:** wire entitlements + signing identity placeholders in tauri.conf ([3973f34](https://github.com/ReyemTech/stint/commit/3973f349418fd5a3619908a1ce544f1ce13ba441))
* **cli:** checksum verify + atomic replace primitives for self-update ([f2d87c5](https://github.com/ReyemTech/stint/commit/f2d87c50d89a50a1777b901743dba282a7d60864))
* **cli:** GitHub Releases API client for stint update ([43dc70a](https://github.com/ReyemTech/stint/commit/43dc70ab17e44164b80f98f9514b06c497695e56))
* **cli:** install-method detector for stint update ([59ccea4](https://github.com/ReyemTech/stint/commit/59ccea455c0329f22728bbaa01f95db26eb6f8f8))
* **cli:** stint update — self-update standalone install, defer to GUI/brew for .app-bundled ([209a65c](https://github.com/ReyemTech/stint/commit/209a65cab4cce9f42ad6d35c01cc6dc4e2adb4be))
* **release:** add RELEASE_TOKEN to bootstrap-secrets.sh inventory ([b101ae3](https://github.com/ReyemTech/stint/commit/b101ae37ec761494011d16b0ad71e883b2424229))
* **release:** bootstrap-secrets.sh for interactive secret setup ([fe33b45](https://github.com/ReyemTech/stint/commit/fe33b45ec97391228b6ae003091a5c08743e42d5))
* **release:** bump-versions.sh to keep Cargo+Tauri+UI in sync ([5cd010e](https://github.com/ReyemTech/stint/commit/5cd010e5740dffb57a95555d7f55ff073cadba7e))
* **release:** curl|sh install script template + renderer ([e0a876d](https://github.com/ReyemTech/stint/commit/e0a876df33096d195f3b048abd9d9e185dfbbd7c))
* **release:** generate-latest-json.sh composes tauri-plugin-updater manifest ([16ae9a8](https://github.com/ReyemTech/stint/commit/16ae9a864fbfd7a31716115c0fca1f6e0bae0093))
* **release:** notarize.sh wraps xcrun notarytool with retry on transient errors ([36b56fa](https://github.com/ReyemTech/stint/commit/36b56fa75a69314cba58a56ad344cb9012e28989))
* **release:** publish-install-script.sh pushes rendered install.sh to docs-pages ([34865b7](https://github.com/ReyemTech/stint/commit/34865b70ef4ec387bc55803032c00371cd25a73f))
* **release:** push-tap-pr.sh opens auto-merging tap PR with new cask version ([bc450b0](https://github.com/ReyemTech/stint/commit/bc450b0695de66617ee5ebdb11fcfda31b6109a4))
* **release:** sync-beta-latest.sh maintains moving beta-latest GitHub Release ([c29d2ef](https://github.com/ReyemTech/stint/commit/c29d2ef4be93318fd60ca1f31352a2359a6d4f33))
* **release:** test-cask-locally.sh dry-runs the tap install round-trip ([baec505](https://github.com/ReyemTech/stint/commit/baec5054367a133fb4e3f193abb9560a962aa7fa))
* **release:** update-cask.sh in-place version+sha256 edit ([fa28b18](https://github.com/ReyemTech/stint/commit/fa28b187e91db0bbb117feb2a5c1d143363e5f03))
* **ui:** Settings → Updates panel (channel switch, check, install) ([c997b41](https://github.com/ReyemTech/stint/commit/c997b41becddeecdc5aa3ad2d27073bb15badd34))
* **ui:** update banner + status indicator for available updates ([25f4299](https://github.com/ReyemTech/stint/commit/25f4299415b1e6f12bb3abf8c51360e9e84a9438))
* **ui:** updater IPC wrapper (channel get/set, check, apply) ([f328de1](https://github.com/ReyemTech/stint/commit/f328de1de89aec76930e21edcd35f1ed100ee5ca))
* **updater:** add tauri-plugin-updater behind feature flag ([c512157](https://github.com/ReyemTech/stint/commit/c512157a5b50e7e24519e2242f4a709e133b620b))
* **updater:** channel→endpoint resolver ([5537b84](https://github.com/ReyemTech/stint/commit/5537b84a8b34cd4e25d678cdc5608431ec63d877))
* **updater:** check_for_updates + apply_update Tauri commands ([5e8e6eb](https://github.com/ReyemTech/stint/commit/5e8e6eb6b587272bc13c0ca4a62ffc07a5140644))
* **updater:** embed Tauri updater public key in tauri.conf.json ([a4ac8b4](https://github.com/ReyemTech/stint/commit/a4ac8b4fb17b41f5d1e7221e2d0a48462703e7ca))
* **updater:** register tauri-plugin-updater (cfg-gated on "updater" feature) ([7a024a6](https://github.com/ReyemTech/stint/commit/7a024a692673460e912a7f0907275632fe4e23b9))

### Bug Fixes

* **ci:** bump versions before build to align compiled binary with release artifact names ([5b94842](https://github.com/ReyemTech/stint/commit/5b9484235ed2660aa3d7a1afe65d2e040ee3c5d6))
* **ci:** checksums.txt uses bare filenames (CLI parser expects no ./ prefix) ([d9da634](https://github.com/ReyemTech/stint/commit/d9da634a62c9a66cfe0d8d649acc8c062c6dbaf8))
* **ci:** install tauri-cli on the runner (prebuilt via taiki-e/install-action) ([5ded468](https://github.com/ReyemTech/stint/commit/5ded4682c83011fd86dd542fe32b1160947bfc10))
* **ci:** semrelease version regex case-insensitive (handles "the next release version is X") ([abc1b41](https://github.com/ReyemTech/stint/commit/abc1b41f6f1e452841a2ef42fae6c0172bd89649))
* **ci:** tauri bundle path was crates/stint-app/target — workspace target is the actual location; install.sh.tpl drop hdiutil -quiet so we can parse the volume path ([0688f63](https://github.com/ReyemTech/stint/commit/0688f63c306408505b9c0195061bfca8a9492b33))
* **ci:** use RELEASE_TOKEN PAT for semrelease push (bypass ruleset) ([425abfa](https://github.com/ReyemTech/stint/commit/425abfa83b2fb7ce4f8eadf03d2cd3d92620e27b))
* **release:** add missing conventional-changelog-conventionalcommits peer dep ([8a11f0b](https://github.com/ReyemTech/stint/commit/8a11f0bf8dfccb12169536cba9d71448c2c4a066))
* **release:** bootstrap-secrets.sh — bash version guard, p12 cleanup, key rotation ([53eb62d](https://github.com/ReyemTech/stint/commit/53eb62df8fe729140eb009f35a5daf72a71ebb47))
* **release:** install.sh.tpl — arg bounds, uninstall paths, mount safety, UX ([4ef0201](https://github.com/ReyemTech/stint/commit/4ef0201c6608a1969682e8ea6230c7032f813925))
* **release:** line-based Cargo.toml version bump (regex was failing in CI) ([612a19d](https://github.com/ReyemTech/stint/commit/612a19d278bc7b4f586b75fdaf34907b984c9110))
* **release:** sync-beta-latest mirrors versioned DMG name (matches new beta cask URL) ([22b412c](https://github.com/ReyemTech/stint/commit/22b412c90593093ba524ba32fe9d573d70ba87f3))
* **release:** TAURI_SIGNING_PRIVATE_KEY uses cat (key file is already base64; double-encoding broke CI signer) ([1df09c4](https://github.com/ReyemTech/stint/commit/1df09c495d1f4bbedb1524cdba74841a5095e7e0))
* **updater:** configure plugins.updater in tauri.conf with placeholder pubkey + bootstrap auto-substitution ([e2b2073](https://github.com/ReyemTech/stint/commit/e2b207348073c2aa7f4287ad013842e03abeff59))

# Changelog

All notable changes to stint are documented here. Generated by
[semantic-release](https://semantic-release.gitbook.io) from Conventional
Commits in `main` and `beta` branches.

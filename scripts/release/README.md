# Release scripts

Scripts invoked by the CD pipeline and by maintainers. None of them run in
normal day-to-day dev; see CLAUDE.md for `dev-cli.sh` / `dev-app.sh` instead.

- `bootstrap-secrets.sh` — interactive walkthrough to push the twelve
  GitHub Actions secrets the pipeline needs. Idempotent.
- `bump-versions.sh` — called by `@semantic-release/exec` to bump
  versions across the workspace.
- `notarize.sh` — wraps `xcrun notarytool submit --wait` with retry on
  transient Apple 5xx responses.
- `generate-latest-json.sh` — composes the `tauri-plugin-updater`
  manifest from build artifacts.
- `render-install-script.sh` — substitutes `@@…@@` placeholders in
  `scripts/install.sh.tpl` with the release's version and SHA256s.
- `update-cask.sh` — in-place edit of `version` and `sha256` in the
  tap repo's cask formulas.
- `test-cask-locally.sh` — builds a fake-release `.dmg` and verifies the
  cask formula installs/uninstalls cleanly without touching the real
  tap or GitHub Releases.

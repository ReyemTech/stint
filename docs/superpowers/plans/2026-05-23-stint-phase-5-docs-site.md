# stint Phase 5: Documentation Site Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a public documentation site at `stint.reyem.tech` covering installation, quickstart, Solidtime + calendar setup, CLI reference, keyboard shortcuts, and troubleshooting. The existing `curl | sh` installer continues to work at `stint.reyem.tech/install.sh` without disruption.

**Architecture:** Astro + Starlight scaffolded under `site/` in the main repo. CI builds Starlight to static output on push to `main` (when `site/**` or `CHANGELOG.md` changes) and syncs the build into the existing `docs-pages` branch, preserving `install.sh`, `install.sh.sha256`, and `CNAME`. The existing Pages deploy workflow (`deploy-pages.yml`) re-deploys on every push to `docs-pages`, no changes required there. Same domain, same Pages site — Starlight at `/`, installer still at `/install.sh`.

**Tech Stack:** Astro 5 · Starlight 0.34 · pnpm 9 · Node 20 · GitHub Actions (Ubuntu runner — Starlight builds are platform-agnostic, no need for macos-14). Authored in markdown/MDX, no client-side JavaScript needed for content pages. Pagefind (bundled with Starlight) provides client-side search.

**No separate design spec.** Phase 5 is small enough that design decisions live inline below. The roadmap entry in `README.md` and CLAUDE.md is the only prior reference.

---

## Why Starlight (not MkDocs Material, not hand-rolled)

The two viable alternatives were considered:

1. **MkDocs Material.** Markdown-only, Python tooling, zero JS in the repo. Lighter dependency surface than Astro.
2. **Hand-rolled static site** (a few HTML files). Minimum dependencies, maximum control, but every navigation/search/theme feature is reinvention.

Starlight chosen because:

- **Already on the JS toolchain.** Repo has pnpm + Node for the Tauri UI. Adding Astro doesn't introduce a new ecosystem.
- **Component-friendly.** Future macOS-screenshot embeds, code blocks with file/line refs, callout boxes — all easier with MDX than pure markdown.
- **Built-in features earn their weight.** Auto-generated sidebar from frontmatter, dark mode, Pagefind search, OG card generation, sitemap — Starlight ships these without configuration.
- **Active project, conservative defaults.** Site builds reproducibly; no surprise breakage.

Trade-off: marginally larger CI build time vs MkDocs (Astro builds in ~5–10s for a small site). Negligible.

---

## Why `site/` in main + CI sync (not authoring directly on `docs-pages`)

Two options:

1. **`site/` in main, CI syncs to `docs-pages`.** Doc edits flow through normal PR review, branch protection, the merge-commit ritual. CI does the boring part.
2. **Authoring directly on `docs-pages`.** Edit-and-push, no PR overhead. But: changes bypass review, conflict with the existing install.sh deploy script that also pushes to that branch, and lose connection to code changes (e.g. CLI flag added → docs page should update in the same PR).

Option 1 chosen. Doc changes that accompany code changes (new CLI subcommand, new keyboard shortcut) land in the same PR as the code, reviewed together. CI handles the publish.

---

## How install.sh + docs site coexist

The `docs-pages` branch currently contains:

```
CNAME
index.html         # to be replaced by Starlight's generated index.html
install.sh
install.sh.sha256
.github/           # nothing relevant
```

The Starlight build emits a full static site (index.html, assets/, content pages, sitemap.xml, etc.). The deploy script needs to:

1. Clone `docs-pages`
2. **Delete** old Starlight-managed files (everything except `install.sh`, `install.sh.sha256`, `CNAME`, and `.github/`)
3. Copy fresh Starlight build output on top
4. Commit + push

`publish-install-script.sh` is unchanged — it touches only `install.sh` and `install.sh.sha256`. Two deploy scripts target the same branch but disjoint file sets; race-condition risk is low (Starlight rebuilds only on `site/**` changes, installer rebuilds only on release).

Worst-case race: both push at the same instant, one loses with non-fast-forward. Retry-on-conflict baked into the new deploy script handles it.

---

## File Structure

```
site/                                          # new
├─ astro.config.mjs
├─ package.json
├─ pnpm-lock.yaml
├─ tsconfig.json
├─ public/
│  └─ favicon.svg                              # reuse ui/src/components/StintIcon source
└─ src/
   ├─ content/
   │  ├─ config.ts                             # Starlight content collections
   │  └─ docs/
   │     ├─ index.mdx                          # Welcome (landing)
   │     ├─ install.md                         # all four install methods
   │     ├─ getting-started/
   │     │  ├─ quickstart.md                   # GUI + CLI walkthroughs
   │     │  ├─ solidtime.md                    # PAT + OAuth
   │     │  └─ calendar.md                     # Google (only one shipped)
   │     ├─ reference/
   │     │  ├─ cli.md                          # all stint subcommands
   │     │  └─ shortcuts.md                    # keyboard shortcuts
   │     └─ help/
   │        ├─ troubleshooting.md              # keychain prompts, OAuth not configured, sync fails
   │        └─ faq.md                          # common questions
   └─ styles/
      └─ custom.css                            # accent color overrides

Sidebar (configured in astro.config.mjs):

  Welcome           → index.mdx
  Install           → install.md
  GETTING STARTED
    · Quickstart    → getting-started/quickstart.md
    · Solidtime setup → getting-started/solidtime.md
    · Calendar setup  → getting-started/calendar.md
  REFERENCE
    · CLI commands  → reference/cli.md
    · Keyboard shortcuts → reference/shortcuts.md
  HELP
    · Troubleshooting → help/troubleshooting.md
    · FAQ           → help/faq.md

OG cards: Starlight default per-page generation (site title + page title on
gradient bg). No custom OG image work for v1; iterate to a custom design
in a follow-up if the default feels generic.

Commit types: \`docs(scope): …\` throughout. Conventional Commits' default
ruleset treats \`docs:\` as no-version-bump, so the docs site ships without
triggering a release — version bumps remain tied to product changes. Lands
in the auto-generated changelog under "Documentation".

scripts/release/
└─ publish-docs.sh                             # new — syncs Starlight build to docs-pages

.github/workflows/
└─ deploy-docs.yml                             # new — triggers Starlight build + publish on site/** changes
```

Notes:

- **No CHANGELOG mirror page in v1.** Add later if useful; for now link out to https://github.com/reyemtech/stint/releases.
- **No screenshots in v1.** Add once we have a representative dark-mode capture set; they're high-maintenance.
- **No README + in-app links updated.** Save that for a follow-up PR after the site is live and the URLs are stable.

---

## Tasks

### Task 1 — Scaffold Astro + Starlight under `site/`

- [ ] Create `site/` directory
- [ ] Initialize Astro + Starlight following the official `pnpm create astro@latest site -- --template starlight --typescript strict --no-install` (or the manual setup if the template's defaults conflict with the structure above)
- [ ] Configure `site/astro.config.mjs`:
  - `site: 'https://stint.reyem.tech'`
  - Title: `stint`
  - Description: `macOS time tracker that syncs with self-hosted Solidtime`
  - Sidebar structure matching the file layout under "File Structure" above
  - GitHub link: `https://github.com/reyemtech/stint`
  - Social card: configure OG image
- [ ] Add `site/src/styles/custom.css` with stint's indigo accent (matches `StintIcon` gradient)
- [ ] Add favicon at `site/public/favicon.svg` (copy from `ui/src/components/StintIcon.tsx` SVG path)
- [ ] `pnpm install` inside `site/` works
- [ ] `pnpm --filter site dev` (or `cd site && pnpm dev`) serves the default Starlight page

**Acceptance:** local dev server renders, navigation works, dark mode toggles.

### Task 2 — Author MVP content pages

For each page, draft markdown content covering the items listed below. Each page should be ~300–600 words; pages with code blocks can be longer.

- [ ] `index.mdx` — Welcome (landing page)
  - Hero with stint name + one-line description
  - Three-column feature grid: local-first, CLI + GUI, Solidtime sync
  - Install CTA buttons (Homebrew, DMG, curl|sh) linking to `/install`
  - Link to GitHub repo + report-an-issue
- [ ] `install.md` — Install methods
  - macOS 13+ requirement banner up top
  - All four methods from current README:
    1. Homebrew (recommended)
    2. Direct DMG
    3. `curl | sh` (CLI only)
    4. `curl | sh --gui` (CLI + GUI)
  - First-run setup: keychain prompts (link to dev script story is internal — skip; users won't hit this)
  - Uninstall instructions
- [ ] `quickstart.md` — First 5 minutes
  - Set up Solidtime connection (point to `/setup/solidtime`)
  - Optionally add Google Calendar (point to `/setup/calendar`)
  - Start a timer (CLI + GUI examples side by side)
  - Stop, see today, sync
- [ ] `setup/solidtime.md` — Solidtime connection
  - Two auth modes: PAT (legacy) + OAuth (recommended)
  - PAT setup: where to generate, `stint config set solidtime.token`
  - OAuth setup: requires `php artisan passport:client` on Solidtime host (per CLAUDE.md note)
  - Picking org + (eventually) member_id (auto-backfilled now)
  - Verifying connection with `stint config test`
- [ ] `setup/calendar.md` — Calendar integration
  - Currently Google only (MS Graph + CalDAV deferred to Phase 7)
  - Forks must register their own Google OAuth client (per CLAUDE.md note about `STINT_GOOGLE_CLIENT_ID` / `STINT_GOOGLE_CLIENT_SECRET`)
  - For the canonical Reyem Tech build, OAuth is wired and `stint calendar add google` works directly
  - Picking calendars to include
  - Default project for calendar-logged entries
- [ ] `reference/cli.md` — CLI reference
  - All subcommands with arguments, examples, exit codes
  - Generated where possible from `stint <cmd> --help` output (manual paste for v1; auto-generation in a follow-up if useful)
- [ ] `reference/shortcuts.md` — Keyboard shortcuts
  - Main window: ⌘1/2/3 for routes, ⌘, for Settings
  - Popover: Esc to dismiss
  - Tray menu items
- [ ] `help/troubleshooting.md` — Common issues
  - Keychain prompts on every launch (dev-only — direct users to fresh install or `relax-keychain-acl.sh`)
  - "OAuth not configured" for Google Calendar on a fresh fork
  - Sync errors and the in-app `SyncErrorBanner`
  - macOS Spotlight not finding stint (rebuild index — link to general macOS docs)
  - Update install + restart flow (the new v0.2.0 two-step button)
- [ ] `help/faq.md` — FAQ
  - Why macOS only? — Tauri shell + macOS-specific Keychain integration
  - Does stint work offline? — Yes, fully. Sync happens when online.
  - Can I use stint without Solidtime? — Currently no; sync is mandatory. (Standalone-mode is a roadmap candidate.)
  - How is my data stored? — Local SQLite at `~/Library/Application Support/stint/stint.db`; secrets in Keychain.
  - Does stint phone home? — No analytics. Only outbound traffic is to your Solidtime instance + (optional) Google Calendar + the auto-updater check.
  - Where does the CLI binary live after `brew install`? — `/opt/homebrew/bin/stint` (symlinked into Stint.app's embedded CLI).
  - Can I use brew install + curl|sh together? — Yes, they coexist at different paths; whichever is first in PATH wins.

**Acceptance:** `pnpm --filter site build` produces clean output. All sidebar links resolve. Pagefind index includes all pages. Visual spot-check of each page on local server.

### Task 3 — Add `publish-docs.sh` deploy script

- [ ] Create `scripts/release/publish-docs.sh`:
  - Clone `docs-pages` to a temp dir using `GITHUB_TOKEN`
  - Remove all top-level files/dirs EXCEPT `install.sh`, `install.sh.sha256`, `CNAME`, `.github/`
  - Copy `site/dist/*` into the cloned dir
  - Configure git identity (same `release@reyem.tech` / `stint-release-bot` as `publish-install-script.sh`)
  - `git add -A`, commit (with skip if no diff), push
  - Retry once on non-fast-forward push failure (race with install.sh deploy)
- [ ] `chmod +x scripts/release/publish-docs.sh`
- [ ] Shellcheck clean

**Acceptance:** Script can be smoke-tested locally with a dry-run flag (or by pointing `--repo` at a scratch fork). Defer real-world smoke to Task 5 wiring.

### Task 4 — Add `deploy-docs.yml` CI workflow

- [ ] Create `.github/workflows/deploy-docs.yml`:
  - Trigger: `push` to `main` with `paths: [site/**, .github/workflows/deploy-docs.yml]`. Also `workflow_dispatch` for manual rebuilds.
  - Concurrency: `group: docs-pages` (won't conflict with release workflow's `release-${{ github.ref }}` group)
  - Single job on `ubuntu-latest`:
    - Checkout
    - `pnpm/action-setup@v4`
    - `actions/setup-node@v4` with node 20 + pnpm cache
    - `pnpm --filter site install --frozen-lockfile`
    - `pnpm --filter site build`
    - `scripts/release/publish-docs.sh` with `GITHUB_TOKEN`
- [ ] YAML parses cleanly
- [ ] No secrets needed beyond `GITHUB_TOKEN` (which has push access to `docs-pages` via default permissions when scoped to the same repo)

**Acceptance:** Manual `workflow_dispatch` run on the branch succeeds, push to docs-pages happens, Pages deploy triggers, site renders at `stint.reyem.tech`.

### Task 5 — End-to-end deploy verification

- [ ] Push the PR branch with all of the above
- [ ] CI builds Starlight successfully
- [ ] Merge to main triggers `deploy-docs.yml`
- [ ] `docs-pages` branch receives a commit from `stint-release-bot`
- [ ] `deploy-pages.yml` (existing) triggers on the docs-pages push and deploys
- [ ] `https://stint.reyem.tech` renders the new Starlight landing page
- [ ] `https://stint.reyem.tech/install.sh` still returns the installer script (verify: `curl -fsSL https://stint.reyem.tech/install.sh | head -3` shows `#!/bin/sh` + version line)
- [ ] Pagefind search works (type "solidtime" in the search box, results show)
- [ ] Dark mode toggle works
- [ ] All sidebar links resolve (no 404s)

### Task 6 — Update CHANGELOG entry (auto via semrelease) + tag if doing as standalone

This phase ships as part of normal release cadence (one or more `feat:` / `chore:` commits land via PR, semrelease cuts the next version automatically). No special tagging required for the docs site itself.

- [ ] After merge: confirm semrelease cuts a release (likely `feat(docs): ship documentation site` → minor bump unless we frame as `chore`)
- [ ] Optional: `git tag phase-5-complete` from green main once the site is live and verified

---

## Out of scope (deferred to follow-ups)

- README pointer to docs site (one-liner change, do after stint.reyem.tech is live)
- About page footer link to docs site (one-liner change, same timing)
- Screenshot embeds (high maintenance; add once we have a stable visual story)
- Auto-generated CLI reference from `clap` (requires either `clap-markdown` integration in the CLI or manual scripting; manual paste is fine for v1)
- CHANGELOG mirror page (link out to GitHub releases is sufficient)
- Versioned docs (only one supported version at any time — no need)
- i18n (English only)
- Analytics (deliberately none — local-first project)

---

## Open questions

None blocking. Defer the OG card image specifics to Task 2 — pick a reasonable Starlight default and iterate later.

# Stint for Raycast

Five commands to drive [stint](https://github.com/reyemtech/stint) time
tracking from Raycast.

## Install

Until this is in the Raycast Store, install locally:

1. Clone the stint repo.
2. From this directory, `pnpm install --ignore-workspace`.
   (The repo's pnpm-workspace.yaml covers `ui/` and `site/` only — this
   extension is intentionally outside the workspace so it can ship as a
   standalone Raycast package.)
3. In Raycast, run "Import Extension" and select the `raycast-stint/`
   folder.

## Configure

The extension needs the `stint` CLI in your `PATH` or specified in
Raycast preferences. Default discovery order:

- `/usr/local/bin/stint`
- `~/.cargo/bin/stint`
- `/Applications/Stint.app/Contents/MacOS/stint`

## Commands

- **Start Timer** — Form with description, project, task, billable
- **Stop Timer** — One-shot stop
- **Current Timer** — Inspect the running entry
- **Recent Entries** — Browse and restart
- **Switch Project** — Stop and start on a different project

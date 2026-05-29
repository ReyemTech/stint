# Stint for Alfred

Four keyword shortcuts for [stint](https://github.com/reyemtech/stint):

| Keyword | What it does |
|---|---|
| `s <description>` | Start a timer with that description |
| `sstop` | Stop the running timer |
| `scur` | Show the running timer |
| `srec` | List recent entries; ⏎ restarts, ⌥⏎ opens in Stint |

## Install

1. Double-click `Stint.alfredworkflow` from the GitHub Releases page.
2. Alfred prompts to import.
3. Make sure the `stint` CLI is in PATH (or set the Workflow Environment
   Variable `STINT_BIN`).

## First-time setup after import

This directory ships a minimal `info.plist` skeleton — Alfred needs the
four keywords wired to the corresponding scripts. After importing:

1. Open Alfred Preferences → Workflows → Stint.
2. Add four objects:
   - Keyword `s` (argument required) → Run Script (`bash`) →
     `./start.sh "{query}"`.
   - Keyword `sstop` → Run Script (`bash`) → `./stop.sh`.
   - Script Filter, keyword `scur` → `bash` → `./current.sh`. Open URL on
     selection.
   - Script Filter, keyword `srec` → `bash` → `./recent.sh`. ⏎ runs
     `./start.sh "$(./describe.sh {query})"`, ⌥⏎ opens the URL.
3. Export the workflow over this directory to lock in the wiring.

## Build from source

This directory IS the workflow source. Bundle:

```bash
zip -r Stint.alfredworkflow . -x ".*"
```

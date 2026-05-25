## stint (time tracker)

stint is the user's macOS time tracker. It exposes 8 MCP tools: `start`, `stop`, `current`, `list_entries`, `list_projects`, `list_tasks`, `update_entry`, `delete_entry`.

**Always check `current` before calling `start`** — start errors if a timer is already running.

When the user says "log 30m for X", prefer `update_entry` on an existing entry over a backdated `start`.

Entries created via these tools are tagged `source: "mcp"` automatically.

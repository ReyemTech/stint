import { Detail, ActionPanel, Action } from "@raycast/api";
import { useEffect, useState } from "react";
import { stint } from "./lib/stint";
import type { EntryDTO } from "./lib/types";

export default function Command() {
  const [entry, setEntry] = useState<EntryDTO | null>(null);
  const [loading, setLoading] = useState(true);

  async function refresh() {
    try {
      const e = await stint<EntryDTO | null>("current");
      setEntry(e);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    refresh();
    const id = setInterval(refresh, 5000);
    return () => clearInterval(id);
  }, []);

  if (loading) return <Detail isLoading={true} markdown="" />;
  if (!entry) return <Detail markdown="# No active timer" />;

  const start = new Date(entry.start_at);
  const elapsedMins = Math.round((Date.now() - start.getTime()) / 60_000);
  const md = `# ${entry.description || "(no description)"}

**Project:** ${entry.project_id ?? "(none)"}
**Elapsed:** ${elapsedMins} minutes
**Billable:** ${entry.billable ? "yes" : "no"}
**Started:** ${start.toLocaleString()}
`;

  return (
    <Detail
      markdown={md}
      actions={
        <ActionPanel>
          <Action.OpenInBrowser
            url={`stint://entry/${entry.local_uuid}`}
            title="Open in Stint"
          />
        </ActionPanel>
      }
    />
  );
}

import { List, ActionPanel, Action, showToast, Toast } from "@raycast/api";
import { useEffect, useState } from "react";
import { stint } from "./lib/stint";
import type { EntryDTO } from "./lib/types";

export default function Command() {
  const [entries, setEntries] = useState<EntryDTO[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    stint<EntryDTO[]>("list", "--limit", "50")
      .then(setEntries)
      .catch((e) =>
        showToast({
          style: Toast.Style.Failure,
          title: "Failed",
          message: String(e),
        }),
      )
      .finally(() => setLoading(false));
  }, []);

  async function handleRestart(entry: EntryDTO) {
    try {
      await stint("restart", entry.local_uuid);
      await showToast({
        style: Toast.Style.Success,
        title: `Restarted '${entry.description}'`,
      });
    } catch (e) {
      await showToast({
        style: Toast.Style.Failure,
        title: "Restart failed",
        message: String(e),
      });
    }
  }

  return (
    <List isLoading={loading}>
      {entries.map((e) => (
        <List.Item
          key={e.local_uuid}
          title={e.description || "(no description)"}
          subtitle={new Date(e.start_at).toLocaleString()}
          accessories={[{ text: e.project_id ?? "" }]}
          actions={
            <ActionPanel>
              <Action title="Restart" onAction={() => handleRestart(e)} />
              <Action.CopyToClipboard
                content={e.description}
                title="Copy Description"
              />
              <Action.OpenInBrowser
                url={`stint://entry/${e.local_uuid}`}
                title="Open in Stint"
              />
            </ActionPanel>
          }
        />
      ))}
    </List>
  );
}

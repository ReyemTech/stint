import { showToast, Toast } from "@raycast/api";
import { stint } from "./lib/stint";
import type { EntryDTO } from "./lib/types";

export default async function Command() {
  try {
    const entry = await stint<EntryDTO>("stop");
    const start = new Date(entry.start_at);
    const end = entry.end_at ? new Date(entry.end_at) : new Date();
    const mins = Math.round((end.getTime() - start.getTime()) / 60_000);
    await showToast({
      style: Toast.Style.Success,
      title: `Stopped (${mins}m)`,
      message: entry.description,
    });
  } catch (e) {
    await showToast({
      style: Toast.Style.Failure,
      title: "Failed to stop",
      message: String(e),
    });
  }
}

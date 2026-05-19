import { openUrl } from "@tauri-apps/plugin-opener";
import { api } from "~/api";

/// Opens the configured Solidtime instance in the user's default browser.
/// No-op if URL isn't configured yet. Optional `path` is appended.
export async function openSolidtime(path?: string): Promise<void> {
  const url = await api.solidtimeUrl();
  if (!url) return;
  const target = path ? `${url}${path.startsWith("/") ? path : `/${path}`}` : url;
  await openUrl(target);
}

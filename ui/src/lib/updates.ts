import { invoke } from "@tauri-apps/api/core";

export interface UpdateInfo {
  available: boolean;
  current_version: string;
  latest_version: string | null;
  notes: string | null;
}

export type Channel = "stable" | "beta";

export async function checkForUpdates(): Promise<UpdateInfo> {
  const channel = await getChannel();
  return invoke<UpdateInfo>("check_for_updates", { channel });
}

export async function applyUpdate(): Promise<void> {
  const channel = await getChannel();
  await invoke("apply_update", { channel });
}

export async function getChannel(): Promise<Channel> {
  const value = await invoke<string | null>("settings_get", { key: "update.channel" });
  return value === "beta" ? "beta" : "stable";
}

export async function setChannel(channel: Channel): Promise<void> {
  await invoke("settings_set", { key: "update.channel", value: channel });
}

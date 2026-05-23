import { createSignal } from "solid-js";
import { invoke } from "@tauri-apps/api/core";

export interface UpdateInfo {
  available: boolean;
  current_version: string;
  latest_version: string | null;
  notes: string | null;
}

export type Channel = "stable" | "beta";

/**
 * Monotonic counter incremented whenever something asks for an explicit
 * update check (menu item, tray item, etc.). UpdatesPanel reacts to the
 * counter so the check fires even if the panel mounts AFTER the request
 * lands. Module-level so the signal survives panel mount/unmount cycles.
 */
const [checkRequested, setCheckRequested] = createSignal(0);
export { checkRequested };

export function requestCheckForUpdates(): void {
  setCheckRequested((n) => n + 1);
}

export async function checkForUpdates(): Promise<UpdateInfo> {
  const channel = await getChannel();
  return invoke<UpdateInfo>("check_for_updates", { channel });
}

export async function installUpdate(): Promise<void> {
  const channel = await getChannel();
  await invoke("install_update", { channel });
}

export async function restartApp(): Promise<void> {
  await invoke("restart_app");
}

export async function getChannel(): Promise<Channel> {
  const value = await invoke<string | null>("settings_get", { key: "update.channel" });
  return value === "beta" ? "beta" : "stable";
}

export async function setChannel(channel: Channel): Promise<void> {
  await invoke("settings_set", { key: "update.channel", value: channel });
}

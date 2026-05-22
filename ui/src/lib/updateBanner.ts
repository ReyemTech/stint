import { createSignal, onCleanup, onMount } from "solid-js";
import { type UpdateInfo, checkForUpdates } from "./updates";

const CHECK_INTERVAL_MS = 24 * 60 * 60 * 1000; // 24h
const STARTUP_DELAY_MS = 5_000;

/**
 * Polls for app updates and exposes the latest result as a signal.
 *
 * - Runs an initial check ~5s after mount (gives the app room to settle).
 * - Re-polls every 24h after that.
 * - Errors are swallowed: an auto-check failing should never surface noise
 *   to the user. The Settings "Check now" button is the explicit path
 *   that does surface errors.
 *
 * Must be called inside a SolidJS reactive scope (a component) because
 * it relies on `onMount` / `onCleanup`.
 */
export function useUpdateBanner() {
  const [info, setInfo] = createSignal<UpdateInfo | null>(null);

  const check = async () => {
    try {
      const result = await checkForUpdates();
      if (result.available) setInfo(result);
    } catch {
      /* silent on auto-check */
    }
  };

  onMount(() => {
    const startupTimer = setTimeout(check, STARTUP_DELAY_MS);
    const interval = setInterval(check, CHECK_INTERVAL_MS);
    onCleanup(() => {
      clearTimeout(startupTimer);
      clearInterval(interval);
    });
  });

  return info;
}

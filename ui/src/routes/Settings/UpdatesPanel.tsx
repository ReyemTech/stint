import { Show, createResource, createSignal } from "solid-js";
import Button from "~/components/ui/Button";
import {
  type Channel,
  type UpdateInfo,
  applyUpdate,
  checkForUpdates,
  getChannel,
  setChannel,
} from "~/lib/updates";

/**
 * Settings panel for the auto-updater. Lets the user pick an update channel,
 * trigger a check, and install when one is available. Mirrors the styling of
 * the existing Solidtime + Calendar sections (see Settings.tsx).
 */
export default function UpdatesPanel() {
  const [channel, { refetch: refetchChannel }] = createResource(getChannel);
  const [info, setInfo] = createSignal<UpdateInfo | undefined>();
  const [checking, setChecking] = createSignal(false);
  const [installing, setInstalling] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  const check = async () => {
    setChecking(true);
    setError(null);
    try {
      setInfo(await checkForUpdates());
    } catch (e) {
      setError((e as { message?: string }).message ?? String(e));
    } finally {
      setChecking(false);
    }
  };

  const switchChannel = async (next: Channel) => {
    try {
      await setChannel(next);
      await refetchChannel();
      setInfo(undefined);
      setError(null);
    } catch (e) {
      setError((e as { message?: string }).message ?? String(e));
    }
  };

  const install = async () => {
    setInstalling(true);
    setError(null);
    try {
      await applyUpdate();
    } catch (e) {
      setError((e as { message?: string }).message ?? String(e));
    } finally {
      setInstalling(false);
    }
  };

  return (
    <div class="space-y-5">
      <div class="grid grid-cols-3 gap-4">
        <div>
          <label class="block text-sm font-medium text-zinc-900 dark:text-zinc-100">
            Channel
          </label>
          <p class="mt-0.5 text-xs text-zinc-500">
            Stable is the default. Beta receives pre-release builds.
          </p>
        </div>
        <div class="col-span-2">
          <select
            class="w-full rounded-md border border-zinc-300 bg-white px-3 py-1.5 text-sm focus:border-zinc-900 focus:outline-none dark:border-zinc-700 dark:bg-zinc-950 dark:focus:border-zinc-100"
            value={channel() ?? "stable"}
            onChange={(e) =>
              switchChannel(e.currentTarget.value as Channel)
            }
          >
            <option value="stable">Stable</option>
            <option value="beta">Beta</option>
          </select>
          <Show when={channel() === "beta"}>
            <p class="mt-2 text-xs text-amber-600 dark:text-amber-400">
              Switching back to Stable won't downgrade you. Reinstall via
              Homebrew or DMG to return to the current stable release.
            </p>
          </Show>
        </div>
      </div>

      <Show when={info()?.available} fallback={
        <Show when={info() && !info()!.available}>
          <p class="text-xs text-zinc-500">
            You're on the latest version ({info()!.current_version}).
          </p>
        </Show>
      }>
        <div class="rounded-md border border-emerald-300 bg-emerald-50 p-3 text-sm dark:border-emerald-900 dark:bg-emerald-950">
          <p class="font-medium text-emerald-900 dark:text-emerald-200">
            Update available: {info()!.latest_version}
          </p>
          <Show when={info()!.notes}>
            <pre class="mt-1 whitespace-pre-wrap text-xs text-zinc-700 dark:text-zinc-300">
              {info()!.notes}
            </pre>
          </Show>
          <div class="mt-3">
            <Button
              variant="primary"
              size="sm"
              disabled={installing()}
              onClick={install}
            >
              {installing() ? "Installing…" : "Install & restart"}
            </Button>
          </div>
        </div>
      </Show>

      <Show when={error()}>
        <p class="text-xs text-red-600 dark:text-red-400">{error()}</p>
      </Show>

      <div class="flex items-center gap-3 border-t border-black/[0.05] pt-4 dark:border-white/[0.04]">
        <Button onClick={check} disabled={checking()}>
          {checking() ? "Checking…" : "Check now"}
        </Button>
        <p class="text-xs text-zinc-500">
          Current version: {info()?.current_version ?? "—"}
        </p>
      </div>
    </div>
  );
}

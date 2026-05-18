import { For, Show, createResource, createSignal } from "solid-js";
import { api } from "~/api";

export default function Settings() {
  const [config, { refetch }] = createResource(() => api.configShow());
  const [status, setStatus] = createSignal<string | null>(null);

  async function setValue(key: string, value: string) {
    await api.configSet(key, value);
    setStatus(`Saved ${key}.`);
    refetch();
  }

  async function test() {
    try {
      const who = await api.configTest();
      setStatus(`✓ connected as ${who}`);
    } catch (e) {
      setStatus(`✗ ${(e as { message: string }).message}`);
    }
  }

  return (
    <div class="mx-auto max-w-2xl p-6">
      <header class="mb-4 flex items-baseline justify-between">
        <h1 class="text-lg font-semibold">Settings</h1>
        <nav class="text-xs text-zinc-500">
          <a class="mr-3 hover:underline" href="/#/today">Today</a>
          <a class="hover:underline" href="/#/settings">Settings</a>
        </nav>
      </header>

      <Show when={status()}>
        <div class="mb-3 text-xs text-zinc-500">{status()}</div>
      </Show>

      <Show when={config()}>
        <ul class="space-y-2">
          <For each={config()!}>
            {(c) => (
              <li class="flex items-center gap-2">
                <label class="w-48 text-xs text-zinc-500">{c.key}</label>
                <Show
                  when={!c.is_secret}
                  fallback={
                    <input
                      type="password"
                      placeholder={c.present ? "•••• (set)" : "(unset)"}
                      class="flex-1 rounded border border-zinc-300 px-2 py-1 text-sm dark:border-zinc-700 dark:bg-zinc-800"
                      onChange={(e) => setValue(c.key, e.currentTarget.value)}
                    />
                  }
                >
                  <input
                    value={c.value ?? ""}
                    class="flex-1 rounded border border-zinc-300 px-2 py-1 text-sm dark:border-zinc-700 dark:bg-zinc-800"
                    onChange={(e) => setValue(c.key, e.currentTarget.value)}
                  />
                </Show>
              </li>
            )}
          </For>
        </ul>
      </Show>

      <button
        class="mt-4 rounded bg-zinc-900 px-3 py-1 text-sm text-white dark:bg-white dark:text-zinc-900"
        onClick={test}
      >
        Test connection
      </button>
    </div>
  );
}

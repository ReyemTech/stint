import { createResource, For } from "solid-js";
import { getVersion, getTauriVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import MainNav from "~/components/MainNav";
import Button from "~/components/ui/Button";
import SectionLabel from "~/components/ui/SectionLabel";
import { openSolidtime } from "~/lib/openSolidtime";

const CREDITS = [
  { name: "Tauri", purpose: "macOS shell + IPC", url: "https://tauri.app" },
  { name: "Rust", purpose: "core, store, sync", url: "https://www.rust-lang.org" },
  { name: "SolidJS", purpose: "reactive UI", url: "https://www.solidjs.com" },
  { name: "Tailwind CSS", purpose: "styling", url: "https://tailwindcss.com" },
  { name: "SQLite", purpose: "local persistence", url: "https://www.sqlite.org" },
  { name: "Solidtime", purpose: "sync target", url: "https://www.solidtime.io" },
];

export default function About() {
  const [appVersion] = createResource(() => getVersion(), {
    initialValue: "0.0.0",
  });
  const [tauriVersion] = createResource(() => getTauriVersion(), {
    initialValue: "",
  });

  return (
    <div class="min-h-screen bg-zinc-50/60 dark:bg-zinc-950">
      <div class="mx-auto max-w-3xl px-6 py-8">
        <header class="mb-6 flex items-center justify-between">
          <div>
            <h1 class="text-2xl font-semibold tracking-tight">About</h1>
            <p class="mt-0.5 text-xs text-zinc-500 dark:text-zinc-400">
              What this is and what it's built with.
            </p>
          </div>
          <MainNav active="about" />
        </header>

        {/* Identity card */}
        <section class="mb-6 rounded-2xl border border-black/[0.06] bg-white p-6 dark:border-white/[0.06] dark:bg-zinc-900">
          <div class="flex items-start gap-5">
            <div class="flex h-16 w-16 shrink-0 items-center justify-center rounded-2xl bg-gradient-to-br from-indigo-500 to-violet-600 text-2xl font-semibold text-white shadow-sm">
              s
            </div>
            <div class="flex-1">
              <h2 class="text-xl font-semibold tracking-tight">Stint</h2>
              <p class="mt-0.5 text-sm text-zinc-500 dark:text-zinc-400">
                A macOS time tracker that syncs with a self-hosted Solidtime
                instance. Menu bar + main window + CLI, sharing one local
                database.
              </p>
              <dl class="mt-4 grid grid-cols-2 gap-x-6 gap-y-2 text-xs">
                <div>
                  <dt class="text-zinc-400 dark:text-zinc-500">Version</dt>
                  <dd class="font-mono tabular-nums text-zinc-700 dark:text-zinc-200">
                    {appVersion()}
                  </dd>
                </div>
                <div>
                  <dt class="text-zinc-400 dark:text-zinc-500">Tauri</dt>
                  <dd class="font-mono tabular-nums text-zinc-700 dark:text-zinc-200">
                    {tauriVersion()}
                  </dd>
                </div>
                <div>
                  <dt class="text-zinc-400 dark:text-zinc-500">Author</dt>
                  <dd class="text-zinc-700 dark:text-zinc-200">Reyem Tech</dd>
                </div>
                <div>
                  <dt class="text-zinc-400 dark:text-zinc-500">License</dt>
                  <dd class="text-zinc-700 dark:text-zinc-200">MIT</dd>
                </div>
              </dl>
            </div>
          </div>

          <div class="mt-5 flex flex-wrap gap-2 border-t border-black/[0.05] pt-4 dark:border-white/[0.04]">
            <Button onClick={() => openSolidtime()}>Open Solidtime</Button>
            <Button
              variant="secondary"
              onClick={() => openUrl("https://github.com/reyemtech/stint")}
            >
              GitHub
            </Button>
            <Button
              variant="secondary"
              onClick={() => openUrl("https://github.com/reyemtech/stint/issues")}
            >
              Report an issue
            </Button>
          </div>
        </section>

        {/* Built with */}
        <section class="rounded-2xl border border-black/[0.06] bg-white p-6 dark:border-white/[0.06] dark:bg-zinc-900">
          <div class="mb-3">
            <SectionLabel>Built with</SectionLabel>
          </div>
          <ul class="divide-y divide-black/[0.04] dark:divide-white/[0.04]">
            <For each={CREDITS}>
              {(c) => (
                <li class="flex items-center justify-between py-2">
                  <div>
                    <div class="text-sm font-medium text-zinc-900 dark:text-zinc-100">
                      {c.name}
                    </div>
                    <div class="text-xs text-zinc-500 dark:text-zinc-400">
                      {c.purpose}
                    </div>
                  </div>
                  <button
                    class="rounded-md px-2 py-1 text-xs text-zinc-500 transition hover:bg-zinc-100 hover:text-zinc-900 dark:text-zinc-400 dark:hover:bg-zinc-800 dark:hover:text-zinc-100"
                    onClick={() => openUrl(c.url)}
                  >
                    Visit ↗
                  </button>
                </li>
              )}
            </For>
          </ul>
        </section>

        <p class="mt-6 text-center text-[11px] text-zinc-400 dark:text-zinc-500">
          Made with care · Phase 1 + 2 shipped
        </p>
      </div>
    </div>
  );
}

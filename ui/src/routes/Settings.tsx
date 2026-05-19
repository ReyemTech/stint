import { For, Show, createResource, createSignal } from "solid-js";
import { api } from "~/api";

type FieldKind = "text" | "password";

type FieldDef = {
  key: string;
  label: string;
  hint?: string;
  placeholder?: string;
  kind: FieldKind;
};

const FIELDS: FieldDef[] = [
  {
    key: "solidtime.url",
    label: "Solidtime URL",
    hint: "Base URL of your self-hosted Solidtime instance.",
    placeholder: "https://time.example.com",
    kind: "text",
  },
  {
    key: "solidtime.token",
    label: "API token",
    hint: "Personal access token. Stored in macOS Keychain — never in the database.",
    placeholder: "Paste token, press Enter",
    kind: "password",
  },
  {
    key: "solidtime.org",
    label: "Organization ID",
    hint: "UUID of the organization you want to log time into.",
    placeholder: "00000000-0000-0000-0000-000000000000",
    kind: "text",
  },
  {
    key: "solidtime.default-project",
    label: "Default project ID",
    hint: "Optional. New timers pre-fill this project if no other is given.",
    placeholder: "(none)",
    kind: "text",
  },
];

export default function Settings() {
  const [config, { refetch }] = createResource(() => api.configShow());
  const [status, setStatus] = createSignal<string | null>(null);
  const [statusKind, setStatusKind] = createSignal<"ok" | "err" | null>(null);

  const lookup = (key: string) => {
    const list = config();
    if (!list) return undefined;
    return list.find((c) => c.key === key);
  };

  async function setValue(key: string, value: string) {
    if (!value.trim()) return;
    try {
      await api.configSet(key, value.trim());
      setStatusKind("ok");
      setStatus(`Saved ${key}.`);
      refetch();
    } catch (e) {
      setStatusKind("err");
      setStatus(`Failed to save ${key}: ${(e as { message: string }).message}`);
    }
  }

  async function test() {
    setStatusKind(null);
    setStatus("Testing…");
    try {
      const who = await api.configTest();
      setStatusKind("ok");
      setStatus(`✓ Connected as ${who}`);
    } catch (e) {
      setStatusKind("err");
      setStatus(`✗ ${(e as { message: string }).message}`);
    }
  }

  async function syncNow() {
    setStatusKind(null);
    setStatus("Draining sync queue…");
    try {
      const n = await api.syncNow();
      setStatusKind("ok");
      setStatus(`✓ Synced ${n} item${n === 1 ? "" : "s"}`);
    } catch (e) {
      setStatusKind("err");
      setStatus(`✗ ${(e as { message: string }).message}`);
    }
  }

  return (
    <div class="mx-auto max-w-2xl p-6">
      <header class="mb-6 flex items-baseline justify-between">
        <h1 class="text-xl font-semibold">Settings</h1>
        <nav class="text-sm text-zinc-500">
          <a class="mr-4 hover:text-zinc-900 dark:hover:text-zinc-100" href="#/today">
            Today
          </a>
          <a class="text-zinc-900 dark:text-zinc-100" href="#/settings">
            Settings
          </a>
        </nav>
      </header>

      <Show when={status()}>
        <div
          class="mb-4 rounded-md border px-3 py-2 text-sm"
          classList={{
            "border-green-200 bg-green-50 text-green-800 dark:border-green-900 dark:bg-green-950 dark:text-green-200":
              statusKind() === "ok",
            "border-red-200 bg-red-50 text-red-800 dark:border-red-900 dark:bg-red-950 dark:text-red-200":
              statusKind() === "err",
            "border-zinc-200 bg-zinc-50 text-zinc-700 dark:border-zinc-800 dark:bg-zinc-900 dark:text-zinc-300":
              statusKind() === null,
          }}
        >
          {status()}
        </div>
      </Show>

      <section class="rounded-lg border border-zinc-200 bg-white p-5 shadow-sm dark:border-zinc-800 dark:bg-zinc-900">
        <h2 class="mb-1 text-sm font-semibold uppercase tracking-wide text-zinc-500">
          Solidtime connection
        </h2>
        <p class="mb-5 text-xs text-zinc-500">
          Saved values apply to both the CLI and this app. Both share the same database and Keychain.
        </p>

        <div class="space-y-5">
          <For each={FIELDS}>
            {(f) => {
              const current = () => lookup(f.key);
              const initial = () => current()?.value ?? "";
              const isSet = () => {
                const c = current();
                if (!c) return false;
                return c.is_secret ? c.present : Boolean(c.value);
              };
              return (
                <FieldRow
                  def={f}
                  initialValue={initial()}
                  isSet={isSet()}
                  onSave={(v) => setValue(f.key, v)}
                />
              );
            }}
          </For>
        </div>

        <div class="mt-6 flex items-center gap-3 border-t border-zinc-200 pt-4 dark:border-zinc-800">
          <button
            class="rounded-md bg-zinc-900 px-3 py-1.5 text-sm font-semibold text-white transition hover:bg-zinc-700 dark:bg-white dark:text-zinc-900 dark:hover:bg-zinc-200"
            onClick={test}
          >
            Test connection
          </button>
          <button
            class="rounded-md border border-zinc-300 px-3 py-1.5 text-sm transition hover:bg-zinc-50 dark:border-zinc-700 dark:hover:bg-zinc-800"
            onClick={syncNow}
          >
            Sync now
          </button>
        </div>
      </section>
    </div>
  );
}

function FieldRow(props: {
  def: FieldDef;
  initialValue: string;
  isSet: boolean;
  onSave: (value: string) => Promise<void>;
}) {
  const [value, setValue] = createSignal(props.initialValue);
  const [editing, setEditing] = createSignal(false);

  return (
    <div class="grid grid-cols-3 gap-4">
      <div>
        <label class="block text-sm font-medium text-zinc-900 dark:text-zinc-100">
          {props.def.label}
        </label>
        <Show when={props.def.hint}>
          <p class="mt-0.5 text-xs text-zinc-500">{props.def.hint}</p>
        </Show>
      </div>
      <div class="col-span-2">
        <input
          type={props.def.kind === "password" ? "password" : "text"}
          class="w-full rounded-md border border-zinc-300 bg-white px-3 py-1.5 text-sm placeholder:text-zinc-400 focus:border-zinc-900 focus:outline-none dark:border-zinc-700 dark:bg-zinc-950 dark:focus:border-zinc-100"
          placeholder={
            props.def.kind === "password" && props.isSet
              ? "•••• (set in Keychain — type to replace)"
              : props.def.placeholder ?? ""
          }
          value={props.def.kind === "password" ? "" : value()}
          onInput={(e) => {
            setValue(e.currentTarget.value);
            setEditing(true);
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              (e.currentTarget as HTMLInputElement).blur();
            }
          }}
          onBlur={async () => {
            if (!editing()) return;
            await props.onSave(value());
            setEditing(false);
            if (props.def.kind === "password") setValue("");
          }}
        />
        <Show when={props.isSet && props.def.kind !== "password"}>
          <p class="mt-1 text-xs text-zinc-400">Currently: {value()}</p>
        </Show>
      </div>
    </div>
  );
}

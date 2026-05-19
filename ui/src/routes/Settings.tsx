import {
  For,
  Show,
  createEffect,
  createMemo,
  createResource,
  createSignal,
} from "solid-js";
import { api } from "~/api";
import Button from "~/components/ui/Button";
import type { OrgChoice, Project } from "~/types";

const LABELS: Record<string, string> = {
  "solidtime.url": "Solidtime URL",
  "solidtime.token": "API token",
  "solidtime.org": "Organization",
  "solidtime.member_id": "Membership",
  "solidtime.default-project": "Default project",
};
const labelFor = (key: string) => LABELS[key] ?? key;

export default function Settings() {
  const [config, { refetch: refetchConfig }] = createResource(() => api.configShow());
  const [status, setStatus] = createSignal<string | null>(null);
  const [statusKind, setStatusKind] = createSignal<"ok" | "err" | "info" | null>(
    null,
  );

  const lookup = (key: string) => config()?.find((c) => c.key === key);
  const urlSet = () => Boolean(lookup("solidtime.url")?.value);
  const tokenSet = () => Boolean(lookup("solidtime.token")?.present);
  const orgId = () => lookup("solidtime.org")?.value ?? "";
  const defaultProjectId = () => lookup("solidtime.default-project")?.value ?? "";
  const canFetchOrgs = createMemo(() => urlSet() && tokenSet());

  // Organizations: fetched once URL+token are set.
  const [orgs, { refetch: refetchOrgs }] = createResource<OrgChoice[], boolean>(
    canFetchOrgs,
    async (enabled): Promise<OrgChoice[]> => {
      if (!enabled) return [];
      try {
        return await api.listOrganizations();
      } catch {
        return [];
      }
    },
  );

  // Projects for the chosen org: refetched whenever org changes.
  const [projects, { refetch: refetchProjects }] = createResource<Project[], boolean>(
    () => Boolean(orgId()),
    async (enabled): Promise<Project[]> => {
      if (!enabled) return [];
      try {
        await api.refreshProjects();
        return await api.listProjects();
      } catch {
        try {
          return await api.listProjects();
        } catch {
          return [];
        }
      }
    },
  );

  const orgList = () => orgs() ?? [];
  const projectList = () => projects() ?? [];

  // Backfill: when memberships first load and the user already has an org
  // saved but no member_id, set it automatically. Without member_id every
  // time-entry write to Solidtime fails with 422.
  createEffect(() => {
    const list = orgList();
    if (list.length === 0) return;
    const currentOrg = orgId();
    if (!currentOrg) return;
    const memberSet = lookup("solidtime.member_id")?.value;
    if (memberSet) return;
    const m = list.find((o) => o.id === currentOrg);
    if (m) {
      void saveValue("solidtime.member_id", m.member_id);
    }
  });

  function flash(kind: "ok" | "err" | "info", msg: string) {
    setStatusKind(kind);
    setStatus(msg);
  }

  async function saveValue(key: string, value: string) {
    try {
      await api.configSet(key, value);
      flash("ok", `Saved ${labelFor(key)}.`);
      refetchConfig();
      if (key === "solidtime.token" || key === "solidtime.url") refetchOrgs();
      if (key === "solidtime.org") refetchProjects();
    } catch (e) {
      flash(
        "err",
        `Failed to save ${labelFor(key)}: ${(e as { message: string }).message}`,
      );
    }
  }

  async function testConnection() {
    flash("info", "Testing connection…");
    try {
      const who = await api.configTest();
      flash("ok", `✓ Connected as ${who}`);
      refetchOrgs();
    } catch (e) {
      flash("err", `✗ ${(e as { message: string }).message}`);
    }
  }

  async function syncNow() {
    flash("info", "Draining sync queue…");
    try {
      const n = await api.syncNow();
      flash("ok", `✓ Synced ${n} item${n === 1 ? "" : "s"}`);
    } catch (e) {
      flash("err", `✗ ${(e as { message: string }).message}`);
    }
  }

  return (
    <div class="min-h-screen bg-zinc-50/60 dark:bg-zinc-950">
      <div class="mx-auto max-w-3xl px-6 py-8">
        <header class="mb-6 flex items-center justify-between">
          <div>
            <h1 class="text-2xl font-semibold tracking-tight">Settings</h1>
            <p class="mt-0.5 text-xs text-zinc-500 dark:text-zinc-400">
              Connection, organization, and defaults.
            </p>
          </div>
          <nav class="flex items-center gap-1 text-xs">
            <a
              class="rounded-md px-2.5 py-1.5 text-zinc-500 transition hover:bg-zinc-100 hover:text-zinc-900 dark:text-zinc-400 dark:hover:bg-zinc-800 dark:hover:text-zinc-100"
              href="#/today"
            >
              Today
            </a>
            <a
              class="rounded-md px-2.5 py-1.5 text-zinc-900 dark:text-zinc-100"
              href="#/settings"
            >
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
              statusKind() === "info" || statusKind() === null,
          }}
        >
          {status()}
        </div>
      </Show>

      <section class="rounded-2xl border border-black/[0.06] bg-white p-6 dark:border-white/[0.06] dark:bg-zinc-900">
        <h2 class="mb-1 text-sm font-semibold uppercase tracking-wide text-zinc-500">
          Solidtime connection
        </h2>
        <p class="mb-5 text-xs text-zinc-500">
          Saved values apply to both the CLI and this app — they share the same
          database and Keychain.
        </p>

        <div class="space-y-5">
          <TextField
            label="Solidtime URL"
            hint="Base URL of your self-hosted Solidtime instance."
            placeholder="https://time.example.com"
            value={lookup("solidtime.url")?.value ?? ""}
            onSave={(v) => saveValue("solidtime.url", v)}
          />

          <SecretField
            label="API token"
            hint="Personal access token. Stored in macOS Keychain — never in the database."
            isSet={tokenSet()}
            onSave={(v) => saveValue("solidtime.token", v)}
          />

          <Show
            when={canFetchOrgs() && orgList().length > 0}
            fallback={
              <TextField
                label="Organization ID"
                hint={
                  canFetchOrgs()
                    ? orgs.loading
                      ? "Loading organizations…"
                      : "Couldn't load orgs — paste UUID manually or click Test connection."
                    : "Set URL + token first; once connected this becomes a dropdown."
                }
                placeholder="00000000-0000-0000-0000-000000000000"
                value={orgId()}
                onSave={(v) => saveValue("solidtime.org", v)}
              />
            }
          >
            <SelectField
              label="Organization"
              hint="Time entries are logged into this org."
              value={orgId()}
              options={orgList().map((o) => ({ value: o.id, label: o.name }))}
              onChange={async (v) => {
                // Save org and the matching membership id together — the API
                // requires member_id on every time-entry write.
                const m = orgList().find((o) => o.id === v);
                await saveValue("solidtime.org", v);
                if (m) await saveValue("solidtime.member_id", m.member_id);
              }}
              placeholder="Select an organization…"
            />
          </Show>

          <Show
            when={Boolean(orgId()) && projectList().length > 0}
            fallback={
              <Show when={Boolean(orgId())}>
                <TextField
                  label="Default project ID"
                  hint={
                    projects.loading
                      ? "Loading projects…"
                      : "Optional. Couldn't load projects — paste UUID manually or save the org first."
                  }
                  placeholder="(none)"
                  value={defaultProjectId()}
                  onSave={(v) => saveValue("solidtime.default-project", v)}
                />
              </Show>
            }
          >
            <SelectField
              label="Default project"
              hint="Optional. New timers pre-fill this project."
              value={defaultProjectId()}
              options={[
                { value: "", label: "(none)" },
                ...projectList().map((p) => ({ value: p.id, label: p.name })),
              ]}
              onChange={(v) => saveValue("solidtime.default-project", v)}
              placeholder="Select a project…"
            />
          </Show>
        </div>

        <div class="mt-6 flex items-center gap-3 border-t border-black/[0.05] pt-4 dark:border-white/[0.04]">
          <Button onClick={testConnection}>Test connection</Button>
          <Button variant="secondary" onClick={syncNow}>Sync now</Button>
        </div>
      </section>
      </div>
    </div>
  );
}

function FieldShell(props: { label: string; hint?: string; children: any }) {
  return (
    <div class="grid grid-cols-3 gap-4">
      <div>
        <label class="block text-sm font-medium text-zinc-900 dark:text-zinc-100">
          {props.label}
        </label>
        <Show when={props.hint}>
          <p class="mt-0.5 text-xs text-zinc-500">{props.hint}</p>
        </Show>
      </div>
      <div class="col-span-2">{props.children}</div>
    </div>
  );
}

function TextField(props: {
  label: string;
  hint?: string;
  placeholder?: string;
  value: string;
  onSave: (value: string) => void | Promise<void>;
}) {
  const [draft, setDraft] = createSignal(props.value);
  const [dirty, setDirty] = createSignal(false);
  return (
    <FieldShell label={props.label} hint={props.hint}>
      <input
        type="text"
        class="w-full rounded-md border border-zinc-300 bg-white px-3 py-1.5 text-sm placeholder:text-zinc-400 focus:border-zinc-900 focus:outline-none dark:border-zinc-700 dark:bg-zinc-950 dark:focus:border-zinc-100"
        placeholder={props.placeholder ?? ""}
        value={dirty() ? draft() : props.value}
        onInput={(e) => {
          setDraft(e.currentTarget.value);
          setDirty(true);
        }}
        onKeyDown={(e) => {
          if (e.key === "Enter") (e.currentTarget as HTMLInputElement).blur();
        }}
        onBlur={async () => {
          if (!dirty()) return;
          await props.onSave(draft().trim());
          setDirty(false);
        }}
      />
    </FieldShell>
  );
}

function SecretField(props: {
  label: string;
  hint?: string;
  isSet: boolean;
  onSave: (value: string) => void | Promise<void>;
}) {
  const [draft, setDraft] = createSignal("");
  return (
    <FieldShell label={props.label} hint={props.hint}>
      <input
        type="password"
        class="w-full rounded-md border border-zinc-300 bg-white px-3 py-1.5 text-sm placeholder:text-zinc-400 focus:border-zinc-900 focus:outline-none dark:border-zinc-700 dark:bg-zinc-950 dark:focus:border-zinc-100"
        placeholder={
          props.isSet ? "•••• (set in Keychain — type to replace)" : "Paste token"
        }
        value={draft()}
        onInput={(e) => setDraft(e.currentTarget.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") (e.currentTarget as HTMLInputElement).blur();
        }}
        onBlur={async () => {
          if (!draft().trim()) return;
          await props.onSave(draft().trim());
          setDraft("");
        }}
      />
    </FieldShell>
  );
}

function SelectField(props: {
  label: string;
  hint?: string;
  value: string;
  options: { value: string; label: string }[];
  placeholder?: string;
  onChange: (value: string) => void | Promise<void>;
}) {
  return (
    <FieldShell label={props.label} hint={props.hint}>
      <select
        class="w-full rounded-md border border-zinc-300 bg-white px-3 py-1.5 text-sm focus:border-zinc-900 focus:outline-none dark:border-zinc-700 dark:bg-zinc-950 dark:focus:border-zinc-100"
        value={props.value}
        onChange={(e) => props.onChange(e.currentTarget.value)}
      >
        <Show when={!props.value}>
          <option value="" disabled>
            {props.placeholder ?? "Select…"}
          </option>
        </Show>
        <For each={props.options}>
          {(o) => <option value={o.value}>{o.label}</option>}
        </For>
      </select>
    </FieldShell>
  );
}

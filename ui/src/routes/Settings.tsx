import {
  For,
  Show,
  createEffect,
  createMemo,
  createResource,
  createSignal,
} from "solid-js";
import { Popover } from "@kobalte/core/popover";
import { api, calendarApi, oauthSolidtimeLogout, oauthSolidtimeStart, oauthSolidtimeStatus } from "~/api";
import MainNav from "~/components/MainNav";
import Accordion from "~/components/ui/Accordion";
import Button from "~/components/ui/Button";
import Pill from "~/components/ui/Pill";
import ProjectPicker from "~/components/ui/ProjectPicker";
import Toggle from "~/components/ui/Toggle";
import IntegrationsPanel from "~/routes/Settings/IntegrationsPanel";
import UpdatesPanel from "~/routes/Settings/UpdatesPanel";
import type { CalendarAccount, CalendarRow, OrgChoice, Project } from "~/types";

const LABELS: Record<string, string> = {
  "solidtime.url": "Solidtime URL",
  "solidtime.token": "API token",
  "solidtime.org": "Organization",
  "solidtime.member_id": "Membership",
  "solidtime.default-project": "Default project",
  "idle.enabled": "Idle detection",
  "idle.threshold_secs": "Idle threshold",
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

  // Idle detection — default enabled, threshold 600s. Stored as plain
  // strings in the settings table; we coerce here so the controls work
  // against typed values.
  const idleEnabled = () => lookup("idle.enabled")?.value !== "false";
  const idleThreshold = () => {
    const raw = lookup("idle.threshold_secs")?.value;
    const n = raw ? parseInt(raw, 10) : NaN;
    return Number.isFinite(n) ? n : 600;
  };

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

  // OAuth auth status
  const [authStatus, { refetch: refetchAuthStatus }] = createResource(() =>
    oauthSolidtimeStatus(),
  );
  const [authMode, setAuthMode] = createSignal<"api_token" | "oauth">(
    "api_token",
  );

  createEffect(() => {
    const s = authStatus();
    if (s) setAuthMode(s.mode);
  });

  async function handleSignIn() {
    try {
      await oauthSolidtimeStart();
      await refetchAuthStatus();
    } catch (e) {
      flash("err", `OAuth sign-in failed: ${(e as { message: string }).message}`);
    }
  }

  async function handleSignOut() {
    try {
      await oauthSolidtimeLogout();
      await refetchAuthStatus();
    } catch (e) {
      flash("err", `Sign-out failed: ${(e as { message: string }).message}`);
    }
  }

  // Calendar accounts
  const [accounts, { refetch: refetchAccounts }] = createResource(() =>
    calendarApi.listAccounts(),
  );

  async function handleAddGoogle() {
    flash("info", "Opening Google sign-in…");
    try {
      const a = await calendarApi.addGoogle();
      flash("ok", `Connected Google account: ${a.identifier}`);
      refetchAccounts();
    } catch (e) {
      flash("err", `Failed: ${(e as { message: string }).message}`);
    }
  }

  async function handleRemoveAccount(id: string) {
    try {
      await calendarApi.removeAccount(id);
      flash("ok", "Account removed.");
      refetchAccounts();
    } catch (e) {
      console.error("calendar removeAccount failed:", e);
      flash("err", `Failed: ${(e as { message: string }).message}`);
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
          <MainNav active="settings" />
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

      <div class="space-y-6">
      <Accordion
        title="Solidtime connection"
        hint="Saved values apply to both the CLI and this app — they share the same database and Keychain."
      >
        <div class="space-y-5">
          <TextField
            label="Solidtime URL"
            hint="Base URL of your self-hosted Solidtime instance."
            placeholder="https://time.example.com"
            value={lookup("solidtime.url")?.value ?? ""}
            onSave={(v) => saveValue("solidtime.url", v)}
          />

          {/* Authentication method selector */}
          <FieldShell
            label="Authentication method"
            hint="API token for personal access tokens; OAuth to sign in via the browser."
          >
            <div class="flex gap-5 pt-0.5">
              <label class="flex items-center gap-2 text-sm text-zinc-700 dark:text-zinc-300 cursor-pointer">
                <input
                  type="radio"
                  name="auth_mode"
                  value="api_token"
                  checked={authMode() === "api_token"}
                  onChange={() => setAuthMode("api_token")}
                  class="accent-zinc-900 dark:accent-zinc-100"
                />
                API token
              </label>
              <label class="flex items-center gap-2 text-sm text-zinc-700 dark:text-zinc-300 cursor-pointer">
                <input
                  type="radio"
                  name="auth_mode"
                  value="oauth"
                  checked={authMode() === "oauth"}
                  onChange={() => setAuthMode("oauth")}
                  class="accent-zinc-900 dark:accent-zinc-100"
                />
                Sign in with Solidtime (OAuth)
              </label>
            </div>
          </FieldShell>

          {/* OAuth panel — visible when OAuth is selected */}
          <Show when={authMode() === "oauth"}>
            <TextField
              label="OAuth client ID"
              hint="Client ID of your Solidtime OAuth application."
              placeholder="00000000-0000-0000-0000-000000000000"
              value={lookup("solidtime.oauth.client_id")?.value ?? ""}
              onSave={(v) => saveValue("solidtime.oauth.client_id", v)}
            />

            <FieldShell label="Sign-in status">
              <Show
                when={authStatus()?.signed_in}
                fallback={
                  <Button onClick={handleSignIn}>
                    Sign in with Solidtime
                  </Button>
                }
              >
                <div class="flex flex-wrap items-center gap-2">
                  <Pill tone="emerald">Signed in</Pill>
                  <Show when={authStatus()?.scope}>
                    <span class="text-xs text-zinc-500">
                      scope: {authStatus()?.scope}
                    </span>
                  </Show>
                  <Button variant="ghost" size="sm" onClick={handleSignOut}>
                    Sign out
                  </Button>
                </div>
              </Show>
            </FieldShell>
          </Show>

          <Show when={authMode() === "api_token"}>
            <SecretField
              label="API token"
              hint="Personal access token. Stored in macOS Keychain — never in the database."
              isSet={tokenSet()}
              onSave={(v) => saveValue("solidtime.token", v)}
            />
          </Show>

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
            <FieldShell
              label="Default project"
              hint="Optional. New timers pre-fill this project."
            >
              <ProjectPicker
                value={defaultProjectId() || null}
                onChange={(id) => saveValue("solidtime.default-project", id ?? "")}
                projects={projectList()}
                placeholder="No default project"
              />
            </FieldShell>
          </Show>
        </div>

        <div class="mt-6 flex items-center gap-3 border-t border-black/[0.05] pt-4 dark:border-white/[0.04]">
          <Button onClick={testConnection}>Test connection</Button>
          <Button variant="secondary" onClick={syncNow}>Sync now</Button>
        </div>
      </Accordion>

      <Accordion
        title="Calendar accounts"
        hint='Read-only — events appear on the Today view with a "Log this" action.'
      >
        <Show
          when={(accounts() ?? []).length > 0}
          fallback={
            <p class="text-sm text-zinc-500">
              No calendar accounts connected yet.
            </p>
          }
        >
          <ul class="space-y-2">
            <For each={accounts() ?? []}>
              {(a) => (
                <CalendarAccountRow
                  account={a}
                  flash={flash}
                  onRemove={() => handleRemoveAccount(a.id)}
                />
              )}
            </For>
          </ul>
        </Show>

        <div class="mt-4 border-t border-black/[0.05] pt-4 dark:border-white/[0.04]">
          <Button onClick={handleAddGoogle}>Add Google account</Button>
        </div>
      </Accordion>

      <Accordion
        title="Integrations"
        hint="Local HTTP API, AI agents (MCP), and the stint:// URL scheme."
      >
        <IntegrationsPanel />
      </Accordion>

      <Accordion
        title="Idle detection"
        hint="Catch idle periods so they don't end up on your timesheet."
      >
        <div class="space-y-4">
          <div class="flex items-center justify-between gap-3">
            <div>
              <div class="text-sm font-medium">Detect when I leave my desk</div>
              <div class="mt-0.5 text-xs text-zinc-500">
                When a timer is running and you stop moving the mouse / typing,
                stint waits for you to come back and offers to discard the gap.
              </div>
            </div>
            <Toggle
              label={idleEnabled() ? "On" : "Off"}
              checked={idleEnabled()}
              onChange={(next) =>
                saveValue("idle.enabled", next ? "true" : "false")
              }
            />
          </div>
          <label class="flex items-center justify-between gap-3">
            <span class="text-sm">After</span>
            <select
              class="rounded-md border border-black/10 bg-white px-2 py-1 text-sm dark:border-white/10 dark:bg-zinc-900"
              value={String(idleThreshold())}
              onChange={(e) =>
                saveValue("idle.threshold_secs", e.currentTarget.value)
              }
            >
              <option value="300">5 minutes</option>
              <option value="600">10 minutes</option>
              <option value="900">15 minutes</option>
              <option value="1800">30 minutes</option>
            </select>
          </label>
        </div>
      </Accordion>

      <Accordion
        title="Updates"
        hint="Choose a release channel and check for new versions."
      >
        <UpdatesPanel />
      </Accordion>
      </div>
      </div>
    </div>
  );
}

function CalendarAccountRow(props: {
  account: CalendarAccount;
  flash: (kind: "ok" | "err" | "info", msg: string) => void;
  onRemove: () => void;
}) {
  const [status] = createResource(
    () => props.account.id,
    (id) => calendarApi.oauthStatus(id),
  );

  return (
    <li class="flex items-center justify-between rounded-md border border-black/[0.05] bg-white px-3 py-2 dark:border-white/[0.05] dark:bg-zinc-950">
      <div class="min-w-0">
        <div class="flex items-center gap-2">
          <span class="text-sm font-medium truncate">
            {props.account.identifier}
          </span>
          <Show when={status()?.signed_in} fallback={<Pill tone="amber">Reconnect</Pill>}>
            <Pill tone="emerald">Signed in</Pill>
          </Show>
        </div>
        <div class="mt-0.5 text-xs text-zinc-500">
          {props.account.provider} · {props.account.id.slice(0, 8)}
          <Show when={status()?.scope}>
            {" · "}
            <span title={status()?.scope ?? ""}>scope ✓</span>
          </Show>
        </div>
      </div>
      <div class="flex items-center gap-2">
        <CalendarsManager accountId={props.account.id} flash={props.flash} />
        <Button variant="ghost" size="sm" onClick={props.onRemove}>
          Remove
        </Button>
      </div>
    </li>
  );
}

function CalendarsManager(props: {
  accountId: string;
  flash: (kind: "ok" | "err" | "info", msg: string) => void;
}) {
  const [open, setOpen] = createSignal(false);
  const [cals, { refetch }] = createResource(
    () => (open() ? props.accountId : null),
    async (id): Promise<CalendarRow[]> => {
      if (!id) return [];
      return calendarApi.listCalendars(id);
    },
  );
  const [projects] = createResource(() => api.listProjects(), {
    initialValue: [],
  });

  async function toggle(id: string, included: boolean) {
    try {
      await calendarApi.setCalendarIncluded(id, included);
      refetch();
    } catch (e) {
      props.flash("err", `Toggle failed: ${(e as { message: string }).message}`);
    }
  }

  async function setDefault(calId: string, projectId: string | null) {
    try {
      await calendarApi.setDefaultProject(calId, projectId);
      refetch();
    } catch (e) {
      props.flash(
        "err",
        `Set default failed: ${(e as { message: string }).message}`,
      );
    }
  }

  return (
    <Popover open={open()} onOpenChange={setOpen}>
      <Popover.Trigger class="inline-flex items-center gap-1 rounded-lg px-2.5 py-1.5 text-xs font-medium text-zinc-700 outline-none transition hover:bg-zinc-100 focus-visible:ring-2 focus-visible:ring-indigo-400 dark:text-zinc-300 dark:hover:bg-zinc-800">
        Calendars
        <svg
          class="h-3.5 w-3.5 text-zinc-400"
          viewBox="0 0 20 20"
          fill="currentColor"
          aria-hidden="true"
        >
          <path
            fill-rule="evenodd"
            d="M5.23 7.21a.75.75 0 0 1 1.06.02L10 11.17l3.71-3.94a.75.75 0 1 1 1.08 1.04l-4.25 4.5a.75.75 0 0 1-1.08 0l-4.25-4.5a.75.75 0 0 1 .02-1.06Z"
            clip-rule="evenodd"
          />
        </svg>
      </Popover.Trigger>
      <Popover.Portal>
        <Popover.Content class="z-50 mt-1 w-80 rounded-md border border-black/[0.08] bg-white p-3 shadow-lg outline-none dark:border-white/[0.08] dark:bg-zinc-950">
          <Show
            when={(cals() ?? []).length > 0}
            fallback={
              <p class="text-xs text-zinc-500">
                {cals.loading ? "Loading…" : "No calendars."}
              </p>
            }
          >
            <ul class="space-y-2">
              <For each={cals() ?? []}>
                {(c) => (
                  <li class="space-y-1">
                    <label class="flex cursor-pointer items-center gap-2 text-sm">
                      <input
                        type="checkbox"
                        checked={c.included}
                        onChange={(e) =>
                          toggle(c.id, e.currentTarget.checked)
                        }
                      />
                      <span class="flex-1">{c.name}</span>
                    </label>
                    <Show when={c.included}>
                      <div class="pl-6">
                        <ProjectPicker
                          value={c.default_project_id}
                          onChange={(id) => setDefault(c.id, id)}
                          projects={projects() ?? []}
                          placeholder="No default project"
                          size="sm"
                        />
                      </div>
                    </Show>
                  </li>
                )}
              </For>
            </ul>
          </Show>
        </Popover.Content>
      </Popover.Portal>
    </Popover>
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

import {
  For,
  Show,
  createEffect,
  createMemo,
  createResource,
  createSignal,
  onCleanup,
} from "solid-js";
import { listen } from "@tauri-apps/api/event";
import { api } from "~/api";
import type { Entry, OverlapCandidate, SyncError } from "~/types";
import EditEntryDialog from "./EditEntryDialog";
import Button from "./ui/Button";

function fmtTime(iso: string): string {
  try {
    return new Date(iso).toLocaleTimeString(undefined, {
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch {
    return iso;
  }
}

/// Translate a raw Solidtime error body into something a user can act on.
/// Currently focuses on overlapping_time_entry (the most common stuck
/// case); other keys fall through to a generic "rejected by Solidtime"
/// message with the original error text exposed via the details toggle.
function friendlyMessage(raw: string | null): string {
  if (!raw) return "Sync failed.";
  if (raw.includes("overlapping_time_entry")) {
    return "Conflicts with another entry in Solidtime. Edit this entry's times to a non-overlapping range, or remove the conflicting entry in Solidtime.";
  }
  if (raw.includes("validation") || raw.includes("invalid")) {
    return "Solidtime rejected this entry as invalid. Edit it or delete it locally.";
  }
  return "Solidtime rejected this entry. Adjust it or delete it locally.";
}

export default function SyncErrorBanner() {
  const [errors, { refetch }] = createResource(() => api.listSyncErrors(), {
    initialValue: [],
  });
  const [expanded, setExpanded] = createSignal(false);
  const [editing, setEditing] = createSignal<Entry | null>(null);
  const [busyUuid, setBusyUuid] = createSignal<string | null>(null);
  // Overlap lookups are lazy — only fetched once the user expands the
  // banner. Keyed by local_uuid; null means "fetching", undefined means
  // "not yet requested", [] means "fetched, nothing overlaps".
  const [overlaps, setOverlaps] = createSignal<Record<string, OverlapCandidate[] | null>>({});

  // Re-fetch whenever sync state changes — covers fresh failures, manual
  // sync-now, and the worker's periodic drain.
  const unlisten = listen("entries:changed", () => refetch());
  onCleanup(() => {
    unlisten.then((fn) => fn()).catch(() => {});
  });

  const list = createMemo<SyncError[]>(() => errors() ?? []);
  const abandoned = createMemo(() => list().filter((e) => e.abandoned));
  // Only show the banner once an entry hits the abandoned-permanently state
  // — transient retries (server 5xx, network flakes) don't need user attention.
  const visible = createMemo(() => abandoned().length > 0);

  // Trigger one overlap fetch per abandoned row whenever the banner is
  // expanded. Each row's result is cached in the `overlaps` signal so
  // collapsing + re-expanding doesn't re-request.
  createEffect(() => {
    if (!expanded()) return;
    for (const row of abandoned()) {
      const uuid = row.local_uuid;
      if (!uuid) continue;
      if (overlaps()[uuid] !== undefined) continue;
      setOverlaps((m) => ({ ...m, [uuid]: null }));
      api
        .getSyncErrorOverlaps(uuid)
        .then((list) => setOverlaps((m) => ({ ...m, [uuid]: list })))
        .catch((err) => {
          console.error("getSyncErrorOverlaps failed:", err);
          setOverlaps((m) => ({ ...m, [uuid]: [] }));
        });
    }
  });

  async function dismissAndDelete(uuid: string | null) {
    if (!uuid) return;
    setBusyUuid(uuid);
    try {
      await api.deleteEntry(uuid);
      refetch();
    } catch (e) {
      console.error("delete from sync banner failed:", e);
    } finally {
      setBusyUuid(null);
    }
  }

  async function openEdit(err: SyncError) {
    if (!err.local_uuid || !err.start_at) return;
    // Reconstruct just enough of an Entry to feed the dialog. Description
    // is empty-tolerant; sync_state is irrelevant to the dialog rendering.
    setEditing({
      local_uuid: err.local_uuid,
      solidtime_id: null,
      description: err.description ?? "",
      project_id: null,
      task_id: null,
      start_at: err.start_at,
      end_at: err.end_at ?? null,
      billable: false,
      sync_state: "pending_create",
      source: "cli",
    });
  }

  return (
    <Show when={visible()}>
      <div
        class="mb-4 rounded-lg border border-red-200 bg-red-50 dark:border-red-900/60 dark:bg-red-950/40"
        role="alert"
      >
        <button
          type="button"
          class="flex w-full items-center justify-between gap-3 px-4 py-3 text-left"
          onClick={() => setExpanded((v) => !v)}
        >
          <div class="flex min-w-0 items-center gap-3">
            <svg
              class="h-4 w-4 shrink-0 text-red-600 dark:text-red-300"
              viewBox="0 0 20 20"
              fill="none"
              stroke="currentColor"
              stroke-width="1.75"
              stroke-linecap="round"
              stroke-linejoin="round"
              aria-hidden="true"
            >
              <circle cx="10" cy="10" r="8" />
              <path d="M10 6v5" />
              <circle cx="10" cy="14" r="0.5" fill="currentColor" />
            </svg>
            <span class="text-sm font-medium text-red-800 dark:text-red-200">
              {abandoned().length}{" "}
              {abandoned().length === 1 ? "entry" : "entries"} couldn't sync to
              Solidtime
            </span>
          </div>
          <span class="text-xs text-red-700 dark:text-red-300">
            {expanded() ? "Hide" : "Show"}
          </span>
        </button>

        <Show when={expanded()}>
          <ul class="border-t border-red-200 dark:border-red-900/60">
            <For each={abandoned()}>
              {(err) => (
                <li class="space-y-2 px-4 py-3 text-sm">
                  <div class="font-medium text-red-900 dark:text-red-100">
                    {err.description?.trim() || (
                      <span class="italic text-red-700/70">(no description)</span>
                    )}
                    <Show when={err.start_at}>
                      <span class="ml-2 font-mono text-[11px] font-normal text-red-700/70 dark:text-red-300/70">
                        {err.start_at}
                        {err.end_at ? ` → ${err.end_at}` : " (still running)"}
                      </span>
                    </Show>
                  </div>
                  <p class="text-xs text-red-800 dark:text-red-200">
                    {friendlyMessage(err.last_error)}
                  </p>
                  <Show when={err.local_uuid}>
                    <OverlapList
                      uuid={err.local_uuid!}
                      state={overlaps()[err.local_uuid!]}
                    />
                  </Show>
                  <details class="text-[11px] text-red-700/80 dark:text-red-300/70">
                    <summary class="cursor-pointer select-none">
                      Solidtime error detail
                    </summary>
                    <pre class="mt-1 whitespace-pre-wrap break-words">
                      {err.last_error ?? "(no details)"}
                    </pre>
                  </details>
                  <div class="flex flex-wrap items-center gap-2 pt-1">
                    <Show when={err.local_uuid && err.start_at}>
                      <Button
                        variant="secondary"
                        size="sm"
                        onClick={() => openEdit(err)}
                      >
                        Edit times
                      </Button>
                    </Show>
                    <Button
                      variant="danger"
                      size="sm"
                      disabled={busyUuid() === err.local_uuid}
                      onClick={() => dismissAndDelete(err.local_uuid)}
                    >
                      Delete entry
                    </Button>
                  </div>
                </li>
              )}
            </For>
          </ul>
        </Show>
      </div>

      <Show when={editing()}>
        {(entry) => (
          <EditEntryDialog
            entry={entry()}
            onClose={() => setEditing(null)}
            onSaved={() => {
              setEditing(null);
              refetch();
            }}
          />
        )}
      </Show>
    </Show>
  );
}

function OverlapList(props: {
  uuid: string;
  state: OverlapCandidate[] | null | undefined;
}) {
  return (
    <div class="rounded-md bg-red-100/60 px-2.5 py-1.5 text-[11px] text-red-900 dark:bg-red-950/60 dark:text-red-100">
      <Show
        when={props.state !== undefined}
        fallback={
          <span class="text-red-700/70 dark:text-red-300/70">
            Checking Solidtime…
          </span>
        }
      >
        <Show
          when={props.state === null}
          fallback={
            <Show
              when={(props.state ?? []).length > 0}
              fallback={
                <span class="text-red-700/70 dark:text-red-300/70">
                  Solidtime reports no overlapping entries — try{" "}
                  <code class="rounded bg-red-200/60 px-1 dark:bg-red-900/60">
                    stint sync diagnose
                  </code>{" "}
                  for raw details, or delete this entry.
                </span>
              }
            >
              <div class="mb-1 font-medium">Conflicts with:</div>
              <ul class="space-y-0.5">
                <For each={props.state ?? []}>
                  {(o) => (
                    <li class="flex items-baseline gap-2">
                      <span class="truncate font-medium">
                        {o.description.trim() || "(no description)"}
                      </span>
                      <span class="font-mono text-[10px] text-red-700/70 dark:text-red-300/70">
                        {fmtTime(o.start)}
                        {o.end ? ` → ${fmtTime(o.end)}` : " (running)"}
                      </span>
                    </li>
                  )}
                </For>
              </ul>
            </Show>
          }
        >
          <span class="text-red-700/70 dark:text-red-300/70">
            Checking Solidtime…
          </span>
        </Show>
      </Show>
    </div>
  );
}

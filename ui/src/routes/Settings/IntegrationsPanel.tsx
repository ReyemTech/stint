import { Show, createResource, createSignal } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import Button from "~/components/ui/Button";
import Toggle from "~/components/ui/Toggle";

/**
 * Shape of the `ApiIntegrationState` payload returned by the Rust commands
 * in `crates/stint-app/src/commands/integrations.rs`. Kept inline (rather
 * than in `~/types`) because this is the only consumer for now.
 */
type ApiIntegrationState = {
  enabled: boolean;
  host: string;
  port: number | null;
  base_url: string | null;
  /**
   * True when the in-process HTTP server actually bound a port this session.
   * When `enabled` is true but this is false, the user has flipped the
   * toggle but hasn't restarted yet.
   */
  bound_this_session: boolean;
};

const getApiIntegrationState = () =>
  invoke<ApiIntegrationState>("get_api_integration_state");
const setApiEnabled = (enabled: boolean) =>
  invoke<ApiIntegrationState>("set_api_enabled", { enabled });

const AI_DOCS_URL = "https://stint.reyem.tech/ai-integration/";

/**
 * Settings panel for stint's non-Solidtime integrations: the local HTTP API
 * (loopback-only, opt-in), the bundled MCP server, and the `stint://` URL
 * scheme. Mirrors the styling of the other Settings accordion panels.
 *
 * The HTTP server is bound once at app startup, so toggling here only
 * persists `api.enabled` — the panel surfaces a "restart required" hint
 * when the persisted state diverges from what was actually bound.
 */
export default function IntegrationsPanel() {
  const [state, { refetch }] = createResource<ApiIntegrationState>(
    getApiIntegrationState,
  );
  const [busy, setBusy] = createSignal(false);
  const [copied, setCopied] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  // True when the user has flipped enabled but the in-process server's
  // bound state doesn't match — either off→on (needs restart to start the
  // server) or on→off (needs restart to stop it).
  const needsRestart = () => {
    const s = state();
    if (!s) return false;
    return s.enabled !== s.bound_this_session;
  };

  async function toggleEnabled(next: boolean) {
    setBusy(true);
    setError(null);
    try {
      await setApiEnabled(next);
      await refetch();
    } catch (e) {
      setError((e as { message?: string }).message ?? String(e));
    } finally {
      setBusy(false);
    }
  }

  async function copyBaseUrl() {
    const url = state()?.base_url;
    if (!url) return;
    try {
      await navigator.clipboard.writeText(url);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch (e) {
      setError((e as { message?: string }).message ?? String(e));
    }
  }

  async function openAiDocs() {
    try {
      await openUrl(AI_DOCS_URL);
    } catch (e) {
      setError((e as { message?: string }).message ?? String(e));
    }
  }

  async function testUrlScheme() {
    try {
      await openUrl("stint://current");
    } catch (e) {
      setError((e as { message?: string }).message ?? String(e));
    }
  }

  return (
    <div class="space-y-8">
      {/* ── Local HTTP API ──────────────────────────────────────────── */}
      <section class="space-y-3">
        <header>
          <h3 class="text-sm font-medium text-zinc-900 dark:text-zinc-100">
            Local HTTP API
          </h3>
          <p class="mt-0.5 text-xs text-zinc-500">
            Off by default. Enable to let scripts and widgets read entries via{" "}
            <code class="rounded bg-zinc-100 px-1 py-0.5 text-[11px] dark:bg-zinc-800">
              /v1/*
            </code>{" "}
            endpoints. Loopback-only (127.0.0.1); never reachable externally.
          </p>
        </header>

        <Show when={state()} fallback={
          <p class="text-xs text-zinc-500">Loading…</p>
        }>
          <div class="flex items-center gap-3">
            <Toggle
              label={state()!.enabled ? "Enabled" : "Disabled"}
              checked={state()!.enabled}
              disabled={busy()}
              onChange={toggleEnabled}
            />
            <Show when={busy()}>
              <span class="text-xs text-zinc-500">Saving…</span>
            </Show>
          </div>

          <Show
            when={
              state()!.enabled &&
              state()!.bound_this_session &&
              state()!.base_url
            }
          >
            <div class="rounded-md border border-emerald-200 bg-emerald-50 p-3 text-sm dark:border-emerald-900 dark:bg-emerald-950/40">
              <dl class="grid grid-cols-[5rem_1fr] gap-x-3 gap-y-1 text-xs">
                <dt class="text-zinc-500">Host</dt>
                <dd class="font-mono text-zinc-900 dark:text-zinc-100">
                  {state()!.host}
                </dd>
                <dt class="text-zinc-500">Port</dt>
                <dd class="font-mono text-zinc-900 dark:text-zinc-100">
                  {state()!.port}
                </dd>
                <dt class="text-zinc-500">URL</dt>
                <dd class="flex items-center gap-2">
                  <span class="font-mono text-zinc-900 dark:text-zinc-100">
                    {state()!.base_url}
                  </span>
                  <Button size="sm" variant="ghost" onClick={copyBaseUrl}>
                    {copied() ? "Copied" : "Copy"}
                  </Button>
                </dd>
              </dl>
            </div>
          </Show>

          <Show when={needsRestart()}>
            <p class="rounded-md border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-800 dark:border-amber-900 dark:bg-amber-950/40 dark:text-amber-200">
              <Show
                when={state()!.enabled}
                fallback={<>Restart Stint to stop the API server.</>}
              >
                Restart Stint to start the API server.
              </Show>
            </p>
          </Show>
        </Show>

        <Show when={error()}>
          <p class="text-xs text-red-600 dark:text-red-400">{error()}</p>
        </Show>
      </section>

      {/* ── AI agents (informational) ──────────────────────────────── */}
      <section class="space-y-2 border-t border-black/[0.05] pt-5 dark:border-white/[0.04]">
        <header>
          <h3 class="text-sm font-medium text-zinc-900 dark:text-zinc-100">
            AI agents
          </h3>
          <p class="mt-0.5 text-xs text-zinc-500">
            Drive stint from Claude Code, Codex, OpenCode, or any other MCP
            client.
          </p>
        </header>
        <p class="text-xs text-zinc-500">
          Run{" "}
          <code class="rounded bg-zinc-100 px-1 py-0.5 text-[11px] dark:bg-zinc-800">
            stint skill install &lt;claude|codex|opencode&gt;
          </code>{" "}
          in your shell to wire it up.
        </p>
        <div>
          <button
            type="button"
            class="text-xs text-indigo-600 underline-offset-2 hover:underline dark:text-indigo-400"
            onClick={openAiDocs}
          >
            Learn more →
          </button>
        </div>
      </section>

      {/* ── URL scheme (informational) ─────────────────────────────── */}
      <section class="space-y-2 border-t border-black/[0.05] pt-5 dark:border-white/[0.04]">
        <header>
          <h3 class="text-sm font-medium text-zinc-900 dark:text-zinc-100">
            URL scheme
          </h3>
          <p class="mt-0.5 text-xs text-zinc-500">
            Stint registers{" "}
            <code class="rounded bg-zinc-100 px-1 py-0.5 text-[11px] dark:bg-zinc-800">
              stint://
            </code>{" "}
            URLs for Raycast, Alfred, Shortcuts, and the like.
          </p>
        </header>
        <ul class="ml-4 list-disc space-y-1 text-xs text-zinc-600 dark:text-zinc-400">
          <li>
            <code class="rounded bg-zinc-100 px-1 py-0.5 text-[11px] dark:bg-zinc-800">
              stint://start?description=…
            </code>
          </li>
          <li>
            <code class="rounded bg-zinc-100 px-1 py-0.5 text-[11px] dark:bg-zinc-800">
              stint://stop
            </code>
          </li>
          <li>
            <code class="rounded bg-zinc-100 px-1 py-0.5 text-[11px] dark:bg-zinc-800">
              stint://current
            </code>
          </li>
          <li>
            <code class="rounded bg-zinc-100 px-1 py-0.5 text-[11px] dark:bg-zinc-800">
              stint://entry/&lt;local-uuid&gt;
            </code>
          </li>
        </ul>
        <div>
          <Button size="sm" variant="secondary" onClick={testUrlScheme}>
            Test stint://current
          </Button>
        </div>
      </section>
    </div>
  );
}

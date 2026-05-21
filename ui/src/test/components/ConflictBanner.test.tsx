import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render } from "@solidjs/testing-library";

// Capture the listener registered for `pull:conflict` so the test can
// invoke it directly to simulate a backend event.
type Listener = (event: { payload: unknown }) => void;
let listeners: Map<string, Listener>;

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (name: string, fn: Listener) => {
    listeners.set(name, fn);
    return () => listeners.delete(name);
  }),
}));

vi.mock("~/api", () => ({
  conflictResolve: vi.fn().mockResolvedValue(undefined),
}));

import ConflictBanner from "~/components/ConflictBanner";
import { conflictResolve } from "~/api";

const sampleConflict = {
  remote_id: "remote-1",
  remote_description: "Web timer",
  remote_start_at: "2026-05-20T09:00:00Z",
  local_local_uuid: "uuid-local",
  local_description: "Local work",
};

beforeEach(() => {
  listeners = new Map();
  vi.mocked(conflictResolve).mockClear();
});

const flushMicrotasks = () => new Promise<void>((r) => setTimeout(r, 0));

async function emit(payload: unknown) {
  const fn = listeners.get("pull:conflict");
  if (!fn) throw new Error("no listener registered yet");
  fn({ payload });
  await flushMicrotasks();
}

describe("ConflictBanner", () => {
  it("is hidden until a pull:conflict event arrives", () => {
    const { queryByText } = render(() => <ConflictBanner />);
    expect(queryByText(/Another timer is running/i)).toBeNull();
  });

  it("shows the remote description after a pull:conflict event", async () => {
    const { findByText } = render(() => <ConflictBanner />);
    await flushMicrotasks(); // onMount runs listen()
    await emit(sampleConflict);
    const banner = await findByText(/Web timer/);
    expect(banner).toBeDefined();
  });

  it("Stop it remotely → conflictResolve('stop_remote', remote_id) and hides", async () => {
    const { findByRole, queryByText } = render(() => <ConflictBanner />);
    await flushMicrotasks();
    await emit(sampleConflict);
    const btn = await findByRole("button", { name: /Stop it remotely/i });
    fireEvent.click(btn);
    await flushMicrotasks();
    expect(conflictResolve).toHaveBeenCalledWith("stop_remote", "remote-1");
    expect(queryByText(/Another timer is running/i)).toBeNull();
  });

  it("Switch to it → conflictResolve('switch', remote_id)", async () => {
    const { findByRole } = render(() => <ConflictBanner />);
    await flushMicrotasks();
    await emit(sampleConflict);
    fireEvent.click(await findByRole("button", { name: /Switch to it/i }));
    await flushMicrotasks();
    expect(conflictResolve).toHaveBeenCalledWith("switch", "remote-1");
  });

  it("Dismiss → conflictResolve('dismiss', remote_id)", async () => {
    const { findByRole } = render(() => <ConflictBanner />);
    await flushMicrotasks();
    await emit(sampleConflict);
    fireEvent.click(await findByRole("button", { name: /Dismiss/i }));
    await flushMicrotasks();
    expect(conflictResolve).toHaveBeenCalledWith("dismiss", "remote-1");
  });
});

import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render } from "@solidjs/testing-library";
import { createSignal } from "solid-js";
import type { RunningTimer } from "~/types";

const [running, setRunning] = createSignal<RunningTimer | null>(null);
const [elapsedSecs, setElapsedSecs] = createSignal(0);

const storeMock = {
  running,
  elapsedSecs,
  refresh: vi.fn(),
  start: vi.fn().mockResolvedValue(undefined),
  stop: vi.fn().mockResolvedValue(undefined),
};
vi.mock("~/stores/timer", () => ({
  useTimerStore: () => storeMock,
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));
vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: vi.fn().mockResolvedValue(undefined),
}));
vi.mock("~/lib/openSolidtime", () => ({
  openSolidtime: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("~/api", () => ({
  api: {
    listToday: vi.fn().mockResolvedValue([]),
    listProjects: vi.fn().mockResolvedValue([
      { id: "p-1", name: "Tet", color: null, client_id: null, client_name: null, archived: 0 },
    ]),
  },
}));

import Popover from "~/routes/Popover";
import { invoke } from "@tauri-apps/api/core";
import { openSolidtime } from "~/lib/openSolidtime";
import { api } from "~/api";

const flushMicrotasks = () => new Promise<void>((r) => setTimeout(r, 0));

/// Build a minimal `RunningTimer` (= `EntryView`) for tests. Defaults
/// every field the UI doesn't read so call sites can stay focused on the
/// few fields they care about.
const runningTimer = (overrides: Partial<RunningTimer>): RunningTimer => ({
  local_uuid: "uuid-1",
  solidtime_id: null,
  description: "",
  project_id: null,
  task_id: null,
  billable: false,
  start_at: new Date().toISOString(),
  end_at: null,
  source: "gui",
  ...overrides,
});

beforeEach(() => {
  setRunning(null);
  setElapsedSecs(0);
  storeMock.start.mockClear();
  storeMock.stop.mockClear();
  vi.mocked(invoke).mockReset();
  vi.mocked(openSolidtime).mockClear();
  vi.mocked(api.listToday).mockResolvedValue([]);
});

describe("<Popover> — idle state", () => {
  it("renders the Today header, count, and start form", async () => {
    const { getByText, getByPlaceholderText } = render(() => <Popover />);
    await flushMicrotasks();
    expect(getByText("Today")).toBeDefined();
    expect(getByText("0 entries")).toBeDefined();
    expect(getByPlaceholderText("What are you working on?")).toBeDefined();
  });

  it("Start timer button is disabled until a description is typed", async () => {
    const { getByPlaceholderText, getByRole } = render(() => <Popover />);
    await flushMicrotasks();
    const btn = getByRole("button", { name: /Start timer/ }) as HTMLButtonElement;
    expect(btn.disabled).toBe(true);
    const input = getByPlaceholderText("What are you working on?") as HTMLInputElement;
    input.value = "work";
    fireEvent.input(input);
    expect(btn.disabled).toBe(false);
  });

  it("submitting the form calls timer.start with the description", async () => {
    const { getByPlaceholderText, container } = render(() => <Popover />);
    await flushMicrotasks();
    const input = getByPlaceholderText("What are you working on?") as HTMLInputElement;
    input.value = "deep work";
    fireEvent.input(input);
    fireEvent.submit(container.querySelector("form")!);
    await flushMicrotasks();
    expect(storeMock.start).toHaveBeenCalledWith(
      "deep work",
      undefined,
      false,
      undefined,
    );
  });
});

describe("<Popover> — running state", () => {
  it("renders Tracking + Live indicator + Stop button", async () => {
    setRunning(
      runningTimer({
        description: "morning standup",
        start_at: new Date(Date.now() - 60_000).toISOString(),
      }),
    );
    setElapsedSecs(60);
    const { getByText } = render(() => <Popover />);
    await flushMicrotasks();
    expect(getByText("Tracking")).toBeDefined();
    expect(getByText("Live")).toBeDefined();
    expect(getByText("Stop timer")).toBeDefined();
    expect(getByText("morning standup")).toBeDefined();
  });

  it("Stop timer button invokes timer.stop()", async () => {
    setRunning({
      local_uuid: "uuid-1",
      solidtime_id: null,
      description: "x",
      project_id: null,
      task_id: null,
      billable: false,
      start_at: new Date().toISOString(),
      end_at: null,
      source: "test",
    });
    const { getByText } = render(() => <Popover />);
    await flushMicrotasks();
    fireEvent.click(getByText("Stop timer"));
    await flushMicrotasks();
    expect(storeMock.stop).toHaveBeenCalled();
  });
});

describe("<Popover> — footer actions", () => {
  it("Open Stint → invokes show_main_window via IPC", async () => {
    const { getByText } = render(() => <Popover />);
    await flushMicrotasks();
    fireEvent.click(getByText(/Open Stint/));
    await flushMicrotasks();
    expect(invoke).toHaveBeenCalledWith("show_main_window");
  });

  it("Solidtime ↗ calls openSolidtime()", async () => {
    const { getByText } = render(() => <Popover />);
    await flushMicrotasks();
    fireEvent.click(getByText(/Solidtime/));
    expect(openSolidtime).toHaveBeenCalledTimes(1);
  });
});

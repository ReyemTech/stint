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

vi.mock("~/api", () => ({
  api: {
    listProjects: vi.fn().mockResolvedValue([
      { id: "p-1", name: "Tet", color: null, client_id: null, client_name: null, archived: 0 },
    ]),
    setEntryProject: vi.fn().mockResolvedValue(undefined),
    setEntryBillable: vi.fn().mockResolvedValue(undefined),
  },
}));

import TimerCard from "~/components/TimerCard";
import { api } from "~/api";

const flushMicrotasks = () => new Promise<void>((r) => setTimeout(r, 0));

/// Build a minimal `RunningTimer` (alias of `EntryView`) for tests that
/// only care about the few fields the UI reads. Defaults the new
/// post-verbs fields (`solidtime_id`, `task_id`, `end_at`, `source`) to
/// nulls / "gui" so callers stay terse.
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
  storeMock.refresh.mockClear();
  vi.mocked(api.setEntryProject).mockClear();
  vi.mocked(api.setEntryBillable).mockClear();
});

describe("<TimerCard> — start form (no timer running)", () => {
  it("renders the description input, a project picker, and a Start button", () => {
    const { getByPlaceholderText, getByText, getByLabelText } = render(() => (
      <TimerCard />
    ));
    expect(getByPlaceholderText("What are you working on?")).toBeDefined();
    expect(getByLabelText("Open project list")).toBeDefined();
    expect(getByText("Start")).toBeDefined();
  });

  it("Start is disabled until the description is non-empty", async () => {
    const { getByPlaceholderText, getByRole } = render(() => <TimerCard />);
    await flushMicrotasks();
    const startBtn = getByRole("button", { name: /Start/ }) as HTMLButtonElement;
    expect(startBtn.disabled).toBe(true);

    const input = getByPlaceholderText("What are you working on?") as HTMLInputElement;
    input.value = "design review";
    fireEvent.input(input);
    expect(startBtn.disabled).toBe(false);
  });

  it("submitting the form calls timer.start with the description + project + billable", async () => {
    const { getByPlaceholderText, getByRole, container } = render(() => <TimerCard />);
    await flushMicrotasks();

    const input = getByPlaceholderText("What are you working on?") as HTMLInputElement;
    input.value = "design review";
    fireEvent.input(input);
    const billableToggle = getByRole("switch");
    fireEvent.click(billableToggle);
    // Form submit (via Start button).
    const form = container.querySelector("form")!;
    fireEvent.submit(form);
    await flushMicrotasks();
    expect(storeMock.start).toHaveBeenCalledWith(
      "design review",
      undefined,
      true,
      undefined,
    );
  });

  it("does not call start when the description is blank", async () => {
    const { container } = render(() => <TimerCard />);
    await flushMicrotasks();
    const form = container.querySelector("form")!;
    fireEvent.submit(form);
    await flushMicrotasks();
    expect(storeMock.start).not.toHaveBeenCalled();
  });
});

describe("<TimerCard> — running timer panel", () => {
  it("shows the elapsed duration + description and a Stop button", () => {
    setRunning(
      runningTimer({
        description: "morning standup",
        start_at: new Date(Date.now() - 60_000).toISOString(),
      }),
    );
    setElapsedSecs(60);
    const { getByText } = render(() => <TimerCard />);
    expect(getByText("morning standup")).toBeDefined();
    expect(getByText("00:01:00")).toBeDefined();
    expect(getByText("Stop")).toBeDefined();
  });

  it("clicking Stop invokes timer.stop()", async () => {
    setRunning(runningTimer({ description: "x" }));
    const { getByText } = render(() => <TimerCard />);
    fireEvent.click(getByText("Stop"));
    await flushMicrotasks();
    expect(storeMock.stop).toHaveBeenCalledTimes(1);
  });

  it("running panel shows the ProjectPicker for live project changes", async () => {
    setRunning(runningTimer({ description: "x" }));
    const { getByLabelText } = render(() => <TimerCard />);
    await flushMicrotasks();
    expect(getByLabelText("Open project list")).toBeDefined();
  });
});

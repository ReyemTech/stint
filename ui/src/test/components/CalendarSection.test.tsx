import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render } from "@solidjs/testing-library";
import type { CalendarAccount, CalendarEventWithDecision } from "~/types";

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

vi.mock("~/api", () => ({
  calendarApi: {
    listAccounts: vi.fn(),
    listEventsInRange: vi.fn(),
    logEvent: vi.fn().mockResolvedValue("uuid-1"),
    ignoreEvent: vi.fn().mockResolvedValue(undefined),
  },
}));

import CalendarSection from "~/components/CalendarSection";
import { calendarApi } from "~/api";

const flushMicrotasks = () => new Promise<void>((r) => setTimeout(r, 0));

const account: CalendarAccount = {
  id: "acc-1",
  provider: "google",
  display_name: "Me",
  identifier: "me@example.com",
  caldav_url: null,
  enabled: true,
  created_at: "2026-05-20T00:00:00Z",
};

function event(overrides: Partial<CalendarEventWithDecision> = {}): CalendarEventWithDecision {
  return {
    id: "evt-1",
    account_id: "acc-1",
    calendar_id: "cal-1",
    title: "Standup",
    start_at: "2026-05-20T09:00:00Z",
    end_at: "2026-05-20T09:30:00Z",
    is_all_day: false,
    attendee_status: "accepted",
    recurring_root: null,
    fetched_at: "2026-05-20T08:00:00Z",
    decision: null,
    linked_local_uuid: null,
    ...overrides,
  };
}

beforeEach(() => {
  vi.mocked(calendarApi.listAccounts).mockReset();
  vi.mocked(calendarApi.listEventsInRange).mockReset();
  vi.mocked(calendarApi.logEvent).mockClear();
  vi.mocked(calendarApi.ignoreEvent).mockClear();
});

describe("<CalendarSection>", () => {
  it("renders nothing when there are no events", async () => {
    vi.mocked(calendarApi.listAccounts).mockResolvedValue([account]);
    vi.mocked(calendarApi.listEventsInRange).mockResolvedValue([]);
    const { container } = render(() => (
      <CalendarSection onEntriesChanged={() => {}} />
    ));
    await flushMicrotasks();
    await flushMicrotasks();
    // The whole section is wrapped in a Show whose condition is total > 0.
    // With zero events, no Accordion / no event rows render.
    expect(container.querySelector("section")).toBeNull();
  });

  it("renders the accordion with the event count when events are present", async () => {
    vi.mocked(calendarApi.listAccounts).mockResolvedValue([account]);
    vi.mocked(calendarApi.listEventsInRange).mockResolvedValue([event()]);
    const { findByText } = render(() => (
      <CalendarSection onEntriesChanged={() => {}} />
    ));
    const count = await findByText(/1 event today/);
    expect(count).toBeDefined();
  });

  it("clicking Log this invokes calendarApi.logEvent and the onEntriesChanged callback", async () => {
    vi.mocked(calendarApi.listAccounts).mockResolvedValue([account]);
    vi.mocked(calendarApi.listEventsInRange).mockResolvedValue([event()]);
    const onChanged = vi.fn();
    const { findByRole } = render(() => (
      <CalendarSection onEntriesChanged={onChanged} />
    ));
    // Expand the accordion to reach the buttons.
    const accordionHeader = await findByRole("button", { name: /Calendar/ });
    fireEvent.click(accordionHeader);
    const logBtn = await findByRole("button", { name: /Log this/ });
    fireEvent.click(logBtn);
    await flushMicrotasks();
    expect(calendarApi.logEvent).toHaveBeenCalledWith(
      "acc-1",
      "evt-1",
      "2026-05-20T09:00:00Z",
    );
    expect(onChanged).toHaveBeenCalledTimes(1);
  });

  it("clicking Ignore invokes calendarApi.ignoreEvent", async () => {
    vi.mocked(calendarApi.listAccounts).mockResolvedValue([account]);
    vi.mocked(calendarApi.listEventsInRange).mockResolvedValue([event()]);
    const { findByRole } = render(() => (
      <CalendarSection onEntriesChanged={() => {}} />
    ));
    fireEvent.click(await findByRole("button", { name: /Calendar/ }));
    fireEvent.click(await findByRole("button", { name: /Ignore/ }));
    await flushMicrotasks();
    expect(calendarApi.ignoreEvent).toHaveBeenCalledWith(
      "acc-1",
      "evt-1",
      "2026-05-20T09:00:00Z",
    );
  });

  it("an already-logged event shows the Logged pill and hides the action buttons", async () => {
    vi.mocked(calendarApi.listAccounts).mockResolvedValue([account]);
    vi.mocked(calendarApi.listEventsInRange).mockResolvedValue([
      event({ decision: "logged_manual", linked_local_uuid: "uuid-x" }),
    ]);
    const { findByRole, queryByRole, findByText } = render(() => (
      <CalendarSection onEntriesChanged={() => {}} />
    ));
    fireEvent.click(await findByRole("button", { name: /Calendar/ }));
    expect(await findByText("Logged")).toBeDefined();
    expect(queryByRole("button", { name: /Log this/ })).toBeNull();
    expect(queryByRole("button", { name: /Ignore/ })).toBeNull();
  });
});

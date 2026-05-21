import { describe, expect, it, vi } from "vitest";
import { fireEvent, render } from "@solidjs/testing-library";
import StartAtPicker from "~/components/StartAtPicker";

describe("<StartAtPicker>", () => {
  it("renders the 'Start now' label when value is null and trigger is closed", () => {
    const { getByText } = render(() => (
      <StartAtPicker value={null} onChange={vi.fn()} />
    ));
    expect(getByText(/Start now/)).toBeDefined();
  });

  it("clicking the trigger reveals the preset buttons", () => {
    const { getByText, queryByText, getByLabelText } = render(() => (
      <StartAtPicker value={null} onChange={vi.fn()} />
    ));
    expect(queryByText("5 min ago")).toBeNull();
    fireEvent.click(getByLabelText("Open start-time picker"));
    expect(getByText("5 min ago")).toBeDefined();
    expect(getByText("15 min ago")).toBeDefined();
    expect(getByText("30 min ago")).toBeDefined();
    expect(getByText("1 hour ago")).toBeDefined();
  });

  it("clicking a preset calls onChange with an ISO timestamp", () => {
    const onChange = vi.fn();
    const { getByText, getByLabelText } = render(() => (
      <StartAtPicker value={null} onChange={onChange} />
    ));
    fireEvent.click(getByLabelText("Open start-time picker"));
    fireEvent.click(getByText("15 min ago"));
    expect(onChange).toHaveBeenCalled();
    const arg = onChange.mock.calls[0][0];
    expect(typeof arg).toBe("string");
    // Should parse as a Date about 15 min in the past.
    const diff = Date.now() - new Date(arg).getTime();
    expect(diff).toBeGreaterThan(14 * 60_000);
    expect(diff).toBeLessThan(16 * 60_000);
  });

  it("clicking 'Now' clears via onChange(null)", () => {
    const onChange = vi.fn();
    const { getByText, getByLabelText } = render(() => (
      <StartAtPicker value="2026-05-20T09:00:00Z" onChange={onChange} />
    ));
    fireEvent.click(getByLabelText("Open start-time picker"));
    fireEvent.click(getByText("Now"));
    expect(onChange).toHaveBeenCalledWith(null);
  });
});

import { describe, expect, it, vi } from "vitest";
import { fireEvent, render } from "@solidjs/testing-library";
import Button from "~/components/ui/Button";
import Pill from "~/components/ui/Pill";
import Toggle from "~/components/ui/Toggle";
import StatusDot from "~/components/ui/StatusDot";
import SectionLabel from "~/components/ui/SectionLabel";
import Accordion from "~/components/ui/Accordion";

describe("Button", () => {
  it("renders children and defaults to primary type=button", () => {
    const { getByRole } = render(() => <Button>Save</Button>);
    const btn = getByRole("button") as HTMLButtonElement;
    expect(btn.textContent).toBe("Save");
    expect(btn.type).toBe("button");
    expect(btn.className).toContain("bg-zinc-900");
  });

  it("applies the danger variant", () => {
    const { getByRole } = render(() => <Button variant="danger">Stop</Button>);
    expect(getByRole("button").className).toContain("bg-red-500");
  });

  it("applies the secondary variant", () => {
    const { getByRole } = render(() => <Button variant="secondary">Cancel</Button>);
    expect(getByRole("button").className).toContain("border-zinc-200");
  });

  it("applies the ghost variant", () => {
    const { getByRole } = render(() => <Button variant="ghost">More</Button>);
    expect(getByRole("button").className).toContain("hover:bg-zinc-100");
  });

  it("applies sm size", () => {
    const { getByRole } = render(() => <Button size="sm">Tiny</Button>);
    expect(getByRole("button").className).toContain("text-xs");
  });

  it("block adds w-full", () => {
    const { getByRole } = render(() => <Button block>Wide</Button>);
    expect(getByRole("button").className).toContain("w-full");
  });

  it("disabled prevents clicks", () => {
    const onClick = vi.fn();
    const { getByRole } = render(() => (
      <Button disabled onClick={onClick}>
        Off
      </Button>
    ));
    const btn = getByRole("button") as HTMLButtonElement;
    expect(btn.disabled).toBe(true);
    fireEvent.click(btn);
    expect(onClick).not.toHaveBeenCalled();
  });

  it("calls onClick when clicked", () => {
    const onClick = vi.fn();
    const { getByRole } = render(() => <Button onClick={onClick}>Go</Button>);
    fireEvent.click(getByRole("button"));
    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it("respects type=submit", () => {
    const { getByRole } = render(() => <Button type="submit">Send</Button>);
    expect((getByRole("button") as HTMLButtonElement).type).toBe("submit");
  });
});

describe("Pill", () => {
  it("defaults to the neutral tone", () => {
    const { container } = render(() => <Pill>x</Pill>);
    const span = container.querySelector("span")!;
    expect(span.className).toContain("bg-zinc-100");
  });

  it.each(["emerald", "amber", "red", "indigo"] as const)(
    "applies %s tone classes",
    (tone) => {
      const { container } = render(() => <Pill tone={tone}>x</Pill>);
      const span = container.querySelector("span")!;
      expect(span.className).toMatch(new RegExp(`bg-${tone}-`));
    },
  );
});

describe("Toggle", () => {
  it("renders the label and exposes role=switch", () => {
    const { getByRole } = render(() => (
      <Toggle label="Billable" checked={false} onChange={() => {}} />
    ));
    const btn = getByRole("switch");
    expect(btn.textContent).toContain("Billable");
    expect(btn.getAttribute("aria-checked")).toBe("false");
  });

  it("reflects checked=true", () => {
    const { getByRole } = render(() => (
      <Toggle label="On" checked={true} onChange={() => {}} />
    ));
    const btn = getByRole("switch");
    expect(btn.getAttribute("aria-checked")).toBe("true");
    expect(btn.className).toContain("emerald");
  });

  it("invokes onChange with the inverted value when clicked", () => {
    const onChange = vi.fn();
    const { getByRole } = render(() => (
      <Toggle label="x" checked={false} onChange={onChange} />
    ));
    fireEvent.click(getByRole("switch"));
    expect(onChange).toHaveBeenCalledWith(true);
  });

  it("applies indigo tone when configured", () => {
    const { getByRole } = render(() => (
      <Toggle label="x" checked={true} tone="indigo" onChange={() => {}} />
    ));
    expect(getByRole("switch").className).toContain("indigo");
  });

  it("respects disabled", () => {
    const onChange = vi.fn();
    const { getByRole } = render(() => (
      <Toggle label="x" checked={false} disabled onChange={onChange} />
    ));
    const btn = getByRole("switch") as HTMLButtonElement;
    expect(btn.disabled).toBe(true);
    fireEvent.click(btn);
    expect(onChange).not.toHaveBeenCalled();
  });
});

describe("StatusDot", () => {
  it.each(["emerald", "amber", "red", "zinc", "indigo"] as const)(
    "applies fill class for tone=%s",
    (tone) => {
      const { container } = render(() => <StatusDot tone={tone} />);
      expect(container.querySelector(`.bg-${tone === "zinc" ? "zinc-300" : `${tone}-500`}`))
        .toBeTruthy();
    },
  );

  it("does not render the ping span by default", () => {
    const { container } = render(() => <StatusDot tone="emerald" />);
    expect(container.querySelector(".animate-ping")).toBeNull();
  });

  it("renders the ping span when ping=true", () => {
    const { container } = render(() => <StatusDot tone="emerald" ping />);
    expect(container.querySelector(".animate-ping")).toBeTruthy();
  });

  it("applies size classes (xs / sm / md)", () => {
    const { container: xs } = render(() => <StatusDot tone="emerald" size="xs" />);
    const { container: md } = render(() => <StatusDot tone="emerald" size="md" />);
    expect(xs.querySelector(".h-1")).toBeTruthy();
    expect(md.querySelector(".h-2")).toBeTruthy();
  });
});

describe("SectionLabel", () => {
  it("wraps children in an uppercase styled span", () => {
    const { container } = render(() => <SectionLabel>tracking</SectionLabel>);
    const span = container.querySelector("span")!;
    expect(span.textContent).toBe("tracking");
    expect(span.className).toContain("uppercase");
  });
});

describe("Accordion", () => {
  it("renders the title and starts closed", () => {
    const { getByRole, queryByText } = render(() => (
      <Accordion title="Calendar">body</Accordion>
    ));
    const btn = getByRole("button");
    expect(btn.getAttribute("aria-expanded")).toBe("false");
    expect(queryByText("body")).toBeNull();
  });

  it("starts open when defaultOpen is true", () => {
    const { getByRole, getByText } = render(() => (
      <Accordion title="Calendar" defaultOpen>
        body
      </Accordion>
    ));
    expect(getByRole("button").getAttribute("aria-expanded")).toBe("true");
    expect(getByText("body")).toBeDefined();
  });

  it("toggles when the header is clicked", () => {
    const { getByRole, queryByText } = render(() => (
      <Accordion title="Calendar">body</Accordion>
    ));
    const btn = getByRole("button");
    expect(queryByText("body")).toBeNull();
    fireEvent.click(btn);
    expect(btn.getAttribute("aria-expanded")).toBe("true");
    expect(queryByText("body")).not.toBeNull();
    fireEvent.click(btn);
    expect(queryByText("body")).toBeNull();
  });

  it("renders the right slot", () => {
    const { getByText } = render(() => (
      <Accordion title="Calendar" right={<span>3 events</span>}>
        body
      </Accordion>
    ));
    expect(getByText("3 events")).toBeDefined();
  });

  it("renders the optional hint", () => {
    const { getByText } = render(() => (
      <Accordion title="Calendar" hint="connect a calendar">
        body
      </Accordion>
    ));
    expect(getByText("connect a calendar")).toBeDefined();
  });
});

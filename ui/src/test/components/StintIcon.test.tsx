import { describe, expect, it } from "vitest";
import { render } from "@solidjs/testing-library";
import StintIcon from "~/components/StintIcon";

describe("<StintIcon>", () => {
  it("renders an SVG with the default size of 44", () => {
    const { container } = render(() => <StintIcon />);
    const svg = container.querySelector("svg")!;
    expect(svg).toBeDefined();
    expect(svg.getAttribute("width")).toBe("44");
    expect(svg.getAttribute("height")).toBe("44");
    expect(svg.getAttribute("viewBox")).toBe("0 0 44 44");
    expect(svg.getAttribute("aria-hidden")).toBe("true");
  });

  it("respects custom size", () => {
    const { container } = render(() => <StintIcon size={64} />);
    const svg = container.querySelector("svg")!;
    expect(svg.getAttribute("width")).toBe("64");
    expect(svg.getAttribute("height")).toBe("64");
  });

  it("applies fill to all child shapes", () => {
    const { container } = render(() => <StintIcon fill="red" />);
    const rects = container.querySelectorAll("rect");
    expect(rects.length).toBeGreaterThan(0);
    rects.forEach((r) => expect(r.getAttribute("fill")).toBe("red"));
  });

  it("applies the optional class to the svg root", () => {
    const { container } = render(() => <StintIcon class="my-icon" />);
    const svg = container.querySelector("svg")!;
    expect(svg.classList.contains("my-icon")).toBe(true);
  });
});

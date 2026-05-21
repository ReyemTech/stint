import { describe, expect, it } from "vitest";

// Proof-of-life: confirms the vitest + jsdom harness loads, the `~/` path
// alias resolves, and a DOM is available. Real coverage lands in
// stores/timer.test.ts, lib/openSolidtime.test.ts, etc.
describe("test harness", () => {
  it("runs in a jsdom-backed environment", () => {
    expect(typeof window).toBe("object");
    expect(typeof document).toBe("object");
  });

  it("can construct a DOM node", () => {
    const div = document.createElement("div");
    div.textContent = "hello";
    expect(div.textContent).toBe("hello");
  });
});

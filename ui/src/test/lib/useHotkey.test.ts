import { describe, expect, it, vi, afterEach } from "vitest";
import { createRoot } from "solid-js";
import { useHotkey } from "~/lib/useHotkey";

// jsdom's navigator.platform is usually a non-Mac value. Override it
// per-test where the mac vs ctrl detection matters, then restore.
const ORIGINAL_PLATFORM = navigator.platform;

function setPlatform(value: string) {
  Object.defineProperty(navigator, "platform", {
    value,
    configurable: true,
    writable: true,
  });
}

afterEach(() => {
  setPlatform(ORIGINAL_PLATFORM);
});

/// Dispatch a synthetic keydown on `window` (or the given target) so the
/// listener attached by useHotkey picks it up.
function pressKey(opts: {
  key: string;
  metaKey?: boolean;
  ctrlKey?: boolean;
  shiftKey?: boolean;
  altKey?: boolean;
  target?: HTMLElement;
}) {
  const ev = new KeyboardEvent("keydown", {
    key: opts.key,
    metaKey: opts.metaKey ?? false,
    ctrlKey: opts.ctrlKey ?? false,
    shiftKey: opts.shiftKey ?? false,
    altKey: opts.altKey ?? false,
    bubbles: true,
  });
  if (opts.target) {
    opts.target.dispatchEvent(ev);
  } else {
    window.dispatchEvent(ev);
  }
  return ev;
}

describe("useHotkey", () => {
  it("fires the callback for a bare letter key", () => {
    const fn = vi.fn();
    createRoot((dispose) => {
      useHotkey("s", fn);
      pressKey({ key: "s" });
      expect(fn).toHaveBeenCalledTimes(1);
      dispose();
    });
  });

  it("does not fire when the wrong key is pressed", () => {
    const fn = vi.fn();
    createRoot((dispose) => {
      useHotkey("s", fn);
      pressKey({ key: "x" });
      expect(fn).not.toHaveBeenCalled();
      dispose();
    });
  });

  it("requires the mod key — naked letter alone does not fire mod+letter", () => {
    setPlatform("Linux x86_64");
    const fn = vi.fn();
    createRoot((dispose) => {
      useHotkey("mod+s", fn);
      pressKey({ key: "s" });
      expect(fn).not.toHaveBeenCalled();
      dispose();
    });
  });

  it("treats mod as Cmd on macOS", () => {
    setPlatform("MacIntel");
    const fn = vi.fn();
    createRoot((dispose) => {
      useHotkey("mod+s", fn);
      pressKey({ key: "s", metaKey: true });
      expect(fn).toHaveBeenCalledTimes(1);
      // Ctrl alone does NOT match on Mac.
      fn.mockClear();
      pressKey({ key: "s", ctrlKey: true });
      expect(fn).not.toHaveBeenCalled();
      dispose();
    });
  });

  it("treats mod as Ctrl on non-Mac platforms", () => {
    setPlatform("Linux x86_64");
    const fn = vi.fn();
    createRoot((dispose) => {
      useHotkey("mod+s", fn);
      pressKey({ key: "s", ctrlKey: true });
      expect(fn).toHaveBeenCalledTimes(1);
      // Cmd alone does NOT match on non-Mac.
      fn.mockClear();
      pressKey({ key: "s", metaKey: true });
      expect(fn).not.toHaveBeenCalled();
      dispose();
    });
  });

  it("respects shift modifier", () => {
    const fn = vi.fn();
    createRoot((dispose) => {
      useHotkey("shift+a", fn);
      pressKey({ key: "a", shiftKey: true });
      expect(fn).toHaveBeenCalledTimes(1);
      fn.mockClear();
      pressKey({ key: "a" });
      expect(fn).not.toHaveBeenCalled();
      dispose();
    });
  });

  it("maps esc → Escape", () => {
    const fn = vi.fn();
    createRoot((dispose) => {
      useHotkey("esc", fn);
      pressKey({ key: "Escape" });
      expect(fn).toHaveBeenCalledTimes(1);
      dispose();
    });
  });

  it("does NOT fire when typing inside an INPUT element", () => {
    const fn = vi.fn();
    createRoot((dispose) => {
      useHotkey("s", fn);
      const input = document.createElement("input");
      document.body.appendChild(input);
      pressKey({ key: "s", target: input });
      expect(fn).not.toHaveBeenCalled();
      input.remove();
      dispose();
    });
  });

  it("does NOT fire when typing inside a TEXTAREA", () => {
    const fn = vi.fn();
    createRoot((dispose) => {
      useHotkey("s", fn);
      const ta = document.createElement("textarea");
      document.body.appendChild(ta);
      pressKey({ key: "s", target: ta });
      expect(fn).not.toHaveBeenCalled();
      ta.remove();
      dispose();
    });
  });

  // (contentEditable case not asserted here — jsdom doesn't implement
  // `HTMLElement.isContentEditable` reliably. Production browsers do,
  // and the production code checks it the same way they expose it.)

  it("stops firing after the root is disposed", () => {
    const fn = vi.fn();
    let dispose: () => void = () => {};
    createRoot((d) => {
      dispose = d;
      useHotkey("s", fn);
    });
    pressKey({ key: "s" });
    expect(fn).toHaveBeenCalledTimes(1);
    dispose();
    pressKey({ key: "s" });
    expect(fn).toHaveBeenCalledTimes(1); // not 2 — listener removed
  });
});

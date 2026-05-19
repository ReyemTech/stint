import { onCleanup } from "solid-js";

/**
 * Binds a keyboard shortcut for the lifetime of the calling component.
 *
 * Combo syntax (case-insensitive, plus-separated):
 *   "mod+,"      → ⌘, on macOS, Ctrl+, elsewhere
 *   "mod+shift+s" → ⌘⇧S
 *   "esc"        → bare Escape
 *
 * Only matches when the focused element isn't an editable text field,
 * so typing "s" in a description input doesn't trigger a sync shortcut.
 */
export function useHotkey(combo: string, fn: (e: KeyboardEvent) => void) {
  const wants = parse(combo);

  const handler = (e: KeyboardEvent) => {
    if (!matches(wants, e)) return;
    if (isTypingTarget(e.target)) return;
    e.preventDefault();
    fn(e);
  };

  window.addEventListener("keydown", handler);
  onCleanup(() => window.removeEventListener("keydown", handler));
}

type ParsedCombo = {
  key: string;
  mod: boolean;
  shift: boolean;
  alt: boolean;
};

function parse(combo: string): ParsedCombo {
  const parts = combo.toLowerCase().split("+").map((s) => s.trim());
  const out: ParsedCombo = {
    key: "",
    mod: false,
    shift: false,
    alt: false,
  };
  for (const p of parts) {
    if (p === "mod" || p === "cmd" || p === "ctrl") out.mod = true;
    else if (p === "shift") out.shift = true;
    else if (p === "alt" || p === "option" || p === "opt") out.alt = true;
    else out.key = p;
  }
  return out;
}

function matches(c: ParsedCombo, e: KeyboardEvent): boolean {
  const isMac = navigator.platform.toLowerCase().includes("mac");
  const mod = isMac ? e.metaKey : e.ctrlKey;
  if (c.mod !== mod) return false;
  if (c.shift !== e.shiftKey) return false;
  if (c.alt !== e.altKey) return false;
  const key = (e.key ?? "").toLowerCase();
  if (c.key === "esc") return key === "escape";
  return key === c.key;
}

function isTypingTarget(t: EventTarget | null): boolean {
  if (!(t instanceof HTMLElement)) return false;
  if (t.isContentEditable) return true;
  const tag = t.tagName;
  return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT";
}

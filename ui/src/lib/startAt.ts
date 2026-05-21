/// Compute an ISO string at `minutesAgo` before `now`. Pure helper so the
/// StartAtPicker preset buttons are independently testable.
export function isoMinutesAgo(minutesAgo: number, now: Date = new Date()): string {
  return new Date(now.getTime() - minutesAgo * 60_000).toISOString();
}

/// Take a "HH:MM" string and return an ISO timestamp for that time TODAY.
/// If the resulting moment is in the future relative to `now`, treats it as
/// yesterday at that time (a 09:30 entry typed at 06:00 means yesterday).
/// Returns null on invalid input.
export function parseLocalHHMMTodayOrYesterday(
  hhmm: string,
  now: Date = new Date(),
): string | null {
  const trimmed = hhmm.trim();
  if (!/^\d{1,2}:\d{2}$/.test(trimmed)) return null;
  const [hStr, mStr] = trimmed.split(":");
  const h = parseInt(hStr, 10);
  const m = parseInt(mStr, 10);
  if (h > 23 || m > 59) return null;
  const out = new Date(now);
  out.setHours(h, m, 0, 0);
  if (out.getTime() > now.getTime()) {
    out.setDate(out.getDate() - 1);
  }
  return out.toISOString();
}

/// Compose a human-readable label for the StartAtPicker trigger. Pure so
/// the rendering decision is testable without booting the component.
export function startAtLabel(
  value: string | null,
  now: Date = new Date(),
): string {
  if (!value) return "Start now";
  const diffMs = now.getTime() - new Date(value).getTime();
  // Anything under a minute reads as "now" — Math.round would call 30s "1 min".
  if (diffMs < 60_000) return "Start now";
  const minsAgo = Math.round(diffMs / 60_000);
  if (minsAgo < 60) return `Start ${minsAgo} min ago`;
  const hrs = Math.round(minsAgo / 6) / 10;
  return `Start ${hrs}h ago`;
}

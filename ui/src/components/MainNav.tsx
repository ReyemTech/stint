import { openSolidtime } from "~/lib/openSolidtime";

type Route = "today" | "settings" | "about";

const LINKS: { route: Route; label: string; href: string }[] = [
  { route: "today", label: "Today", href: "#/today" },
  { route: "settings", label: "Settings", href: "#/settings" },
  { route: "about", label: "About", href: "#/about" },
];

/**
 * Top-right navigation shown on every main-window route. Pass the
 * current route key as `active`; pass optional leading content (e.g. the
 * sync badge) via the `leading` prop.
 */
export default function MainNav(props: {
  active: Route;
  leading?: any;
}) {
  return (
    <nav class="flex items-center gap-1 text-xs">
      {props.leading}
      {LINKS.map((l) => (
        <a
          href={l.href}
          class="rounded-md px-2.5 py-1.5 transition"
          classList={{
            "text-zinc-900 dark:text-zinc-100": props.active === l.route,
            "text-zinc-500 hover:bg-zinc-100 hover:text-zinc-900 dark:text-zinc-400 dark:hover:bg-zinc-800 dark:hover:text-zinc-100":
              props.active !== l.route,
          }}
        >
          {l.label}
        </a>
      ))}
      <button
        class="rounded-md px-2.5 py-1.5 text-zinc-500 transition hover:bg-zinc-100 hover:text-zinc-900 dark:text-zinc-400 dark:hover:bg-zinc-800 dark:hover:text-zinc-100"
        onClick={() => openSolidtime()}
        title="Open Solidtime in browser"
      >
        Solidtime ↗
      </button>
    </nav>
  );
}

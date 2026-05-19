import type { JSX } from "solid-js";

type Variant = "primary" | "secondary" | "danger" | "ghost";
type Size = "sm" | "md";

const BASE =
  "inline-flex items-center justify-center gap-1.5 rounded-lg font-semibold shadow-sm transition active:scale-[0.99] disabled:cursor-not-allowed disabled:opacity-40";

const VARIANTS: Record<Variant, string> = {
  primary:
    "bg-zinc-900 text-white hover:bg-zinc-700 dark:bg-white dark:text-zinc-900 dark:hover:bg-zinc-100",
  secondary:
    "border border-zinc-200 bg-white text-zinc-700 hover:bg-zinc-50 shadow-none dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-zinc-800",
  danger: "bg-red-500 text-white hover:bg-red-600",
  ghost:
    "shadow-none text-zinc-500 hover:bg-zinc-100 hover:text-zinc-900 dark:text-zinc-400 dark:hover:bg-zinc-800 dark:hover:text-zinc-100",
};

const SIZES: Record<Size, string> = {
  sm: "px-2.5 py-1 text-xs",
  md: "px-3.5 py-1.5 text-sm",
};

/**
 * Single source of truth for buttons. Variant + size keep the visual
 * language consistent across the app.
 */
export default function Button(props: {
  variant?: Variant;
  size?: Size;
  type?: "button" | "submit" | "reset";
  disabled?: boolean;
  block?: boolean;
  title?: string;
  onClick?: (e: MouseEvent) => void | Promise<void>;
  children: JSX.Element;
}) {
  const variant = () => props.variant ?? "primary";
  const size = () => props.size ?? "md";

  return (
    <button
      type={props.type ?? "button"}
      disabled={props.disabled}
      title={props.title}
      onClick={(e) => props.onClick?.(e)}
      class={`${BASE} ${VARIANTS[variant()]} ${SIZES[size()]} ${
        props.block ? "w-full" : ""
      }`}
    >
      {props.children}
    </button>
  );
}

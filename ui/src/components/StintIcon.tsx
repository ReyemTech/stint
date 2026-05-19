/**
 * Stint stopwatch logo. Same silhouette as the menu-bar tray icon
 * (crates/stint-app/icons/src/tray.svg). Kept inline so both surfaces
 * share the source of truth — if the SVG drifts, change both.
 */
export default function StintIcon(props: {
  size?: number;
  class?: string;
  fill?: string;
}) {
  const size = props.size ?? 44;
  const fill = props.fill ?? "currentColor";
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 44 44"
      fill="none"
      class={props.class}
      aria-hidden="true"
    >
      <rect x="19" y="4" width="6" height="4" rx="1.2" fill={fill} />
      <rect
        x="30"
        y="7.5"
        width="3.5"
        height="3"
        rx="0.8"
        transform="rotate(45 31.75 9)"
        fill={fill}
      />
      <circle
        cx="22"
        cy="26"
        r="13.5"
        stroke={fill}
        stroke-width="3"
        fill="none"
      />
      <rect x="21" y="13.5" width="2" height="3" fill={fill} />
      <rect
        x="21"
        y="17"
        width="2"
        height="11"
        rx="1"
        fill={fill}
        transform="rotate(45 22 26)"
      />
      <circle cx="22" cy="26" r="2" fill={fill} />
    </svg>
  );
}

import type { CSSProperties } from "react";

/**
 * Minimal inline-SVG icon set standing in for the SF Symbols used by the Swift
 * views. The project has no icon library, so these are hand-drawn and sized
 * relative to `font-size` (1em square) to inherit the surrounding text scale.
 */

export type IconName =
  | "pause"
  | "play"
  | "stop"
  | "chevron-up"
  | "chevron-down"
  | "eraser"
  | "gear"
  | "sparkles"
  | "checkmark"
  | "checkmark-circle"
  | "captions-bubble"
  | "text-quote"
  | "waveform"
  | "waveform-slash"
  | "key"
  | "exclamation-triangle";

interface IconProps {
  name: IconName;
  className?: string;
  style?: CSSProperties;
}

export function Icon({ name, className, style }: IconProps) {
  return (
    <svg
      viewBox="0 0 24 24"
      width="1em"
      height="1em"
      fill="currentColor"
      className={className}
      style={style}
      aria-hidden="true"
    >
      {renderPath(name)}
    </svg>
  );
}

function renderPath(name: IconName) {
  switch (name) {
    case "pause":
      return (
        <>
          <rect x="6" y="4" width="4" height="16" rx="1.2" />
          <rect x="14" y="4" width="4" height="16" rx="1.2" />
        </>
      );
    case "play":
      return (
        <path d="M7.5 4.6a1 1 0 0 1 1.53-.85l11.5 7.4a1 1 0 0 1 0 1.7l-11.5 7.4a1 1 0 0 1-1.53-.85V4.6z" />
      );
    case "stop":
      return <rect x="6" y="6" width="12" height="12" rx="2.4" />;
    case "chevron-up":
      return (
        <path
          d="M6 14.5l6-6 6 6"
          fill="none"
          stroke="currentColor"
          strokeWidth="2.5"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      );
    case "chevron-down":
      return (
        <path
          d="M6 9.5l6 6 6-6"
          fill="none"
          stroke="currentColor"
          strokeWidth="2.5"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      );
    case "eraser":
      // SF Symbol "eraser.fill": a solid, chunky eraser tilted at 45° with
      // rounded ends. Drawn as a round-capped thick stroke (the same
      // geometry as a filled stadium) — near-180° arc pairs are numerically
      // degenerate and render self-intersecting in some engines.
      return (
        <path
          d="M6.5 18.5L17.5 7.5"
          fill="none"
          stroke="currentColor"
          strokeWidth="5.6"
          strokeLinecap="round"
        />
      );
    case "gear":
      return (
        <>
          <path
            d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.7"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
          <circle cx="12" cy="12" r="3.2" fill="currentColor" />
        </>
      );
    case "sparkles":
      return (
        <path d="M12 3.4c.55 4.2 2.4 6.05 6.6 6.6-4.2.55-6.05 2.4-6.6 6.6-.55-4.2-2.4-6.05-6.6-6.6 4.2-.55 6.05-2.4 6.6-6.6z" />
      );
    case "checkmark":
      return (
        <path
          d="M5 12.5l4.5 4.5L19 7.5"
          fill="none"
          stroke="currentColor"
          strokeWidth="2.2"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      );
    case "checkmark-circle":
      return (
        <>
          <circle cx="12" cy="12" r="9" />
          <path
            d="M7.5 12.6l3 3L16.5 9.4"
            fill="none"
            stroke="#ffffff"
            strokeWidth="1.9"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </>
      );
    case "captions-bubble":
      return (
        <>
          <path
            d="M4 5.5A3.5 3.5 0 0 1 7.5 2h9A3.5 3.5 0 0 1 20 5.5v8a3.5 3.5 0 0 1-3.5 3.5H11l-4.3 3.6a1 1 0 0 1-1.7-.8V17A3.5 3.5 0 0 1 4 13.5v-8z"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.8"
          />
          <rect x="6.8" y="6.6" width="10.4" height="1.9" rx="0.95" />
          <rect x="6.8" y="10.6" width="6.4" height="1.9" rx="0.95" />
        </>
      );
    case "text-quote":
      return (
        <path d="M7 5.5A3.5 3.5 0 0 0 3.5 9v3A2 2 0 0 0 5.5 14h.2a1.5 1.5 0 0 0 1.5-1.5V9H5.2V9A2.3 2.3 0 0 1 7.3 6.7H7V5.5zm10 0A3.5 3.5 0 0 0 13.5 9v3a2 2 0 0 0 2 2h.2a1.5 1.5 0 0 0 1.5-1.5V9h-2V9a2.3 2.3 0 0 1 2.1-2.3h-.3V5.5z" />
      );
    case "waveform":
      return (
        <>
          <rect x="3.4" y="10.5" width="2.2" height="3" rx="1.1" />
          <rect x="7.4" y="7.5" width="2.2" height="9" rx="1.1" />
          <rect x="11.4" y="4.5" width="2.2" height="15" rx="1.1" />
          <rect x="15.4" y="7.5" width="2.2" height="9" rx="1.1" />
          <rect x="19.4" y="10.5" width="2.2" height="3" rx="1.1" />
        </>
      );
    case "waveform-slash":
      return (
        <>
          <rect x="3.4" y="10.5" width="2.2" height="3" rx="1.1" />
          <rect x="7.4" y="7.5" width="2.2" height="9" rx="1.1" />
          <rect x="11.4" y="4.5" width="2.2" height="15" rx="1.1" />
          <rect x="15.4" y="7.5" width="2.2" height="9" rx="1.1" />
          <rect x="19.4" y="10.5" width="2.2" height="3" rx="1.1" />
          <path
            d="M4 4l16 16"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
          />
        </>
      );
    case "key":
      return (
        <>
          <circle
            cx="7.6"
            cy="15.4"
            r="3.6"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.8"
          />
          <path
            d="M10.2 12.8l8.3-8.3 1.8 1.8-8.3 8.3"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.8"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
          <path
            d="M15.4 6.4l2 2"
            stroke="currentColor"
            strokeWidth="1.8"
            strokeLinecap="round"
          />
        </>
      );
    case "exclamation-triangle":
      return (
        <>
          <path d="M12 3.2 21.3 20H2.7L12 3.2z" />
          <rect x="11" y="8.6" width="2" height="6" rx="1" fill="#ffffff" />
          <rect x="11" y="16.6" width="2" height="2" rx="1" fill="#ffffff" />
        </>
      );
  }
}

import type { CSSProperties } from "react";
import {
  AlertTriangle,
  AudioLines,
  Captions,
  CircleCheck,
  Check,
  ChevronDown,
  ChevronUp,
  CircleSlash,
  Eraser,
  Key,
  Pause,
  Play,
  Quote,
  Settings,
  Sparkles,
  Square,
  type LucideIcon,
} from "lucide-react";

/**
 * Icon set backed by lucide-react (modern, MIT, tree-shakeable SVG
 * components). The `IconName` union mirrors the SF Symbols used by the
 * Swift views; the component renders 1em-square and inherits the current
 * text color, so call sites keep using `fontSize`/`color` styles.
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

const ICONS: Record<IconName, LucideIcon> = {
  pause: Pause,
  play: Play,
  stop: Square,
  "chevron-up": ChevronUp,
  "chevron-down": ChevronDown,
  eraser: Eraser,
  gear: Settings,
  sparkles: Sparkles,
  checkmark: Check,
  "checkmark-circle": CircleCheck,
  "captions-bubble": Captions,
  "text-quote": Quote,
  waveform: AudioLines,
  "waveform-slash": CircleSlash,
  key: Key,
  "exclamation-triangle": AlertTriangle,
};

interface IconProps {
  name: IconName;
  className?: string;
  style?: CSSProperties;
}

export function Icon({ name, className, style }: IconProps) {
  const Component = ICONS[name];
  return (
    <Component
      className={className}
      style={style}
      width="1em"
      height="1em"
      aria-hidden="true"
    />
  );
}

/** The underlying lucide icon for a name (used where an icon needs to be
 * passed as a component). */
export function iconFor(name: IconName): LucideIcon {
  return ICONS[name];
}

import type { CSSProperties } from "react";
import {
  AlertTriangle,
  AppWindow,
  Captions,
  CircleCheck,
  Check,
  ChevronDown,
  ChevronUp,
  Cloud,
  Eraser,
  Key,
  Languages,
  LockKeyhole,
  Pause,
  Play,
  Plus,
  Settings,
  ShieldCheck,
  Sparkles,
  Square,
  Trash2,
  Waves,
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
  | "key"
  | "exclamation-triangle"
  | "app-window"
  | "cloud"
  | "languages"
  | "lock"
  | "plus"
  | "shield-check"
  | "trash"
  | "waves";

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
  key: Key,
  "exclamation-triangle": AlertTriangle,
  "app-window": AppWindow,
  cloud: Cloud,
  languages: Languages,
  lock: LockKeyhole,
  plus: Plus,
  "shield-check": ShieldCheck,
  trash: Trash2,
  waves: Waves,
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

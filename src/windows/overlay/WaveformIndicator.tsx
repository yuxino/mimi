import {
  OVERLAY_ACTIVITY_PHASES,
  overlayPhaseColor,
  type OverlayActivityPhaseKind,
} from "../../lib/types";
import { useReducedMotion, useTimelineTime } from "./animation";

interface WaveformIndicatorProps {
  phase: OverlayActivityPhaseKind;
  compact?: boolean;
}

const REDUCED_HEIGHTS = [11, 18, 26, 33, 38, 33, 26, 18, 11];
const REDUCED_HEIGHTS_COMPACT = [6, 11, 15, 11, 6];

export function WaveformIndicator({
  phase,
  compact = false,
}: WaveformIndicatorProps) {
  const reduceMotion = useReducedMotion();
  const active = !reduceMotion && phase !== "paused";
  const time = useTimelineTime(active);
  const info = OVERLAY_ACTIVITY_PHASES[phase];

  const barCount = compact ? 5 : 9;
  const spacing = compact ? 3.25 : 4;
  const barWidth = compact ? 2.75 : 4;

  const bars: Array<{ height: number; opacity: number }> = [];
  for (let index = 0; index < barCount; index += 1) {
    bars.push({
      height: barHeight(index, time, reduceMotion, compact, info),
      opacity: barOpacity(index, compact),
    });
  }

  return (
    <div className="flex items-center" style={{ gap: `${spacing}px` }}>
      {bars.map((bar, index) => (
        <div
          key={index}
          style={{
            width: `${barWidth}px`,
            height: `${bar.height}px`,
            borderRadius: 999,
            background: overlayPhaseColor(phase, bar.opacity),
          }}
        />
      ))}
    </div>
  );
}

function barHeight(
  index: number,
  time: number,
  reduceMotion: boolean,
  compact: boolean,
  info: (typeof OVERLAY_ACTIVITY_PHASES)[OverlayActivityPhaseKind],
): number {
  if (reduceMotion) {
    return compact ? REDUCED_HEIGHTS_COMPACT[index] : REDUCED_HEIGHTS[index];
  }
  const wave =
    (Math.sin(time * info.animationSpeed + index * (compact ? 1.2 : 0.85)) + 1) /
    2;
  const base = compact
    ? 5.5 + Math.abs(index - 2) * 1.9
    : 10 + Math.abs(index - 4) * 2.2;
  return base + wave * info.amplitude * (compact ? 2.1 : 3.2);
}

function barOpacity(index: number, compact: boolean): number {
  const center = compact ? Math.abs(index - 2) : Math.abs(index - 4);
  return 0.92 - center * 0.09;
}

import {
  OVERLAY_ACTIVITY_PHASES,
  overlayPhaseColor,
  type OverlayActivityPhaseKind,
} from "../../lib/types";
import { useReducedMotion, useTimelineTime } from "./animation";

interface RecognitionActivityIndicatorProps {
  phase: OverlayActivityPhaseKind;
}

const REDUCED_HEIGHTS = [4, 8, 5.5];

export function RecognitionActivityIndicator({
  phase,
}: RecognitionActivityIndicatorProps) {
  const reduceMotion = useReducedMotion();
  const active = !reduceMotion && phase !== "paused";
  const time = useTimelineTime(active);
  const info = OVERLAY_ACTIVITY_PHASES[phase];
  const translating = phase === "translating";
  const pulse = pulseProgress(time, reduceMotion);
  const barColor = overlayPhaseColor(phase, 1);

  return (
    <div
      className="relative flex items-center justify-center"
      style={{ width: 22, height: 22 }}
    >
      {translating && (
        <>
          <div
            style={{
              position: "absolute",
              width: innerGlowSize(pulse),
              height: innerGlowSize(pulse),
              borderRadius: "50%",
              background: overlayPhaseColor(phase, innerGlowOpacity(pulse)),
            }}
          />
          <div
            style={{
              position: "absolute",
              width: outerRingSize(pulse),
              height: outerRingSize(pulse),
              borderRadius: "50%",
              border: `1px solid ${overlayPhaseColor(phase, outerRingOpacity(pulse))}`,
            }}
          />
        </>
      )}
      <div
        className="relative flex items-center"
        style={{ width: 11, height: 14, gap: 1.75 }}
      >
        {[0, 1, 2].map((index) => (
          <div
            key={index}
            style={{
              width: 2.25,
              height: barHeight(index, time, reduceMotion, info),
              borderRadius: 999,
              background: barColor,
            }}
          />
        ))}
      </div>
    </div>
  );
}

function barHeight(
  index: number,
  time: number,
  reduceMotion: boolean,
  info: (typeof OVERLAY_ACTIVITY_PHASES)[OverlayActivityPhaseKind],
): number {
  if (reduceMotion) return REDUCED_HEIGHTS[index];
  const wave =
    (Math.sin(time * info.animationSpeed + index * 1.7) + 1) / 2;
  return 2.5 + wave * info.amplitude * 1.3;
}

function pulseProgress(time: number, reduceMotion: boolean): number {
  if (reduceMotion) return 0.45;
  return (Math.sin(time * 3.2) + 1) / 2;
}

function innerGlowSize(pulse: number): number {
  return 10 + pulse * 4;
}

function innerGlowOpacity(pulse: number): number {
  return 0.13 + pulse * 0.15;
}

function outerRingSize(pulse: number): number {
  return 10 + pulse * 8;
}

function outerRingOpacity(pulse: number): number {
  return 0.34 - pulse * 0.22;
}

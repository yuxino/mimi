import { memo, useEffect, useRef } from "react";
import {
  OVERLAY_ACTIVITY_PHASES,
  overlayPhaseColor,
  type OverlayActivityPhaseKind,
} from "../../lib/types";
import { useReducedMotion } from "./animation";

interface WaveformIndicatorProps {
  phase: OverlayActivityPhaseKind;
  /** The status-bar variant: the identical 9-bar wave scaled down. Shape,
   * cadence and per-bar offsets match the large variant exactly, so the
   * empty-state wave and the status-bar wave move the same way. */
  compact?: boolean;
}

// Static (reduced-motion) bar heights, matching the Swift table. The
// compact variant is the same 9-bar shape scaled by 0.58.
const REDUCED_HEIGHTS = [11, 18, 26, 33, 38, 33, 26, 18, 11];
const REDUCED_HEIGHTS_COMPACT = REDUCED_HEIGHTS.map((h) => Math.round(h * 0.58));

// The tallest a bar can reach for each variant, used as the fixed-height
// container the bar scales inside. scaleY never triggers layout, so the
// wave runs on the compositor thread instead of reflowing every frame.
const MAX_HEIGHT = 38;
const MAX_HEIGHT_COMPACT = 22;

/**
 * The recognition waveform. Each bar is a fixed-height div scaled with
 * `transform: scaleY()` written directly to the DOM from a requestAnimationFrame
 * loop — never via React state and never via `height` (which would reflow the
 * layout every frame and stutter inside the transparent overlay window).
 * Memoized: the overlay re-renders on every session-state event, but the wave
 * only needs to re-render when its phase or variant actually changes.
 */
export const WaveformIndicator = memo(function WaveformIndicator({
  phase,
  compact = false,
}: WaveformIndicatorProps) {
  const reduceMotion = useReducedMotion();
  const active = !reduceMotion && phase !== "paused";

  // Both variants share the identical 9-bar shape; compact only scales the
  // whole wave (spacing/width/height × 0.58), never the shape or cadence.
  const barCount = 9;
  const spacing = compact ? 2.6 : 4;
  const barWidth = compact ? 2.2 : 4;
  const maxHeight = compact ? MAX_HEIGHT_COMPACT : MAX_HEIGHT;
  const staticHeights = compact ? REDUCED_HEIGHTS_COMPACT : REDUCED_HEIGHTS;
  const scale = compact ? 0.58 : 1;

  // Ref arrays updated imperatively; the bars are rendered once and their
  // transforms are driven by the rAF loop below.
  const barRefs = useRef<Array<HTMLDivElement | null>>([]);
  const phaseRef = useRef(phase);

  // Keep the rAF loop reading the latest phase without restarting it.
  useEffect(() => {
    phaseRef.current = phase;
  }, [phase]);

  useEffect(() => {
    const elements = barRefs.current;

    if (!active) {
      // Static wave: one fixed scale per bar (reduced motion / paused).
      elements.forEach((element, index) => {
        if (element) {
          element.style.transform = `scaleY(${staticHeights[index] / maxHeight})`;
        }
      });
      return;
    }

    let raf = 0;
    const loop = (now: number) => {
      const time = now / 1000;
      const currentInfo = OVERLAY_ACTIVITY_PHASES[phaseRef.current];
      elements.forEach((element, index) => {
        if (!element) return;
        const height = barHeight(index, time, scale, currentInfo);
        element.style.transform = `scaleY(${height / maxHeight})`;
      });
      raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(raf);
  }, [active, maxHeight, staticHeights, scale]);

  return (
    <div className="flex items-center" style={{ gap: `${spacing}px` }}>
      {Array.from({ length: barCount }, (_, index) => (
        <div
          key={index}
          ref={(element) => {
            barRefs.current[index] = element;
          }}
          style={{
            width: `${barWidth}px`,
            height: `${maxHeight}px`,
            borderRadius: 999,
            transformOrigin: "center",
            // Deliberately NO `willChange: transform`: promoting each bar to
            // its own compositing layer makes the transparent overlay window
            // software-composite N layers per frame (the macOS WKWebView
            // transparent-window path does not use GPU compositing), which is
            // the stutter source. One shared layer keeps a single composite.
            background: overlayPhaseColor(phase, barOpacity(index)),
          }}
        />
      ))}
    </div>
  );
});

function barHeight(
  index: number,
  time: number,
  scale: number,
  info: (typeof OVERLAY_ACTIVITY_PHASES)[OverlayActivityPhaseKind],
): number {
  const wave = (Math.sin(time * info.animationSpeed + index * 0.85) + 1) / 2;
  const base = (10 + Math.abs(index - 4) * 2.2) * scale;
  return base + wave * info.amplitude * 3.2 * scale;
}

function barOpacity(index: number): number {
  return 0.92 - Math.abs(index - 4) * 0.09;
}

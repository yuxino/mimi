import { memo, useEffect, useRef } from "react";
import {
  OVERLAY_ACTIVITY_PHASES,
  overlayPhaseColor,
  type OverlayActivityPhaseKind,
} from "../../lib/types";
import { useReducedMotion } from "./animation";

interface WaveformIndicatorProps {
  phase: OverlayActivityPhaseKind;
  compact?: boolean;
}

// Static (reduced-motion) bar heights, matching the Swift table.
const REDUCED_HEIGHTS = [11, 18, 26, 33, 38, 33, 26, 18, 11];
const REDUCED_HEIGHTS_COMPACT = [6, 11, 15, 11, 6];

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

  const barCount = compact ? 5 : 9;
  const spacing = compact ? 3.25 : 4;
  const barWidth = compact ? 2.75 : 4;
  const maxHeight = compact ? MAX_HEIGHT_COMPACT : MAX_HEIGHT;
  const staticHeights = compact ? REDUCED_HEIGHTS_COMPACT : REDUCED_HEIGHTS;

  // Ref arrays updated imperatively; the bars are rendered once and their
  // transforms are driven by the rAF loop below.
  const barRefs = useRef<Array<HTMLDivElement | null>>([]);
  const phaseRef = useRef(phase);
  const compactRef = useRef(compact);

  // Keep the rAF loop reading the latest props without restarting it.
  useEffect(() => {
    phaseRef.current = phase;
    compactRef.current = compact;
  }, [phase, compact]);

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
    // Phase accumulates by `dt * speed` each frame instead of deriving from
    // absolute time: (a) `animationSpeed` is cycles-per-second, so the wave
    // actually moves at the intended rate, and (b) when the phase changes
    // (listening → translating → recognizing) the motion continues smoothly
    // instead of jumping to an unrelated sin() phase.
    let wavePhase = 0;
    let lastNow = 0;
    const loop = (now: number) => {
      if (lastNow !== 0) {
        const dt = Math.min((now - lastNow) / 1000, 0.1);
        wavePhase +=
          dt * OVERLAY_ACTIVITY_PHASES[phaseRef.current].animationSpeed * 2 * Math.PI;
      }
      lastNow = now;
      const currentInfo = OVERLAY_ACTIVITY_PHASES[phaseRef.current];
      const currentCompact = compactRef.current;
      elements.forEach((element, index) => {
        if (!element) return;
        const height = barHeight(index, wavePhase, currentCompact, currentInfo);
        element.style.transform = `scaleY(${height / maxHeight})`;
      });
      raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(raf);
  }, [active, maxHeight, staticHeights]);

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
            background: overlayPhaseColor(phase, barOpacity(index, compact)),
          }}
        />
      ))}
    </div>
  );
});

function barHeight(
  index: number,
  wavePhase: number,
  compact: boolean,
  info: (typeof OVERLAY_ACTIVITY_PHASES)[OverlayActivityPhaseKind],
): number {
  const wave =
    (Math.sin(wavePhase + index * (compact ? 1.2 : 0.85)) + 1) / 2;
  const base = compact
    ? 5.5 + Math.abs(index - 2) * 1.9
    : 10 + Math.abs(index - 4) * 2.2;
  return base + wave * info.amplitude * (compact ? 2.1 : 3.2);
}

function barOpacity(index: number, compact: boolean): number {
  const center = compact ? Math.abs(index - 2) : Math.abs(index - 4);
  return 0.92 - center * 0.09;
}

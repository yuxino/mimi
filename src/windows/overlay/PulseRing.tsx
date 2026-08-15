import { memo, useEffect, useRef } from "react";
import {
  overlayPhaseColor,
  type OverlayActivityPhaseKind,
} from "../../lib/types";
import { useReducedMotion } from "./animation";

interface PulseRingProps {
  phase: OverlayActivityPhaseKind;
  /** The status-bar variant: the same pulse scaled down. */
  compact?: boolean;
}

// Three rings spread across the pulse cycle (0°, 120°, 240°), so the wave
// always has a ring mid-expansion. All active phases share one cadence; the
// phase only changes the color, keeping every state's motion identical.
const RING_COUNT = 3;
const PULSE_PERIOD_MS = 1400;

/**
 * The recognition activity indicator: a glowing center dot with rings that
 * ripple outward and fade. The rings expand via `transform: scale()` written
 * directly to the DOM from an rAF loop — no React state per frame, no layout
 * work — so it stays smooth inside the transparent always-on-top overlay.
 * Compact is the identical animation scaled down.
 */
export const PulseRing = memo(function PulseRing({
  phase,
  compact = false,
}: PulseRingProps) {
  const reduceMotion = useReducedMotion();
  const active = !reduceMotion && phase !== "paused";

  const dotRef = useRef<HTMLDivElement | null>(null);
  const ringRefs = useRef<Array<HTMLDivElement | null>>([]);
  const phaseRef = useRef(phase);

  useEffect(() => {
    phaseRef.current = phase;
  }, [phase]);

  useEffect(() => {
    const rings = ringRefs.current;
    const dot = dotRef.current;

    if (!active) {
      // Static pulse: one mid-size ring at rest.
      rings.forEach((ring, index) => {
        if (ring) {
          const t = index / RING_COUNT;
          ring.style.transform = `scale(${0.35 + t * 0.25})`;
          ring.style.opacity = String(0.3 - t * 0.08);
        }
      });
      if (dot) {
        dot.style.opacity = "0.9";
      }
      return;
    }

    let raf = 0;
    let lastNow = 0;
    const loop = (now: number) => {
      if (lastNow !== 0) {
        // The dot keeps a gentle breathing pulse; the rings ripple on their
        // own cadence independent of the phase table.
        const progress = (now % PULSE_PERIOD_MS) / PULSE_PERIOD_MS;
        if (dot) {
          const breathe = 0.75 + 0.25 * Math.sin((now / 700) * Math.PI * 2);
          dot.style.opacity = String(breathe);
        }
        rings.forEach((ring, index) => {
          if (!ring) return;
          const offset = index / RING_COUNT;
          const t = (progress + offset) % 1;
          // Expand from center and fade out as it travels.
          ring.style.transform = `scale(${0.2 + t * 1.1})`;
          ring.style.opacity = String(0.55 * (1 - t));
        });
      }
      lastNow = now;
      raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(raf);
  }, [active]);

  const base = compact ? 18 : 40;
  const color = overlayPhaseColor(phase, 1);

  return (
    <div
      className="relative"
      style={{ width: base, height: base }}
      aria-hidden="true"
    >
      {/* Ripple rings */}
      {Array.from({ length: RING_COUNT }, (_, index) => (
        <div
          key={index}
          ref={(element) => {
            ringRefs.current[index] = element;
          }}
          style={{
            position: "absolute",
            inset: 0,
            borderRadius: "50%",
            border: `${compact ? 1 : 1.5}px solid ${color}`,
            transformOrigin: "center",
            // Rings are drawn centered; the base box is the dot, rings grow
            // beyond it via scale.
          }}
        />
      ))}
      {/* Center dot */}
      <div
        ref={dotRef}
        style={{
          position: "absolute",
          left: "50%",
          top: "50%",
          width: compact ? 7 : 14,
          height: compact ? 7 : 14,
          marginLeft: compact ? -3.5 : -7,
          marginTop: compact ? -3.5 : -7,
          borderRadius: "50%",
          background: color,
          boxShadow: `0 0 ${compact ? 6 : 12}px ${overlayPhaseColor(phase, 0.6)}`,
          opacity: 0.9,
        }}
      />
    </div>
  );
});

import { useEffect, useRef } from "react";
import {
  OVERLAY_ACTIVITY_PHASES,
  overlayPhaseColor,
  type OverlayActivityPhaseKind,
} from "../../lib/types";
import { useReducedMotion } from "./animation";

interface RecognitionActivityIndicatorProps {
  phase: OverlayActivityPhaseKind;
}

const REDUCED_HEIGHTS = [4, 8, 5.5];

// Fixed-size containers the animated pieces scale inside (scaleY/scale never
// triggers layout, so the compact indicator runs on the compositor thread).
const BAR_MAX_HEIGHT = 11;
const GLOW_BOX = 14;
const RING_BOX = 18;

/**
 * Compact 3-bar recognition indicator (collapsed overlay). Sizes are driven
 * by `transform: scale()` written directly to the DOM from an rAF loop —
 * never via React state and never via `width`/`height` (which would reflow
 * the layout every frame and stutter inside the transparent overlay window).
 */
export function RecognitionActivityIndicator({
  phase,
}: RecognitionActivityIndicatorProps) {
  const reduceMotion = useReducedMotion();
  const active = !reduceMotion && phase !== "paused";
  const translating = phase === "translating";

  const barRefs = useRef<Array<HTMLDivElement | null>>([]);
  const glowRef = useRef<HTMLDivElement | null>(null);
  const ringRef = useRef<HTMLDivElement | null>(null);
  const phaseRef = useRef(phase);

  // Keep the rAF loop reading the latest phase without restarting it.
  useEffect(() => {
    phaseRef.current = phase;
  }, [phase]);

  useEffect(() => {
    if (!active) {
      // Static wave: fixed scales (reduced motion / paused).
      barRefs.current.forEach((element, index) => {
        if (element) {
          element.style.transform = `scaleY(${REDUCED_HEIGHTS[index] / BAR_MAX_HEIGHT})`;
        }
      });
      return;
    }

    let raf = 0;
    const loop = (now: number) => {
      const time = now / 1000;
      const currentInfo = OVERLAY_ACTIVITY_PHASES[phaseRef.current];
      const pulse = pulseProgress(time);
      barRefs.current.forEach((element, index) => {
        if (!element) return;
        const height = barHeight(index, time, currentInfo);
        element.style.transform = `scaleY(${height / BAR_MAX_HEIGHT})`;
      });
      if (glowRef.current) {
        glowRef.current.style.transform = `scale(${innerGlowSize(pulse) / GLOW_BOX})`;
        glowRef.current.style.opacity = String(innerGlowOpacity(pulse));
      }
      if (ringRef.current) {
        ringRef.current.style.transform = `scale(${outerRingSize(pulse) / RING_BOX})`;
        ringRef.current.style.opacity = String(outerRingOpacity(pulse));
      }
      raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(raf);
  }, [active]);

  const barColor = overlayPhaseColor(phase, 1);

  return (
    <div
      className="relative flex items-center justify-center"
      style={{ width: 22, height: 22 }}
    >
      {translating && (
        <>
          <div
            ref={(element) => {
              glowRef.current = element;
            }}
            style={{
              position: "absolute",
              width: GLOW_BOX,
              height: GLOW_BOX,
              borderRadius: "50%",
              transformOrigin: "center",
              background: overlayPhaseColor(phase, innerGlowOpacity(0.45)),
            }}
          />
          <div
            ref={(element) => {
              ringRef.current = element;
            }}
            style={{
              position: "absolute",
              width: RING_BOX,
              height: RING_BOX,
              borderRadius: "50%",
              transformOrigin: "center",
              border: `1px solid ${overlayPhaseColor(phase, outerRingOpacity(0.45))}`,
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
            ref={(element) => {
              barRefs.current[index] = element;
            }}
            style={{
              width: 2.25,
              height: BAR_MAX_HEIGHT,
              borderRadius: 999,
              transformOrigin: "center",
              willChange: "transform",
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
  info: (typeof OVERLAY_ACTIVITY_PHASES)[OverlayActivityPhaseKind],
): number {
  const wave = (Math.sin(time * info.animationSpeed + index * 1.7) + 1) / 2;
  return 2.5 + wave * info.amplitude * 1.3;
}

function pulseProgress(time: number): number {
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

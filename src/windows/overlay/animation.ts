import { useEffect, useRef, useState } from "react";

/** Tracks the user's reduced-motion preference reactively. */
export function useReducedMotion(): boolean {
  const [reduced, setReduced] = useState(
    () =>
      typeof window !== "undefined" &&
      window.matchMedia("(prefers-reduced-motion: reduce)").matches,
  );

  useEffect(() => {
    const query = window.matchMedia("(prefers-reduced-motion: reduce)");
    const onChange = (event: MediaQueryListEvent) => setReduced(event.matches);
    query.addEventListener("change", onChange);
    return () => query.removeEventListener("change", onChange);
  }, []);

  return reduced;
}

/**
 * A monotonically increasing time value (seconds) that advances at ~24fps
 * while `active` is true, matching the Swift `TimelineView` cadence. When
 * inactive it returns 0 so paused/reduced-motion waveforms render statically.
 */
export function useTimelineTime(active: boolean): number {
  const [time, setTime] = useState(0);
  const lastFrame = useRef(0);

  useEffect(() => {
    if (!active) {
      setTime(0);
      lastFrame.current = 0;
      return;
    }

    let raf = 0;
    const loop = (now: number) => {
      if (now - lastFrame.current >= 1000 / 24) {
        lastFrame.current = now;
        setTime(now / 1000);
      }
      raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(raf);
  }, [active]);

  return time;
}

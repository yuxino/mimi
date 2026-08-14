import { useEffect, useState } from "react";

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
 * A monotonically increasing time value (seconds) that advances on every
 * animation frame while `active` is true. Driven by requestAnimationFrame so
 * the wave motion stays smooth on any display refresh rate (a throttled 24Hz
 * cadence read as stutter in the webview). When inactive it returns 0 so
 * paused/reduced-motion waveforms render statically.
 */
export function useTimelineTime(active: boolean): number {
  const [time, setTime] = useState(0);

  useEffect(() => {
    if (!active) {
      // The hook returns 0 while inactive; nothing to subscribe.
      return;
    }

    let raf = 0;
    const loop = (now: number) => {
      setTime(now / 1000);
      raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(raf);
  }, [active]);

  return active ? time : 0;
}

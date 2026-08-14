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

/**
 * Stabilizes a streaming text value: the returned value only advances after
 * `settleMs` pass without a change, and even under a continuous stream it
 * advances at least every `maxWaitMs`. This turns per-character draft
 * churn into calm, chunked updates (the preview settles between speech
 * pauses instead of flickering on every recognition block).
 */
export function useStableText(
  text: string,
  settleMs = 400,
  maxWaitMs = 1500,
): string {
  const [stable, setStable] = useState(text);
  const latestRef = useRef(text);
  latestRef.current = text;
  const maxTimerRef = useRef<number | null>(null);

  useEffect(() => {
    if (text === stable) {
      // Already in sync: cancel any pending force-sync timer.
      if (maxTimerRef.current !== null) {
        window.clearTimeout(maxTimerRef.current);
        maxTimerRef.current = null;
      }
      return;
    }

    // Settle timer: restarts on every change, so the value only advances
    // after a pause.
    const settleTimer = window.setTimeout(() => {
      if (maxTimerRef.current !== null) {
        window.clearTimeout(maxTimerRef.current);
        maxTimerRef.current = null;
      }
      setStable(latestRef.current);
    }, settleMs);

    // Force-sync timer: started once when the value first goes stale and
    // never reset by subsequent changes, so a continuous stream still
    // advances the preview at least every `maxWaitMs`.
    if (maxTimerRef.current === null) {
      maxTimerRef.current = window.setTimeout(() => {
        maxTimerRef.current = null;
        setStable(latestRef.current);
      }, maxWaitMs);
    }

    return () => window.clearTimeout(settleTimer);
  }, [text, stable, settleMs, maxWaitMs]);

  // Unmount cleanup for the ref-held force-sync timer.
  useEffect(
    () => () => {
      if (maxTimerRef.current !== null) {
        window.clearTimeout(maxTimerRef.current);
      }
    },
    [],
  );

  return stable;
}

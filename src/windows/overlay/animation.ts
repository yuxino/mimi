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
  streamKey = "",
): string {
  const [stable, setStable] = useState({ text, streamKey });
  const latestRef = useRef({ text, streamKey });
  const maxTimerRef = useRef<number | null>(null);

  // Keep the latest text available to the (non-resetting) force-sync timer.
  useEffect(() => {
    latestRef.current = { text, streamKey };
  }, [text, streamKey]);

  useEffect(() => {
    if (text === stable.text && streamKey === stable.streamKey) {
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
  }, [text, streamKey, stable, settleMs, maxWaitMs]);

  // Unmount cleanup for the ref-held force-sync timer.
  useEffect(
    () => () => {
      if (maxTimerRef.current !== null) {
        window.clearTimeout(maxTimerRef.current);
      }
    },
    [],
  );

  // Removal, confirmation, and display-mode changes take effect during the
  // render itself. A previous source preview must not survive in the timer's
  // cached value when the next preview is a translation.
  return text === "" || settleMs === 0 || streamKey !== stable.streamKey
    ? text
    : stable.text;
}

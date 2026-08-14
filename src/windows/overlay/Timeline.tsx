import { memo, useEffect, useRef } from "react";
import { hexToRgba } from "../../lib/types";
import type { SubtitleRow } from "./overlayModel";

const ACCENT = "#7AA8FF";
const MONO_FONT =
  '"SF Mono", Menlo, Consolas, "Courier New", monospace';

interface TimelineProps {
  rows: SubtitleRow[];
  fontSize: number;
  /** True when the trailing row(s) are the live draft preview. Draft rows
   * (id prefix `draft-`) render dimmed with a trailing ellipsis so the
   * in-progress line does not dominate the stable history above it; history
   * rows never take this style. */
  draft?: boolean;
}

/** Scrolling subtitle history; auto-scrolls to the newest row. Memoized:
 * during live streaming the overlay re-renders on every session-state event,
 * but the timeline DOM only needs rebuilding when its rows actually change. */
export const Timeline = memo(function Timeline({
  rows,
  fontSize,
  draft = false,
}: TimelineProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  // Keep the newest content pinned to the bottom: rows.length changes when a
  // new row appears, and the last row's text length changes while a draft
  // grows (wrapping into more lines) without changing the row count.
  const lastTextLength = rows[rows.length - 1]?.text.length ?? 0;
  const prevRowCountRef = useRef(rows.length);

  useEffect(() => {
    const element = containerRef.current;
    if (!element) return;
    if (rows.length !== prevRowCountRef.current) {
      // A new row arrived: glide to the bottom (Swift's scrollTo animates
      // too). The draft-growth pinning below is skipped this render so the
      // two never fight.
      prevRowCountRef.current = rows.length;
      element.scrollTo({ top: element.scrollHeight, behavior: "smooth" });
    } else {
      // Same row, text grew: pin instantly so per-character streaming
      // never stutters.
      element.scrollTop = element.scrollHeight;
    }
  }, [rows.length, lastTextLength]);

  return (
    <div
      ref={containerRef}
      className="min-h-0 flex-1 overflow-y-auto"
      style={{ overscrollBehavior: "contain" }}
    >
      {rows.map((row, index) => {
        const isLast = index === rows.length - 1;
        const distance = rows.length - 1 - index;
        // Draft rows are the trailing `draft-*` rows of the live preview
        // line (identified by id prefix, never by createdAt — history rows
        // beyond the first segment also carry null timestamps).
        const isDraftRow = draft && row.id.startsWith("draft-");
        return (
          <div
            key={row.id}
            className="flex items-baseline"
            style={{
              gap: 8,
              paddingLeft: 18,
              paddingRight: 18,
              paddingTop: isLast ? 7 : 5,
              paddingBottom: isLast ? 7 : 5,
              // New rows settle in with a brief rise-and-fade (CSS animation
              // runs once on mount; the key is stable per row, so streaming
              // text updates do not re-trigger it).
              animation: "subtitle-row-enter 240ms ease-out",
            }}
          >
            {row.createdAt !== null && !isDraftRow ? (
              <span
                style={{
                  width: 31,
                  flexShrink: 0,
                  textAlign: "right",
                  fontSize: 9,
                  fontWeight: 500,
                  fontFamily: MONO_FONT,
                  fontVariantNumeric: "tabular-nums",
                  color: hexToRgba(ACCENT, distance <= 1 ? 0.46 : 0.28),
                }}
              >
                {formatTimestamp(row.createdAt)}
              </span>
            ) : (
              <span style={{ width: 31, height: 1, flexShrink: 0 }} />
            )}
            <span
              className="min-w-0 flex-1 text-left"
              style={{
                fontSize: rowFontSize(index, rows.length, fontSize),
                fontWeight: isLast ? 500 : 400,
                color: isDraftRow
                  ? "rgba(255,255,255,0.72)"
                  : `rgba(255,255,255,${rowOpacity(distance)})`,
                lineHeight: 1.45,
                overflowWrap: "break-word",
              }}
            >
              {row.text}
              {isDraftRow && (
                <span style={{ opacity: 0.55 }} aria-hidden="true">
                  {"…"}
                </span>
              )}
            </span>
          </div>
        );
      })}
    </div>
  );
});

function rowFontSize(
  index: number,
  count: number,
  fontSize: number,
): number {
  return index === count - 1 ? fontSize : Math.max(12, fontSize * 0.82);
}

function rowOpacity(distance: number): number {
  switch (distance) {
    case 0:
      return 1;
    case 1:
      return 0.58;
    default:
      return 0.34;
  }
}

/** HH:mm in local time, 24h (matches Swift's `.hour(.twoDigits(amPM: .omitted))`). */
function formatTimestamp(createdAt: number): string {
  const date = new Date(createdAt);
  const hours = String(date.getHours()).padStart(2, "0");
  const minutes = String(date.getMinutes()).padStart(2, "0");
  return `${hours}:${minutes}`;
}

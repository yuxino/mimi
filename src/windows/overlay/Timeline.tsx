import { useEffect, useRef } from "react";
import { hexToRgba } from "../../lib/types";
import type { SubtitleRow } from "./overlayModel";

const ACCENT = "#7AA8FF";
const MONO_FONT =
  '"SF Mono", Menlo, Consolas, "Courier New", monospace';

interface TimelineProps {
  rows: SubtitleRow[];
  fontSize: number;
  /**
   * The current line is a live draft (streaming, not yet final). Draft rows
   * (the current-line segments) render dimmed with a trailing ellipsis so the
   * in-progress line does not dominate the stable history above it; when the
   * line becomes final it settles to full opacity.
   */
  draft?: boolean;
}

/** Scrolling subtitle timeline; auto-scrolls to the newest row. */
export function Timeline({ rows, fontSize, draft = false }: TimelineProps) {
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
      // Same row, draft text grew: pin instantly so per-character streaming
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
        // Current-line rows carry no timestamp; they are the live draft
        // segments while the line is streaming.
        const isDraftRow = draft && row.createdAt === null;
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
            {row.createdAt !== null ? (
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
                // Gentle settle: dimmed while streaming, full once final.
                transition: "color 200ms ease",
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
}

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

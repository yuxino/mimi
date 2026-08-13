import { useEffect, useRef } from "react";
import { hexToRgba } from "../../lib/types";
import type { SubtitleRow } from "./overlayModel";

const ACCENT = "#7AA8FF";
const MONO_FONT =
  '"SF Mono", Menlo, Consolas, "Courier New", monospace';

interface TimelineProps {
  rows: SubtitleRow[];
  fontSize: number;
}

/** Scrolling subtitle timeline; auto-scrolls to the newest row. */
export function Timeline({ rows, fontSize }: TimelineProps) {
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const element = containerRef.current;
    if (element) element.scrollTop = element.scrollHeight;
  }, [rows.length]);

  return (
    <div
      ref={containerRef}
      className="min-h-0 flex-1 overflow-y-auto"
      style={{ overscrollBehavior: "contain" }}
    >
      {rows.map((row, index) => {
        const isLast = index === rows.length - 1;
        const distance = rows.length - 1 - index;
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
                color: `rgba(255,255,255,${rowOpacity(distance)})`,
                lineHeight: 1.45,
                overflowWrap: "break-word",
              }}
            >
              {row.text}
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

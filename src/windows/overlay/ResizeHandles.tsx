import { useRef } from "react";
import type { CSSProperties, PointerEvent as ReactPointerEvent } from "react";

type Region =
  | "top"
  | "left"
  | "bottom"
  | "right"
  | "topLeft"
  | "topRight"
  | "bottomLeft"
  | "bottomRight";

export const OVERLAY_MIN_WIDTH = 360;
export const OVERLAY_MAX_WIDTH = 1200;
export const OVERLAY_MIN_HEIGHT = 100;
export const OVERLAY_MAX_HEIGHT = 600;

const CURSORS: Record<Region, string> = {
  top: "ns-resize",
  bottom: "ns-resize",
  left: "ew-resize",
  right: "ew-resize",
  topLeft: "nwse-resize",
  bottomRight: "nwse-resize",
  topRight: "nesw-resize",
  bottomLeft: "nesw-resize",
};

const HANDLES: Array<{ region: Region; style: CSSProperties }> = [
  { region: "topLeft", style: { left: 0, top: 0, width: 14, height: 14 } },
  { region: "topRight", style: { right: 0, top: 0, width: 14, height: 14 } },
  { region: "bottomLeft", style: { left: 0, bottom: 0, width: 14, height: 14 } },
  {
    region: "bottomRight",
    style: { right: 0, bottom: 0, width: 14, height: 14 },
  },
  { region: "top", style: { left: 14, top: 0, right: 14, height: 6 } },
  { region: "bottom", style: { left: 14, bottom: 0, right: 14, height: 6 } },
  { region: "left", style: { left: 0, top: 14, bottom: 14, width: 6 } },
  { region: "right", style: { right: 0, top: 14, bottom: 14, width: 6 } },
];

interface ResizeHandlesProps {
  disabled: boolean;
  onResize: (width: number, height: number) => void;
}

interface DragState {
  region: Region;
  startX: number;
  startY: number;
  startWidth: number;
  startHeight: number;
}

/** Self-drawn resize handles for the overlay's eight edge/corner regions. */
export function ResizeHandles({ disabled, onResize }: ResizeHandlesProps) {
  const rootRef = useRef<HTMLDivElement>(null);
  const dragRef = useRef<DragState | null>(null);

  const start = (region: Region, event: ReactPointerEvent<HTMLDivElement>) => {
    if (disabled) return;
    const rect = rootRef.current?.getBoundingClientRect();
    if (!rect) return;
    dragRef.current = {
      region,
      startX: event.clientX,
      startY: event.clientY,
      startWidth: rect.width,
      startHeight: rect.height,
    };
    event.currentTarget.setPointerCapture(event.pointerId);
  };

  const move = (event: ReactPointerEvent<HTMLDivElement>) => {
    const drag = dragRef.current;
    if (!drag) return;
    const dx = event.clientX - drag.startX;
    const dy = event.clientY - drag.startY;

    let width = drag.startWidth;
    let height = drag.startHeight;
    if (drag.region.includes("left")) width = drag.startWidth - dx;
    if (drag.region.includes("right")) width = drag.startWidth + dx;
    if (drag.region.includes("top")) height = drag.startHeight - dy;
    if (drag.region.includes("bottom")) height = drag.startHeight + dy;

    onResize(
      clamp(Math.round(width), OVERLAY_MIN_WIDTH, OVERLAY_MAX_WIDTH),
      clamp(Math.round(height), OVERLAY_MIN_HEIGHT, OVERLAY_MAX_HEIGHT),
    );
  };

  const end = () => {
    dragRef.current = null;
  };

  return (
    <div
      ref={rootRef}
      // The root must never swallow pointer events: it overlays the whole
      // window and would otherwise block the drag handle and the control
      // buttons beneath it. Only the eight edge handles stay interactive.
      className="absolute inset-0"
      style={{ pointerEvents: "none" }}
    >
      {HANDLES.map(({ region, style }) => (
        <div
          key={region}
          style={{
            position: "absolute",
            cursor: CURSORS[region],
            pointerEvents: disabled ? "none" : "auto",
            ...style,
          }}
          onPointerDown={(event) => start(region, event)}
          onPointerMove={move}
          onPointerUp={end}
          onPointerCancel={end}
        />
      ))}
    </div>
  );
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

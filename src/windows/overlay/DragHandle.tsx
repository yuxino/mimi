import { useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { I18N } from "../../lib/i18n";
import { isTauri } from "../../lib/ipc";

interface DragHandleProps {
  onToggleCollapsed: () => void;
  /** Compact variant uses the 42×30 collapsed-overlay drag area. */
  compact?: boolean;
  /** Expanded-handle width (default 120); narrowed on small windows so the
   * handle never overlaps the language capsule or the control buttons. */
  width?: number;
}

/**
 * The drag handle, mirroring `WindowDragArea`: a primary-button press drags
 * the overlay window (via `startDragging`, which works regardless of which
 * child element the press lands on), and a double-click collapses/expands it.
 * The hover pill matches the Swift original (accent 78% / white 28%).
 */
export function DragHandle({
  onToggleCollapsed,
  compact = false,
  width = 120,
}: DragHandleProps) {
  const [hovered, setHovered] = useState(false);
  const handleWidth = compact ? 42 : width;
  const height = compact ? 30 : 18;

  const handleMouseDown = (event: React.MouseEvent<HTMLDivElement>) => {
    if (event.button !== 0 || !isTauri) return;
    if (event.detail === 2) {
      // Second press of a double-click: toggle instead of dragging, exactly
      // like the Swift `mouseDown` `clickCount == 2` branch.
      onToggleCollapsed();
      return;
    }
    void getCurrentWindow().startDragging();
  };

  return (
    <div
      onMouseDown={handleMouseDown}
      onDoubleClick={onToggleCollapsed}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      title={I18N.overlay.dragTooltip}
      className="relative flex items-center justify-center"
      style={{ width: handleWidth, height }}
    >
      <div
        style={{
          width: hovered ? 40 : 32,
          height: 3,
          borderRadius: 1.5,
          background: hovered
            ? "rgba(122, 168, 255, 0.78)"
            : "rgba(255, 255, 255, 0.28)",
        }}
      />
    </div>
  );
}

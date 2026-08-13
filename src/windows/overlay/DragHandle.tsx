import { useState } from "react";
import { I18N } from "../../lib/i18n";

interface DragHandleProps {
  onToggleCollapsed: () => void;
  /** Compact variant uses the 42×30 collapsed-overlay drag area. */
  compact?: boolean;
}

/**
 * The drag handle. In Tauri the `data-tauri-drag-region` attribute enables
 * native window dragging; a double-click collapses/expands the overlay. The
 * hover pill matches `WindowDragArea` (accent 78% / white 28%).
 */
export function DragHandle({
  onToggleCollapsed,
  compact = false,
}: DragHandleProps) {
  const [hovered, setHovered] = useState(false);
  const width = compact ? 42 : 120;
  const height = compact ? 30 : 18;

  return (
    <div
      data-tauri-drag-region
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      onDoubleClick={onToggleCollapsed}
      title={I18N.overlay.dragTooltip}
      className="relative flex items-center justify-center"
      style={{ width, height }}
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

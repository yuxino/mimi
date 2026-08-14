import { useMemo, useState } from "react";
import { I18N } from "../../lib/i18n";
import { getCurrentWindow, LogicalPosition } from "@tauri-apps/api/window";
import { isTauri, overlaySetSize } from "../../lib/ipc";
import { useStore } from "../../lib/store";
import { OVERLAY_ACTIVITY_PHASES, hexToRgba } from "../../lib/types";
import { ControlButton } from "./ControlButton";
import { DragHandle } from "./DragHandle";
import { LanguagePickerPopover } from "./LanguagePickerPopover";
import { RecognitionActivityIndicator } from "./RecognitionActivityIndicator";
import { ResizeHandles } from "./ResizeHandles";
import { Timeline } from "./Timeline";
import { WaveformIndicator } from "./WaveformIndicator";
import {
  computeActivityPhase,
  computeVisibleRows,
  emptyStateIsError,
  emptyStateText,
  hasSubtitleContent,
  isWaitingForFinalTranslation,
  languageStatus,
  subtitleSegmentLength,
} from "./overlayModel";

const ACCENT = "#7AA8FF";

/** Floating subtitle overlay; 1:1 port of `SubtitleOverlayView.swift`. */
export function OverlayWindow() {
  const session = useStore((state) => state.session);
  const settings = useStore((state) => state.settings);
  const togglePaused = useStore((state) => state.togglePaused);
  const clearSubtitles = useStore((state) => state.clearSubtitles);
  const switchSourceLanguage = useStore((state) => state.switchSourceLanguage);
  const switchTranslationMode = useStore(
    (state) => state.switchTranslationMode,
  );
  const setOverlayCollapsed = useStore((state) => state.setOverlayCollapsed);
  const showSettings = useStore((state) => state.showSettings);

  const [isHovering, setIsHovering] = useState(false);
  const [overlaySize, setOverlaySize] = useState({ width: 640, height: 136 });
  // Keep the drag handle clear of the language capsule (~122px) and the
  // control buttons (~140px) when the window is narrow.
  const windowWidth =
    typeof window !== "undefined" ? window.innerWidth : overlaySize.width;
  const dragHandleWidth = Math.max(48, Math.min(120, windowWidth - 290));

  const collapsed = session.isOverlayCollapsed;
  const phase = computeActivityPhase(session, settings);
  const detectedLanguage = session.detectedLanguage;
  const isWaiting = isWaitingForFinalTranslation(
    settings,
    detectedLanguage,
    session.isTranslationPending,
  );
  const segmentLength = subtitleSegmentLength(
    settings.targetLanguage,
    detectedLanguage,
  );
  // Recompute rows only when the subtitle content actually changes; the
  // overlay re-renders on every session-state event (status/isActive/flags),
  // but running the segmenter over the whole history per event is the main
  // cost during live streaming.
  const rows = useMemo(
    () => computeVisibleRows(session.subtitles, segmentLength),
    [session.subtitles, segmentLength],
  );
  const status = languageStatus(settings, detectedLanguage);
  const hasContent = hasSubtitleContent(session.subtitles);

  const phaseLabel = OVERLAY_ACTIVITY_PHASES[phase].accessibilityLabel;
  const pauseLabel = session.isPaused
    ? I18N.overlay.resume
    : I18N.overlay.pause;

  const toggleCollapsed = () => {
    void setOverlayCollapsed(!collapsed);
  };

  const handleResize = (width: number, height: number, x?: number, y?: number) => {
    setOverlaySize({ width, height });
    if (!isTauri) return;
    if (x !== undefined && y !== undefined) {
      void getCurrentWindow().setPosition(new LogicalPosition(x, y)).catch(() => {});
    }
    void overlaySetSize(width, height);
  };

  const content = (
    <>
      <div className="h-full w-full" style={{ padding: 6 }}>
        <div
          key={collapsed ? "collapsed" : "expanded"}
          className={
            collapsed ? "overlay-swap-collapsed" : "overlay-swap-expanded"
          }
          style={{ height: "100%", width: "100%" }}
        >
          {collapsed ? renderCompact() : renderExpanded()}
        </div>
      </div>
      {!settings.isOverlayLocked && !collapsed && (
        <ResizeHandles disabled={false} onResize={handleResize} />
      )}
    </>
  );

  if (isTauri) {
    return <div className="relative h-full w-full">{content}</div>;
  }

  // Plain `vite dev` preview: a fixed box anchored near the bottom-center.
  return (
    <div className="flex h-screen w-screen items-end justify-center pb-[72px]">
      <div
        className="relative"
        style={{ width: overlaySize.width, height: overlaySize.height }}
      >
        {content}
      </div>
    </div>
  );

  function renderExpanded() {
    const hoverHighlight = isHovering && !settings.isOverlayLocked;
    const borderColor = hoverHighlight
      ? hexToRgba(ACCENT, 0.34)
      : "rgba(255,255,255,0.12)";
    const borderWidth = hoverHighlight ? 1 : 0.75;

    return (
      <div
        className="relative h-full w-full overflow-hidden"
        style={{
          borderRadius: 16,
          background: "rgba(0,0,0,0.62)",
          border: `${borderWidth}px solid ${borderColor}`,
        }}
        onMouseEnter={() => setIsHovering(true)}
        onMouseLeave={() => setIsHovering(false)}
      >
        <div
          style={{
            position: "absolute",
            inset: 0,
            borderRadius: 16,
            background:
              "linear-gradient(to bottom, rgba(255,255,255,0.035), rgba(255,255,255,0))",
            pointerEvents: "none",
          }}
        />

        <div className="relative flex h-full flex-col" style={{ padding: 5 }}>
          <div
            className="absolute inset-x-0 top-0 flex items-end justify-center"
            style={{
              height: (session.isActive ? 38 : 24) + 5,
              // Always-visible drag affordance: dimmed while idle, full on
              // hover. A fully transparent handle leaves no cue that the
              // overlay can be moved.
              opacity: isHovering ? 1 : 0.45,
              transition: "opacity 160ms ease-out",
            }}
          >
            <DragHandle
              onToggleCollapsed={toggleCollapsed}
              width={dragHandleWidth}
            />
          </div>

          <div
            className="flex min-h-0 flex-col"
            style={{
              // The top band (drag handle / language capsule / control
              // buttons) floats over the canvas in this port, so reserve its
              // exact height here — mirroring the Swift in-flow drag-area
              // frame — otherwise subtitle rows slide underneath the
              // controls and overlap them. (+5 matches the handle's lowered
              // position so the pill never overlaps the first row.)
              paddingTop: (session.isActive ? 38 : 24) + 5,
              height: "100%",
            }}
          >
          {rows.length === 0 ? (
            <div
              className="flex flex-1 flex-col items-center justify-center"
              style={{ gap: 12 }}
            >
              {session.isActive && (
                <div className="flex items-center" style={{ height: 56 }}>
                  <WaveformIndicator phase={phase} />
                </div>
              )}
              <div
                style={{
                  fontSize: Math.max(12, settings.fontSize * 0.68),
                  fontWeight: 500,
                  color: emptyStateIsError(session)
                    ? "rgba(255,69,58,0.9)"
                    : "rgba(255,255,255,0.5)",
                  textAlign: "center",
                  padding: "0 24px",
                }}
              >
                {emptyStateText(session, settings)}
              </div>
            </div>
          ) : (
            <Timeline
              rows={rows}
              fontSize={settings.fontSize}
              draft={
                session.subtitles.translation.text !== "" &&
                !session.subtitles.translation.isFinal
              }
            />
          )}

          {rows.length > 0 && session.isActive && (
            <div
              className="flex items-center"
              style={{ gap: 7, height: 24, padding: "5px 18px 7px" }}
            >
              <span className="flex-1" />
              <div style={{ opacity: 0.65 }}>
                <WaveformIndicator phase={phase} compact />
              </div>
              <span
                style={{
                  fontSize: 10,
                  fontWeight: 500,
                  color: "rgba(255,255,255,0.6)",
                }}
              >
                {phaseLabel}
              </span>
              <span className="flex-1" />
            </div>
          )}
          </div>
        </div>

        {/* Quick language / translation-mode switcher. Always visible so the
            user can switch before starting to listen, not only while active
            (the Swift original only showed it while listening). */}
        {status !== null && (
          <div className="absolute" style={{ top: 10, left: 12 }}>
            <LanguagePickerPopover
              phase={phase}
              isHovering={isHovering}
              isPaused={session.isPaused}
              isWaitingForFinalTranslation={isWaiting}
              settings={settings}
              detectedLanguage={detectedLanguage}
              onSwitchSourceLanguage={(language) =>
                void switchSourceLanguage(language)
              }
              onSwitchTranslationMode={(mode) =>
                void switchTranslationMode(mode)
              }
            />
          </div>
        )}

        {session.isActive && !settings.isOverlayLocked && (
          <div
            className="absolute flex"
            style={{ top: 10, right: 10, gap: 4 }}
          >
            <ControlButton
              icon={session.isPaused ? "play" : "pause"}
              label={pauseLabel}
              onClick={() => void togglePaused()}
            />
            <ControlButton
              icon="chevron-up"
              label={I18N.overlay.collapseSubtitle}
              onClick={() => void setOverlayCollapsed(true)}
            />
            {hasContent && (
              <ControlButton
                icon="eraser"
                label={I18N.overlay.clearSubtitles}
                onClick={() => void clearSubtitles()}
              />
            )}
            <ControlButton
              icon="gear"
              label={I18N.overlay.openSettings}
              onClick={() => void showSettings()}
            />
          </div>
        )}
      </div>
    );
  }

  function renderCompact() {
    return (
      <div
        className="relative h-full w-full"
        role="group"
        aria-label={`字幕已收起，${phaseLabel}`}
        style={{
          borderRadius: 14,
          background: "rgba(0,0,0,0.68)",
          border: `0.75px solid ${hexToRgba(ACCENT, isHovering ? 0.3 : 0.16)}`,
        }}
        onMouseEnter={() => setIsHovering(true)}
        onMouseLeave={() => setIsHovering(false)}
      >
        <div
          style={{
            position: "absolute",
            inset: 0,
            borderRadius: 14,
            background:
              "linear-gradient(to bottom, rgba(255,255,255,0.05), rgba(255,255,255,0))",
            pointerEvents: "none",
          }}
        />
        <div
          className="relative flex h-full items-center"
          style={{ gap: 8, padding: "0 10px" }}
        >
          <DragHandle onToggleCollapsed={toggleCollapsed} compact />
          <RecognitionActivityIndicator phase={phase} />
          <span
            className="truncate"
            style={{ fontSize: 11, fontWeight: 500, color: "rgba(255,255,255,0.76)" }}
          >
            {phaseLabel}
          </span>
          <span className="flex-1" style={{ minWidth: 4 }} />
          <ControlButton
            icon={session.isPaused ? "play" : "pause"}
            label={pauseLabel}
            onClick={() => void togglePaused()}
          />
          <ControlButton
            icon="chevron-down"
            label={I18N.overlay.expandSubtitle}
            onClick={() => void setOverlayCollapsed(false)}
          />
        </div>
      </div>
    );
  }
}

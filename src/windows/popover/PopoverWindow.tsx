import { useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Icon } from "../../components/Icon";
import { I18N } from "../../lib/i18n";
import { isTauri, overlayPopoverHide } from "../../lib/ipc";
import { useStore } from "../../lib/store";
import {
  SOURCE_LANGUAGE_QUICK_CASES,
  TRANSLATION_MODE_CASES,
  TRANSLATION_MODE_DISPLAY_NAMES,
  type SourceLanguage,
  type TranslationMode,
} from "../../lib/types";
import { sourceLanguageButtonTitle } from "../overlay/overlayModel";

const ACCENT = "#7AA8FF";

/**
 * The language/mode picker menu, rendered inside its own window anchored
 * under the overlay's language capsule (the Swift original used an
 * NSPopover). Because the menu is a separate window, the subtitle overlay's
 * size and position are never affected by the menu.
 */
export function PopoverWindow() {
  // Narrow selectors: this window never shows subtitle text, so subscribing
  // to the whole session object would re-render it on every streaming event.
  const isPaused = useStore((state) => state.session.isPaused);
  const settings = useStore((state) => state.settings);
  const switchSourceLanguage = useStore((state) => state.switchSourceLanguage);
  const switchTranslationMode = useStore(
    (state) => state.switchTranslationMode,
  );

  // Interactive whenever not paused: idle, listening, connecting and
  // stopping all allow quick language/mode switching (the backend rebuilds
  // the session when a switch lands mid-connect).
  const canInteract = !isPaused;
  // Automatic source runs the low-latency pipeline regardless of the stored
  // mode, so the picker reflects (and locks onto) the effective mode.
  const automaticSource = settings.sourceLanguage === "auto";
  const effectiveMode = automaticSource
    ? "lowLatency"
    : settings.translationMode;

  const close = () => {
    if (isTauri) {
      void overlayPopoverHide().catch(() => {});
      void getCurrentWindow().hide().catch(() => {});
    }
  };

  // Escape dismisses the menu, matching the transient NSPopover behavior.
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") close();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  const selectSourceLanguage = (language: SourceLanguage) => {
    close();
    void switchSourceLanguage(language);
  };

  const selectTranslationMode = (mode: TranslationMode) => {
    close();
    void switchTranslationMode(mode);
  };

  return (
    <div className="relative h-full w-full">
      <div
        className="absolute"
        style={{
          // 8px margin keeps the panel's drop shadow inside the window.
          left: 8,
          right: 8,
          top: 8,
          bottom: 8,
          padding: 8,
          borderRadius: 10,
          background: "#242424",
          border: "0.5px solid rgba(255,255,255,0.14)",
          boxShadow: "0 8px 24px rgba(0,0,0,0.4)",
          opacity: canInteract ? 1 : 0.72,
        }}
      >
        <PopoverHeader>{I18N.overlay.sourceLanguage}</PopoverHeader>
        {SOURCE_LANGUAGE_QUICK_CASES.map((language) => (
          <PickerRow
            key={language}
            title={sourceLanguageButtonTitle(language)}
            selected={settings.sourceLanguage === language}
            disabled={!canInteract}
            onSelect={() => selectSourceLanguage(language)}
          />
        ))}

        <div
          style={{
            height: 1,
            background: "rgba(255,255,255,0.1)",
            margin: "5px 0",
          }}
        />
        <PopoverHeader>{I18N.overlay.translationMode}</PopoverHeader>
        {TRANSLATION_MODE_CASES.map((mode) => (
          <PickerRow
            key={mode}
            title={TRANSLATION_MODE_DISPLAY_NAMES[mode]}
            selected={effectiveMode === mode}
            disabled={!canInteract || automaticSource}
            onSelect={() => selectTranslationMode(mode)}
          />
        ))}
      </div>
    </div>
  );
}

function PopoverHeader({ children }: { children: string }) {
  return (
    <p
      style={{
        fontSize: 11,
        fontWeight: 600,
        color: "rgba(255,255,255,0.5)",
        padding: "0 8px 2px",
        margin: 0,
      }}
    >
      {children}
    </p>
  );
}

function PickerRow({
  title,
  selected,
  disabled,
  onSelect,
}: {
  title: string;
  selected: boolean;
  disabled: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onSelect}
      disabled={disabled}
      className="ux-hover flex w-full items-center"
      style={{
        gap: 8,
        height: 26,
        padding: "0 8px",
        borderRadius: 6,
        border: "none",
        background: "transparent",
        color: "rgba(255,255,255,0.9)",
        fontSize: 13,
        cursor: disabled ? "default" : "pointer",
      }}
    >
      <span className="flex-1 text-left">{title}</span>
      {selected && <Icon name="checkmark" style={{ color: ACCENT }} />}
    </button>
  );
}

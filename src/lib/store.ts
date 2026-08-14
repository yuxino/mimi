/**
 * Global zustand store. In Tauri it forwards every action to the Rust backend
 * and applies `session-state` / `settings-changed` events as they arrive. In a
 * plain `vite dev` session (no `__TAURI_INTERNALS__`) it emulates the backend
 * locally so the UI can be developed without the Rust side running.
 */

import { create } from "zustand";
import type { UnlistenFn } from "@tauri-apps/api/event";
import {
  appQuit,
  appShowSettings,
  isTauri,
  listenSessionState,
  listenSettingsChanged,
  overlaySetCollapsed,
  overlaySetLocked,
  overlayShow,
  sessionClearSubtitles,
  sessionGetState,
  sessionStart,
  sessionStop,
  sessionSwitchSourceLanguage,
  sessionSwitchTranslationMode,
  sessionTogglePaused,
  settingsGet,
  settingsSave,
  trayPanelHide,
} from "./ipc";
import type {
  SessionStateEvent,
  SettingsDraft,
  SettingsSnapshot,
  SourceLanguage,
  SubtitleSnapshot,
  TranslationMode,
} from "./types";
import { targetLanguageAfterQuickSwitch } from "./types";

const EMPTY_SUBTITLES: SubtitleSnapshot = {
  source: { text: "", isFinal: false },
  translation: { text: "", isFinal: false },
  history: [],
};

const INITIAL_SESSION: SessionStateEvent = {
  status: { kind: "idle" },
  isActive: false,
  isPaused: false,
  isOverlayCollapsed: false,
  subtitles: EMPTY_SUBTITLES,
  detectedLanguage: null,
  isTranslationPending: false,
};

const INITIAL_SETTINGS: SettingsSnapshot = {
  workspaceID: "",
  apiKey: "",
  hasAPIKey: false,
  sourceLanguage: "auto",
  targetLanguage: "zh",
  translationMode: "lowLatency",
  fontSize: 18,
  isOverlayLocked: false,
  credentialLoadError: null,
};

interface StoreState {
  session: SessionStateEvent;
  settings: SettingsSnapshot;
  initialized: boolean;
  init: () => Promise<void>;
  start: () => Promise<void>;
  stop: () => Promise<void>;
  togglePaused: () => Promise<void>;
  clearSubtitles: () => Promise<void>;
  switchSourceLanguage: (language: SourceLanguage) => Promise<void>;
  switchTranslationMode: (mode: TranslationMode) => Promise<void>;
  saveSettings: (draft: SettingsDraft) => Promise<void>;
  setOverlayCollapsed: (collapsed: boolean) => Promise<void>;
  setOverlayLocked: (locked: boolean) => Promise<void>;
  showOverlay: () => Promise<void>;
  hideTrayPanel: () => Promise<void>;
  showSettings: () => Promise<void>;
  quit: () => Promise<void>;
}

const unlisteners: UnlistenFn[] = [];

export const useStore = create<StoreState>()((set, get) => ({
  session: INITIAL_SESSION,
  settings: INITIAL_SETTINGS,
  initialized: false,

  init: async () => {
    if (get().initialized) return;
    if (isTauri) {
      try {
        const [snapshot, session] = await Promise.all([
          settingsGet(),
          sessionGetState(),
        ]);
        set({ settings: snapshot, session });
      } catch {
        // Boot without settings if the backend is unreachable; the event
        // listeners below will fill in the real state when it emits.
      }
      try {
        unlisteners.push(
          await listenSessionState((session) => set({ session })),
          await listenSettingsChanged((settings) => set({ settings })),
        );
      } catch {
        // No listeners, no live updates — the window still renders its boot
        // snapshot.
      }
    }
    set({ initialized: true });
  },

  start: async () => {
    if (isTauri) {
      await sessionStart();
      return;
    }
    // Mock: flip into listening and seed a couple of Japanese→Chinese lines.
    const now = Date.now();
    set((state) => ({
      session: {
        status: { kind: "listening" },
        isActive: true,
        isPaused: false,
        isOverlayCollapsed: state.session.isOverlayCollapsed,
        subtitles: {
          source: { text: "今日は映画について話しましょう。", isFinal: true },
          translation: { text: "今天咱们聊聊电影吧。", isFinal: true },
          history: [
            {
              source: "今日は映画について話しましょう。",
              translation: "今天咱们聊聊电影吧。",
              createdAt: now - 6000,
            },
            {
              source: "主人公は駅で友達を待っています。",
              translation: "主人公正在车站等朋友呢。",
              createdAt: now - 3000,
            },
          ],
        },
        detectedLanguage: "ja",
        isTranslationPending: false,
      },
    }));
  },

  stop: async () => {
    if (isTauri) {
      await sessionStop();
      return;
    }
    set((state) => ({
      session: {
        ...state.session,
        status: { kind: "idle" },
        isActive: false,
        isPaused: false,
        isTranslationPending: false,
      },
    }));
  },

  togglePaused: async () => {
    if (isTauri) {
      await sessionTogglePaused();
      return;
    }
    set((state) => ({
      session: { ...state.session, isPaused: !state.session.isPaused },
    }));
  },

  clearSubtitles: async () => {
    if (isTauri) {
      await sessionClearSubtitles();
      return;
    }
    set((state) => ({
      session: { ...state.session, subtitles: EMPTY_SUBTITLES },
    }));
  },

  switchSourceLanguage: async (language) => {
    if (isTauri) {
      await sessionSwitchSourceLanguage(language);
      return;
    }
    set((state) => ({
      settings: {
        ...state.settings,
        sourceLanguage: language,
        targetLanguage: targetLanguageAfterQuickSwitch(
          language,
          state.settings.sourceLanguage,
          state.settings.targetLanguage,
        ),
      },
    }));
  },

  switchTranslationMode: async (mode) => {
    if (isTauri) {
      await sessionSwitchTranslationMode(mode);
      return;
    }
    set((state) => ({
      settings: { ...state.settings, translationMode: mode },
    }));
  },

  saveSettings: async (draft) => {
    // Optimistic local merge so the UI updates immediately (mirrors Swift's
    // `@Published` settings bindings); the backend snapshot then confirms it.
    set((state) => ({ settings: mergeSettings(state.settings, draft) }));
    if (!isTauri) return;
    const snapshot = await settingsSave(draft);
    set({ settings: snapshot });
  },

  setOverlayCollapsed: async (collapsed) => {
    // Optimistic local update so the overlay layout switches immediately;
    // the backend event confirms it afterwards.
    set((state) => ({
      session: { ...state.session, isOverlayCollapsed: collapsed },
    }));
    if (isTauri) {
      await overlaySetCollapsed(collapsed);
    }
  },

  setOverlayLocked: async (locked) => {
    if (isTauri) {
      await overlaySetLocked(locked);
      return;
    }
    set((state) => ({
      settings: { ...state.settings, isOverlayLocked: locked },
    }));
  },

  showOverlay: async () => {
    if (isTauri) await overlayShow();
  },

  hideTrayPanel: async () => {
    if (isTauri) await trayPanelHide();
  },

  showSettings: async () => {
    if (isTauri) await appShowSettings();
  },

  quit: async () => {
    if (isTauri) await appQuit();
  },
}));

function mergeSettings(
  current: SettingsSnapshot,
  draft: SettingsDraft,
): SettingsSnapshot {
  return {
    ...current,
    workspaceID: draft.workspaceID ?? current.workspaceID,
    sourceLanguage: draft.sourceLanguage ?? current.sourceLanguage,
    targetLanguage: draft.targetLanguage ?? current.targetLanguage,
    translationMode: draft.translationMode ?? current.translationMode,
    fontSize: draft.fontSize ?? current.fontSize,
    isOverlayLocked: draft.isOverlayLocked ?? current.isOverlayLocked,
    hasAPIKey:
      draft.apiKey !== undefined
        ? draft.apiKey.length > 0
        : current.hasAPIKey,
  };
}

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
  profileCreate,
  profileDelete,
  profileDeleteAPIKey,
  profileSaveAPIKey,
  profileSelect,
  profileUpdate,
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
  type SettingsNavigationTarget,
} from "./ipc";
import { effectiveUiLanguage, setStoredUiLanguage } from "./i18n";
import {
  sourceLanguagesForSettings,
  translationModesForSettings,
} from "./providerCapabilities";
import {
  initializeSnapshotStreams,
  mergeSettingsSnapshot,
  SettingsSaveCoordinator,
  SnapshotResponseGate,
} from "./settingsState";
import type {
  SessionStateEvent,
  SettingsDraft,
  SettingsSnapshot,
  ServiceProvider,
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
  isTranslationTimedOut: false,
};

const INITIAL_SETTINGS: SettingsSnapshot = {
  profiles: [
    {
      id: "alibaba-default",
      name: "Alibaba Cloud",
      provider: "alibabaCloud",
      credentialState: isTauri ? "unavailable" : "present",
    },
  ],
  activeProfileId: "alibaba-default",
  sourceLanguage: "auto",
  targetLanguage: "zh",
  translationMode: "lowLatency",
  fontSize: 18,
  subtitleAlignment: "center",
  subtitleBlendsWithBackground: false,
  isOverlayLocked: false,
  uiLanguage: null,
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
  createProfile: (
    provider: ServiceProvider,
    name: string,
  ) => Promise<SettingsSnapshot>;
  updateProfile: (
    profileId: string,
    name: string,
  ) => Promise<SettingsSnapshot>;
  selectProfile: (profileId: string) => Promise<SettingsSnapshot>;
  deleteProfile: (profileId: string) => Promise<SettingsSnapshot>;
  saveProfileAPIKey: (
    profileId: string,
    apiKey: string,
  ) => Promise<SettingsSnapshot>;
  deleteProfileAPIKey: (profileId: string) => Promise<SettingsSnapshot>;
  setOverlayCollapsed: (collapsed: boolean) => Promise<void>;
  setOverlayLocked: (locked: boolean) => Promise<void>;
  showOverlay: () => Promise<void>;
  hideTrayPanel: () => Promise<void>;
  showSettings: (target?: SettingsNavigationTarget) => Promise<void>;
  quit: () => Promise<void>;
}

const unlisteners: UnlistenFn[] = [];
const settingsSaveCoordinator = new SettingsSaveCoordinator();
const settingsResponseGate = new SnapshotResponseGate();
let initializationRetryTimer: number | undefined;
let initializationRetryDelay = 500;

/**
 * The UI language this window's module-level i18n constants (I18N, display
 * name tables) were computed with. Captured once at module load, before any
 * reload. Because all Tauri windows share one localStorage origin, the stored
 * override is already updated by the window that initiated the switch, so we
 * compare against this rendered-language snapshot instead of localStorage to
 * decide whether this window needs a reload.
 */
const renderedUiLanguage = effectiveUiLanguage();

function systemEffectiveLanguage(): "zh" | "en" | "ja" {
  const system =
    typeof navigator !== "undefined" ? (navigator.language ?? "") : "";
  if (system.toLowerCase().startsWith("zh")) return "zh";
  if (system.toLowerCase().startsWith("ja")) return "ja";
  return "en";
}

export const useStore = create<StoreState>()((set, get) => ({
  session: INITIAL_SESSION,
  settings: INITIAL_SETTINGS,
  initialized: false,

  init: async () => {
    if (get().initialized) return;
    // Set the guard synchronously: React StrictMode double-invokes effects
    // in development, and both calls would otherwise register listeners.
    set({ initialized: true });
    if (isTauri) {
      try {
        unlisteners.push(
          ...(await initializeSnapshotStreams(
            {
              listenSettings: listenSettingsChanged,
              listenSession: listenSessionState,
              getSettings: settingsGet,
              getSession: sessionGetState,
            },
            {
              applySettings: (settings) => {
                settingsResponseGate.advance();
                settingsSaveCoordinator.invalidate();
                set({ settings });
                // Language switches initiated from any window reach every other
                // window through this event; reload so module-level i18n
                // constants (I18N, display-name tables) are recomputed.
                syncUiLanguageFromSettings(settings);
              },
              applySession: (session) => set({ session }),
            },
          )),
        );
        initializationRetryDelay = 500;
      } catch {
        // Stay fail-closed and retry listener + snapshot setup as one unit.
        // Partial listeners are removed by initializeSnapshotStreams.
        set({ initialized: false });
        if (initializationRetryTimer === undefined) {
          const delay = initializationRetryDelay;
          initializationRetryDelay = Math.min(delay * 2, 8_000);
          initializationRetryTimer = window.setTimeout(() => {
            initializationRetryTimer = undefined;
            void get().init();
          }, delay);
        }
      }
    }
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
        isTranslationTimedOut: false,
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
        isTranslationTimedOut: false,
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
      session: {
        ...state.session,
        subtitles: EMPTY_SUBTITLES,
        isTranslationTimedOut: false,
      },
    }));
  },

  switchSourceLanguage: async (language) => {
    const current = get();
    if (
      sessionSettingsAreChanging(current.session) ||
      !sourceLanguagesForSettings(current.settings).includes(language)
    ) {
      return;
    }
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
    const current = get();
    if (
      sessionSettingsAreChanging(current.session) ||
      !translationModesForSettings(current.settings).includes(mode)
    ) {
      return;
    }
    if (isTauri) {
      await sessionSwitchTranslationMode(mode);
      return;
    }
    set((state) => ({
      settings: { ...state.settings, translationMode: mode },
    }));
  },

  saveSettings: async (draft) => {
    const previous = get().settings;
    if (!isTauri) {
      set({ settings: mergeSettingsSnapshot(previous, draft) });
      return;
    }
    await settingsSaveCoordinator.save(previous, draft, settingsSave, (settings) =>
      {
        settingsResponseGate.advance();
        set({ settings });
      },
    );
  },

  createProfile: async (provider, name) => {
    ensureProfileMutationsAllowed(get().session);
    if (isTauri) {
      const revision = settingsResponseGate.capture();
      const snapshot = await profileCreate(provider, name);
      if (settingsResponseGate.applyIfCurrent(revision)) {
        settingsSaveCoordinator.invalidate();
        set({ settings: snapshot });
        return snapshot;
      }
      return get().settings;
    }
    const current = get().settings;
    if (current.profiles.length >= 20) throw new Error("profile-limit");
    const id = `mock-${provider}-${Date.now()}`;
    const snapshot: SettingsSnapshot = {
      ...current,
      profiles: [
        ...current.profiles,
        { id, name, provider, credentialState: "missing" },
      ],
    };
    set({ settings: snapshot });
    return snapshot;
  },

  updateProfile: async (profileId, name) => {
    ensureProfileMutationsAllowed(get().session);
    if (isTauri) {
      const revision = settingsResponseGate.capture();
      const snapshot = await profileUpdate(profileId, name);
      if (settingsResponseGate.applyIfCurrent(revision)) {
        settingsSaveCoordinator.invalidate();
        set({ settings: snapshot });
        return snapshot;
      }
      return get().settings;
    }
    const current = get().settings;
    const snapshot: SettingsSnapshot = {
      ...current,
      profiles: current.profiles.map((profile) =>
        profile.id === profileId ? { ...profile, name } : profile,
      ),
    };
    set({ settings: snapshot });
    return snapshot;
  },

  selectProfile: async (profileId) => {
    ensureProfileMutationsAllowed(get().session);
    if (isTauri) {
      const revision = settingsResponseGate.capture();
      const snapshot = await profileSelect(profileId);
      if (settingsResponseGate.applyIfCurrent(revision)) {
        settingsSaveCoordinator.invalidate();
        set({ settings: snapshot });
        return snapshot;
      }
      return get().settings;
    }
    const current = get().settings;
    const selected = current.profiles.find((profile) => profile.id === profileId);
    if (!selected) throw new Error("profile-not-found");
    const snapshot = settingsAfterMockProfileSelection(current, selected.provider);
    snapshot.activeProfileId = profileId;
    set({ settings: snapshot });
    return snapshot;
  },

  deleteProfile: async (profileId) => {
    ensureProfileMutationsAllowed(get().session);
    if (isTauri) {
      const revision = settingsResponseGate.capture();
      const snapshot = await profileDelete(profileId);
      if (settingsResponseGate.applyIfCurrent(revision)) {
        settingsSaveCoordinator.invalidate();
        set({ settings: snapshot });
        return snapshot;
      }
      return get().settings;
    }
    const current = get().settings;
    if (current.profiles.length <= 1) throw new Error("last-profile");
    const profiles = current.profiles.filter((profile) => profile.id !== profileId);
    const activeProfileId =
      current.activeProfileId === profileId
        ? (profiles[0]?.id ?? "")
        : current.activeProfileId;
    const snapshot: SettingsSnapshot = { ...current, profiles, activeProfileId };
    set({ settings: snapshot });
    return snapshot;
  },

  saveProfileAPIKey: async (profileId, apiKey) => {
    ensureProfileMutationsAllowed(get().session);
    if (isTauri) {
      const revision = settingsResponseGate.capture();
      const snapshot = await profileSaveAPIKey(profileId, apiKey);
      if (settingsResponseGate.applyIfCurrent(revision)) {
        settingsSaveCoordinator.invalidate();
        set({ settings: snapshot });
        return snapshot;
      }
      return get().settings;
    }
    if (!apiKey.trim()) throw new Error("credential-empty");
    const current = get().settings;
    const snapshot: SettingsSnapshot = {
      ...current,
      profiles: current.profiles.map((profile) =>
        profile.id === profileId
          ? { ...profile, credentialState: "present" }
          : profile,
      ),
    };
    set({ settings: snapshot });
    return snapshot;
  },

  deleteProfileAPIKey: async (profileId) => {
    ensureProfileMutationsAllowed(get().session);
    if (isTauri) {
      const revision = settingsResponseGate.capture();
      const snapshot = await profileDeleteAPIKey(profileId);
      if (settingsResponseGate.applyIfCurrent(revision)) {
        settingsSaveCoordinator.invalidate();
        set({ settings: snapshot });
        return snapshot;
      }
      return get().settings;
    }
    const current = get().settings;
    const snapshot: SettingsSnapshot = {
      ...current,
      profiles: current.profiles.map((profile) =>
        profile.id === profileId
          ? { ...profile, credentialState: "missing" }
          : profile,
      ),
    };
    set({ settings: snapshot });
    return snapshot;
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

  showSettings: async (target) => {
    if (isTauri) await appShowSettings(target);
  },

  quit: async () => {
    if (isTauri) await appQuit();
  },
}));

function ensureProfileMutationsAllowed(session: SessionStateEvent): void {
  if (session.isActive) throw new Error("session-active");
}

function sessionSettingsAreChanging(session: SessionStateEvent): boolean {
  return (
    session.status.kind === "connecting" || session.status.kind === "stopping"
  );
}

function settingsAfterMockProfileSelection(
  current: SettingsSnapshot,
  provider: ServiceProvider,
): SettingsSnapshot {
  if (provider === "alibabaCloud") return { ...current };
  return {
    ...current,
    sourceLanguage: "auto",
    targetLanguage:
      current.targetLanguage === "original" ? "zh" : current.targetLanguage,
    translationMode: "turbo",
  };
}

/**
 * Reconciles this window's rendered UI language with the backend preference.
 * All windows share one localStorage origin, so a switch initiated in another
 * window already updated the stored override by the time this event arrives;
 * compare the backend preference against the language this window rendered
 * with and reload only when they differ, so the module-level i18n constants
 * are recomputed. When they agree this is a no-op, so a reload only ever
 * happens once per switch.
 */
function syncUiLanguageFromSettings(settings: SettingsSnapshot): void {
  const target =
    settings.uiLanguage === "zh" ||
    settings.uiLanguage === "en" ||
    settings.uiLanguage === "ja"
      ? settings.uiLanguage
      : systemEffectiveLanguage();
  if (target !== renderedUiLanguage) {
    setStoredUiLanguage(settings.uiLanguage ?? "system");
    window.location.reload();
  }
}

import { describe, expect, it } from "vitest";
import type { SettingsSnapshot } from "./types";
import {
  initializeSnapshotStreams,
  mergeSettingsSnapshot,
  SettingsSaveCoordinator,
  SnapshotResponseGate,
} from "./settingsState";

const SETTINGS: SettingsSnapshot = {
  profiles: [
    {
      id: "alibaba-default",
      name: "Alibaba Cloud",
      provider: "alibabaCloud",
      credentialState: "present",
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

describe("mergeSettingsSnapshot", () => {
  it("merges runtime-safe subtitle presentation preferences", () => {
    expect(
      mergeSettingsSnapshot(SETTINGS, {
        subtitleAlignment: "right",
        subtitleBlendsWithBackground: true,
      }),
    ).toMatchObject({
      subtitleAlignment: "right",
      subtitleBlendsWithBackground: true,
    });
  });
});

describe("SettingsSaveCoordinator", () => {
  it("rolls the latest optimistic draft back when persistence fails", async () => {
    const coordinator = new SettingsSaveCoordinator();
    let current = SETTINGS;

    await expect(
      coordinator.save(
        current,
        { fontSize: 20 },
        async () => {
          throw new Error("save failed");
        },
        (settings) => {
          current = settings;
        },
      ),
    ).rejects.toThrow("save failed");

    expect(current.fontSize).toBe(18);
  });

  it("does not let an older failure roll back a newer successful save", async () => {
    const coordinator = new SettingsSaveCoordinator();
    let current = SETTINGS;
    let rejectFirst: ((error: Error) => void) | undefined;

    const first = coordinator.save(
      current,
      { fontSize: 19 },
      () =>
        new Promise<SettingsSnapshot>((_resolve, reject) => {
          rejectFirst = reject;
        }),
      (settings) => {
        current = settings;
      },
    );
    const second = coordinator.save(
      current,
      { fontSize: 20 },
      async () => ({ ...SETTINGS, fontSize: 20 }),
      (settings) => {
        current = settings;
      },
    );

    await Promise.resolve();
    rejectFirst?.(new Error("stale failure"));
    await expect(first).rejects.toThrow("stale failure");
    await second;

    expect(current.fontSize).toBe(20);
  });

  it("serializes persistence so the backend cannot finish saves out of order", async () => {
    const coordinator = new SettingsSaveCoordinator();
    let current = SETTINGS;
    let persistedFontSize = SETTINGS.fontSize;
    let finishFirst: (() => void) | undefined;
    let secondStarted = false;

    const persist = (draft: { fontSize?: number }) => {
      if (draft.fontSize === 19) {
        return new Promise<SettingsSnapshot>((resolve) => {
          finishFirst = () => {
            persistedFontSize = 19;
            resolve({ ...SETTINGS, fontSize: 19 });
          };
        });
      }
      secondStarted = true;
      persistedFontSize = 20;
      return Promise.resolve({ ...SETTINGS, fontSize: 20 });
    };

    const first = coordinator.save(
      current,
      { fontSize: 19 },
      persist,
      (settings) => {
        current = settings;
      },
    );
    const second = coordinator.save(
      current,
      { fontSize: 20 },
      persist,
      (settings) => {
        current = settings;
      },
    );

    await Promise.resolve();
    expect(secondStarted).toBe(false);
    expect(current.fontSize).toBe(20);

    finishFirst?.();
    await first;
    await second;

    expect(secondStarted).toBe(true);
    expect(persistedFontSize).toBe(20);
    expect(current.fontSize).toBe(20);
  });

  it("rolls a newer failed save back to the last confirmed snapshot", async () => {
    const coordinator = new SettingsSaveCoordinator();
    let current = SETTINGS;

    const first = coordinator.save(
      current,
      { fontSize: 19 },
      async () => ({ ...SETTINGS, fontSize: 19 }),
      (settings) => {
        current = settings;
      },
    );
    const second = coordinator.save(
      current,
      { fontSize: 20 },
      async () => {
        throw new Error("latest save failed");
      },
      (settings) => {
        current = settings;
      },
    );

    await first;
    await expect(second).rejects.toThrow("latest save failed");

    expect(current.fontSize).toBe(19);
  });

  it("ignores a pending response after an external snapshot", async () => {
    const coordinator = new SettingsSaveCoordinator();
    let current = SETTINGS;
    let resolveSave: ((snapshot: SettingsSnapshot) => void) | undefined;
    const pending = coordinator.save(
      current,
      { fontSize: 19 },
      () =>
        new Promise<SettingsSnapshot>((resolve) => {
          resolveSave = resolve;
        }),
      (settings) => {
        current = settings;
      },
    );

    await Promise.resolve();
    coordinator.invalidate();
    current = { ...SETTINGS, fontSize: 20 };
    resolveSave?.({ ...SETTINGS, fontSize: 19 });
    await pending;

    expect(current.fontSize).toBe(20);
  });
});

describe("initializeSnapshotStreams", () => {
  it("cleans a partial failure so initialization can be retried", async () => {
    let cleanupCount = 0;
    const appliedSettings: string[] = [];
    const appliedSessions: string[] = [];

    await expect(
      initializeSnapshotStreams(
        {
          listenSettings: async () => () => {
            cleanupCount += 1;
          },
          listenSession: async () => {
            throw new Error("listener unavailable");
          },
          getSettings: async () => "unused-settings",
          getSession: async () => "unused-session",
        },
        {
          applySettings: (settings) => appliedSettings.push(settings),
          applySession: (session) => appliedSessions.push(session),
        },
      ),
    ).rejects.toThrow("snapshot-listener-unavailable");

    expect(cleanupCount).toBe(1);
    expect(appliedSettings).toEqual([]);
    expect(appliedSessions).toEqual([]);

    await initializeSnapshotStreams(
      {
        listenSettings: async () => () => {},
        listenSession: async () => () => {},
        getSettings: async () => "retry-settings",
        getSession: async () => "retry-session",
      },
      {
        applySettings: (settings) => appliedSettings.push(settings),
        applySession: (session) => appliedSessions.push(session),
      },
    );

    expect(appliedSettings).toEqual(["retry-settings"]);
    expect(appliedSessions).toEqual(["retry-session"]);
  });

  it("keeps events received while stale boot snapshots are in flight", async () => {
    let settingsHandler: ((settings: string) => void) | undefined;
    let sessionHandler: ((session: string) => void) | undefined;
    let finishSettingsListener: (() => void) | undefined;
    let finishSessionListener: (() => void) | undefined;
    let resolveSettingsSnapshot: ((settings: string) => void) | undefined;
    let resolveSessionSnapshot: ((session: string) => void) | undefined;
    const appliedSettings: string[] = [];
    const appliedSessions: string[] = [];
    const settingsSnapshot = new Promise<string>((resolve) => {
      resolveSettingsSnapshot = resolve;
    });
    const sessionSnapshot = new Promise<string>((resolve) => {
      resolveSessionSnapshot = resolve;
    });

    const initialization = initializeSnapshotStreams(
      {
        listenSettings: (handler) => {
          settingsHandler = handler;
          return new Promise((resolve) => {
            finishSettingsListener = () => resolve(() => {});
          });
        },
        listenSession: (handler) => {
          sessionHandler = handler;
          return new Promise((resolve) => {
            finishSessionListener = () => resolve(() => {});
          });
        },
        getSettings: () => settingsSnapshot,
        getSession: () => sessionSnapshot,
      },
      {
        applySettings: (settings) => appliedSettings.push(settings),
        applySession: (session) => appliedSessions.push(session),
      },
    );

    // Both listeners are requested concurrently. An event can arrive while
    // native listener setup is still completing and must survive the later
    // snapshot response.
    expect(settingsHandler).toBeDefined();
    expect(sessionHandler).toBeDefined();
    settingsHandler?.("new-settings");
    sessionHandler?.("new-session");
    finishSettingsListener?.();
    finishSessionListener?.();
    await Promise.resolve();

    resolveSettingsSnapshot?.("old-settings");
    resolveSessionSnapshot?.("old-session");
    await initialization;

    expect(appliedSettings).toEqual(["new-settings"]);
    expect(appliedSessions).toEqual(["new-session"]);
  });

  it("continues live updates after boot reconciliation", async () => {
    let settingsHandler: ((settings: string) => void) | undefined;
    let sessionHandler: ((session: string) => void) | undefined;
    const appliedSettings: string[] = [];
    const appliedSessions: string[] = [];

    await initializeSnapshotStreams(
      {
        listenSettings: async (handler) => {
          settingsHandler = handler;
          return () => {};
        },
        listenSession: async (handler) => {
          sessionHandler = handler;
          return () => {};
        },
        getSettings: async () => "boot-settings",
        getSession: async () => "boot-session",
      },
      {
        applySettings: (settings) => appliedSettings.push(settings),
        applySession: (session) => appliedSessions.push(session),
      },
    );

    settingsHandler?.("live-settings");
    sessionHandler?.("live-session");

    expect(appliedSettings).toEqual(["boot-settings", "live-settings"]);
    expect(appliedSessions).toEqual(["boot-session", "live-session"]);
  });
});

describe("SnapshotResponseGate", () => {
  it("rejects an older response after a newer settings event", () => {
    const gate = new SnapshotResponseGate();
    const oldResponse = gate.capture();

    gate.advance();

    expect(gate.applyIfCurrent(oldResponse)).toBe(false);
    expect(gate.applyIfCurrent(gate.capture())).toBe(true);
  });
});

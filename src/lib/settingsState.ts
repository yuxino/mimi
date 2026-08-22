import type { SettingsDraft, SettingsSnapshot } from "./types";

type Unlisten = () => void;

interface SnapshotStreamSources<Settings, Session> {
  listenSettings: (
    handler: (settings: Settings) => void,
  ) => Promise<Unlisten>;
  listenSession: (handler: (session: Session) => void) => Promise<Unlisten>;
  getSettings: () => Promise<Settings>;
  getSession: () => Promise<Session>;
}

interface SnapshotStreamConsumers<Settings, Session> {
  applySettings: (settings: Settings) => void;
  applySession: (session: Session) => void;
}

/**
 * Installs event listeners before requesting boot snapshots. Events received
 * while either snapshot is in flight are buffered and win over the older
 * response, closing the otherwise unavoidable listen-after-read race.
 */
export async function initializeSnapshotStreams<Settings, Session>(
  sources: SnapshotStreamSources<Settings, Session>,
  consumers: SnapshotStreamConsumers<Settings, Session>,
): Promise<Unlisten[]> {
  let isBootstrapping = true;
  let bufferedSettings: Settings | undefined;
  let bufferedSession: Session | undefined;

  const receiveSettings = (settings: Settings) => {
    if (isBootstrapping) {
      bufferedSettings = settings;
    } else {
      consumers.applySettings(settings);
    }
  };
  const receiveSession = (session: Session) => {
    if (isBootstrapping) {
      bufferedSession = session;
    } else {
      consumers.applySession(session);
    }
  };

  const listenerResults = await Promise.allSettled([
    sources.listenSettings(receiveSettings),
    sources.listenSession(receiveSession),
  ]);
  const unlisteners = listenerResults.flatMap((result) =>
    result.status === "fulfilled" ? [result.value] : [],
  );

  if (listenerResults.some((result) => result.status === "rejected")) {
    isBootstrapping = false;
    for (const unlisten of unlisteners) {
      try {
        unlisten();
      } catch {
        // A failed listener is already unusable; continue cleaning the rest.
      }
    }
    throw new Error("snapshot-listener-unavailable");
  }

  const [settingsResult, sessionResult] = await Promise.allSettled([
    sources.getSettings(),
    sources.getSession(),
  ]);

  if (settingsResult.status === "rejected" || sessionResult.status === "rejected") {
    isBootstrapping = false;
    for (const unlisten of unlisteners) {
      try {
        unlisten();
      } catch {
        // Snapshot retry must not retain a partially initialized listener.
      }
    }
    throw new Error("boot-snapshot-unavailable");
  }

  const settings =
    bufferedSettings ?? settingsResult.value;
  const session =
    bufferedSession ?? sessionResult.value;

  // JavaScript runs these assignments and callbacks in one turn, so an event
  // cannot slip between selecting the buffered values and enabling live mode.
  isBootstrapping = false;
  consumers.applySettings(settings);
  consumers.applySession(session);

  return unlisteners;
}

export function mergeSettingsSnapshot(
  current: SettingsSnapshot,
  draft: SettingsDraft,
): SettingsSnapshot {
  return {
    ...current,
    sourceLanguage: draft.sourceLanguage ?? current.sourceLanguage,
    targetLanguage: draft.targetLanguage ?? current.targetLanguage,
    translationMode: draft.translationMode ?? current.translationMode,
    fontSize: draft.fontSize ?? current.fontSize,
    isOverlayLocked: draft.isOverlayLocked ?? current.isOverlayLocked,
    uiLanguage: draft.uiLanguage ?? current.uiLanguage,
  };
}

/**
 * Applies settings drafts immediately, persists them in call order, and keeps
 * the last confirmed snapshot for rollback. External snapshots invalidate
 * pending UI responses without allowing their older results to overwrite the
 * store.
 */
export class SettingsSaveCoordinator {
  private generation = 0;
  private epoch = 0;
  private persistenceQueue: Promise<void> = Promise.resolve();
  private confirmedSnapshot: SettingsSnapshot | null = null;
  private activeSaves = 0;

  invalidate(): void {
    this.generation += 1;
    this.epoch += 1;
    this.confirmedSnapshot = null;
  }

  async save(
    previous: SettingsSnapshot,
    draft: SettingsDraft,
    persist: (draft: SettingsDraft) => Promise<SettingsSnapshot>,
    apply: (settings: SettingsSnapshot) => void,
  ): Promise<void> {
    const generation = ++this.generation;
    const epoch = this.epoch;
    if (this.confirmedSnapshot === null) {
      this.confirmedSnapshot = previous;
    }
    this.activeSaves += 1;
    apply(mergeSettingsSnapshot(previous, draft));

    const operation = this.persistenceQueue.then(async () => {
      try {
        const snapshot = await persist(draft);
        if (epoch === this.epoch) {
          this.confirmedSnapshot = snapshot;
          if (generation === this.generation) apply(snapshot);
        }
      } catch (error) {
        if (epoch === this.epoch && generation === this.generation) {
          apply(this.confirmedSnapshot ?? previous);
        }
        throw error;
      } finally {
        this.activeSaves -= 1;
        if (this.activeSaves === 0) this.confirmedSnapshot = null;
      }
    });

    // A failed save must reject its own caller without poisoning later saves.
    this.persistenceQueue = operation.catch(() => {});
    return operation;
  }
}

/** Prevents an older command response from overwriting a newer event or save. */
export class SnapshotResponseGate {
  private revision = 0;

  capture(): number {
    return this.revision;
  }

  advance(): void {
    this.revision += 1;
  }

  applyIfCurrent(expectedRevision: number): boolean {
    if (expectedRevision !== this.revision) return false;
    this.advance();
    return true;
  }
}

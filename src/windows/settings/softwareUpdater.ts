export type UpdaterPlatform = "windows" | "other";

export type UpdateDownloadEvent =
  | { event: "Started"; data: { contentLength?: number } }
  | { event: "Progress"; data: { chunkLength: number } }
  | { event: "Finished" };

export interface UpdateCandidate {
  readonly version: string;
  readonly notes?: string;
  download(onEvent: (event: UpdateDownloadEvent) => void): Promise<void>;
  install(options?: { restartAfterInstall?: boolean }): Promise<void>;
  close(): Promise<void>;
}

export interface SoftwareUpdater {
  readonly currentVersion: string;
  readonly platform: UpdaterPlatform;
  check(): Promise<UpdateCandidate | null>;
  relaunch(): Promise<void>;
}

export interface UpdaterFixtureOptions {
  currentVersion?: string;
  updateVersion?: string | null;
  notes?: string;
  contentLength?: number;
  chunks?: readonly number[];
  checkError?: Error;
  downloadError?: Error;
  installError?: Error;
  relaunchError?: Error;
  platform?: UpdaterPlatform;
}

/** Lazily imports the native plugins so browser previews and unit fixtures do
 * not invoke or require Tauri internals. */
export async function createTauriSoftwareUpdater(): Promise<SoftwareUpdater> {
  const [{ getVersion }, { relaunch }, { check }] = await Promise.all([
    import("@tauri-apps/api/app"),
    import("@tauri-apps/plugin-process"),
    import("@tauri-apps/plugin-updater"),
  ]);
  const currentVersion = await getVersion();

  return {
    currentVersion,
    platform: isWindowsUserAgent() ? "windows" : "other",
    async check() {
      const update = await check({ timeout: 15_000 });
      if (!update) return null;

      return {
        version: update.version,
        notes: update.body,
        download: (onEvent) => update.download(onEvent),
        install: (options) => update.install(options),
        close: () => update.close(),
      };
    },
    relaunch,
  };
}

/** Deterministic, network-free updater used by browser previews, native UI
 * tests, and focused failure-path tests. */
export function createFixtureSoftwareUpdater(
  options: UpdaterFixtureOptions = {},
): SoftwareUpdater {
  const updateVersion =
    options.updateVersion === undefined ? "9.9.9" : options.updateVersion;
  const chunks = options.chunks ?? [32, 48, 20];

  return {
    currentVersion: options.currentVersion ?? "fixture",
    platform: options.platform ?? "other",
    async check() {
      if (options.checkError) throw options.checkError;
      if (updateVersion === null) return null;

      return {
        version: updateVersion,
        notes: options.notes ?? "Signed updater fixture release notes.",
        async download(onEvent) {
          onEvent({
            event: "Started",
            data: { contentLength: options.contentLength },
          });
          for (const chunkLength of chunks) {
            onEvent({ event: "Progress", data: { chunkLength } });
          }
          onEvent({ event: "Finished" });
          if (options.downloadError) throw options.downloadError;
        },
        async install() {
          if (options.installError) throw options.installError;
        },
        async close() {},
      };
    },
    async relaunch() {
      if (options.relaunchError) throw options.relaunchError;
    },
  };
}

export function isWindowsUserAgent(
  userAgent = typeof navigator === "undefined" ? "" : navigator.userAgent,
): boolean {
  return /Windows/i.test(userAgent);
}

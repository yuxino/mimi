import type { SessionArchiveState } from "../../lib/types";

/** Serial polling with explicit invalidation around native mutations. Content
 * never crosses IPC; only archive sizes and limit flags are observed. */
export function monitorSessionArchive(
  read: () => Promise<SessionArchiveState>,
  receive: (state: SessionArchiveState) => void,
  failed: () => void,
) {
  let disposed = false;
  let inFlight = false;
  let revision = 0;
  let refreshPending = false;
  let timer: ReturnType<typeof setTimeout> | undefined;

  const poll = async () => {
    if (disposed || inFlight) return;
    inFlight = true;
    refreshPending = false;
    const requestRevision = revision;
    try {
      const state = await read();
      if (!disposed && requestRevision === revision) receive(state);
    } catch {
      if (!disposed && requestRevision === revision) failed();
    } finally {
      inFlight = false;
      if (!disposed) {
        if (refreshPending) void poll();
        else timer = setTimeout(() => void poll(), 1_000);
      }
    }
  };

  void poll();
  return {
    refresh() {
      revision += 1;
      refreshPending = true;
      clearTimeout(timer);
      void poll();
    },
    stop() {
      disposed = true;
      revision += 1;
      clearTimeout(timer);
    },
  };
}

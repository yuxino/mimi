import { afterEach, describe, expect, it, vi } from "vitest";
import type { SessionArchiveState } from "../../lib/types";
import { monitorSessionArchive } from "./sessionArchiveMonitor";

const empty: SessionArchiveState = {
  transcriptCount: 0,
  transcriptLimited: false,
  audioBytes: 0,
  audioLimited: false,
  sampleRate: 24_000,
};

function deferred() {
  let resolve!: (value: SessionArchiveState) => void;
  const promise = new Promise<SessionArchiveState>((done) => { resolve = done; });
  return { promise, resolve };
}

afterEach(() => vi.useRealTimers());

describe("session archive polling", () => {
  it("serializes refreshes and discards a snapshot superseded by a mutation", async () => {
    vi.useFakeTimers();
    const oldRequest = deferred();
    const newRequest = deferred();
    const read = vi.fn().mockReturnValueOnce(oldRequest.promise).mockReturnValueOnce(newRequest.promise);
    const receive = vi.fn();
    const monitor = monitorSessionArchive(read, receive, vi.fn());
    monitor.refresh();
    monitor.refresh();
    await vi.advanceTimersByTimeAsync(5_000);
    expect(read).toHaveBeenCalledTimes(1);
    oldRequest.resolve({ ...empty, transcriptCount: 100 });
    await vi.advanceTimersByTimeAsync(0);
    expect(receive).not.toHaveBeenCalled();
    expect(read).toHaveBeenCalledTimes(2);
    newRequest.resolve(empty);
    await vi.advanceTimersByTimeAsync(0);
    expect(receive).toHaveBeenCalledExactlyOnceWith(empty);
    monitor.stop();
  });

  it("ignores in-flight responses after unmount and does not schedule another read", async () => {
    vi.useFakeTimers();
    const request = deferred();
    const read = vi.fn().mockReturnValue(request.promise);
    const receive = vi.fn();
    const monitor = monitorSessionArchive(read, receive, vi.fn());
    monitor.stop();
    request.resolve(empty);
    await vi.advanceTimersByTimeAsync(5_000);
    expect(receive).not.toHaveBeenCalled();
    expect(read).toHaveBeenCalledTimes(1);
  });

  it("retries a failed status read after one second", async () => {
    vi.useFakeTimers();
    const read = vi.fn().mockRejectedValueOnce(new Error("unavailable")).mockResolvedValue(empty);
    const receive = vi.fn();
    const failed = vi.fn();
    const monitor = monitorSessionArchive(read, receive, failed);
    await vi.advanceTimersByTimeAsync(0);
    expect(failed).toHaveBeenCalledOnce();
    await vi.advanceTimersByTimeAsync(999);
    expect(read).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(1);
    expect(receive).toHaveBeenCalledExactlyOnceWith(empty);
    monitor.stop();
  });
});

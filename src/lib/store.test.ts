import { describe, expect, it } from "vitest";
import { isTauri } from "./ipc";
import {
  selectHasRecognizingSourceDraft,
  selectSessionErrorMessage,
  selectSessionStatusKind,
  useStore,
} from "./store";

describe("local preview store", () => {
  it("keeps the synthetic provider ready outside Tauri", () => {
    expect(isTauri).toBe(false);
    expect(
      useStore.getState().settings.profiles[0]?.credentialState,
    ).toBe("present");
  });

  it("keeps subtitle churn out of native-window session selectors", () => {
    const current = useStore.getState();
    const first = {
      ...current,
      session: {
        ...current.session,
        status: { kind: "listening" as const },
        subtitles: {
          ...current.session.subtitles,
          source: { text: "draft one", isFinal: false },
        },
      },
    };
    const replacement = {
      ...first,
      session: {
        ...first.session,
        subtitles: {
          ...first.session.subtitles,
          source: { text: "draft two", isFinal: false },
          translation: { text: "preview", isFinal: false },
        },
      },
    };

    expect(
      Object.is(
        selectSessionStatusKind(first),
        selectSessionStatusKind(replacement),
      ),
    ).toBe(true);
    expect(
      Object.is(
        selectSessionErrorMessage(first),
        selectSessionErrorMessage(replacement),
      ),
    ).toBe(true);
    expect(
      Object.is(
        selectHasRecognizingSourceDraft(first),
        selectHasRecognizingSourceDraft(replacement),
      ),
    ).toBe(true);
  });

  it("still exposes a changed session error message to the tray", () => {
    const current = useStore.getState();
    const first = {
      ...current,
      session: {
        ...current.session,
        status: { kind: "error" as const, message: "first failure" },
      },
    };
    const replacement = {
      ...first,
      session: {
        ...first.session,
        status: { kind: "error" as const, message: "second failure" },
      },
    };

    expect(selectSessionErrorMessage(first)).not.toBe(
      selectSessionErrorMessage(replacement),
    );
  });
});

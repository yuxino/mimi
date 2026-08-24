import { describe, expect, it } from "vitest";
import type { SettingsSnapshot } from "./types";
import {
  activeServiceProfile,
  effectiveTranslationModeForSettings,
  sourceLanguagesForSettings,
  subtitlePreferencesChanged,
  targetLanguagesForSettings,
  translationModesForSettings,
} from "./providerCapabilities";

const BASE_SETTINGS: SettingsSnapshot = {
  profiles: [
    {
      id: "ali",
      name: "Alibaba Cloud",
      provider: "alibabaCloud",
      credentialState: "present",
    },
    {
      id: "openai",
      name: "OpenAI Realtime",
      provider: "openAIRealtime",
      credentialState: "missing",
    },
  ],
  activeProfileId: "ali",
  sourceLanguage: "auto",
  targetLanguage: "zh",
  translationMode: "highQuality",
  fontSize: 18,
  subtitleAlignment: "center",
  subtitleBlendsWithBackground: false,
  isOverlayLocked: false,
  uiLanguage: null,
};

describe("provider capabilities", () => {
  it("exposes the full existing control set for manual Alibaba input", () => {
    const settings = { ...BASE_SETTINGS, sourceLanguage: "ja" as const };
    expect(sourceLanguagesForSettings(settings)).toEqual([
      "auto",
      "ja",
      "en",
      "ko",
      "zh",
    ]);
    expect(targetLanguagesForSettings(settings)).toEqual([
      "original",
      "zh",
      "en",
      "ja",
    ]);
    expect(translationModesForSettings(settings)).toEqual([
      "lowLatency",
      "highQuality",
      "turbo",
    ]);
  });

  it("uses Alibaba's low-latency path for automatic detection", () => {
    expect(translationModesForSettings(BASE_SETTINGS)).toEqual([
      "lowLatency",
      "turbo",
    ]);
    expect(effectiveTranslationModeForSettings(BASE_SETTINGS)).toBe(
      "lowLatency",
    );
  });

  it("keeps Alibaba on turbo even with automatic detection", () => {
    expect(
      effectiveTranslationModeForSettings({
        ...BASE_SETTINGS,
        translationMode: "turbo",
      }),
    ).toBe("turbo");
  });

  it("limits OpenAI Realtime to auto, supported targets, and turbo", () => {
    const settings = { ...BASE_SETTINGS, activeProfileId: "openai" };

    expect(activeServiceProfile(settings)?.provider).toBe("openAIRealtime");
    expect(sourceLanguagesForSettings(settings)).toEqual(["auto"]);
    expect(targetLanguagesForSettings(settings)).toEqual(["zh", "en", "ja"]);
    expect(translationModesForSettings(settings)).toEqual(["turbo"]);
    expect(effectiveTranslationModeForSettings(settings)).toBe("turbo");
  });

  it("keeps OpenAI's effective mode on turbo for a stale stored mode", () => {
    const settings = {
      ...BASE_SETTINGS,
      activeProfileId: "openai",
      translationMode: "lowLatency" as const,
    };

    expect(effectiveTranslationModeForSettings(settings)).toBe("turbo");
  });

  it("falls back to Alibaba capabilities when the active id is stale", () => {
    const settings = { ...BASE_SETTINGS, activeProfileId: "missing" };
    expect(activeServiceProfile(settings)).toBeUndefined();
    expect(sourceLanguagesForSettings(settings)).toContain("ko");
  });

  it("reports when profile selection normalizes subtitle preferences", () => {
    expect(
      subtitlePreferencesChanged(BASE_SETTINGS, {
        ...BASE_SETTINGS,
        sourceLanguage: "auto",
        targetLanguage: "zh",
        translationMode: "turbo",
      }),
    ).toBe(true);
    expect(subtitlePreferencesChanged(BASE_SETTINGS, { ...BASE_SETTINGS })).toBe(
      false,
    );
  });
});

import { describe, expect, it } from "vitest";
import type { SettingsSnapshot } from "./types";
import {
  SERVICE_PROVIDERS,
  activeServiceProfile,
  effectiveTranslationModeForSettings,
  sourceLanguagesForSettings,
  subtitlePreferencesChanged,
  targetLanguageAfterSourceSwitch,
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

  it("keeps the full target set for Alibaba and OpenAI automatic input", () => {
    expect(targetLanguagesForSettings(BASE_SETTINGS)).toEqual([
      "original",
      "zh",
      "en",
      "ja",
    ]);
    expect(
      targetLanguagesForSettings({
        ...BASE_SETTINGS,
        activeProfileId: "openai",
        sourceLanguage: "auto",
      }),
    ).toEqual(["zh", "en", "ja"]);
  });

  it.each([
    ["zh", ["en", "ja"]],
    ["en", ["zh", "ja"]],
    ["ja", ["zh", "en"]],
  ] as const)(
    "removes the %s target for providers with an explicit source",
    (sourceLanguage, expectedTargets) => {
      expect(
        targetLanguagesForSettings({
          ...BASE_SETTINGS,
          profiles: [
            {
              id: "tencent",
              name: "Tencent Cloud",
              provider: "tencentCloud",
              credentialState: "missing",
            },
          ],
          activeProfileId: "tencent",
          sourceLanguage,
        }),
      ).toEqual(expectedTargets);
    },
  );

  it("keeps OpenAI's effective mode on turbo for a stale stored mode", () => {
    const settings = {
      ...BASE_SETTINGS,
      activeProfileId: "openai",
      translationMode: "lowLatency" as const,
    };

    expect(effectiveTranslationModeForSettings(settings)).toBe("turbo");
  });

  it("registers every built-in provider exactly once", () => {
    expect(new Set(SERVICE_PROVIDERS).size).toBe(8);
    expect(SERVICE_PROVIDERS).toEqual([
      "alibabaCloud",
      "openAIRealtime",
      "googleGeminiLive",
      "azureOpenAIRealtime",
      "volcanoEngine",
      "tencentCloud",
      "baiduTranslate",
      "xAIRealtime",
    ]);
  });

  it("uses automatic recognition only where the official protocol supports it", () => {
    const automatic = {
      ...BASE_SETTINGS,
      profiles: [
        {
          id: "google",
          name: "Google Gemini",
          provider: "googleGeminiLive" as const,
          credentialState: "missing" as const,
        },
      ],
      activeProfileId: "google",
    };
    expect(sourceLanguagesForSettings(automatic)).toEqual(["auto"]);
    expect(effectiveTranslationModeForSettings(automatic)).toBe("turbo");

    const explicit = {
      ...automatic,
      profiles: [
        {
          ...automatic.profiles[0],
          provider: "volcanoEngine" as const,
        },
      ],
    };
    expect(sourceLanguagesForSettings(explicit)).toEqual(["ja", "en", "zh"]);

    for (const provider of ["tencentCloud", "baiduTranslate"] as const) {
      expect(
        sourceLanguagesForSettings({
          ...automatic,
          profiles: [{ ...automatic.profiles[0], provider }],
        }),
      ).toEqual(["ja", "en", "ko", "zh"]);
    }
  });

  it("keeps translation enabled when an explicit provider switches to Chinese", () => {
    const settings = {
      ...BASE_SETTINGS,
      profiles: [
        {
          id: "tencent",
          name: "Tencent Cloud",
          provider: "tencentCloud" as const,
          credentialState: "missing" as const,
        },
      ],
      activeProfileId: "tencent",
      sourceLanguage: "ja" as const,
      targetLanguage: "en" as const,
    };

    expect(targetLanguageAfterSourceSwitch(settings, "zh")).toBe("en");
    expect(
      targetLanguageAfterSourceSwitch(
        { ...settings, targetLanguage: "zh" },
        "zh",
      ),
    ).toBe("en");
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

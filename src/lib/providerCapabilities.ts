import {
  SOURCE_LANGUAGE_QUICK_CASES,
  TRANSLATION_MODE_CASES,
  type ProviderCapabilities,
  type ServiceProfile,
  type ServiceProvider,
  type SettingsSnapshot,
  type SourceLanguage,
  type TargetLanguage,
  type TranslationMode,
} from "./types";

const PROVIDER_CAPABILITIES: Readonly<
  Record<ServiceProvider, ProviderCapabilities>
> = {
  alibabaCloud: {
    sourceLanguages: SOURCE_LANGUAGE_QUICK_CASES,
    targetLanguages: ["original", "zh", "en", "ja"],
    translationModes: TRANSLATION_MODE_CASES,
  },
  openAIRealtime: {
    sourceLanguages: ["auto"],
    targetLanguages: ["zh", "en", "ja"],
    translationModes: ["turbo"],
  },
};

export function activeServiceProfile(
  settings: Pick<SettingsSnapshot, "profiles" | "activeProfileId">,
): ServiceProfile | undefined {
  return settings.profiles.find(
    (profile) => profile.id === settings.activeProfileId,
  );
}

function capabilitiesForSettings(
  settings: Pick<SettingsSnapshot, "profiles" | "activeProfileId">,
): ProviderCapabilities {
  const provider = activeServiceProfile(settings)?.provider ?? "alibabaCloud";
  return PROVIDER_CAPABILITIES[provider];
}

export function sourceLanguagesForSettings(
  settings: Pick<SettingsSnapshot, "profiles" | "activeProfileId">,
): readonly SourceLanguage[] {
  return capabilitiesForSettings(settings).sourceLanguages;
}

export function targetLanguagesForSettings(
  settings: Pick<SettingsSnapshot, "profiles" | "activeProfileId">,
): readonly TargetLanguage[] {
  return capabilitiesForSettings(settings).targetLanguages;
}

export function translationModesForSettings(
  settings: Pick<
    SettingsSnapshot,
    "profiles" | "activeProfileId" | "sourceLanguage"
  >,
): readonly TranslationMode[] {
  const provider = activeServiceProfile(settings)?.provider ?? "alibabaCloud";
  if (provider === "alibabaCloud" && settings.sourceLanguage === "auto") {
    return ["lowLatency", "turbo"];
  }
  return capabilitiesForSettings(settings).translationModes;
}

/**
 * Mirrors the backend's effective-mode priority: OpenAI always uses turbo;
 * Alibaba preserves an explicitly selected turbo mode, then falls back to
 * low latency for automatic detection.
 */
export function effectiveTranslationModeForSettings(
  settings: Pick<
    SettingsSnapshot,
    | "profiles"
    | "activeProfileId"
    | "sourceLanguage"
    | "translationMode"
  >,
): TranslationMode {
  const provider = activeServiceProfile(settings)?.provider ?? "alibabaCloud";
  const supportedModes = translationModesForSettings(settings);

  if (provider === "openAIRealtime") return "turbo";
  if (settings.translationMode === "turbo") return "turbo";
  if (settings.sourceLanguage === "auto") {
    return "lowLatency";
  }
  if (supportedModes.includes(settings.translationMode)) {
    return settings.translationMode;
  }
  return supportedModes[0] ?? settings.translationMode;
}

export function subtitlePreferencesChanged(
  before: Pick<
    SettingsSnapshot,
    "sourceLanguage" | "targetLanguage" | "translationMode"
  >,
  after: Pick<
    SettingsSnapshot,
    "sourceLanguage" | "targetLanguage" | "translationMode"
  >,
): boolean {
  return (
    before.sourceLanguage !== after.sourceLanguage ||
    before.targetLanguage !== after.targetLanguage ||
    before.translationMode !== after.translationMode
  );
}

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
  targetLanguageAfterQuickSwitch,
} from "./types";

export const SERVICE_PROVIDERS: readonly ServiceProvider[] = [
  "alibabaCloud",
  "openAIRealtime",
  "googleGeminiLive",
  "azureOpenAIRealtime",
  "volcanoEngine",
  "tencentCloud",
  "baiduTranslate",
  "xAIRealtime",
];

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
  googleGeminiLive: {
    sourceLanguages: ["auto"],
    targetLanguages: ["zh", "en", "ja"],
    translationModes: ["turbo"],
  },
  azureOpenAIRealtime: {
    sourceLanguages: ["auto"],
    targetLanguages: ["zh", "en", "ja"],
    translationModes: ["turbo"],
  },
  volcanoEngine: {
    sourceLanguages: ["ja", "en", "zh"],
    targetLanguages: ["zh", "en", "ja"],
    translationModes: ["turbo"],
  },
  tencentCloud: {
    sourceLanguages: ["ja", "en", "ko", "zh"],
    targetLanguages: ["zh", "en", "ja"],
    translationModes: ["turbo"],
  },
  baiduTranslate: {
    sourceLanguages: ["ja", "en", "ko", "zh"],
    targetLanguages: ["zh", "en", "ja"],
    translationModes: ["turbo"],
  },
  xAIRealtime: {
    sourceLanguages: ["auto"],
    targetLanguages: ["zh", "en", "ja"],
    translationModes: ["turbo"],
  },
};

export function capabilitiesForProvider(
  provider: ServiceProvider,
): ProviderCapabilities {
  return PROVIDER_CAPABILITIES[provider];
}

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
  return capabilitiesForProvider(provider);
}

export function sourceLanguagesForSettings(
  settings: Pick<SettingsSnapshot, "profiles" | "activeProfileId">,
): readonly SourceLanguage[] {
  return capabilitiesForSettings(settings).sourceLanguages;
}

export function targetLanguagesForSettings(
  settings: Pick<
    SettingsSnapshot,
    "profiles" | "activeProfileId" | "sourceLanguage"
  >,
): readonly TargetLanguage[] {
  const targetLanguages = capabilitiesForSettings(settings).targetLanguages;
  if (
    settings.sourceLanguage === "auto" ||
    targetLanguages.includes("original")
  ) {
    return targetLanguages;
  }
  return targetLanguages.filter(
    (target) => !sourceMatchesTarget(settings.sourceLanguage, target),
  );
}

function sourceMatchesTarget(
  source: SourceLanguage,
  target: TargetLanguage,
): boolean {
  return (
    (source === "zh" && target === "zh") ||
    (source === "en" && target === "en") ||
    (source === "ja" && target === "ja")
  );
}

export function targetLanguageAfterSourceSwitch(
  settings: Pick<
    SettingsSnapshot,
    | "profiles"
    | "activeProfileId"
    | "sourceLanguage"
    | "targetLanguage"
  >,
  sourceLanguage: SourceLanguage,
): TargetLanguage {
  const capabilities = capabilitiesForSettings(settings);
  if (capabilities.targetLanguages.includes("original")) {
    return targetLanguageAfterQuickSwitch(
      sourceLanguage,
      settings.sourceLanguage,
      settings.targetLanguage,
    );
  }

  if (
    capabilities.targetLanguages.includes(settings.targetLanguage) &&
    !sourceMatchesTarget(sourceLanguage, settings.targetLanguage)
  ) {
    return settings.targetLanguage;
  }

  return (
    capabilities.targetLanguages.find(
      (target) => !sourceMatchesTarget(sourceLanguage, target),
    ) ?? settings.targetLanguage
  );
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
 * Mirrors the backend's effective-mode priority: every non-Alibaba adapter
 * uses turbo; Alibaba preserves an explicitly selected turbo mode, then falls
 * back to low latency for automatic detection.
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

  if (provider !== "alibabaCloud") return "turbo";
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

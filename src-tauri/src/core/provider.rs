//! Provider identity, service profiles, and capability normalization.
//!
//! Profiles contain display metadata only. Credentials deliberately live in
//! the OS keychain and must never be serialized with a profile.

use crate::core::models::{SourceLanguage, TargetLanguage, TranslationMode};
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

pub const DEFAULT_ALIBABA_PROFILE_ID: &str = "alibaba-default";

/// A stable identifier used in settings JSON and provider-scoped keychain
/// account names. The serde values are part of the frontend contract.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProviderKind {
    #[default]
    #[serde(rename = "alibabaCloud")]
    AlibabaCloud,
    #[serde(rename = "openAIRealtime")]
    OpenAIRealtime,
    #[serde(rename = "googleGeminiLive")]
    GoogleGeminiLive,
    #[serde(rename = "azureOpenAIRealtime")]
    AzureOpenAIRealtime,
    #[serde(rename = "volcanoEngine")]
    VolcanoEngine,
    #[serde(rename = "tencentCloud")]
    TencentCloud,
    #[serde(rename = "baiduTranslate")]
    BaiduTranslate,
    #[serde(rename = "xAIRealtime")]
    XAIRealtime,
}

impl ProviderKind {
    pub const fn wire_value(self) -> &'static str {
        match self {
            Self::AlibabaCloud => "alibabaCloud",
            Self::OpenAIRealtime => "openAIRealtime",
            Self::GoogleGeminiLive => "googleGeminiLive",
            Self::AzureOpenAIRealtime => "azureOpenAIRealtime",
            Self::VolcanoEngine => "volcanoEngine",
            Self::TencentCloud => "tencentCloud",
            Self::BaiduTranslate => "baiduTranslate",
            Self::XAIRealtime => "xAIRealtime",
        }
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::AlibabaCloud => "Alibaba Cloud",
            Self::OpenAIRealtime => "OpenAI Realtime",
            Self::GoogleGeminiLive => "Google Gemini",
            Self::AzureOpenAIRealtime => "Azure OpenAI",
            Self::VolcanoEngine => "Volcano Engine",
            Self::TencentCloud => "Tencent Cloud",
            Self::BaiduTranslate => "Baidu Translate",
            Self::XAIRealtime => "xAI Grok",
        }
    }

    pub fn capabilities(self) -> ProviderCapabilities {
        match self {
            Self::AlibabaCloud => ProviderCapabilities {
                source_languages: vec![
                    SourceLanguage::Automatic,
                    SourceLanguage::Chinese,
                    SourceLanguage::English,
                    SourceLanguage::Japanese,
                    SourceLanguage::Korean,
                ],
                target_languages: vec![
                    TargetLanguage::Original,
                    TargetLanguage::SimplifiedChinese,
                    TargetLanguage::English,
                    TargetLanguage::Japanese,
                ],
                translation_modes: vec![
                    TranslationMode::LowLatency,
                    TranslationMode::HighQuality,
                    TranslationMode::Turbo,
                ],
                input_sample_rate_hz: 16_000,
            },
            Self::OpenAIRealtime | Self::AzureOpenAIRealtime | Self::XAIRealtime => {
                realtime_capabilities(vec![SourceLanguage::Automatic], 24_000)
            }
            Self::GoogleGeminiLive => {
                realtime_capabilities(vec![SourceLanguage::Automatic], 16_000)
            }
            Self::VolcanoEngine => realtime_capabilities(
                vec![
                    SourceLanguage::Japanese,
                    SourceLanguage::English,
                    SourceLanguage::Chinese,
                ],
                16_000,
            ),
            Self::TencentCloud | Self::BaiduTranslate => realtime_capabilities(
                vec![
                    SourceLanguage::Japanese,
                    SourceLanguage::English,
                    SourceLanguage::Korean,
                    SourceLanguage::Chinese,
                ],
                16_000,
            ),
        }
    }

    pub const fn uses_api_key_only(self) -> bool {
        matches!(
            self,
            Self::AlibabaCloud
                | Self::OpenAIRealtime
                | Self::GoogleGeminiLive
                | Self::VolcanoEngine
                | Self::XAIRealtime
        )
    }
}

fn realtime_capabilities(
    source_languages: Vec<SourceLanguage>,
    input_sample_rate_hz: u32,
) -> ProviderCapabilities {
    ProviderCapabilities {
        source_languages,
        target_languages: vec![
            TargetLanguage::SimplifiedChinese,
            TargetLanguage::English,
            TargetLanguage::Japanese,
        ],
        translation_modes: vec![TranslationMode::Turbo],
        input_sample_rate_hz,
    }
}

impl fmt::Display for ProviderKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.display_name())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderCapabilities {
    pub source_languages: Vec<SourceLanguage>,
    pub target_languages: Vec<TargetLanguage>,
    pub translation_modes: Vec<TranslationMode>,
    pub input_sample_rate_hz: u32,
}

impl ProviderCapabilities {
    /// Normalizes stale preferences after changing providers. The settings UI
    /// compares the before/after snapshots when it needs to explain a change.
    pub fn normalize(&self, preferences: ProviderPreferences) -> ProviderPreferences {
        let mut normalized = preferences;

        if !self.source_languages.contains(&normalized.source_language) {
            if let Some(fallback) = self
                .source_languages
                .iter()
                .copied()
                .find(|source| !source_matches_target(*source, normalized.target_language))
                .or_else(|| self.source_languages.first().copied())
            {
                normalized.source_language = fallback;
            }
        }
        if !self.target_languages.contains(&normalized.target_language) {
            if let Some(fallback) = self.target_languages.first().copied() {
                normalized.target_language = fallback;
            }
        }
        if !self
            .translation_modes
            .contains(&normalized.translation_mode)
        {
            if let Some(fallback) = self.translation_modes.first().copied() {
                normalized.translation_mode = fallback;
            }
        }

        // Dedicated translation services require different explicit source
        // and target languages. Avoid persisting a no-op pair after a source
        // switch or when loading stale preferences. Alibaba keeps its
        // separate Original-subtitle mode, so its established behavior is
        // intentionally left unchanged here.
        if !self.target_languages.contains(&TargetLanguage::Original)
            && source_matches_target(normalized.source_language, normalized.target_language)
        {
            if let Some(fallback) = self
                .target_languages
                .iter()
                .copied()
                .find(|target| !source_matches_target(normalized.source_language, *target))
            {
                normalized.target_language = fallback;
            }
        }

        normalized
    }

    pub fn target_language_after_source_switch(
        &self,
        source_language: SourceLanguage,
        previous_source: SourceLanguage,
        current_target: TargetLanguage,
    ) -> TargetLanguage {
        if self.target_languages.contains(&TargetLanguage::Original) {
            return source_language
                .target_language_after_quick_switch(previous_source, current_target);
        }

        self.normalize(ProviderPreferences {
            source_language,
            target_language: current_target,
            translation_mode: self
                .translation_modes
                .first()
                .copied()
                .unwrap_or(TranslationMode::Turbo),
        })
        .target_language
    }
}

fn source_matches_target(source: SourceLanguage, target: TargetLanguage) -> bool {
    matches!(
        (source, target),
        (SourceLanguage::Chinese, TargetLanguage::SimplifiedChinese)
            | (SourceLanguage::English, TargetLanguage::English)
            | (SourceLanguage::Japanese, TargetLanguage::Japanese)
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderPreferences {
    pub source_language: SourceLanguage,
    pub target_language: TargetLanguage,
    pub translation_mode: TranslationMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ServiceProfileError {
    #[error("The service profile ID is invalid.")]
    InvalidID,
    #[error("The service profile name is required.")]
    EmptyName,
    #[error("The service profile name is too long.")]
    NameTooLong,
}

/// Non-secret metadata for one named provider configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceProfile {
    pub id: String,
    pub name: String,
    pub provider: ProviderKind,
}

impl ServiceProfile {
    pub const MAXIMUM_ID_LENGTH: usize = 64;
    pub const MAXIMUM_NAME_LENGTH: usize = 64;

    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        provider: ProviderKind,
    ) -> Result<Self, ServiceProfileError> {
        let id = id.into();
        let name = name.into().trim().to_string();
        if id.is_empty()
            || id.len() > Self::MAXIMUM_ID_LENGTH
            || !id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(ServiceProfileError::InvalidID);
        }
        if name.is_empty() {
            return Err(ServiceProfileError::EmptyName);
        }
        if name.chars().count() > Self::MAXIMUM_NAME_LENGTH {
            return Err(ServiceProfileError::NameTooLong);
        }
        Ok(Self { id, name, provider })
    }

    pub fn alibaba_default() -> Self {
        Self {
            id: DEFAULT_ALIBABA_PROFILE_ID.to_string(),
            name: ProviderKind::AlibabaCloud.display_name().to_string(),
            provider: ProviderKind::AlibabaCloud,
        }
    }

    pub fn validated(&self) -> Result<Self, ServiceProfileError> {
        Self::new(self.id.clone(), self.name.clone(), self.provider)
    }
}

impl Default for ServiceProfile {
    fn default() -> Self {
        Self::alibaba_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn provider_wire_values_are_stable() {
        let cases = [
            (ProviderKind::AlibabaCloud, "alibabaCloud"),
            (ProviderKind::OpenAIRealtime, "openAIRealtime"),
            (ProviderKind::GoogleGeminiLive, "googleGeminiLive"),
            (ProviderKind::AzureOpenAIRealtime, "azureOpenAIRealtime"),
            (ProviderKind::VolcanoEngine, "volcanoEngine"),
            (ProviderKind::TencentCloud, "tencentCloud"),
            (ProviderKind::BaiduTranslate, "baiduTranslate"),
            (ProviderKind::XAIRealtime, "xAIRealtime"),
        ];
        for (provider, wire_value) in cases {
            assert_eq!(serde_json::to_value(provider).unwrap(), json!(wire_value));
            assert_eq!(
                serde_json::from_value::<ProviderKind>(json!(wire_value)).unwrap(),
                provider
            );
            assert_eq!(provider.wire_value(), wire_value);
        }
    }

    #[test]
    fn provider_capabilities_match_transport_constraints() {
        let alibaba = ProviderKind::AlibabaCloud.capabilities();
        assert_eq!(alibaba.source_languages.len(), 5);
        assert_eq!(alibaba.target_languages.len(), 4);
        assert_eq!(alibaba.translation_modes.len(), 3);
        assert_eq!(alibaba.input_sample_rate_hz, 16_000);

        let openai = ProviderKind::OpenAIRealtime.capabilities();
        assert_eq!(openai.source_languages, vec![SourceLanguage::Automatic]);
        assert_eq!(
            openai.target_languages,
            vec![
                TargetLanguage::SimplifiedChinese,
                TargetLanguage::English,
                TargetLanguage::Japanese
            ]
        );
        assert_eq!(openai.translation_modes, vec![TranslationMode::Turbo]);
        assert_eq!(openai.input_sample_rate_hz, 24_000);

        for provider in [
            ProviderKind::GoogleGeminiLive,
            ProviderKind::AzureOpenAIRealtime,
            ProviderKind::VolcanoEngine,
            ProviderKind::TencentCloud,
            ProviderKind::BaiduTranslate,
            ProviderKind::XAIRealtime,
        ] {
            let capabilities = provider.capabilities();
            assert_eq!(capabilities.target_languages.len(), 3);
            assert_eq!(capabilities.translation_modes, vec![TranslationMode::Turbo]);
        }
        assert_eq!(
            ProviderKind::GoogleGeminiLive
                .capabilities()
                .input_sample_rate_hz,
            16_000
        );
        assert_eq!(
            ProviderKind::AzureOpenAIRealtime
                .capabilities()
                .input_sample_rate_hz,
            24_000
        );
        assert_eq!(
            ProviderKind::VolcanoEngine.capabilities().source_languages,
            vec![
                SourceLanguage::Japanese,
                SourceLanguage::English,
                SourceLanguage::Chinese
            ]
        );
        for provider in [ProviderKind::TencentCloud, ProviderKind::BaiduTranslate] {
            let capabilities = provider.capabilities();
            assert_eq!(
                capabilities.source_languages,
                vec![
                    SourceLanguage::Japanese,
                    SourceLanguage::English,
                    SourceLanguage::Korean,
                    SourceLanguage::Chinese
                ]
            );
            assert!(!capabilities
                .source_languages
                .contains(&SourceLanguage::Automatic));
        }
    }

    #[test]
    fn openai_normalization_uses_supported_fallbacks() {
        let normalized =
            ProviderKind::OpenAIRealtime
                .capabilities()
                .normalize(ProviderPreferences {
                    source_language: SourceLanguage::Japanese,
                    target_language: TargetLanguage::Original,
                    translation_mode: TranslationMode::HighQuality,
                });
        assert_eq!(
            normalized,
            ProviderPreferences {
                source_language: SourceLanguage::Automatic,
                target_language: TargetLanguage::SimplifiedChinese,
                translation_mode: TranslationMode::Turbo,
            }
        );
    }

    #[test]
    fn explicit_source_providers_do_not_fallback_to_the_target_language() {
        let normalized =
            ProviderKind::VolcanoEngine
                .capabilities()
                .normalize(ProviderPreferences {
                    source_language: SourceLanguage::Automatic,
                    target_language: TargetLanguage::Japanese,
                    translation_mode: TranslationMode::HighQuality,
                });
        assert_eq!(normalized.source_language, SourceLanguage::English);
        assert_eq!(normalized.translation_mode, TranslationMode::Turbo);
    }

    #[test]
    fn explicit_source_providers_keep_chinese_translation_enabled() {
        let capabilities = ProviderKind::TencentCloud.capabilities();
        assert_eq!(
            capabilities.target_language_after_source_switch(
                SourceLanguage::Chinese,
                SourceLanguage::Japanese,
                TargetLanguage::English,
            ),
            TargetLanguage::English
        );
        assert_eq!(
            capabilities.target_language_after_source_switch(
                SourceLanguage::Chinese,
                SourceLanguage::Japanese,
                TargetLanguage::SimplifiedChinese,
            ),
            TargetLanguage::English
        );
    }

    #[test]
    fn profiles_are_non_secret_and_trim_names() {
        let profile =
            ServiceProfile::new("openai-1", "  OpenAI Work  ", ProviderKind::OpenAIRealtime)
                .unwrap();
        assert_eq!(profile.name, "OpenAI Work");
        let json = serde_json::to_value(&profile).unwrap();
        assert_eq!(json["provider"], "openAIRealtime");
        assert!(json.get("apiKey").is_none());
        assert!(json.get("credential").is_none());
    }

    #[test]
    fn default_profile_preserves_alibaba_compatibility() {
        let profile = ServiceProfile::default();
        assert_eq!(profile.id, DEFAULT_ALIBABA_PROFILE_ID);
        assert_eq!(profile.provider, ProviderKind::AlibabaCloud);
    }
}

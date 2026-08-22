//! Immutable, provider-resolved live-translation configuration.

use crate::core::models::{SourceLanguage, TargetLanguage, TranslationMode};
use crate::core::provider::ProviderKind;
use std::fmt;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LiveTranslationConfigurationError {
    #[error("Add the API key for {0} in Settings.")]
    MissingAPIKey(ProviderKind),
    #[error("The selected service does not support this source language.")]
    UnsupportedSourceLanguage,
    #[error("The selected service does not support this target language.")]
    UnsupportedTargetLanguage,
    #[error("The selected service does not support this translation mode.")]
    UnsupportedTranslationMode,
}

#[derive(Clone, PartialEq, Eq)]
pub struct LiveTranslationConfiguration {
    pub provider: ProviderKind,
    pub api_key: String,
    pub source_language: SourceLanguage,
    pub target_language: TargetLanguage,
    pub translation_mode: TranslationMode,
}

impl fmt::Debug for LiveTranslationConfiguration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LiveTranslationConfiguration")
            .field("provider", &self.provider)
            .field("api_key", &"[REDACTED]")
            .field("source_language", &self.source_language)
            .field("target_language", &self.target_language)
            .field("translation_mode", &self.translation_mode)
            .finish()
    }
}

impl LiveTranslationConfiguration {
    pub fn new(
        api_key: impl Into<String>,
        source_language: SourceLanguage,
        target_language: TargetLanguage,
        translation_mode: TranslationMode,
    ) -> Self {
        Self::for_provider(
            ProviderKind::AlibabaCloud,
            api_key,
            source_language,
            target_language,
            translation_mode,
        )
    }

    pub fn for_provider(
        provider: ProviderKind,
        api_key: impl Into<String>,
        source_language: SourceLanguage,
        target_language: TargetLanguage,
        translation_mode: TranslationMode,
    ) -> Self {
        Self {
            provider,
            api_key: api_key.into(),
            source_language,
            target_language,
            translation_mode,
        }
    }

    /// The mode actually used for a session: OpenAI always uses turbo;
    /// Alibaba preserves turbo and otherwise routes automatic recognition to
    /// its low-latency pipeline.
    pub fn effective_translation_mode(&self) -> TranslationMode {
        if self.provider == ProviderKind::OpenAIRealtime {
            return TranslationMode::Turbo;
        }
        if self.translation_mode == TranslationMode::Turbo {
            return TranslationMode::Turbo;
        }
        if self.source_language == SourceLanguage::Automatic {
            return TranslationMode::LowLatency;
        }
        self.translation_mode
    }

    /// Returns a trimmed, validated copy of the configuration.
    pub fn validated(&self) -> Result<Self, LiveTranslationConfigurationError> {
        let api_key = self.api_key.trim().to_string();

        if api_key.is_empty() {
            return Err(LiveTranslationConfigurationError::MissingAPIKey(
                self.provider,
            ));
        }

        let capabilities = self.provider.capabilities();
        if !capabilities
            .source_languages
            .contains(&self.source_language)
        {
            return Err(LiveTranslationConfigurationError::UnsupportedSourceLanguage);
        }
        if !capabilities
            .target_languages
            .contains(&self.target_language)
        {
            return Err(LiveTranslationConfigurationError::UnsupportedTargetLanguage);
        }
        if !capabilities
            .translation_modes
            .contains(&self.translation_mode)
        {
            return Err(LiveTranslationConfigurationError::UnsupportedTranslationMode);
        }

        Ok(Self {
            provider: self.provider,
            api_key,
            source_language: self.source_language,
            target_language: self.target_language,
            translation_mode: self.translation_mode,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(api_key: &str, source_language: SourceLanguage) -> LiveTranslationConfiguration {
        LiveTranslationConfiguration::new(
            api_key,
            source_language,
            TargetLanguage::SimplifiedChinese,
            TranslationMode::HighQuality,
        )
    }

    #[test]
    fn automatic_language_resolves_high_quality_to_low_latency() {
        let configuration = config("sk-test", SourceLanguage::Automatic);
        assert_eq!(
            configuration.effective_translation_mode(),
            TranslationMode::LowLatency
        );
    }

    #[test]
    fn turbo_mode_stays_turbo_even_with_automatic_source() {
        let configuration = LiveTranslationConfiguration::new(
            "sk-test",
            SourceLanguage::Automatic,
            TargetLanguage::SimplifiedChinese,
            TranslationMode::Turbo,
        );
        assert_eq!(
            configuration.effective_translation_mode(),
            TranslationMode::Turbo
        );
    }

    #[test]
    fn original_subtitles_preserve_the_strongest_recognition_backend() {
        let configuration = LiveTranslationConfiguration::new(
            "secret",
            SourceLanguage::Japanese,
            TargetLanguage::Original,
            TranslationMode::HighQuality,
        );
        assert_eq!(
            configuration.effective_translation_mode(),
            TranslationMode::HighQuality
        );
    }

    #[test]
    fn configuration_preserves_an_explicit_translation_mode() {
        let configuration = LiveTranslationConfiguration::new(
            "sk-test",
            SourceLanguage::Japanese,
            TargetLanguage::English,
            TranslationMode::HighQuality,
        );
        let validated = configuration.validated().unwrap();
        assert_eq!(validated.translation_mode, TranslationMode::HighQuality);
        assert_eq!(validated.target_language, TargetLanguage::English);
    }

    #[test]
    fn configuration_requires_an_api_key() {
        let configuration = config("   ", SourceLanguage::English);
        assert!(matches!(
            configuration.validated(),
            Err(LiveTranslationConfigurationError::MissingAPIKey(
                ProviderKind::AlibabaCloud
            ))
        ));
    }

    #[test]
    fn configuration_requires_only_the_api_key() {
        // The unified DashScope endpoints authenticate with the API key only.
        let configuration = config("sk-test", SourceLanguage::English);
        let validated = configuration.validated().unwrap();
        assert_eq!(validated.api_key, "sk-test");
    }

    #[test]
    fn configuration_trims_valid_credentials() {
        let configuration = config("  sk-test  ", SourceLanguage::Korean);
        let validated = configuration.validated().unwrap();
        assert_eq!(validated.api_key, "sk-test");
        assert_eq!(validated.source_language, SourceLanguage::Korean);
    }

    #[test]
    fn legacy_constructor_keeps_alibaba_as_the_default() {
        assert_eq!(
            config("sk-test", SourceLanguage::Automatic).provider,
            ProviderKind::AlibabaCloud
        );
    }

    #[test]
    fn openai_configuration_accepts_only_its_capability_matrix() {
        let valid = LiveTranslationConfiguration::for_provider(
            ProviderKind::OpenAIRealtime,
            "sk-openai",
            SourceLanguage::Automatic,
            TargetLanguage::Japanese,
            TranslationMode::Turbo,
        );
        assert_eq!(
            valid.validated().unwrap().provider,
            ProviderKind::OpenAIRealtime
        );

        let invalid_source = LiveTranslationConfiguration::for_provider(
            ProviderKind::OpenAIRealtime,
            "sk-openai",
            SourceLanguage::English,
            TargetLanguage::Japanese,
            TranslationMode::Turbo,
        );
        assert!(matches!(
            invalid_source.validated(),
            Err(LiveTranslationConfigurationError::UnsupportedSourceLanguage)
        ));
        let invalid_target = LiveTranslationConfiguration::for_provider(
            ProviderKind::OpenAIRealtime,
            "sk-openai",
            SourceLanguage::Automatic,
            TargetLanguage::Original,
            TranslationMode::Turbo,
        );
        assert!(matches!(
            invalid_target.validated(),
            Err(LiveTranslationConfigurationError::UnsupportedTargetLanguage)
        ));
    }

    #[test]
    fn provider_capabilities_define_the_session_sample_rate() {
        let alibaba = config("sk-test", SourceLanguage::Automatic);
        assert_eq!(alibaba.provider.capabilities().input_sample_rate_hz, 16_000);
        let openai = LiveTranslationConfiguration::for_provider(
            ProviderKind::OpenAIRealtime,
            "sk-openai",
            SourceLanguage::Automatic,
            TargetLanguage::English,
            TranslationMode::Turbo,
        );
        assert_eq!(openai.provider.capabilities().input_sample_rate_hz, 24_000);
    }

    #[test]
    fn debug_description_redacts_the_api_key() {
        let secret = "sk-private-test-value";
        let configuration = config(secret, SourceLanguage::English);
        let description = format!("{configuration:?}");
        assert!(!description.contains(secret));
        assert!(description.contains("[REDACTED]"));
    }
}

//! Live-translation configuration and validation, ported 1:1 from
//! `Sources/MimiCore/LiveTranslationConfiguration.swift`. Since the clients
//! moved to DashScope's unified endpoints (Bearer API key only), the
//! workspace id is no longer required; it is kept as a deprecated optional
//! field for settings migration.

use crate::core::models::{SourceLanguage, TargetLanguage, TranslationMode};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LiveTranslationConfigurationError {
    #[error("Add your Alibaba Cloud Model Studio API key in Settings.")]
    MissingAPIKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveTranslationConfiguration {
    pub workspace_id: String,
    pub api_key: String,
    pub source_language: SourceLanguage,
    pub target_language: TargetLanguage,
    pub translation_mode: TranslationMode,
}

impl LiveTranslationConfiguration {
    pub fn new(
        workspace_id: impl Into<String>,
        api_key: impl Into<String>,
        source_language: SourceLanguage,
        target_language: TargetLanguage,
        translation_mode: TranslationMode,
    ) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            api_key: api_key.into(),
            source_language,
            target_language,
            translation_mode,
        }
    }

    /// The mode actually used for a session: turbo stays turbo; automatic
    /// source recognition always uses the low-latency pipeline.
    pub fn effective_translation_mode(&self) -> TranslationMode {
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
            return Err(LiveTranslationConfigurationError::MissingAPIKey);
        }

        Ok(Self {
            workspace_id: self.workspace_id.trim().to_string(),
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

    fn config(
        workspace_id: &str,
        api_key: &str,
        source_language: SourceLanguage,
    ) -> LiveTranslationConfiguration {
        LiveTranslationConfiguration::new(
            workspace_id,
            api_key,
            source_language,
            TargetLanguage::SimplifiedChinese,
            TranslationMode::HighQuality,
        )
    }

    #[test]
    fn automatic_language_resolves_high_quality_to_low_latency() {
        let configuration = config("ws-abc123", "sk-test", SourceLanguage::Automatic);
        assert_eq!(
            configuration.effective_translation_mode(),
            TranslationMode::LowLatency
        );
    }

    #[test]
    fn turbo_mode_stays_turbo_even_with_automatic_source() {
        let configuration = LiveTranslationConfiguration::new(
            "ws-abc123",
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
            "workspace",
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
            "ws-abc123",
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
        let configuration = config("ws-abc123", "   ", SourceLanguage::English);
        assert!(matches!(
            configuration.validated(),
            Err(LiveTranslationConfigurationError::MissingAPIKey)
        ));
    }

    #[test]
    fn configuration_works_without_a_workspace_id() {
        // The unified DashScope endpoints authenticate with the API key only,
        // so an empty workspace id must validate.
        let configuration = config("", "sk-test", SourceLanguage::English);
        let validated = configuration.validated().unwrap();
        assert_eq!(validated.workspace_id, "");
        assert_eq!(validated.api_key, "sk-test");
    }

    #[test]
    fn configuration_trims_valid_credentials() {
        let configuration = config("  ws-abc123  ", "  sk-test  ", SourceLanguage::Korean);
        let validated = configuration.validated().unwrap();
        assert_eq!(validated.workspace_id, "ws-abc123");
        assert_eq!(validated.api_key, "sk-test");
        assert_eq!(validated.source_language, SourceLanguage::Korean);
    }
}

//! Core domain models, ported 1:1 from `Sources/MimiCore/Models.swift`.

use serde::{Deserialize, Serialize};

/// The language being recognized in system audio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceLanguage {
    Automatic,
    Chinese,
    English,
    Japanese,
    Korean,
}

impl Serialize for SourceLanguage {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.raw_value())
    }
}

impl<'de> Deserialize<'de> for SourceLanguage {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        match value.as_str() {
            "auto" => Ok(Self::Automatic),
            "zh" => Ok(Self::Chinese),
            "en" => Ok(Self::English),
            "ja" => Ok(Self::Japanese),
            "ko" => Ok(Self::Korean),
            other => Err(serde::de::Error::custom(format!(
                "unknown source language: {other}"
            ))),
        }
    }
}

impl SourceLanguage {
    /// Service wire code used in protocol payloads.
    pub fn raw_value(self) -> &'static str {
        match self {
            SourceLanguage::Automatic => "auto",
            SourceLanguage::Chinese => "zh",
            SourceLanguage::English => "en",
            SourceLanguage::Japanese => "ja",
            SourceLanguage::Korean => "ko",
        }
    }

    /// Parses a normalized language code reported by a service (e.g. `"ja-JP"`,
    /// `"chinese"`, `"mandarin"`) into a `SourceLanguage`.
    pub fn from_detected(detected_language: Option<&str>) -> Option<SourceLanguage> {
        let normalized = detected_language?.trim().to_lowercase();
        if normalized == "zh"
            || normalized.starts_with("zh-")
            || normalized == "chinese"
            || normalized == "mandarin"
        {
            Some(SourceLanguage::Chinese)
        } else if normalized == "ja" || normalized.starts_with("ja-") || normalized == "japanese" {
            Some(SourceLanguage::Japanese)
        } else if normalized == "en" || normalized.starts_with("en-") || normalized == "english" {
            Some(SourceLanguage::English)
        } else if normalized == "ko" || normalized.starts_with("ko-") || normalized == "korean" {
            Some(SourceLanguage::Korean)
        } else {
            None
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            SourceLanguage::Automatic => "自动识别",
            SourceLanguage::Chinese => "中文",
            SourceLanguage::English => "English",
            SourceLanguage::Japanese => "日本語",
            SourceLanguage::Korean => "한국어",
        }
    }

    /// The source-language label shown in the overlay capsule. When automatic,
    /// it reports the detected language; when the detected language equals a
    /// Chinese target, it stays on "自动识别中" (recognizing).
    pub fn status_display_name(
        self,
        detected_language: Option<&DetectedLanguage>,
        target_language: TargetLanguage,
    ) -> String {
        if self != SourceLanguage::Automatic {
            return self.display_name().to_string();
        }
        let Some(detected) = detected_language else {
            return "自动识别中".to_string();
        };
        if target_language == TargetLanguage::SimplifiedChinese && detected.code == "zh" {
            return "自动识别中".to_string();
        }
        format!("自动识别（{}）", detected.display_name())
    }

    /// Target-language adjustment applied when the user quick-switches the
    /// source language from a menu or picker.
    pub fn target_language_after_quick_switch(
        self,
        previous_source: SourceLanguage,
        current_target: TargetLanguage,
    ) -> TargetLanguage {
        if self == SourceLanguage::Chinese {
            return TargetLanguage::Original;
        }
        if previous_source == SourceLanguage::Chinese && current_target == TargetLanguage::Original
        {
            return TargetLanguage::SimplifiedChinese;
        }
        current_target
    }
}

/// A language code reported by the recognition service.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DetectedLanguage {
    pub code: String,
}

impl DetectedLanguage {
    pub fn from_reported(reported_language: Option<&str>) -> Option<Self> {
        let normalized = reported_language?.trim().to_lowercase();
        if normalized.is_empty() {
            return None;
        }
        let code = normalized
            .split('-')
            .next()
            .unwrap_or(&normalized)
            .to_string();
        Some(Self { code })
    }

    pub fn display_name(&self) -> String {
        match self.code.as_str() {
            "zh" | "chinese" | "mandarin" => "中文".to_string(),
            "yue" | "cantonese" => "粤语".to_string(),
            "en" | "english" => "English".to_string(),
            "ja" | "japanese" => "日本語".to_string(),
            "ko" | "korean" => "한국어".to_string(),
            "de" => "Deutsch".to_string(),
            "fr" => "Français".to_string(),
            "es" => "Español".to_string(),
            "pt" => "Português".to_string(),
            "it" => "Italiano".to_string(),
            "ru" => "Русский".to_string(),
            "ar" => "العربية".to_string(),
            "hi" => "हिन्दी".to_string(),
            "id" => "Bahasa Indonesia".to_string(),
            "th" => "ไทย".to_string(),
            "tr" => "Türkçe".to_string(),
            "vi" => "Tiếng Việt".to_string(),
            "uk" => "Українська".to_string(),
            "cs" => "Čeština".to_string(),
            "da" => "Dansk".to_string(),
            "tl" | "fil" => "Filipino".to_string(),
            "fi" => "Suomi".to_string(),
            "is" => "Íslenska".to_string(),
            "ms" => "Bahasa Melayu".to_string(),
            "no" | "nb" => "Norsk".to_string(),
            "pl" => "Polski".to_string(),
            "sv" => "Svenska".to_string(),
            other => other.to_uppercase(),
        }
    }
}

/// The language subtitles are translated into; `Original` means no translation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TargetLanguage {
    Original,
    SimplifiedChinese,
    English,
    Japanese,
}

impl Serialize for TargetLanguage {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.raw_value())
    }
}

impl<'de> Deserialize<'de> for TargetLanguage {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        match value.as_str() {
            "original" => Ok(Self::Original),
            "zh" => Ok(Self::SimplifiedChinese),
            "en" => Ok(Self::English),
            "ja" => Ok(Self::Japanese),
            other => Err(serde::de::Error::custom(format!(
                "unknown target language: {other}"
            ))),
        }
    }
}

impl TargetLanguage {
    pub fn raw_value(self) -> &'static str {
        match self {
            TargetLanguage::Original => "original",
            TargetLanguage::SimplifiedChinese => "zh",
            TargetLanguage::English => "en",
            TargetLanguage::Japanese => "ja",
        }
    }

    /// Service-side language name used in Qwen-MT requests.
    pub fn qwen_mt_name(self) -> &'static str {
        match self {
            TargetLanguage::Original => "",
            TargetLanguage::SimplifiedChinese => "Chinese",
            TargetLanguage::English => "English",
            TargetLanguage::Japanese => "Japanese",
        }
    }

    pub fn translates_audio(self) -> bool {
        self != TargetLanguage::Original
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TranslationMode {
    LowLatency,
    HighQuality,
    Turbo,
}

impl Serialize for TranslationMode {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(match self {
            Self::LowLatency => "lowLatency",
            Self::HighQuality => "highQuality",
            Self::Turbo => "turbo",
        })
    }
}

impl<'de> Deserialize<'de> for TranslationMode {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        match value.as_str() {
            "lowLatency" => Ok(Self::LowLatency),
            "highQuality" => Ok(Self::HighQuality),
            "turbo" => Ok(Self::Turbo),
            other => Err(serde::de::Error::custom(format!(
                "unknown translation mode: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionStatus {
    Idle,
    Connecting,
    Listening,
    Stopping,
    Error(String),
}

impl SessionStatus {
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            SessionStatus::Connecting | SessionStatus::Listening | SessionStatus::Stopping
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubtitleLine {
    pub text: String,
    #[serde(rename = "isFinal")]
    pub is_final: bool,
}

impl SubtitleLine {
    pub fn new(text: impl Into<String>, is_final: bool) -> Self {
        Self {
            text: text.into(),
            is_final,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitlePair {
    pub source: String,
    pub translation: String,
    /// Epoch milliseconds; equality intentionally ignores it (matches Swift's
    /// `SubtitlePair.==`).
    #[serde(rename = "createdAt")]
    pub created_at_ms: u64,
}

impl SubtitlePair {
    pub fn new(source: String, translation: String, created_at_ms: u64) -> Self {
        Self {
            source,
            translation,
            created_at_ms,
        }
    }
}

impl PartialEq for SubtitlePair {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source && self.translation == other.translation
    }
}

impl Eq for SubtitlePair {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubtitleSnapshot {
    pub source: SubtitleLine,
    pub translation: SubtitleLine,
    pub history: Vec<SubtitlePair>,
}

impl SubtitleSnapshot {
    pub fn empty() -> Self {
        Self {
            source: SubtitleLine::new("", false),
            translation: SubtitleLine::new("", false),
            history: Vec::new(),
        }
    }
}

impl Default for SubtitleSnapshot {
    fn default() -> Self {
        Self::empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubtitleEvent {
    SourceDraft(String),
    SourceFinal(String),
    TranslationDraft(String),
    TranslationFinal(String),
    /// Commits a source/translation pair as one reducer operation. Providers
    /// whose two append-only streams are aligned client-side use this event so
    /// finals from different connection generations can never be cross-paired.
    FinalPair {
        source: String,
        translation: String,
    },
    /// Removes the last confirmed history pair so a provisional local commit
    /// can be replaced by the authoritative server final for the same sentence.
    RevokeLastConfirmed,
    Clear,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automatic_language_has_a_clear_display_name() {
        assert_eq!(SourceLanguage::Automatic.display_name(), "自动识别");
    }

    #[test]
    fn chinese_quick_switch_shows_original_subtitles() {
        assert_eq!(
            SourceLanguage::Chinese.target_language_after_quick_switch(
                SourceLanguage::Japanese,
                TargetLanguage::SimplifiedChinese
            ),
            TargetLanguage::Original
        );
    }

    #[test]
    fn leaving_chinese_original_mode_restores_chinese_translation() {
        assert_eq!(
            SourceLanguage::Japanese.target_language_after_quick_switch(
                SourceLanguage::Chinese,
                TargetLanguage::Original
            ),
            TargetLanguage::SimplifiedChinese
        );
    }

    #[test]
    fn ordinary_language_switches_preserve_a_custom_target() {
        assert_eq!(
            SourceLanguage::English.target_language_after_quick_switch(
                SourceLanguage::Japanese,
                TargetLanguage::English
            ),
            TargetLanguage::English
        );
    }

    #[test]
    fn automatic_language_status_includes_the_detected_language() {
        let japanese = DetectedLanguage::from_reported(Some("ja-JP"));
        assert_eq!(
            SourceLanguage::Automatic
                .status_display_name(japanese.as_ref(), TargetLanguage::SimplifiedChinese),
            "自动识别（日本語）"
        );
        assert_eq!(
            SourceLanguage::Automatic.status_display_name(None, TargetLanguage::SimplifiedChinese),
            "自动识别中"
        );
        assert_eq!(
            SourceLanguage::Automatic.status_display_name(
                DetectedLanguage::from_reported(Some("zh")).as_ref(),
                TargetLanguage::SimplifiedChinese
            ),
            "自动识别中"
        );
        assert_eq!(
            SourceLanguage::Automatic.status_display_name(
                DetectedLanguage::from_reported(Some("zh")).as_ref(),
                TargetLanguage::English
            ),
            "自动识别（中文）"
        );
        assert_eq!(
            SourceLanguage::Japanese.status_display_name(
                DetectedLanguage::from_reported(Some("en")).as_ref(),
                TargetLanguage::SimplifiedChinese
            ),
            "日本語"
        );
    }

    #[test]
    fn target_languages_expose_service_codes_and_display_names() {
        assert!(!TargetLanguage::Original.translates_audio());
        assert_eq!(TargetLanguage::SimplifiedChinese.raw_value(), "zh");
        assert_eq!(TargetLanguage::English.qwen_mt_name(), "English");
    }

    #[test]
    fn detected_languages_normalize_service_codes_for_display() {
        assert_eq!(
            DetectedLanguage::from_reported(Some("ja-JP"))
                .unwrap()
                .display_name(),
            "日本語"
        );
        assert_eq!(
            DetectedLanguage::from_reported(Some("yue"))
                .unwrap()
                .display_name(),
            "粤语"
        );
        assert_eq!(
            DetectedLanguage::from_reported(Some("unknown"))
                .unwrap()
                .display_name(),
            "UNKNOWN"
        );
    }

    #[test]
    fn session_status_active_flag_matches_swift() {
        assert!(!SessionStatus::Idle.is_active());
        assert!(SessionStatus::Connecting.is_active());
        assert!(SessionStatus::Listening.is_active());
        assert!(SessionStatus::Stopping.is_active());
        assert!(!SessionStatus::Error("boom".into()).is_active());
    }

    #[test]
    fn subtitle_pair_equality_ignores_creation_time() {
        let a = SubtitlePair::new("s".into(), "t".into(), 1);
        let b = SubtitlePair::new("s".into(), "t".into(), 999);
        assert_eq!(a, b);
    }
}

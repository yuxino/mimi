//! Qwen-MT chat-completions protocol, domain prompts, and filler glossaries,
//! ported 1:1 from `Sources/MimiCore/QwenMTProtocol.swift`. The domain hint
//! strings and filler term tables are verbatim — they are the core of
//! translation quality and must not be reworded.

use crate::core::configuration::is_valid_workspace_id;
use crate::core::models::{SourceLanguage, TargetLanguage};
use crate::core::protocols::live_translate::WORKSPACE_HOST_SUFFIX;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum QwenMTProtocolError {
    #[error("The Workspace ID is not valid.")]
    InvalidWorkspaceID,
    #[error("The Qwen-MT endpoint could not be created.")]
    InvalidEndpoint,
    #[error("Qwen-MT returned an invalid response.")]
    InvalidJSON,
    #[error("Qwen-MT returned no translated text.")]
    MissingTranslation,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum QwenMTClientError {
    #[error("Add an Alibaba Cloud Model Studio API key in Settings.")]
    MissingAPIKey,
    #[error("Qwen-MT returned an invalid HTTP response.")]
    InvalidHTTPResponse,
    #[error("Qwen-MT took too long to respond.")]
    RequestTimedOut,
    #[error("Qwen-MT request failed with HTTP {status_code}.")]
    RequestFailed { status_code: u16, message: String },
}

impl QwenMTClientError {
    pub fn is_authentication_failure(&self) -> bool {
        match self {
            Self::RequestFailed { status_code, .. } => *status_code == 401 || *status_code == 403,
            Self::MissingAPIKey => true,
            Self::InvalidHTTPResponse | Self::RequestTimedOut => false,
        }
    }

    /// Content-free diagnostic label (never includes the server message).
    pub fn diagnostic_label(&self) -> String {
        match self {
            Self::MissingAPIKey => "QwenMTClientError.missingAPIKey".to_string(),
            Self::InvalidHTTPResponse => "QwenMTClientError.invalidHTTPResponse".to_string(),
            Self::RequestTimedOut => "QwenMTClientError.requestTimedOut".to_string(),
            Self::RequestFailed { status_code, .. } => {
                format!("QwenMTClientError.requestFailed(status={status_code})")
            }
        }
    }
}

pub enum QwenMTRetryPolicy {}

impl QwenMTRetryPolicy {
    /// Backoff delay for transient failures, or `None` when the failure is not
    /// retryable. Delay: min(8000, 600 * 2^min(max(attempt-1,0),4)) ms.
    pub fn delay(error: &QwenMTClientError, attempt: usize) -> Option<Duration> {
        let is_transient = match error {
            QwenMTClientError::RequestTimedOut | QwenMTClientError::InvalidHTTPResponse => true,
            QwenMTClientError::RequestFailed { status_code, .. } => {
                *status_code == 408 || *status_code == 429 || *status_code >= 500
            }
            QwenMTClientError::MissingAPIKey => false,
        };
        if !is_transient {
            return None;
        }
        let exponent = attempt.saturating_sub(1).min(4);
        let milliseconds = 8_000u64.min(600u64 << exponent);
        Some(Duration::from_millis(milliseconds))
    }
}

pub struct QwenMTEndpoint {
    pub url: url::Url,
}

impl QwenMTEndpoint {
    pub fn new(workspace_id: &str) -> Result<Self, QwenMTProtocolError> {
        if !is_valid_workspace_id(workspace_id) {
            return Err(QwenMTProtocolError::InvalidWorkspaceID);
        }
        let raw = format!(
            "https://{workspace_id}{WORKSPACE_HOST_SUFFIX}/compatible-mode/v1/chat/completions"
        );
        Ok(Self {
            url: url::Url::parse(&raw).map_err(|_| QwenMTProtocolError::InvalidEndpoint)?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QwenMTModel {
    Lite,
    Flash,
    Plus,
}

impl QwenMTModel {
    pub fn raw_name(self) -> &'static str {
        match self {
            Self::Lite => "qwen-mt-lite",
            Self::Flash => "qwen-mt-flash",
            Self::Plus => "qwen-mt-plus",
        }
    }
}

fn source_lang_name(language: SourceLanguage) -> &'static str {
    match language {
        SourceLanguage::Automatic => "auto",
        SourceLanguage::Chinese => "Chinese",
        SourceLanguage::English => "English",
        SourceLanguage::Japanese => "Japanese",
        SourceLanguage::Korean => "Korean",
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QwenMTTerm {
    pub source: String,
    pub target: String,
}

impl QwenMTTerm {
    pub fn new(source: &str, target: &str) -> Self {
        Self {
            source: source.to_string(),
            target: target.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QwenMTMemoryPair {
    pub source: String,
    pub target: String,
}

impl QwenMTMemoryPair {
    pub fn new(source: &str, target: &str) -> Self {
        Self {
            source: source.to_string(),
            target: target.to_string(),
        }
    }
}

pub enum QwenMTRequestEncoder {}

impl QwenMTRequestEncoder {
    #[allow(clippy::too_many_arguments)]
    pub fn request(
        text: &str,
        source_language: SourceLanguage,
        target_language: TargetLanguage,
        model: QwenMTModel,
        stream: bool,
        domain_hint: Option<&str>,
        terms: &[QwenMTTerm],
        translation_memory: &[QwenMTMemoryPair],
    ) -> Result<Value, QwenMTProtocolError> {
        let mut options = json!({
            "source_lang": source_lang_name(source_language),
            "target_lang": target_language.qwen_mt_name(),
        });
        if let Some(domain_hint) = domain_hint {
            options["domains"] = json!(domain_hint);
        }
        if !terms.is_empty() {
            options["terms"] = json!(terms);
        }
        if !translation_memory.is_empty() {
            options["tm_list"] = json!(translation_memory);
        }
        Ok(json!({
            "model": model.raw_name(),
            "messages": [{ "role": "user", "content": text }],
            "stream": stream,
            "translation_options": options
        }))
    }
}

pub enum QwenMTResponseDecoder {}

impl QwenMTResponseDecoder {
    pub fn decode(text: &str) -> Result<String, QwenMTProtocolError> {
        #[derive(Deserialize)]
        struct Response {
            choices: Vec<Choice>,
        }
        #[derive(Deserialize)]
        struct Choice {
            message: Message,
        }
        #[derive(Deserialize)]
        struct Message {
            content: String,
        }

        let response: Response =
            serde_json::from_str(text).map_err(|_| QwenMTProtocolError::InvalidJSON)?;
        let content = response
            .choices
            .first()
            .map(|c| c.message.content.trim())
            .filter(|c| !c.is_empty())
            .ok_or(QwenMTProtocolError::MissingTranslation)?;
        Ok(content.to_string())
    }
}

pub enum QwenMTStreamDecoder {}

impl QwenMTStreamDecoder {
    pub fn decode_chunk(text: &str) -> Result<Option<String>, QwenMTProtocolError> {
        #[derive(Deserialize)]
        struct Response {
            choices: Vec<Choice>,
        }
        #[derive(Deserialize)]
        struct Choice {
            delta: Delta,
        }
        #[derive(Deserialize)]
        struct Delta {
            content: Option<String>,
        }

        let response: Response =
            serde_json::from_str(text).map_err(|_| QwenMTProtocolError::InvalidJSON)?;
        Ok(response
            .choices
            .first()
            .and_then(|c| c.delta.content.clone()))
    }
}

/// Domain prompts and filler glossaries for spoken-dialogue translation.
/// All strings are verbatim ports of the Swift `QwenMTDomainHint`.
pub enum QwenMTDomainHint {}

impl QwenMTDomainHint {
    pub fn spoken_dialogue(
        source_language: SourceLanguage,
        target_language: TargetLanguage,
    ) -> String {
        let language_guidance = match target_language {
            TargetLanguage::Original => "",
            TargetLanguage::SimplifiedChinese => {
                "Use concise, idiomatic Simplified Chinese, like subtitles for a TV \
                 drama, and keep every natural particle: 嗯、啊、呢、吧、嘛、哦、唉. \
                 Render Japanese fillers (えっと、あの、うーん、あぁ) and English \
                 fillers (um, uh, oh, hmm) with their natural Chinese equivalents; \
                 never drop a meaningful filler."
            }
            TargetLanguage::English => {
                "Use concise, idiomatic conversational English with natural \
                 contractions and interjections (um, uh, oh, hmm, yeah). Keep \
                 Japanese fillers (えっと、あの、うーん、あぁ) as natural English \
                 equivalents; never drop a meaningful filler."
            }
            TargetLanguage::Japanese => {
                "Use natural conversational Japanese with appropriate register, \
                 particles, and sentence endings (ね、よ、な、じゃん). Keep Chinese \
                 and English fillers as natural Japanese equivalents; never drop a \
                 meaningful filler."
            }
        };
        let source_guidance = source_guidance(source_language, target_language);
        format!(
            "Natural spoken dialogue for watching TV dramas and films. {language_guidance} \
             {source_guidance} Preserve the speaker's tone, emotion, and implied subjects \
             from context. The translation memory (tm_list) is the ongoing dialogue: \
             each pair is one finished subtitle line and its translation. Use it as \
             context — keep names, pronouns, and implied subjects consistent with \
             earlier lines, keep the same speaker's tone and register, and resolve \
             ambiguous or truncated phrases from what came before. Never repeat or \
             re-translate anything already in the memory; translate only the new input. \
             Render polite or formal Japanese (です/ます、敬語) as naturally courteous \
             Chinese, never stiff or literal. Avoid literal, explanatory, or translation-like \
             wording. Prefer short, complete sentences that fit one subtitle line. \
             Faithfully preserve every vocalization represented in the source, including \
             interjections, hesitation, breaths, gasps, moans, and cries, rendered \
             naturally in the target language. Do not sanitize, euphemize, censor, or \
             omit explicit dialogue or vocal sounds. Keep deliberate repetition for \
             emphasis; collapse only clear accidental ASR repetition. Output only the \
             translation text."
        )
    }

    /// Returns a glossary that forces common Japanese, English, and Korean
    /// fillers to keep a natural counterpart in the target language.
    pub fn filler_terms(
        source_language: SourceLanguage,
        target_language: TargetLanguage,
    ) -> Vec<QwenMTTerm> {
        let sources: &[SourceLanguage] = if source_language == SourceLanguage::Automatic {
            &[
                SourceLanguage::Japanese,
                SourceLanguage::English,
                SourceLanguage::Korean,
            ]
        } else {
            std::slice::from_ref(&source_language)
        };
        sources
            .iter()
            .flat_map(|source| filler_terms_for(source, target_language))
            .collect()
    }
}

fn filler_terms_for(source: &SourceLanguage, target: TargetLanguage) -> Vec<QwenMTTerm> {
    use SourceLanguage as S;
    use TargetLanguage as T;

    fn terms(pairs: &[(&str, &str)]) -> Vec<QwenMTTerm> {
        pairs
            .iter()
            .map(|(source, target)| QwenMTTerm::new(source, target))
            .collect()
    }

    match (source, target) {
        (S::Japanese, T::SimplifiedChinese) => terms(&[
            ("えっと", "那个"),
            ("えーと", "那个"),
            ("ええと", "那个"),
            ("あの", "那个"),
            ("あのー", "那个"),
            ("あのう", "那个"),
            ("うーん", "嗯"),
            ("う〜ん", "嗯"),
            ("あぁ", "啊"),
            ("ああ", "啊"),
            ("あっ", "啊"),
            ("えっ", "诶"),
            ("ふふ", "呵呵"),
            ("うふふ", "嘿嘿"),
            ("まあ", "嘛"),
            ("ねえ", "那个"),
            ("あら", "哎呀"),
            ("おや", "哎呀"),
            ("うわ", "哇"),
            ("きゃっ", "呀"),
            ("はぁ", "唉"),
            ("んー", "嗯"),
        ]),
        (S::English, T::SimplifiedChinese) => terms(&[
            ("um", "嗯"),
            ("uh", "呃"),
            ("oh", "哦"),
            ("hmm", "嗯"),
            ("ah", "啊"),
            ("wow", "哇"),
            ("hey", "喂"),
            ("yikes", "哎呀"),
        ]),
        (S::Korean, T::SimplifiedChinese) => terms(&[
            ("어", "嗯"),
            ("아", "啊"),
            ("음", "嗯"),
            ("어우", "哎哟"),
            ("헐", "不是吧"),
            ("야", "喂"),
        ]),
        (S::Japanese, T::English) => terms(&[
            ("えっと", "Um"),
            ("えーと", "Um"),
            ("ええと", "Um"),
            ("あの", "Um"),
            ("うーん", "Hmm"),
            ("う〜ん", "Hmm"),
            ("あぁ", "Ah"),
            ("あっ", "Oh"),
            ("えっ", "Huh"),
            ("ふふ", "Heh"),
            ("まあ", "Well"),
            ("ねえ", "Hey"),
            ("あら", "Oh"),
            ("うわ", "Wow"),
            ("きゃっ", "Eek"),
        ]),
        (S::English, T::Japanese) => terms(&[
            ("um", "うーん"),
            ("uh", "あの"),
            ("oh", "あっ"),
            ("hmm", "うーん"),
            ("ah", "ああ"),
            ("wow", "わあ"),
            ("hey", "ねえ"),
            ("yikes", "ひえっ"),
        ]),
        _ => Vec::new(),
    }
}

fn source_guidance(source: SourceLanguage, target: TargetLanguage) -> &'static str {
    use SourceLanguage as S;
    use TargetLanguage as T;

    match source {
        S::Japanese => match target {
            T::SimplifiedChinese => {
                "For every Japanese filler use its natural Chinese counterpart: \
                 えっと/あの→那个，うーん→嗯，あぁ→啊，まあ→嘛，ねえ→那个。 Sentence-final \
                 particles need a counterpart too: ね→呢/吧，よ→啊/哦，な→啊，じゃん→嘛。 \
                 Dropping a filler or particle is an error."
            }
            T::English => {
                "For every Japanese filler use its natural English counterpart: \
                 えっと/あの→Um，うーん→Hmm，あぁ→Ah，まあ→Well，ねえ→Hey。 Sentence-final \
                 particles need a counterpart too: ね→huh/right，よ→you know。 Dropping a \
                 filler or particle is an error."
            }
            _ => "",
        },
        S::English => match target {
            T::SimplifiedChinese => {
                "For every English filler use its natural Chinese counterpart: \
                 um→嗯，uh→呃，oh→哦，hmm→嗯，ah→啊，wow→哇。 Dropping a filler is an error."
            }
            T::Japanese => {
                "For every English filler use its natural Japanese counterpart: \
                 um→うーん，uh→あの，oh→あっ，hmm→うーん，wow→わあ。 Dropping a filler is an error."
            }
            _ => "",
        },
        S::Korean => {
            if target == T::SimplifiedChinese {
                "For every Korean filler use its natural Chinese counterpart: 어→嗯，아→啊，음→嗯。 Dropping a filler is an error."
            } else {
                ""
            }
        }
        S::Chinese | S::Automatic => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(text: &str, source_language: SourceLanguage) -> Value {
        QwenMTRequestEncoder::request(
            text,
            source_language,
            TargetLanguage::SimplifiedChinese,
            QwenMTModel::Lite,
            false,
            None,
            &[],
            &[],
        )
        .unwrap()
    }

    #[test]
    fn endpoint_builds_the_workspace_chat_completions_url() {
        let endpoint = QwenMTEndpoint::new("ws-abc123").unwrap();
        assert_eq!(
            endpoint.url.as_str(),
            "https://ws-abc123.cn-beijing.maas.aliyuncs.com/compatible-mode/v1/chat/completions"
        );
    }

    #[test]
    fn request_selects_lite_and_full_language_names() {
        let json = request("今日は晴れです。", SourceLanguage::Japanese);
        let messages = json["messages"].as_array().unwrap();
        let options = &json["translation_options"];

        assert_eq!(json["model"], "qwen-mt-lite");
        assert_eq!(json["stream"], false);
        assert_eq!(messages[0]["role"], "user");
        assert_eq!(messages[0]["content"], "今日は晴れです。");
        assert_eq!(options["source_lang"], "Japanese");
        assert_eq!(options["target_lang"], "Chinese");
    }

    #[test]
    fn request_can_enable_incremental_streaming() {
        let json = QwenMTRequestEncoder::request(
            "今日は晴れです。",
            SourceLanguage::Japanese,
            TargetLanguage::SimplifiedChinese,
            QwenMTModel::Lite,
            true,
            None,
            &[],
            &[],
        )
        .unwrap();
        assert_eq!(json["stream"], true);
    }

    #[test]
    fn request_selects_an_explicit_target_language() {
        let json = QwenMTRequestEncoder::request(
            "今日は晴れです。",
            SourceLanguage::Japanese,
            TargetLanguage::English,
            QwenMTModel::Lite,
            false,
            None,
            &[],
            &[],
        )
        .unwrap();
        assert_eq!(json["translation_options"]["target_lang"], "English");
    }

    #[test]
    fn request_can_select_the_highest_quality_plus_model() {
        let json = QwenMTRequestEncoder::request(
            "今日はいい天気ですね。",
            SourceLanguage::Japanese,
            TargetLanguage::SimplifiedChinese,
            QwenMTModel::Plus,
            false,
            None,
            &[],
            &[],
        )
        .unwrap();
        assert_eq!(json["model"], "qwen-mt-plus");
    }

    #[test]
    fn automatic_source_uses_auto() {
        let json = request("Hello, world.", SourceLanguage::Automatic);
        assert_eq!(json["translation_options"]["source_lang"], "auto");
    }

    #[test]
    fn spoken_dialogue_guidance_preserves_vocal_sounds() {
        let guidance = QwenMTDomainHint::spoken_dialogue(
            SourceLanguage::Japanese,
            TargetLanguage::SimplifiedChinese,
        );

        assert!(guidance.contains("gasps, moans, and cries"));
        assert!(guidance.contains("嗯、啊、呢、吧、嘛"));
        assert!(guidance.contains("えっと"));
        assert!(guidance.contains("polite or formal Japanese"));
        assert!(guidance.contains("Output only the translation text"));
        assert!(guidance.contains("tm_list"));
        assert!(guidance.contains("ongoing dialogue"));
        assert!(guidance.contains("translate only the new input"));
        assert!(guidance.contains("Do not sanitize, euphemize, censor, or omit"));
        assert!(!guidance.contains("do not mechanically translate every filler"));
    }

    #[test]
    fn filler_glossary_pins_japanese_tone_words_for_chinese() {
        let terms = QwenMTDomainHint::filler_terms(
            SourceLanguage::Japanese,
            TargetLanguage::SimplifiedChinese,
        );
        assert!(terms.contains(&QwenMTTerm::new("えっと", "那个")));
        assert!(terms.contains(&QwenMTTerm::new("うーん", "嗯")));
        assert!(terms.contains(&QwenMTTerm::new("あぁ", "啊")));
        assert!(terms.contains(&QwenMTTerm::new("まあ", "嘛")));
    }

    #[test]
    fn filler_glossary_combines_languages_for_automatic_source() {
        let terms = QwenMTDomainHint::filler_terms(
            SourceLanguage::Automatic,
            TargetLanguage::SimplifiedChinese,
        );
        assert!(terms.contains(&QwenMTTerm::new("えっと", "那个")));
        assert!(terms.contains(&QwenMTTerm::new("um", "嗯")));
    }

    #[test]
    fn request_encodes_the_filler_glossary_in_translation_options() {
        let terms = QwenMTDomainHint::filler_terms(
            SourceLanguage::Japanese,
            TargetLanguage::SimplifiedChinese,
        );
        let json = QwenMTRequestEncoder::request(
            "えっと、うーん、あぁ。",
            SourceLanguage::Japanese,
            TargetLanguage::SimplifiedChinese,
            QwenMTModel::Flash,
            true,
            None,
            &terms,
            &[],
        )
        .unwrap();
        let encoded_terms = json["translation_options"]["terms"].as_array().unwrap();

        assert_eq!(encoded_terms.len(), terms.len());
        assert_eq!(encoded_terms[0]["source"], "えっと");
        assert_eq!(encoded_terms[0]["target"], "那个");
    }

    #[test]
    fn request_omits_terms_when_none_are_provided() {
        let json = request("今日は晴れです。", SourceLanguage::Japanese);
        assert!(json["translation_options"].get("terms").is_none());
    }

    #[test]
    fn request_includes_bounded_translation_memory_pairs() {
        let json = QwenMTRequestEncoder::request(
            "そうなんですね。",
            SourceLanguage::Japanese,
            TargetLanguage::SimplifiedChinese,
            QwenMTModel::Flash,
            false,
            None,
            &[],
            &[QwenMTMemoryPair::new("今日は晴れです。", "今天天气很好。")],
        )
        .unwrap();
        let memory = json["translation_options"]["tm_list"].as_array().unwrap();
        assert_eq!(memory[0]["source"], "今日は晴れです。");
        assert_eq!(memory[0]["target"], "今天天气很好。");
    }

    #[test]
    fn asr_language_reports_resolve_to_explicit_qwen_mt_languages() {
        assert_eq!(
            SourceLanguage::from_detected(Some("ja-JP")),
            Some(SourceLanguage::Japanese)
        );
        assert_eq!(
            SourceLanguage::from_detected(Some("English")),
            Some(SourceLanguage::English)
        );
        assert_eq!(
            SourceLanguage::from_detected(Some("ko")),
            Some(SourceLanguage::Korean)
        );
        assert_eq!(
            SourceLanguage::from_detected(Some("zh-CN")),
            Some(SourceLanguage::Chinese)
        );
        assert_eq!(SourceLanguage::from_detected(Some("unknown")), None);
    }

    #[test]
    fn response_decodes_and_trims_translated_content() {
        let translation = QwenMTResponseDecoder::decode(
            r#"{"choices":[{"message":{"role":"assistant","content":"  今天天气晴朗。  "}}]}"#,
        )
        .unwrap();
        assert_eq!(translation, "今天天气晴朗。");
    }

    #[test]
    fn response_requires_translated_content() {
        assert!(matches!(
            QwenMTResponseDecoder::decode(r#"{"choices":[]}"#),
            Err(QwenMTProtocolError::MissingTranslation)
        ));
    }

    #[test]
    fn stream_chunk_decodes_incremental_content() {
        let content = QwenMTStreamDecoder::decode_chunk(
            r#"{"choices":[{"delta":{"role":"assistant","content":"今天"}}]}"#,
        )
        .unwrap();
        let terminal = QwenMTStreamDecoder::decode_chunk(r#"{"choices":[]}"#).unwrap();
        assert_eq!(content.as_deref(), Some("今天"));
        assert_eq!(terminal, None);
    }

    #[test]
    fn timeout_has_a_useful_error_message() {
        assert_eq!(
            QwenMTClientError::RequestTimedOut.to_string(),
            "Qwen-MT took too long to respond."
        );
    }

    #[test]
    fn diagnostics_retain_status_without_response_content() {
        assert_eq!(
            QwenMTClientError::RequestFailed {
                status_code: 429,
                message: "sensitive response detail".into()
            }
            .diagnostic_label(),
            "QwenMTClientError.requestFailed(status=429)"
        );
    }

    #[test]
    fn retry_policy_backs_off_only_for_transient_failures() {
        assert_eq!(
            QwenMTRetryPolicy::delay(&QwenMTClientError::RequestTimedOut, 1),
            Some(Duration::from_millis(600))
        );
        assert_eq!(
            QwenMTRetryPolicy::delay(
                &QwenMTClientError::RequestFailed {
                    status_code: 429,
                    message: "busy".into()
                },
                3
            ),
            Some(Duration::from_millis(2_400))
        );
        assert_eq!(
            QwenMTRetryPolicy::delay(
                &QwenMTClientError::RequestFailed {
                    status_code: 503,
                    message: "down".into()
                },
                8
            ),
            Some(Duration::from_secs(8))
        );
        assert_eq!(
            QwenMTRetryPolicy::delay(
                &QwenMTClientError::RequestFailed {
                    status_code: 401,
                    message: "bad key".into()
                },
                1
            ),
            None
        );
        assert_eq!(
            QwenMTRetryPolicy::delay(
                &QwenMTClientError::RequestFailed {
                    status_code: 400,
                    message: "bad request".into()
                },
                1
            ),
            None
        );
    }
}

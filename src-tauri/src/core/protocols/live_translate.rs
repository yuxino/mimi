//! Live-translate and realtime-ASR WebSocket protocols for DashScope's shared
//! endpoint. Authentication uses a Bearer API key; the URL has no Workspace
//! ID component.

use crate::core::models::{SourceLanguage, TargetLanguage};
use base64::Engine;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LiveTranslateProtocolError {
    #[error("The live translation endpoint could not be created.")]
    InvalidEndpoint,
    #[error("The live translation service returned invalid JSON.")]
    InvalidJSON,
    #[error("The live translation event is missing its type.")]
    MissingEventType,
}

/// DashScope unified realtime WebSocket endpoint. The old MaaS host put the
/// workspace id in the URL (`{workspace}.cn-beijing.maas.aliyuncs.com`);
/// the unified host authenticates with `Authorization: Bearer <key>` alone.
pub const DASHSCOPE_REALTIME_WS: &str = "wss://dashscope.aliyuncs.com/api-ws/v1/realtime";

fn realtime_url(model: &str) -> Result<url::Url, LiveTranslateProtocolError> {
    let raw = format!("{DASHSCOPE_REALTIME_WS}?model={model}");
    url::Url::parse(&raw).map_err(|_| LiveTranslateProtocolError::InvalidEndpoint)
}

/// `qwen3.5-livetranslate-flash-realtime` endpoint: realtime transcription with
/// simultaneous translation.
#[derive(Clone)]
pub struct LiveTranslateEndpoint {
    pub url: url::Url,
}

impl LiveTranslateEndpoint {
    pub const MODEL: &'static str = "qwen3.5-livetranslate-flash-realtime";

    pub fn new() -> Result<Self, LiveTranslateProtocolError> {
        Ok(Self {
            url: realtime_url(Self::MODEL)?,
        })
    }
}

fn next_event_id() -> String {
    format!("event_{}", Uuid::new_v4().simple())
}

/// Encoder for the live-translate (`qwen3.5-livetranslate-flash-realtime`)
/// session.
pub enum LiveTranslateRequestEncoder {}

impl LiveTranslateRequestEncoder {
    pub fn session_update(
        source_language: SourceLanguage,
        target_language: TargetLanguage,
        hotwords: &BTreeMap<String, String>,
        event_id: Option<&str>,
    ) -> Result<Value, LiveTranslateProtocolError> {
        let mut translation = json!({ "language": target_language.raw_value() });
        if !hotwords.is_empty() {
            translation["corpus"] = json!({ "phrases": hotwords });
        }
        // Automatic source detection omits the transcription language so the
        // server detects it per utterance (mirrors the original app's
        // RealtimeASRProtocol: `sourceLanguage == .automatic ? nil : rawValue`);
        // the per-event language field then drives the detected-language UI.
        let mut transcription = json!({ "model": "qwen3-asr-flash-realtime" });
        if source_language != SourceLanguage::Automatic {
            transcription["language"] = json!(source_language.raw_value());
        }
        Ok(json!({
            "event_id": event_id.unwrap_or(&next_event_id()),
            "type": "session.update",
            "session": {
                "modalities": ["text"],
                "sample_rate": 16_000,
                "input_audio_format": "pcm",
                "input_audio_transcription": transcription,
                "translation": translation
            }
        }))
    }

    pub fn audio_append(
        pcm_data: &[u8],
        event_id: Option<&str>,
    ) -> Result<Value, LiveTranslateProtocolError> {
        let audio = base64::engine::general_purpose::STANDARD.encode(pcm_data);
        Ok(json!({
            "event_id": event_id.unwrap_or(&next_event_id()),
            "type": "input_audio_buffer.append",
            "audio": audio
        }))
    }

    pub fn finish(event_id: Option<&str>) -> Result<Value, LiveTranslateProtocolError> {
        Ok(json!({
            "event_id": event_id.unwrap_or(&next_event_id()),
            "type": "session.finish"
        }))
    }
}

/// Server events shared by every pipeline in the app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiveTranslateServerEvent {
    SessionCreated,
    SessionUpdated,
    SourceDraft {
        text: String,
        language: Option<String>,
    },
    SourceFinal {
        text: String,
        language: Option<String>,
    },
    TranslationStarted,
    TranslationDraft(String),
    TranslationFinal(String),
    SubtitleFinalPair {
        source: String,
        language: Option<String>,
        translation: String,
    },
    SessionFinished,
    Error {
        code: String,
        message: String,
    },
    Ignored {
        kind: String,
    },
}

impl LiveTranslateServerEvent {
    pub fn decode(text: &str) -> Result<Self, LiveTranslateProtocolError> {
        let json: Value =
            serde_json::from_str(text).map_err(|_| LiveTranslateProtocolError::InvalidJSON)?;
        Self::decode_value(&json)
    }

    pub fn decode_value(json: &Value) -> Result<Self, LiveTranslateProtocolError> {
        let kind = json
            .get("type")
            .and_then(Value::as_str)
            .ok_or(LiveTranslateProtocolError::MissingEventType)?;

        match kind {
            "session.created" => Ok(Self::SessionCreated),
            "session.updated" => Ok(Self::SessionUpdated),
            "session.finished" => Ok(Self::SessionFinished),

            "conversation.item.input_audio_transcription.text" => Ok(Self::SourceDraft {
                text: combined_text(json),
                language: json
                    .get("language")
                    .and_then(Value::as_str)
                    .map(String::from),
            }),

            "conversation.item.input_audio_transcription.completed" => Ok(Self::SourceFinal {
                text: json
                    .get("transcript")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .trim()
                    .to_string(),
                language: json
                    .get("language")
                    .and_then(Value::as_str)
                    .map(String::from),
            }),

            "response.text.text" | "response.audio_transcript.text" => {
                Ok(Self::TranslationDraft(combined_text(json)))
            }

            "response.text.done" => Ok(Self::TranslationFinal(
                json.get("text")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .trim()
                    .to_string(),
            )),

            "response.audio_transcript.done" => Ok(Self::TranslationFinal(
                json.get("transcript")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .trim()
                    .to_string(),
            )),

            "error" => {
                let error = json.get("error");
                Ok(Self::Error {
                    code: error
                        .and_then(|e| e.get("code"))
                        .and_then(Value::as_str)
                        .unwrap_or("unknown_error")
                        .to_string(),
                    message: error
                        .and_then(|e| e.get("message"))
                        .and_then(Value::as_str)
                        .unwrap_or("Alibaba Cloud returned an unknown error.")
                        .to_string(),
                })
            }

            other => Ok(Self::Ignored {
                kind: other.to_string(),
            }),
        }
    }
}

/// Confirmed `text` plus tentative `stash`, trimmed — the server's combined
/// preview representation.
fn combined_text(json: &Value) -> String {
    let confirmed = json.get("text").and_then(Value::as_str).unwrap_or("");
    let tentative = json.get("stash").and_then(Value::as_str).unwrap_or("");
    format!("{confirmed}{tentative}").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_builds_the_unified_realtime_url() {
        let endpoint = LiveTranslateEndpoint::new().unwrap();
        assert_eq!(
            endpoint.url.as_str(),
            "wss://dashscope.aliyuncs.com/api-ws/v1/realtime?model=qwen3.5-livetranslate-flash-realtime"
        );
    }

    #[test]
    fn session_update_requests_text_only_chinese_translation_and_source_transcript() {
        let data = LiveTranslateRequestEncoder::session_update(
            SourceLanguage::Japanese,
            TargetLanguage::SimplifiedChinese,
            &BTreeMap::new(),
            Some("event-session"),
        )
        .unwrap();
        let session = &data["session"];
        let transcription = &session["input_audio_transcription"];
        let translation = &session["translation"];

        assert_eq!(data["type"], "session.update");
        assert_eq!(session["modalities"], json!(["text"]));
        assert_eq!(session["sample_rate"], 16_000);
        assert_eq!(session["input_audio_format"], "pcm");
        assert_eq!(transcription["model"], "qwen3-asr-flash-realtime");
        assert_eq!(transcription["language"], "ja");
        assert_eq!(translation["language"], "zh");
    }

    #[test]
    fn session_update_includes_hotword_corpus_when_present() {
        let hotwords = BTreeMap::from([("mimi".to_string(), "耳".to_string())]);
        let data = LiveTranslateRequestEncoder::session_update(
            SourceLanguage::Japanese,
            TargetLanguage::SimplifiedChinese,
            &hotwords,
            Some("event-hotwords"),
        )
        .unwrap();
        assert_eq!(
            data["session"]["translation"]["corpus"]["phrases"]["mimi"],
            "耳"
        );
    }

    #[test]
    fn audio_append_base64_encodes_pcm_bytes() {
        let data =
            LiveTranslateRequestEncoder::audio_append(&[0x00, 0x7F, 0xFF], Some("event-audio"))
                .unwrap();
        assert_eq!(data["type"], "input_audio_buffer.append");
        assert_eq!(data["audio"], "AH//");
    }

    #[test]
    fn session_update_selects_an_explicit_target_language() {
        let data = LiveTranslateRequestEncoder::session_update(
            SourceLanguage::English,
            TargetLanguage::Japanese,
            &BTreeMap::new(),
            Some("event-target"),
        )
        .unwrap();
        assert_eq!(data["session"]["translation"]["language"], "ja");
    }

    #[test]
    fn finish_event_uses_the_documented_type() {
        let json = LiveTranslateRequestEncoder::finish(Some("event-finish")).unwrap();
        assert_eq!(json["type"], "session.finish");
    }

    #[test]
    fn source_preview_combines_confirmed_and_tentative_text() {
        let event = LiveTranslateServerEvent::decode(
            r#"{"type":"conversation.item.input_audio_transcription.text","text":"Hello","stash":" world","language":"en"}"#,
        )
        .unwrap();
        assert_eq!(
            event,
            LiveTranslateServerEvent::SourceDraft {
                text: "Hello world".into(),
                language: Some("en".into())
            }
        );
    }

    #[test]
    fn asr_preview_combines_confirmed_text_and_stash() {
        let event = LiveTranslateServerEvent::decode(
            r#"{"type":"conversation.item.input_audio_transcription.text","text":"今日は","stash":"晴れです","language":"ja"}"#,
        )
        .unwrap();
        assert_eq!(
            event,
            LiveTranslateServerEvent::SourceDraft {
                text: "今日は晴れです".into(),
                language: Some("ja".into())
            }
        );
    }

    #[test]
    fn source_completion_decodes_the_final_transcript() {
        let event = LiveTranslateServerEvent::decode(
            r#"{"type":"conversation.item.input_audio_transcription.completed","transcript":"Hello world.","language":"en"}"#,
        )
        .unwrap();
        assert_eq!(
            event,
            LiveTranslateServerEvent::SourceFinal {
                text: "Hello world.".into(),
                language: Some("en".into())
            }
        );
    }

    #[test]
    fn translation_preview_and_completion_decode() {
        let preview = LiveTranslateServerEvent::decode(
            r#"{"type":"response.text.text","text":"你好","stash":"，世界"}"#,
        )
        .unwrap();
        let final_event = LiveTranslateServerEvent::decode(
            r#"{"type":"response.text.done","text":"你好，世界。"}"#,
        )
        .unwrap();

        assert_eq!(
            preview,
            LiveTranslateServerEvent::TranslationDraft("你好，世界".into())
        );
        assert_eq!(
            final_event,
            LiveTranslateServerEvent::TranslationFinal("你好，世界。".into())
        );
    }

    #[test]
    fn session_and_error_events_decode() {
        let updated = LiveTranslateServerEvent::decode(r#"{"type":"session.updated"}"#).unwrap();
        let finished = LiveTranslateServerEvent::decode(r#"{"type":"session.finished"}"#).unwrap();
        let failure = LiveTranslateServerEvent::decode(
            r#"{"type":"error","error":{"code":"invalid_value","message":"Bad language"}}"#,
        )
        .unwrap();

        assert_eq!(updated, LiveTranslateServerEvent::SessionUpdated);
        assert_eq!(finished, LiveTranslateServerEvent::SessionFinished);
        assert_eq!(
            failure,
            LiveTranslateServerEvent::Error {
                code: "invalid_value".into(),
                message: "Bad language".into()
            }
        );
    }

    #[test]
    fn unknown_events_are_ignored_without_failing_the_receive_loop() {
        let event = LiveTranslateServerEvent::decode(r#"{"type":"response.created"}"#).unwrap();
        assert_eq!(
            event,
            LiveTranslateServerEvent::Ignored {
                kind: "response.created".into()
            }
        );
    }

    #[test]
    fn malformed_json_and_missing_type_fail_cleanly() {
        assert!(matches!(
            LiveTranslateServerEvent::decode("not json"),
            Err(LiveTranslateProtocolError::InvalidJSON)
        ));
        assert!(matches!(
            LiveTranslateServerEvent::decode(r#"{"event_id":"x"}"#),
            Err(LiveTranslateProtocolError::MissingEventType)
        ));
    }
}
